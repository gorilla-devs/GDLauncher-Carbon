//! Verified installed inventory for linked copies that predate packinfo.
//! Resolving never writes live payload or setup. The caller retains the existing
//! instance gate and runtime activity through the final no-clobber publication.
//!
//! Best effort: ownership is only claimed for a file proven to hold the bytes
//! the installed version declares for it. A declared file that can't be
//! checked stays unrecorded, so a version change keeps it, as 2.0.40 kept
//! every file; when the installed version can't be read at all, nothing is
//! recorded (see [`Baseline`]).
use super::packinfo;
use crate::domain::{
    instance::info::Modpack,
    vtask::{DependencyPlatform, PreviousPackFilesReason, TaskWarning},
};
use anyhow::{Context, ensure};
use carbon_net::Checksum;
use sha2::{Digest, Sha512};
use std::{
    collections::HashMap,
    future::Future,
    io::{Read, Write},
    path::Path,
};
use tokio_util::sync::CancellationToken;

pub(super) type Inventory = HashMap<String, Checksum>;

/// What the platform still describes of the installed version.
#[derive(Debug, Default)]
pub(super) struct Resolved {
    /// Each declared file, keyed by its path as the archive or index spells
    /// it, with its digest.
    pub(super) files: Inventory,
    /// Declared files left out of `files`: the platform no longer describes
    /// them, or describes them without a usable digest or name.
    pub(super) unresolved: usize,
}

impl From<Inventory> for Resolved {
    fn from(files: Inventory) -> Self {
        Self {
            files,
            unresolved: 0,
        }
    }
}

/// What a version change can rely on about the files the installed version
/// put in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Baseline {
    /// `packinfo.json` records them: an earlier record, or one just written
    /// from the installed version's whole inventory.
    Recorded,
    /// One was just written, but some declared files could not be checked.
    /// They stay unrecorded, so the change keeps them.
    Partial,
    /// The installed version could not be read for a reason retrying won't
    /// fix (`gone`: the platform no longer serves it), or its record could not
    /// be written. Nothing is recorded, so the change plans without a record
    /// and keeps every file in place, as 2.0.40 did.
    Unavailable { gone: bool },
}

impl Baseline {
    /// What the version change's task tells the user once it succeeds: which
    /// of the replaced version's files it kept because it could not check
    /// them, if any.
    pub(super) fn warning(self, platform: DependencyPlatform) -> Option<TaskWarning> {
        let reason = match self {
            Self::Recorded => return None,
            Self::Partial => PreviousPackFilesReason::SomeFilesUnchecked,
            Self::Unavailable { gone: true } => PreviousPackFilesReason::VersionUnavailable,
            Self::Unavailable { gone: false } => PreviousPackFilesReason::VersionUnreadable,
        };
        Some(TaskWarning::PreviousPackFilesKept { platform, reason })
    }
}

/// Why a version change is refused before its setup starts. Typed so the
/// version modal words it in the player's language; `Display` is the English
/// fallback.
#[derive(Debug, thiserror::Error)]
pub enum ChangeVersionRefusal {
    /// What the installed version added could not be checked, and retrying
    /// can fix that. The change needs the same services to download the new
    /// version, so there is nothing to gain by going ahead without them.
    #[error(
        "Could not reach {} to check which files the installed version of this modpack added. Check your connection, or try again in a few minutes if it is working.",
        service_name(*.platform)
    )]
    Unreachable {
        /// The platform that failed; `None` for the launcher's own services.
        platform: Option<DependencyPlatform>,
        #[source]
        cause: anyhow::Error,
    },
    /// The requested version is the installed one, which the platform no
    /// longer serves, so a setup for it could only fail.
    #[error("This version is no longer available. Pick another version.")]
    VersionGone,
    /// The check was stopped, by the player or by the launcher quitting.
    /// Nothing changed, so there is nothing to tell the player.
    #[error("The version change was cancelled")]
    Cancelled,
}

impl crate::error::FeErrorCode for ChangeVersionRefusal {
    fn error_code(&self) -> &'static str {
        match self {
            Self::Unreachable { .. } => "MODPACK_CHECK_UNREACHABLE",
            Self::VersionGone => "MODPACK_VERSION_GONE",
            Self::Cancelled => "MODPACK_CHECK_CANCELLED",
        }
    }

    fn error_data(&self) -> Option<serde_json::Value> {
        match self {
            Self::Unreachable {
                platform: Some(platform),
                ..
            } => Some(serde_json::json!({ "platform": platform_name(*platform) })),
            _ => None,
        }
    }
}

/// A launcher service, not the platform, could not answer.
#[derive(Debug, thiserror::Error)]
#[error("Could not read the launcher's list of Minecraft versions")]
struct LauncherService(#[source] anyhow::Error);

/// Where `pack` is published.
pub(super) fn platform(pack: &Modpack) -> DependencyPlatform {
    match pack {
        Modpack::Curseforge(_) => DependencyPlatform::Curseforge,
        Modpack::Modrinth(_) => DependencyPlatform::Modrinth,
    }
}

fn platform_name(platform: DependencyPlatform) -> &'static str {
    match platform {
        DependencyPlatform::Curseforge => "CurseForge",
        DependencyPlatform::Modrinth => "Modrinth",
    }
}

fn service_name(platform: Option<DependencyPlatform>) -> &'static str {
    platform.map_or("the online services needed", platform_name)
}

/// Records what the installed version put in place before a version change
/// of a linked pack without `packinfo.json` (an import, or a 2.0.40 copy).
/// Refuses only when `platform` can't be reached, which retrying fixes. Stops
/// as soon as `cancel` fires, with [`ChangeVersionRefusal::Cancelled`] and
/// nothing recorded.
pub(super) async fn ensure_cancellable(
    root: &Path,
    platform: DependencyPlatform,
    cancel: &CancellationToken,
    resolve: impl Future<Output = anyhow::Result<Resolved>>,
) -> anyhow::Result<Baseline> {
    let record = root.join("packinfo.json");
    match tokio::fs::read_to_string(&record).await {
        Ok(text) => return existing_record(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    ensure!(
        !root.join(".setup").try_exists()?,
        "Complete the pending setup before establishing imported ownership"
    );
    // Tests stand in for the provider here, so nothing reaches the network.
    #[cfg(test)]
    let resolve = async move {
        match inventory_fixture::get(root) {
            Some(fixture) => fixture.resolve().await,
            None => resolve.await,
        }
    };
    let resolved = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(ChangeVersionRefusal::Cancelled.into()),
        resolved = resolve => resolved,
    };
    let resolved = match resolved {
        Ok(resolved) => resolved,
        Err(error) if cancelled(&error) => return Err(ChangeVersionRefusal::Cancelled.into()),
        Err(error) if unreachable(&error) => {
            let platform =
                (!error.chain().any(|cause| cause.is::<LauncherService>())).then_some(platform);
            return Err(ChangeVersionRefusal::Unreachable {
                platform,
                cause: error,
            }
            .into());
        }
        Err(error) => {
            let gone = gone(&error);
            tracing::warn!(
                ?error,
                gone,
                "the installed pack version can't be read; the version change keeps every file in place"
            );
            return Ok(Baseline::Unavailable { gone });
        }
    };
    let root = root.to_path_buf();
    let cancel = cancel.clone();
    tokio::task::spawn_blocking(move || record_baseline(&root, resolved, &cancel)).await?
}

/// [`ensure_cancellable`] for tests that never cancel.
#[cfg(test)]
pub(super) async fn ensure(
    root: &Path,
    platform: DependencyPlatform,
    resolve: impl Future<Output = anyhow::Result<Resolved>>,
) -> anyhow::Result<Baseline> {
    ensure_cancellable(root, platform, &CancellationToken::new(), resolve).await
}

/// What a `packinfo.json` already in place tells the change.
fn existing_record(text: &str) -> anyhow::Result<Baseline> {
    packinfo::parse_packinfo(text)?;
    Ok(if left_files_unchecked(text) {
        Baseline::Partial
    } else {
        Baseline::Recorded
    })
}

/// Whether `record` is a baseline [`record_baseline`] wrote that left declared
/// files unchecked. A version change that finishes replaces it with a record
/// of its own, so this holds until the first change goes through, however many
/// attempts that takes.
fn left_files_unchecked(record: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(record)
        .ok()
        .and_then(|record| record["verified_import_inventory"]["unchecked_files"].as_u64())
        .is_some_and(|unchecked| unchecked > 0)
}

fn record_baseline(
    root: &Path,
    resolved: Resolved,
    cancel: &CancellationToken,
) -> anyhow::Result<Baseline> {
    let declared = resolved.files.len() + resolved.unresolved;
    let mut unchecked = resolved.unresolved;
    let mut files = HashMap::new();
    for (key, declared) in owned_keys(resolved.files) {
        // Hashing a large pack takes a while, so a cancel is honoured here too.
        if cancel.is_cancelled() {
            return Err(ChangeVersionRefusal::Cancelled.into());
        }
        // Validated before anything is looked up on disk. An unsafe or
        // malformed declaration owns nothing, so its path stays the user's.
        if let Err(error) = validate_key(&key).and_then(|()| validate_checksum(&declared.checksum))
        {
            tracing::warn!(?error, "installed inventory entry left unrecorded");
            unchecked += 1;
            continue;
        }
        if super::apply_plan::is_saves_path(&key) {
            continue;
        }
        match installed_hashes(root, &key, &declared) {
            Ok(Some(hashes)) => {
                files.insert(key, hashes);
            }
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(
                    ?error,
                    key,
                    "installed file can't be checked; left unrecorded"
                );
                unchecked += 1;
            }
        }
    }
    let info = packinfo::PackInfo { files };
    let text = packinfo::make_packinfo(info)?;
    // Records how much of the declared inventory was checked; historical
    // records need no marker.
    let mut value: serde_json::Value = serde_json::from_str(&text)?;
    value["verified_import_inventory"] = serde_json::json!({
        "schema": 1,
        "declared_files": declared,
        "unchecked_files": unchecked,
    });
    let text = serde_json::to_string_pretty(&value)?;
    // The record only adds cleanup, so a folder that won't take it (an
    // antivirus or sync client holding the file, say) doesn't stop the change.
    if let Err(error) = publish_record(root, text.as_bytes()) {
        // Another writer published one first: that is the record the change
        // reads, so it is what the change can rely on.
        if let Ok(text) = std::fs::read_to_string(root.join("packinfo.json")) {
            return existing_record(&text);
        }
        tracing::warn!(
            ?error,
            "the installed inventory can't be recorded; the version change keeps every file in place"
        );
        return Ok(Baseline::Unavailable { gone: false });
    }
    #[cfg(unix)]
    if let Err(error) = std::fs::File::open(root).and_then(|folder| folder.sync_all()) {
        tracing::warn!(?error, "the installed inventory's folder can't be synced");
    }
    Ok(if unchecked == 0 {
        Baseline::Recorded
    } else {
        Baseline::Partial
    })
}

fn publish_record(root: &Path, record: &[u8]) -> std::io::Result<()> {
    let mut temp = crate::util::publish::temp_in(root)?;
    temp.write_all(record)?;
    temp.as_file().sync_all()?;
    crate::util::publish::persist_noclobber(temp, &root.join("packinfo.json"))
}

/// A declared file under its [`owned_keys`] key.
struct OwnedFile {
    checksum: Checksum,
    /// The pack ships this file only under its `.disabled` spelling.
    only_disabled: bool,
}

/// Keys `files` the way [`packinfo::scan_dir`] keys an installed pack: one
/// trailing `.disabled` is stripped, so a file the pack ships disabled is
/// owned under its enabled name (see [`installed_hashes`] for the spellings
/// on disk that represent it). A pack that ships a file both enabled and
/// disabled installs the enabled one (see
/// `run::modpack::drop_shadowed_disabled_twins`), so that one's digest wins.
fn owned_keys(files: Inventory) -> HashMap<String, OwnedFile> {
    let mut owned = HashMap::with_capacity(files.len());
    for (key, checksum) in files {
        match key.strip_suffix(".disabled") {
            Some(bare) => {
                owned.entry(bare.to_owned()).or_insert(OwnedFile {
                    checksum,
                    only_disabled: true,
                });
            }
            None => {
                owned.insert(
                    key,
                    OwnedFile {
                        checksum,
                        only_disabled: false,
                    },
                );
            }
        }
    }
    owned
}

/// The hashes to record for `key`, when its one file on disk holds the bytes
/// `declared` says. A file the pack ships only disabled that sits enabled on
/// disk was enabled by the user, so the pack no longer speaks for it: it stays
/// unrecorded and an update keeps it, enabled, as 2.0.40 did. A file the pack
/// ships enabled may sit under either spelling.
fn installed_hashes(
    root: &Path,
    key: &str,
    declared: &OwnedFile,
) -> anyhow::Result<Option<packinfo::FileHashes>> {
    let path = root.join("instance").join(&key[1..]);
    let disabled = path.with_file_name(format!(
        "{}.disabled",
        path.file_name().unwrap().to_string_lossy()
    ));
    let bare = path.try_exists()?;
    let twin = disabled.try_exists()?;
    let selected = match (bare, twin) {
        // No unique physical representation.
        (true, true) | (false, false) => return Ok(None),
        (true, false) if declared.only_disabled => return Ok(None),
        (true, false) => path,
        (false, true) => disabled,
    };
    matching_hashes(&selected, &declared.checksum)
}
/// The platform no longer serves the installed version, or its archive.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct Gone(&'static str);

/// Whether `error` says the platform no longer serves what was asked for
/// (404/410, or a version without a downloadable archive).
fn gone(error: &anyhow::Error) -> bool {
    super::super::installer::dependency::not_found(error)
        || error.chain().any(|cause| {
            cause.is::<Gone>()
                || matches!(
                    cause.downcast_ref::<carbon_net::DownloadError>(),
                    Some(carbon_net::DownloadError::Non200StatusCode(_, 404 | 410))
                )
        })
}

/// Whether `error` says the check was cancelled, not that it failed.
fn cancelled(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        matches!(
            cause.downcast_ref::<carbon_net::DownloadError>(),
            Some(carbon_net::DownloadError::Cancelled)
        )
    })
}

/// Whether `error` is a connection or server problem that retrying can fix,
/// as opposed to an answer that the version or a file is gone or unusable, or
/// a request that could never work (a malformed address, a body that arrived
/// whole and is not what was asked for). Those are permanent, so refusing
/// would never clear.
fn unreachable(error: &anyhow::Error) -> bool {
    error.chain().any(cause_unreachable)
}

fn cause_unreachable(cause: &(dyn std::error::Error + 'static)) -> bool {
    use crate::error::request::{MalformedResponseDetails, RequestError, RequestErrorDetails};
    use carbon_net::DownloadError;
    if let Some(request) = cause.downcast_ref::<RequestError>() {
        match &request.error {
            RequestErrorDetails::Timeout
            | RequestErrorDetails::ConnectionFailed
            | RequestErrorDetails::Unknown(_) => true,
            RequestErrorDetails::UnexpectedStatus { status, .. } => retryable(status.as_u16()),
            // The platform clients read a body as text and parse it apart, so
            // this is the read failing: a connection lost mid-body, which
            // reqwest reports as a decode error (see `reqwest_unreachable`).
            RequestErrorDetails::MalformedResponse {
                details: MalformedResponseDetails::UnknownDecodeError,
            } => true,
            RequestErrorDetails::MalformedResponse {
                details: MalformedResponseDetails::JsonDecodeError { .. },
            }
            | RequestErrorDetails::UnusableRequest(_) => false,
        }
    } else if let Some(error) = cause.downcast_ref::<reqwest_middleware::Error>() {
        match error {
            reqwest_middleware::Error::Reqwest(error) => reqwest_unreachable(error),
            // Retries ran out, which only transient failures do, unless the
            // middleware names a cause of its own.
            reqwest_middleware::Error::Middleware(error) => {
                let requests: Vec<_> = error
                    .chain()
                    .filter(|cause| {
                        cause.is::<reqwest::Error>() || cause.is::<reqwest_middleware::Error>()
                    })
                    .collect();
                requests.is_empty() || requests.into_iter().any(cause_unreachable)
            }
        }
    } else if let Some(error) = cause.downcast_ref::<reqwest::Error>() {
        reqwest_unreachable(error)
    } else if let Some(error) = cause.downcast_ref::<DownloadError>() {
        match error {
            DownloadError::NetworkError(error) => reqwest_unreachable(error),
            DownloadError::NetworkError2(error) => cause_unreachable(error),
            DownloadError::GenericDownload(_) => true,
            DownloadError::Non200StatusCode(_, status) => retryable(*status),
            _ => false,
        }
    } else {
        false
    }
}

fn retryable(status: u16) -> bool {
    status >= 500 || status == 408 || status == 429
}

fn reqwest_unreachable(error: &reqwest::Error) -> bool {
    use std::error::Error;
    if let Some(status) = error.status() {
        return retryable(status.as_u16());
    }
    // reqwest also reports a connection lost mid-body as a decode error. Only
    // a body that arrived whole and doesn't parse is the server's answer.
    if error.is_decode() {
        let mut cause = error.source();
        while let Some(current) = cause {
            if current.is::<serde_json::Error>() {
                return false;
            }
            cause = current.source();
        }
        return true;
    }
    error.is_connect() || error.is_timeout() || error.is_request() || error.is_body()
}

/// What [`ensure`] uses for one instance root instead of [`resolve`]'s
/// provider lookup, while the guard lives. Keyed by root like
/// `curseforge::preparation_fixture`, so parallel tests don't share it.
#[cfg(test)]
pub(crate) mod inventory_fixture {
    use super::{Inventory, Resolved};
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::{LazyLock, Mutex},
    };
    #[derive(Clone)]
    pub(crate) enum Fixture {
        Inventory(Inventory),
        /// The platform answers that the installed version is gone (404).
        Gone,
        /// The platform can't be reached.
        Unreachable,
        /// The platform never answers, like a long download.
        Hang,
    }
    impl Fixture {
        pub(super) async fn resolve(self) -> anyhow::Result<Resolved> {
            use crate::error::request::{RequestContext, RequestError, RequestErrorDetails};
            let error = match self {
                Self::Inventory(inventory) => return Ok(inventory.into()),
                Self::Hang => return std::future::pending().await,
                Self::Gone => RequestErrorDetails::UnexpectedStatus {
                    status: reqwest::StatusCode::NOT_FOUND,
                    details: None,
                },
                Self::Unreachable => RequestErrorDetails::ConnectionFailed,
            };
            Err(RequestError {
                context: RequestContext::none(),
                error,
            }
            .into())
        }
    }
    static VALUES: LazyLock<Mutex<HashMap<PathBuf, Fixture>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    pub(crate) struct Guard(PathBuf);
    impl Drop for Guard {
        fn drop(&mut self) {
            VALUES.lock().unwrap().remove(&self.0);
        }
    }
    pub(crate) fn set(root: PathBuf, inventory: Inventory) -> Guard {
        set_fixture(root, Fixture::Inventory(inventory))
    }
    pub(crate) fn set_fixture(root: PathBuf, fixture: Fixture) -> Guard {
        VALUES.lock().unwrap().insert(root.clone(), fixture);
        Guard(root)
    }
    pub(super) fn get(root: &Path) -> Option<Fixture> {
        VALUES.lock().unwrap().get(root).cloned()
    }
}

fn validate_key(key: &str) -> anyhow::Result<()> {
    ensure!(
        key.starts_with('/') && key.len() > 1 && !key.ends_with('/'),
        "Invalid installed inventory path {key}"
    );
    let path = Path::new(&key[1..]);
    ensure!(
        path.components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
            && !key.contains('\\')
            && !key.contains(':')
            && !key.contains("//"),
        "Unsafe installed inventory path {key}"
    );
    Ok(())
}
fn validate_checksum(checksum: &Checksum) -> anyhow::Result<()> {
    let (value, length) = match checksum {
        Checksum::Sha512(v) => (v, 128),
        Checksum::Sha256(v) => (v, 64),
        Checksum::Sha1(v) => (v, 40),
        Checksum::Md5(v) => (v, 32),
    };
    ensure!(
        value.len() == length && value.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid installed inventory digest"
    );
    Ok(())
}
fn matching_hashes(
    path: &Path,
    checksum: &Checksum,
) -> anyhow::Result<Option<packinfo::FileHashes>> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Reading installed source {}", path.display()))?;
    ensure!(
        file.metadata()?.is_file(),
        "Installed path {} is not a file",
        path.display()
    );
    let mut sha512 = Sha512::new();
    let mut md5 = md5::Md5::new();
    let mut sha1 = sha1::Sha1::new();
    let mut sha256 = sha2::Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        sha512.update(&buffer[..n]);
        md5.update(&buffer[..n]);
        sha1.update(&buffer[..n]);
        sha256.update(&buffer[..n]);
    }
    let sha512: [u8; 64] = sha512.finalize().into();
    let md5: [u8; 16] = md5.finalize().into();
    let matches = match checksum {
        Checksum::Sha512(v) => v.eq_ignore_ascii_case(&hex::encode(sha512)),
        Checksum::Md5(v) => v.eq_ignore_ascii_case(&hex::encode(md5)),
        Checksum::Sha1(v) => v.eq_ignore_ascii_case(&hex::encode(sha1.finalize())),
        Checksum::Sha256(v) => v.eq_ignore_ascii_case(&hex::encode(sha256.finalize())),
    };
    Ok(matches.then_some(packinfo::FileHashes { sha512, md5 }))
}

/// The installed version's declared files, from its archive and the
/// platform. A declared file the platform no longer describes is counted in
/// [`Resolved::unresolved`] instead of failing the whole inventory. `cancel`
/// stops the archive download.
pub(super) async fn resolve(
    app: &std::sync::Arc<crate::managers::AppInner>,
    pack: &Modpack,
    cancel: &CancellationToken,
) -> anyhow::Result<Resolved> {
    use carbon_platforms::{curseforge::filters::*, modrinth::search::VersionID};
    let archive = app
        .settings_manager()
        .runtime_path
        .get_temp()
        .maketmpfile()
        .await?;
    match pack {
        Modpack::Modrinth(pack) => {
            let version = app
                .modplatforms_manager()
                .modrinth
                .get_version(VersionID(pack.version_id.clone()))
                .await?;
            ensure!(
                version.id == pack.version_id && version.project_id == pack.project_id,
                "Installed Modrinth association changed"
            );
            let file = version
                .files
                .iter()
                .find(|f| f.primary)
                .or_else(|| version.files.first())
                .ok_or(Gone("Installed pack has no archive"))?;
            download_archive(
                &file.url,
                &archive,
                file.size as u64,
                Checksum::Sha512(file.hashes.sha512.clone()),
                cancel,
            )
            .await?;
            mr_archive(&archive, 64 * 1024 * 1024, 16 * 1024 * 1024 * 1024).await
        }
        Modpack::Curseforge(pack) => {
            let manager = &app.modplatforms_manager().curseforge;
            let file = manager
                .get_mod_file(ModFileParameters {
                    mod_id: pack.project_id as i32,
                    file_id: pack.file_id as i32,
                })
                .await?
                .data;
            ensure!(
                file.id == pack.file_id as i32 && file.mod_id == pack.project_id as i32,
                "Installed CurseForge association changed"
            );
            download_archive(
                file.download_url
                    .as_deref()
                    .ok_or(Gone("Installed pack archive URL is unavailable"))?,
                &archive,
                file.file_length as u64,
                cf_checksum(&file)?,
                cancel,
            )
            .await?;
            let path = archive.to_path_buf();
            let (manifest, overrides) =
                tokio::task::spawn_blocking(move || cf_archive(&path)).await??;
            let mut resolved = Resolved::from(overrides);
            if manifest.files.is_empty() {
                return Ok(resolved);
            }
            let files = manager
                .get_files(FilesParameters {
                    body: FilesParametersBody {
                        file_ids: manifest.files.iter().map(|f| f.file_id).collect(),
                    },
                })
                .await?
                .data;
            let mods = manager
                .get_mods(ModsParameters {
                    body: ModsParametersBody {
                        mod_ids: manifest.files.iter().map(|f| f.project_id).collect(),
                    },
                })
                .await?
                .data;
            let mc = app
                .minecraft_manager()
                .get_minecraft_manifest()
                .await
                .map_err(LauncherService)?;
            add_cf_manifest_files(&mut resolved, &manifest, &files, &mods, &mc);
            Ok(resolved)
        }
    }
}

/// Adds what a CurseForge manifest's entries install to `resolved`. An entry
/// the platform no longer describes, or describes with an unsafe name or no
/// digest, is counted as unresolved and the rest still count.
fn add_cf_manifest_files(
    resolved: &mut Resolved,
    manifest: &carbon_platforms::curseforge::manifest::Manifest,
    files: &[carbon_platforms::curseforge::File],
    mods: &[carbon_platforms::curseforge::Mod],
    mc: &daedalus::minecraft::VersionManifest,
) {
    use carbon_platforms::curseforge as cf;
    // Use the same class/path mapping as the normal installed producer.
    let virtual_root = carbon_rt_path::InstancePath::new(std::path::PathBuf::from("inventory"));
    for declared in &manifest.files {
        let entry = (|| {
            let file = files
                .iter()
                .find(|f| f.id == declared.file_id && f.mod_id == declared.project_id)
                .context("Installed pack file metadata is incomplete")?;
            let addon = mods
                .iter()
                .find(|m| m.id == declared.project_id)
                .context("Installed pack addon metadata is incomplete")?;
            let folder = addon
                .class_id
                .clone()
                .unwrap_or(cf::ClassId::Mods)
                .into_path(&virtual_root, manifest.minecraft.version.clone(), mc);
            ensure!(
                Path::new(&file.file_name).components().count() == 1
                    && !file.file_name.contains(['/', '\\', ':']),
                "Unsafe declared filename"
            );
            let path = folder.join(&file.file_name);
            let relative = path.strip_prefix(virtual_root.get_data_path())?;
            let key = format!("/{}", relative.to_string_lossy().replace('\\', "/"));
            anyhow::Ok((key, cf_checksum(file)?))
        })();
        match entry {
            Ok((key, checksum)) => {
                resolved.files.insert(key, checksum);
            }
            Err(error) => {
                tracing::warn!(
                    ?error,
                    project = declared.project_id,
                    file = declared.file_id,
                    "installed pack file can't be resolved; left unrecorded"
                );
                resolved.unresolved += 1;
            }
        }
    }
}
fn cf_checksum(file: &carbon_platforms::curseforge::File) -> anyhow::Result<Checksum> {
    use carbon_platforms::curseforge::HashAlgo;
    let checksum = file
        .hashes
        .iter()
        .find(|h| matches!(h.algo, HashAlgo::Sha1))
        .map(|h| Checksum::Sha1(h.value.clone()))
        .or_else(|| {
            file.hashes
                .iter()
                .find(|h| matches!(h.algo, HashAlgo::Md5))
                .map(|h| Checksum::Md5(h.value.clone()))
        })
        .context("Pinned CurseForge file has no verifiable digest")?;
    validate_checksum(&checksum)?;
    Ok(checksum)
}
async fn download_archive(
    url: &str,
    path: &Path,
    size: u64,
    checksum: Checksum,
    cancel: &CancellationToken,
) -> anyhow::Result<()> {
    validate_checksum(&checksum)?;
    carbon_net::download_multiple(
        &[carbon_net::Downloadable::new(url, path)
            .with_size(size)
            .with_checksum(Some(checksum))],
        carbon_net::DownloadOptions::builder()
            .concurrency(1)
            .cancel_token(cancel.clone())
            .build(),
    )
    .await?;
    Ok(())
}
/// The override files of a CurseForge pack archive, keyed by their paths as
/// the archive spells them; [`ensure`] validates each before it is used.
fn cf_archive(
    path: &Path,
) -> anyhow::Result<(carbon_platforms::curseforge::manifest::Manifest, Inventory)> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path)?)?;
    let manifest: carbon_platforms::curseforge::manifest::Manifest =
        serde_json::from_reader(archive.by_name("manifest.json")?)?;
    ensure!(
        manifest.manifest_type == "minecraftModpack" && manifest.manifest_version == 1,
        "Unsupported installed CurseForge inventory format"
    );
    ensure!(
        !manifest.overrides.is_empty(),
        "Missing installed overrides directory declaration"
    );
    let mut inventory = HashMap::new();
    let mut budget = 16u64 * 1024 * 1024 * 1024;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(enclosed) = entry.enclosed_name() else {
            continue;
        };
        let Ok(relative) = enclosed.strip_prefix(&manifest.overrides) else {
            continue;
        };
        if entry.is_dir()
            || crate::managers::minecraft::modrinth::is_symlink_mode(entry.unix_mode())
        {
            continue;
        }
        let key = format!("/{}", relative.to_string_lossy().replace('\\', "/"));
        let mut digest = Sha512::new();
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let n = entry.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            budget = budget
                .checked_sub(n as u64)
                .context("Installed override inventory exceeds its work budget")?;
            digest.update(&buffer[..n]);
        }
        inventory.insert(key, Checksum::Sha512(hex::encode(digest.finalize())));
    }
    Ok((manifest, inventory))
}

#[cfg(test)]
mod task13_tests {
    use super::*;
    use sha2::{Digest, Sha512};
    #[tokio::test]
    async fn task13_pinned_baseline_owns_only_pristine_declared_bytes() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("instance/mods");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(data.join("a-v1.jar"), b"old pack").unwrap();
        std::fs::write(data.join("edited.jar"), b"user edit").unwrap();
        std::fs::write(data.join("extra.jar"), b"user addition").unwrap();
        let inventory = HashMap::from([
            (
                "/mods/a-v1.jar".into(),
                Checksum::Sha512(hex::encode(Sha512::digest(b"old pack"))),
            ),
            (
                "/mods/edited.jar".into(),
                Checksum::Sha512(hex::encode(Sha512::digest(b"pristine"))),
            ),
        ]);
        assert_eq!(
            ensure(t.path(), MODRINTH, async { Ok(inventory.into()) })
                .await
                .unwrap(),
            Baseline::Recorded
        );
        let text = std::fs::read_to_string(t.path().join("packinfo.json"))
            .expect("version change must establish installed ownership");
        let info = packinfo::parse_packinfo(&text).unwrap();
        assert_eq!(info.files.len(), 1);
        assert!(info.files.contains_key("/mods/a-v1.jar"));
        assert_eq!(
            std::fs::read(data.join("edited.jar")).unwrap(),
            b"user edit"
        );
        assert_eq!(
            std::fs::read(data.join("extra.jar")).unwrap(),
            b"user addition"
        );
        assert!(!t.path().join(".setup").exists());
    }
    /// An installed version that can't be read (here an archive format the
    /// launcher doesn't know) records nothing, so the change keeps every
    /// file in place, as 2.0.40 did, and says so instead of refusing.
    #[tokio::test]
    async fn task13_unreadable_inventory_records_nothing_and_lets_the_change_keep_files() {
        let t = tempfile::tempdir().unwrap();
        let baseline = ensure(t.path(), MODRINTH, async {
            anyhow::bail!("Unsupported installed MRPack inventory format")
        })
        .await
        .unwrap();
        assert_eq!(baseline, Baseline::Unavailable { gone: false });
        assert_eq!(
            baseline.warning(MODRINTH),
            Some(TaskWarning::PreviousPackFilesKept {
                platform: MODRINTH,
                reason: PreviousPackFilesReason::VersionUnreadable,
            })
        );
        assert!(!t.path().join(".setup").exists());
        assert!(!t.path().join("packinfo.json").exists());
    }

    /// The platform no longer serves the installed version: the change goes
    /// ahead without a record and names why it kept the old files.
    #[tokio::test]
    async fn task13_removed_installed_version_records_nothing_and_lets_the_change_keep_files() {
        let t = tempfile::tempdir().unwrap();
        let _fixture = inventory_fixture::set_fixture(
            t.path().to_path_buf(),
            inventory_fixture::Fixture::Gone,
        );
        let baseline = ensure(t.path(), CURSEFORGE, async {
            panic!("the fixture stands in for the provider")
        })
        .await
        .unwrap();
        assert_eq!(baseline, Baseline::Unavailable { gone: true });
        assert_eq!(
            baseline.warning(CURSEFORGE),
            Some(TaskWarning::PreviousPackFilesKept {
                platform: CURSEFORGE,
                reason: PreviousPackFilesReason::VersionUnavailable,
            })
        );
        assert!(!t.path().join("packinfo.json").exists());
        let archive = ensure(t.path(), MODRINTH, async {
            Err(anyhow::Error::new(Gone("Installed pack has no archive")).context("resolving"))
        })
        .await
        .unwrap();
        assert_eq!(archive, Baseline::Unavailable { gone: true });
    }

    /// Only a platform that can't be reached refuses the change, and the
    /// refusal says what to do; retrying records the baseline.
    #[tokio::test]
    async fn task13_unreachable_platform_refuses_with_a_retry_hint() {
        let t = tempfile::tempdir().unwrap();
        let fixture = inventory_fixture::set_fixture(
            t.path().to_path_buf(),
            inventory_fixture::Fixture::Unreachable,
        );
        let error = ensure(t.path(), MODRINTH, async {
            panic!("the fixture stands in for the provider")
        })
        .await
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Could not reach Modrinth to check which files the installed version of this modpack added. Check your connection, or try again in a few minutes if it is working."
        );
        // The version modal words it in the player's language from this.
        let presented = serde_json::to_value(crate::error::FeError::from_anyhow(&error)).unwrap();
        assert_eq!(presented["cause"][0]["code"], "MODPACK_CHECK_UNREACHABLE");
        assert_eq!(
            presented["cause"][0]["data"],
            serde_json::json!({"platform": "Modrinth"})
        );
        assert!(!t.path().join("packinfo.json").exists());
        assert!(!t.path().join(".setup").exists());
        drop(fixture);
        assert_eq!(
            ensure(t.path(), MODRINTH, async { Ok(Resolved::default()) })
                .await
                .unwrap(),
            Baseline::Recorded
        );
        assert!(t.path().join("packinfo.json").exists());
    }

    #[test]
    fn task13_only_connection_and_server_failures_count_as_unreachable() {
        use crate::error::request::{RequestContext, RequestError, RequestErrorDetails};
        let request = |error| {
            anyhow::Error::new(RequestError {
                context: RequestContext::none(),
                error,
            })
            .context("Cannot read the installed version")
        };
        let status = |status: u16| {
            request(RequestErrorDetails::UnexpectedStatus {
                status: reqwest::StatusCode::from_u16(status).unwrap(),
                details: None,
            })
        };
        let download = |status| {
            anyhow::Error::new(carbon_net::DownloadError::Non200StatusCode(
                carbon_net::Downloadable::new("https://example.invalid/pack.zip", "pack.zip"),
                status,
            ))
        };
        for error in [
            request(RequestErrorDetails::ConnectionFailed),
            request(RequestErrorDetails::Timeout),
            status(503),
            status(429),
            download(502),
            anyhow::Error::new(carbon_net::DownloadError::GenericDownload("stalled".into())),
        ] {
            assert!(unreachable(&error), "{error:?}");
            assert!(!gone(&error), "{error:?}");
        }
        for error in [status(404), status(410), download(404)] {
            assert!(!unreachable(&error), "{error:?}");
            assert!(gone(&error), "{error:?}");
        }
        for error in [
            status(403),
            anyhow::anyhow!("Unsupported installed CurseForge inventory format"),
            anyhow::Error::new(std::io::Error::other("disk full")),
        ] {
            assert!(!unreachable(&error), "{error:?}");
            assert!(!gone(&error), "{error:?}");
        }
    }

    /// A request that can never work and a body that isn't what was asked
    /// for are permanent, so they must not refuse with "try again": the real
    /// errors the download and the fetches give for them, next to a real
    /// connection failure, which does.
    #[tokio::test]
    async fn task13_permanent_request_failures_are_not_unreachable() {
        let t = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        for url in ["http://[bad", "not a url", "ftp://example.invalid/pack.zip"] {
            let error = download_archive(
                url,
                &t.path().join("pack.zip"),
                1,
                Checksum::Sha512(hex::encode(Sha512::digest(b"pack"))),
                &cancel,
            )
            .await
            .unwrap_err();
            assert!(!unreachable(&error), "{url}: {error:?}");
            assert!(!gone(&error), "{url}: {error:?}");
        }
        let response = reqwest::Response::from(
            reqwest::ResponseBuilderExt::url(
                axum::http::Response::builder(),
                "https://example.invalid/manifest.json".parse().unwrap(),
            )
            .body("<html>")
            .unwrap(),
        );
        let decode = anyhow::Error::new(response.json::<serde_json::Value>().await.unwrap_err());
        assert!(!unreachable(&decode), "{decode:?}");
        let builder = reqwest::Client::new()
            .get("http://[bad")
            .build()
            .unwrap_err();
        let request = crate::error::request::RequestError::from_error(builder);
        assert!(!unreachable(&anyhow::Error::new(request)));
        let refused = reqwest::Client::new()
            .get("http://127.0.0.1:1/")
            .send()
            .await
            .unwrap_err();
        assert!(unreachable(&anyhow::Error::new(
            reqwest_middleware::Error::Reqwest(refused)
        )));
        let retries_ran_out = reqwest_middleware::Error::Middleware(anyhow::anyhow!("gave up"));
        assert!(unreachable(&anyhow::Error::new(retries_ran_out)));
    }

    /// Answers one request with `response`, then closes the connection.
    async fn answer_once(response: &'static [u8]) -> reqwest::Response {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 4096];
            let _ = stream.read(&mut request).await;
            stream.write_all(response).await.unwrap();
        });
        reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .get(url)
            .send()
            .await
            .unwrap()
    }

    /// A connection lost while a platform's answer is being read is the same
    /// failure as one lost before it: the check refuses and retrying fixes it.
    /// An answer that arrives whole and isn't what was asked for is permanent.
    #[tokio::test]
    async fn task13_connection_lost_mid_answer_refuses_but_a_bad_answer_does_not() {
        use crate::error::request::GoodJsonRequestError;
        let t = tempfile::tempdir().unwrap();
        let lost = answer_once(b"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\n\r\n{\"data\"")
            .await
            .json_with_context::<serde_json::Value>()
            .await
            .unwrap_err();
        let error = ensure(t.path(), CURSEFORGE, async { Err(lost.into()) })
            .await
            .unwrap_err();
        assert!(matches!(
            error.downcast_ref::<ChangeVersionRefusal>(),
            Some(ChangeVersionRefusal::Unreachable {
                platform: Some(CURSEFORGE),
                ..
            })
        ));
        assert!(!t.path().join("packinfo.json").exists());

        let bad = answer_once(b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\n\r\n<html>")
            .await
            .json_with_context::<serde_json::Value>()
            .await
            .unwrap_err();
        let baseline = ensure(t.path(), CURSEFORGE, async { Err(bad.into()) })
            .await
            .unwrap();
        assert_eq!(baseline, Baseline::Unavailable { gone: false });
    }

    /// A folder that won't take the record doesn't stop the change, which
    /// keeps every file and says so, as when the version can't be read.
    #[tokio::test]
    async fn task13_record_that_cant_be_written_lets_the_change_keep_files() {
        use crate::util::publish::fault::{IN_USE, Step, inject};
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("instance/mods")).unwrap();
        std::fs::write(t.path().join("instance/mods/a.jar"), b"a").unwrap();
        let inventory = Inventory::from([(
            "/mods/a.jar".to_owned(),
            Checksum::Sha512(hex::encode(Sha512::digest(b"a"))),
        )]);
        inject(Step::NoReplace, &t.path().join("packinfo.json"), IN_USE);
        let baseline = ensure(t.path(), CURSEFORGE, async { Ok(inventory.into()) })
            .await
            .unwrap();
        assert_eq!(baseline, Baseline::Unavailable { gone: false });
        assert!(!t.path().join("packinfo.json").exists());
        assert_eq!(
            baseline.warning(CURSEFORGE),
            Some(TaskWarning::PreviousPackFilesKept {
                platform: CURSEFORGE,
                reason: PreviousPackFilesReason::VersionUnreadable,
            })
        );
        // Nothing was left behind for the next attempt to trip on.
        let left: Vec<_> = std::fs::read_dir(t.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(left, ["instance"]);
    }

    /// A record another writer published while the check ran is the one the
    /// change reads, so it stays and the change relies on it.
    #[tokio::test]
    async fn task13_record_published_meanwhile_is_kept_and_relied_on() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("instance/mods")).unwrap();
        std::fs::write(t.path().join("instance/mods/a.jar"), b"a").unwrap();
        let inventory = Inventory::from([(
            "/mods/a.jar".to_owned(),
            Checksum::Sha512(hex::encode(Sha512::digest(b"a"))),
        )]);
        let theirs = packinfo::make_packinfo(packinfo::PackInfo {
            files: HashMap::new(),
        })
        .unwrap();
        let record = t.path().join("packinfo.json");
        let baseline = ensure(t.path(), CURSEFORGE, async {
            std::fs::write(&record, &theirs).unwrap();
            Ok(inventory.into())
        })
        .await
        .unwrap();
        assert_eq!(baseline, Baseline::Recorded);
        assert_eq!(std::fs::read_to_string(&record).unwrap(), theirs);
    }

    /// A launcher service failing is not the platform's fault, and a malformed
    /// answer from it is as permanent as any other.
    #[tokio::test]
    async fn task13_launcher_service_failures_do_not_name_the_platform() {
        use crate::error::request::{RequestContext, RequestError, RequestErrorDetails};
        let t = tempfile::tempdir().unwrap();
        let error = ensure(t.path(), CURSEFORGE, async {
            Err(anyhow::Error::new(LauncherService(
                RequestError {
                    context: RequestContext::none(),
                    error: RequestErrorDetails::ConnectionFailed,
                }
                .into(),
            )))
        })
        .await
        .unwrap_err();
        assert!(
            error
                .to_string()
                .starts_with("Could not reach the online services needed to check which files"),
            "{error}"
        );
        let presented = serde_json::to_value(crate::error::FeError::from_anyhow(&error)).unwrap();
        assert_eq!(presented["cause"][0]["code"], "MODPACK_CHECK_UNREACHABLE");
        assert!(presented["cause"][0].get("data").is_none());
        let malformed = ensure(t.path(), CURSEFORGE, async {
            Err(anyhow::Error::new(LauncherService(anyhow::anyhow!(
                "not the manifest"
            ))))
        })
        .await
        .unwrap();
        assert_eq!(malformed, Baseline::Unavailable { gone: false });
    }

    /// A cancelled check stops the change: it isn't a reason to go ahead
    /// without a record, or to blame the network.
    #[tokio::test]
    async fn task13_cancelled_check_stops_without_recording_or_falling_back() {
        let t = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        cancel.cancel();
        let path = t.path().join("pack.zip");
        let error = ensure(t.path(), MODRINTH, async {
            download_archive(
                "https://example.invalid/pack.zip",
                &path,
                1,
                Checksum::Sha512(hex::encode(Sha512::digest(b"pack"))),
                &cancel,
            )
            .await?;
            Ok(Resolved::default())
        })
        .await
        .unwrap_err();
        let presented = serde_json::to_value(crate::error::FeError::from_anyhow(&error)).unwrap();
        assert_eq!(
            presented["cause"][0]["code"], "MODPACK_CHECK_CANCELLED",
            "{error:?}"
        );
        assert!(!t.path().join("packinfo.json").exists());
    }

    /// A cancel that lands while the files are being hashed also stops the
    /// change and records nothing, so the next attempt starts clean.
    #[test]
    fn task13_cancel_while_hashing_records_nothing() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("instance/mods")).unwrap();
        std::fs::write(t.path().join("instance/mods/a.jar"), b"a").unwrap();
        let cancel = CancellationToken::new();
        cancel.cancel();
        let inventory = Inventory::from([(
            "/mods/a.jar".to_owned(),
            Checksum::Sha512(hex::encode(Sha512::digest(b"a"))),
        )]);
        let error = record_baseline(t.path(), inventory.into(), &cancel).unwrap_err();
        assert!(matches!(
            error.downcast_ref::<ChangeVersionRefusal>(),
            Some(ChangeVersionRefusal::Cancelled)
        ));
        assert!(!t.path().join("packinfo.json").exists());
    }

    /// A partial record stays partial for every attempt until a change goes
    /// through: the warning isn't lost because an earlier attempt wrote it.
    #[tokio::test]
    async fn task13_retry_after_a_partial_record_warns_again() {
        let t = tempfile::tempdir().unwrap();
        let resolved = Resolved {
            files: HashMap::new(),
            unresolved: 2,
        };
        assert_eq!(
            ensure(t.path(), CURSEFORGE, async { Ok(resolved) })
                .await
                .unwrap(),
            Baseline::Partial
        );
        assert_eq!(
            ensure(t.path(), CURSEFORGE, async {
                panic!("a record is never fetched again")
            })
            .await
            .unwrap(),
            Baseline::Partial
        );
    }

    /// One manifest entry the platform no longer describes leaves that entry
    /// unresolved and keeps every other one, instead of failing the pack.
    #[test]
    fn task13_cf_manifest_entries_that_cant_be_resolved_are_counted_not_fatal() {
        use carbon_platforms::curseforge as cf;
        let sha1 = hex::encode(sha1::Sha1::digest(b"bytes"));
        let file = |id: i32, mod_id: i32, name: &str, hashes: serde_json::Value| -> cf::File {
            serde_json::from_value(serde_json::json!({"id":id,"gameId":432,"modId":mod_id,"isAvailable":true,"displayName":name,"fileName":name,
            "releaseType":1,"fileStatus":4,"hashes":hashes,"fileDate":"2026-01-01T00:00:00Z","fileLength":5,"downloadCount":0,"downloadUrl":null,
            "gameVersions":["1.20.1"],"sortableGameVersions":[],"dependencies":[],"fileFingerprint":0})).unwrap()
        };
        let addon = |id: i32, class: i32| -> cf::Mod {
            serde_json::from_value(serde_json::json!({"id":id,"gameId":432,"name":"addon","slug":"addon",
            "links":{"websiteUrl":null,"wikiUrl":null,"issuesUrl":null,"sourceUrl":null},"summary":"","status":4,"downloadCount":0,"isFeatured":false,
            "primaryCategoryId":1,"categories":[],"classId":class,"authors":[],"logo":null,"screenshots":[],"mainFileId":0,"latestFiles":[],"latestFilesIndexes":[],
            "dateCreated":"","dateModified":"","dateReleased":"","allowModDistribution":true,"gamePopularityRank":0,"isAvailable":true,"thumbsUpCount":0})).unwrap()
        };
        let hashes = serde_json::json!([{"algo":1,"value":sha1}]);
        let files = vec![
            file(10, 1, "kept.jar", hashes.clone()),
            file(20, 2, "orphan.jar", hashes.clone()),
            file(40, 4, "../evil.jar", hashes.clone()),
            file(50, 5, "no-digest.jar", serde_json::json!([])),
            file(60, 6, "pack.zip", hashes.clone()),
        ];
        let mods = vec![addon(1, 6), addon(4, 6), addon(5, 6), addon(6, 12)];
        let manifest: cf::manifest::Manifest = serde_json::from_value(serde_json::json!({
            "minecraft":{"version":"1.20.1","modLoaders":[]},"manifestType":"minecraftModpack","manifestVersion":1,
            "name":"Synthetic","author":"Synthetic","overrides":"overrides",
            "files":[
                {"projectID":1,"fileID":10,"required":true},
                {"projectID":2,"fileID":20,"required":true},
                {"projectID":3,"fileID":30,"required":true},
                {"projectID":4,"fileID":40,"required":true},
                {"projectID":5,"fileID":50,"required":true},
                {"projectID":6,"fileID":60,"required":true}
            ]
        }))
        .unwrap();
        let mc: daedalus::minecraft::VersionManifest = serde_json::from_value(serde_json::json!({
            "latest":{"release":"1.20.1","snapshot":"1.20.1"},"versions":[{
            "id":"1.20.1","type":"release","url":"https://task08.invalid/version.json","time":"2026-01-01T00:00:00Z","releaseTime":"2026-01-01T00:00:00Z",
            "sha1":"0000000000000000000000000000000000000000","complianceLevel":1}]
        }))
        .unwrap();
        let mut resolved = Resolved::from(Inventory::from([(
            "/config/override.txt".to_owned(),
            Checksum::Sha512(hex::encode(Sha512::digest(b"override"))),
        )]));
        add_cf_manifest_files(&mut resolved, &manifest, &files, &mods, &mc);
        // An addon missing from the mods, a file missing from the files, an
        // unsafe name and a file with no digest.
        assert_eq!(resolved.unresolved, 4);
        let mut keys: Vec<_> = resolved.files.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "/config/override.txt",
                "/mods/kept.jar",
                "/resourcepacks/pack.zip"
            ]
        );
    }

    /// Files the platform no longer describes, or describes with an unsafe
    /// name or a malformed digest, stay unrecorded (so the change keeps
    /// them); the rest of the inventory is still recorded.
    #[tokio::test]
    async fn task13_unresolvable_entries_leave_a_partial_record() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("instance");
        std::fs::create_dir_all(data.join("mods")).unwrap();
        std::fs::write(data.join("mods/kept.jar"), b"pack").unwrap();
        std::fs::write(data.join("mods/bad.jar"), b"bad").unwrap();
        let sha = |bytes: &[u8]| Checksum::Sha512(hex::encode(Sha512::digest(bytes)));
        let resolved = Resolved {
            files: HashMap::from([
                ("/mods/kept.jar".into(), sha(b"pack")),
                ("/config/a:b.txt".into(), sha(b"odd")),
                ("/mods/bad.jar".into(), Checksum::Sha512("bad".into())),
            ]),
            unresolved: 1,
        };
        let baseline = ensure(t.path(), CURSEFORGE, async { Ok(resolved) })
            .await
            .unwrap();
        assert_eq!(baseline, Baseline::Partial);
        assert_eq!(
            baseline.warning(CURSEFORGE),
            Some(TaskWarning::PreviousPackFilesKept {
                platform: CURSEFORGE,
                reason: PreviousPackFilesReason::SomeFilesUnchecked,
            })
        );
        let text = std::fs::read_to_string(t.path().join("packinfo.json")).unwrap();
        let info = packinfo::parse_packinfo(&text).unwrap();
        assert_eq!(
            info.files.keys().collect::<Vec<_>>(),
            vec!["/mods/kept.jar"]
        );
        let record: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            record["verified_import_inventory"],
            serde_json::json!({"schema": 1, "declared_files": 4, "unchecked_files": 3})
        );
    }
}

#[cfg(test)]
const MODRINTH: DependencyPlatform = DependencyPlatform::Modrinth;
#[cfg(test)]
const CURSEFORGE: DependencyPlatform = DependencyPlatform::Curseforge;

#[cfg(test)]
mod task13_controls {
    use super::super::{apply_plan, disk_scan};
    use super::*;
    #[tokio::test]
    async fn task13_baseline_reconciles_renamed_mod_and_preserves_user_and_disabled_bytes() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path();
        let data = root.join("instance/mods");
        let staging = root.join("staging");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::create_dir_all(staging.join("instance/mods")).unwrap();
        for (name, bytes) in [
            ("a-v1.jar", b"old".as_slice()),
            ("disabled.jar.disabled", b"disabled"),
            ("edit.jar", b"user edit"),
            ("extra.jar", b"extra"),
        ] {
            std::fs::write(data.join(name), bytes).unwrap();
        }
        let inventory = HashMap::from([
            (
                "/mods/a-v1.jar".into(),
                Checksum::Sha512(hex::encode(Sha512::digest(b"old"))),
            ),
            (
                "/mods/disabled.jar".into(),
                Checksum::Sha512(hex::encode(Sha512::digest(b"disabled"))),
            ),
            (
                "/mods/edit.jar".into(),
                Checksum::Sha512(hex::encode(Sha512::digest(b"original"))),
            ),
        ]);
        ensure(root, MODRINTH, async { Ok(inventory.into()) })
            .await
            .unwrap();
        let old =
            packinfo::parse_packinfo(&std::fs::read_to_string(root.join("packinfo.json")).unwrap())
                .unwrap();
        assert!(old.files.contains_key("/mods/disabled.jar"));
        std::fs::write(staging.join("instance/mods/a-v2.jar"), b"new").unwrap();
        std::fs::write(staging.join("instance/mods/disabled.jar"), b"disabled").unwrap();
        let target = packinfo::scan_dir(&staging.join("instance"), None)
            .await
            .unwrap();
        let universe = old
            .files
            .keys()
            .chain(target.files.keys())
            .cloned()
            .collect();
        let disk = disk_scan::scan_disk_state(&root.join("instance"), &universe)
            .await
            .unwrap();
        let plan = apply_plan::plan(apply_plan::PlanInputs {
            old: Some(&old),
            target: &target,
            staged: &target.files.keys().cloned().collect(),
            disk: &disk.states,
            coexisting_disabled_twin_md5: &disk.coexisting_disabled_twin_md5,
            mode: apply_plan::ApplyMode::VersionChange,
            fs_case_insensitive: false,
        })
        .unwrap();
        super::super::super::run::execute_import_baseline_test_plan(&plan, root, &staging, &target)
            .await
            .unwrap();
        assert!(!data.join("a-v1.jar").exists());
        assert_eq!(std::fs::read(data.join("a-v2.jar")).unwrap(), b"new");
        assert_eq!(std::fs::read(data.join("edit.jar")).unwrap(), b"user edit");
        assert_eq!(std::fs::read(data.join("extra.jar")).unwrap(), b"extra");
        assert_eq!(
            std::fs::read(data.join("disabled.jar.disabled")).unwrap(),
            b"disabled"
        );
        assert!(!data.join("disabled.jar").exists());
    }
    /// Plans and applies a version change from the root `packinfo.json` to
    /// `root/staging/instance`, as the pack pipeline does.
    async fn change_version(root: &Path) -> Vec<apply_plan::PlanEntry> {
        let staging = root.join("staging");
        let old =
            packinfo::parse_packinfo(&std::fs::read_to_string(root.join("packinfo.json")).unwrap())
                .unwrap();
        let target = packinfo::scan_dir(&staging.join("instance"), None)
            .await
            .unwrap();
        let universe = old
            .files
            .keys()
            .chain(target.files.keys())
            .cloned()
            .collect();
        let disk = disk_scan::scan_disk_state(&root.join("instance"), &universe)
            .await
            .unwrap();
        let plan = apply_plan::plan(apply_plan::PlanInputs {
            old: Some(&old),
            target: &target,
            staged: &target.files.keys().cloned().collect(),
            disk: &disk.states,
            coexisting_disabled_twin_md5: &disk.coexisting_disabled_twin_md5,
            mode: apply_plan::ApplyMode::VersionChange,
            fs_case_insensitive: false,
        })
        .unwrap();
        super::super::super::run::execute_import_baseline_test_plan(&plan, root, &staging, &target)
            .await
            .unwrap();
        plan
    }

    /// A pack override shipped disabled (`mods/optional.jar.disabled` in the
    /// installed archive) is owned under its enabled name, the way
    /// `packinfo::scan_dir` keys it, so the first update keeps the file
    /// whether or not the new version changes it, instead of deleting it as a
    /// file the pack dropped.
    #[tokio::test]
    async fn task13_disabled_override_survives_the_first_update() {
        for target_bytes in [b"optional".as_slice(), b"optional v2"] {
            let t = tempfile::tempdir().unwrap();
            let root = t.path();
            let data = root.join("instance/mods");
            std::fs::create_dir_all(&data).unwrap();
            std::fs::create_dir_all(root.join("staging/instance/mods")).unwrap();
            std::fs::write(data.join("optional.jar.disabled"), b"optional").unwrap();
            let archive = root.join("installed.zip");
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file("manifest.json", options).unwrap();
            zip.write_all(serde_json::json!({"minecraft":{"version":"1.20.1","modLoaders":[]},"manifestType":"minecraftModpack","manifestVersion":1,"name":"Synthetic","author":"Synthetic","overrides":"overrides","files":[]}).to_string().as_bytes()).unwrap();
            zip.start_file("overrides/mods/optional.jar.disabled", options)
                .unwrap();
            zip.write_all(b"optional").unwrap();
            zip.finish().unwrap();
            let (_, overrides) = cf_archive(&archive).unwrap();
            assert!(overrides.contains_key("/mods/optional.jar.disabled"));
            assert_eq!(
                ensure(root, CURSEFORGE, async { Ok(overrides.into()) })
                    .await
                    .unwrap(),
                Baseline::Recorded
            );
            let old = packinfo::parse_packinfo(
                &std::fs::read_to_string(root.join("packinfo.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(
                old.files.keys().collect::<Vec<_>>(),
                vec!["/mods/optional.jar"]
            );
            std::fs::write(
                root.join("staging/instance/mods/optional.jar.disabled"),
                target_bytes,
            )
            .unwrap();
            let plan = change_version(root).await;
            assert!(
                plan.iter()
                    .all(|entry| entry.action != apply_plan::PlanAction::Delete),
                "{plan:?}"
            );
            assert_eq!(
                std::fs::read(data.join("optional.jar.disabled")).unwrap(),
                b"optional"
            );
            assert!(!data.join("optional.jar").exists());
        }
    }

    /// A pack ships `optional.jar.disabled` and the user enabled it, so the
    /// bare file on disk holds the pack's bytes. The state is the user's: the
    /// first update leaves the mod enabled and in place, whether the new
    /// version changes the file or drops it, as 2.0.40 did.
    #[tokio::test]
    async fn task13_optional_file_the_user_enabled_stays_enabled_on_the_first_update() {
        for target_ships in [None, Some(b"optional".as_slice()), Some(b"optional v2")] {
            let t = tempfile::tempdir().unwrap();
            let root = t.path();
            let data = root.join("instance/mods");
            std::fs::create_dir_all(&data).unwrap();
            std::fs::create_dir_all(root.join("staging/instance/mods")).unwrap();
            std::fs::write(data.join("optional.jar"), b"optional").unwrap();
            std::fs::write(data.join("other.jar"), b"other").unwrap();
            let sha = |bytes: &[u8]| Checksum::Sha512(hex::encode(Sha512::digest(bytes)));
            let inventory = Inventory::from([
                ("/mods/optional.jar.disabled".to_owned(), sha(b"optional")),
                ("/mods/other.jar".to_owned(), sha(b"other")),
            ]);
            assert_eq!(
                ensure(root, CURSEFORGE, async { Ok(inventory.into()) })
                    .await
                    .unwrap(),
                Baseline::Recorded
            );
            let old = packinfo::parse_packinfo(
                &std::fs::read_to_string(root.join("packinfo.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(
                old.files.keys().collect::<Vec<_>>(),
                vec!["/mods/other.jar"]
            );
            std::fs::write(root.join("staging/instance/mods/other.jar"), b"other").unwrap();
            if let Some(bytes) = target_ships {
                std::fs::write(
                    root.join("staging/instance/mods/optional.jar.disabled"),
                    bytes,
                )
                .unwrap();
            }
            let plan = change_version(root).await;
            assert!(
                plan.iter()
                    .all(|entry| matches!(entry.action, apply_plan::PlanAction::Keep)),
                "{target_ships:?}: {plan:?}"
            );
            assert_eq!(
                std::fs::read(data.join("optional.jar")).unwrap(),
                b"optional",
                "{target_ships:?}"
            );
            assert!(
                !data.join("optional.jar.disabled").exists(),
                "{target_ships:?}"
            );
        }
    }

    /// A pack that ships a file both enabled and disabled installs the
    /// enabled one, so the baseline checks the disk against that one's bytes.
    /// A file in place under its other spelling, or both spellings at once,
    /// stays unrecorded and is kept.
    #[tokio::test]
    async fn task13_file_shipped_enabled_and_disabled_is_owned_by_its_enabled_bytes() {
        let sha = |bytes: &[u8]| Checksum::Sha512(hex::encode(Sha512::digest(bytes)));
        let inventory = || -> Inventory {
            HashMap::from([
                ("/mods/both.jar".into(), sha(b"enabled")),
                ("/mods/both.jar.disabled".into(), sha(b"disabled")),
                ("/mods/odd.jar.disabled.disabled".into(), sha(b"odd")),
            ])
        };
        for (files, owned) in [
            (vec![("both.jar", b"enabled".as_slice())], true),
            (vec![("both.jar.disabled", b"enabled")], true),
            (vec![("both.jar.disabled", b"disabled")], false),
            (
                vec![("both.jar", b"enabled"), ("both.jar.disabled", b"disabled")],
                false,
            ),
        ] {
            let t = tempfile::tempdir().unwrap();
            let data = t.path().join("instance/mods");
            std::fs::create_dir_all(&data).unwrap();
            for (name, bytes) in &files {
                std::fs::write(data.join(name), bytes).unwrap();
            }
            std::fs::write(data.join("odd.jar.disabled.disabled"), b"odd").unwrap();
            assert_eq!(
                ensure(t.path(), MODRINTH, async { Ok(inventory().into()) })
                    .await
                    .unwrap(),
                Baseline::Recorded
            );
            let info = packinfo::parse_packinfo(
                &std::fs::read_to_string(t.path().join("packinfo.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(
                info.files.get("/mods/both.jar").map(|h| h.sha512),
                owned.then(|| Sha512::digest(b"enabled").into()),
                "{files:?}"
            );
            assert!(!info.files.contains_key("/mods/both.jar.disabled"));
            assert!(info.files.contains_key("/mods/odd.jar.disabled"));
        }
    }

    #[tokio::test]
    async fn task13_existing_record_and_incomplete_inventory_controls() {
        let t = tempfile::tempdir().unwrap();
        // A malformed digest owns nothing: nothing can match it.
        std::fs::create_dir_all(t.path().join("instance/mods")).unwrap();
        std::fs::write(t.path().join("instance/mods/a.jar"), b"a").unwrap();
        let malformed = HashMap::from([("/mods/a.jar".into(), Checksum::Sha512("bad".into()))]);
        assert_eq!(
            ensure(t.path(), MODRINTH, async { Ok(malformed.into()) })
                .await
                .unwrap(),
            Baseline::Partial
        );
        let written = std::fs::read_to_string(t.path().join("packinfo.json")).unwrap();
        assert!(packinfo::parse_packinfo(&written).unwrap().files.is_empty());
        std::fs::remove_file(t.path().join("packinfo.json")).unwrap();
        let record = packinfo::make_packinfo(packinfo::PackInfo {
            files: HashMap::new(),
        })
        .unwrap();
        std::fs::write(t.path().join("packinfo.json"), &record).unwrap();
        assert_eq!(
            ensure(t.path(), MODRINTH, async {
                panic!("healthy historical record must not fetch a provider")
            })
            .await
            .unwrap(),
            Baseline::Recorded
        );
        assert_eq!(
            std::fs::read_to_string(t.path().join("packinfo.json")).unwrap(),
            record
        );
    }
}

#[cfg(test)]
mod task13_archive_controls {
    use super::*;
    #[test]
    fn task13_pinned_cf_override_inventory_and_format_gate() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("synthetic.zip");
        for format in [1, 2] {
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
            zip.start_file("manifest.json", zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(serde_json::json!({"minecraft":{"version":"1.20.1","modLoaders":[]},"manifestType":"minecraftModpack","manifestVersion":format,"name":"Synthetic","author":"Synthetic","overrides":"overrides","files":[]}).to_string().as_bytes()).unwrap();
            zip.start_file(
                "overrides/config/synthetic.txt",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            zip.write_all(b"declared bytes").unwrap();
            zip.start_file(
                "unrelated/synthetic.txt",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            zip.write_all(b"not installed").unwrap();
            zip.finish().unwrap();
            let result = cf_archive(&path);
            if format == 2 {
                assert!(result.is_err());
                continue;
            }
            let (_, inventory) = result.unwrap();
            assert_eq!(inventory.len(), 1);
            let Checksum::Sha512(hash) = &inventory["/config/synthetic.txt"] else {
                panic!("missing override hash")
            };
            assert_eq!(hash, &hex::encode(Sha512::digest(b"declared bytes")));
        }
    }
}

/// The files of a Modrinth pack archive, keyed by their paths as the index
/// and the archive spell them; [`ensure`] validates each before it is used.
async fn mr_archive(path: &Path, index_budget: u64, total_budget: u64) -> anyhow::Result<Resolved> {
    use carbon_platforms::modrinth::version::ModrinthEnvironmentSupport;
    let (index, overrides) =
        super::origin_check::parse_mrpack_inventory(path, index_budget, total_budget).await?;
    ensure!(
        index.format_version == 1,
        "Unsupported installed MRPack inventory format"
    );
    let mut resolved = Resolved::from(
        overrides
            .into_iter()
            .map(|(p, h)| (p, Checksum::Sha512(hex::encode(h))))
            .collect::<Inventory>(),
    );
    for file in index.files {
        if file
            .env
            .as_ref()
            .is_some_and(|env| matches!(env.client, ModrinthEnvironmentSupport::Unsupported))
        {
            continue;
        }
        let key = format!("/{}", file.path);
        // Pack downloads overwrite extracted overrides in the normal producer,
        // so an override of the same path never says what was installed there.
        match crate::managers::minecraft::modrinth::required_checksum(&file) {
            Ok(checksum) => {
                resolved.files.insert(key, checksum);
            }
            Err(error) => {
                tracing::warn!(
                    ?error,
                    "installed pack file has no usable digest; left unrecorded"
                );
                resolved.files.remove(&key);
                resolved.unresolved += 1;
            }
        }
    }
    Ok(resolved)
}

#[cfg(test)]
mod task13_round01_tests {
    use super::*;
    fn archive(path: &Path, entries: &[(&str, &[u8])]) {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        zip.start_file(
            "modrinth.index.json",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(br#"{"formatVersion":1,"game":"minecraft","versionId":"synthetic","name":"Synthetic","files":[],"dependencies":{}}"#).unwrap();
        for (name, bytes) in entries {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
    }
    #[tokio::test]
    async fn task13_round01_required_inventory_exceeds_preview_entry_limit() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("synthetic.mrpack");
        let bytes = vec![b'x'; 1025];
        archive(&path, &[("overrides/mods/synthetic.jar", &bytes)]);
        std::fs::create_dir_all(t.path().join("instance/mods")).unwrap();
        std::fs::write(t.path().join("instance/mods/synthetic.jar"), &bytes).unwrap();
        let (_, _, preview_complete) =
            super::super::origin_check::parse_mrpack_coverage(&path, 1024, 16 * 1024)
                .await
                .unwrap();
        assert!(
            !preview_complete,
            "optional preview retains its per-entry policy"
        );
        assert_eq!(
            ensure(t.path(), MODRINTH, mr_archive(&path, 1024, 16 * 1024))
                .await
                .unwrap(),
            Baseline::Recorded
        );
        let info = packinfo::parse_packinfo(
            &std::fs::read_to_string(t.path().join("packinfo.json")).unwrap(),
        )
        .unwrap();
        assert!(info.files.contains_key("/mods/synthetic.jar"));
    }
    /// An index file without a usable digest stays unrecorded, and an
    /// override of the same path doesn't stand in for it: the download
    /// replaced that override when the pack was installed.
    #[tokio::test]
    async fn task13_index_file_without_a_digest_is_left_unrecorded() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("synthetic.mrpack");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("modrinth.index.json", options).unwrap();
        zip.write_all(serde_json::json!({"formatVersion":1,"game":"minecraft","versionId":"synthetic","name":"Synthetic","files":[{"path":"mods/a.jar","hashes":{"sha512":"","sha1":""},"downloads":[],"fileSize":1}],"dependencies":{}}).to_string().as_bytes()).unwrap();
        for (name, bytes) in [
            ("overrides/mods/a.jar", b"override".as_slice()),
            ("overrides/config/b.txt", b"b"),
        ] {
            zip.start_file(name, options).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
        let resolved = mr_archive(&path, 1024, 1024).await.unwrap();
        assert_eq!(resolved.unresolved, 1);
        assert_eq!(
            resolved.files.keys().collect::<Vec<_>>(),
            vec!["/config/b.txt"]
        );
    }
    #[tokio::test]
    async fn task13_round01_required_inventory_rejects_aggregate_overshoot() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("synthetic.mrpack");
        archive(
            &path,
            &[
                ("overrides/a", b"12345678"),
                ("client-overrides/a", b"abcdefgh"),
            ],
        );
        assert!(
            mr_archive(&path, 1024, 12).await.is_err(),
            "required coverage must not overshoot cumulative work"
        );
        // An inventory too big to verify records nothing; the change keeps
        // every file in place.
        assert_eq!(
            ensure(t.path(), MODRINTH, mr_archive(&path, 1024, 12))
                .await
                .unwrap(),
            Baseline::Unavailable { gone: false }
        );
        assert!(!t.path().join("packinfo.json").exists());
        assert!(!t.path().join(".setup").exists());
        assert!(
            mr_archive(&path, 1024, 16).await.is_err(),
            "equal-budget input still needs verified EOF"
        );
        assert_eq!(
            ensure(t.path(), MODRINTH, mr_archive(&path, 1024, 16))
                .await
                .unwrap(),
            Baseline::Unavailable { gone: false }
        );
        assert!(!t.path().join("packinfo.json").exists());
        assert!(!t.path().join(".setup").exists());
        let inventory = mr_archive(&path, 1024, 17).await.unwrap();
        let Checksum::Sha512(hash) = &inventory.files["/a"] else {
            panic!("missing client override")
        };
        assert_eq!(hash, &hex::encode(Sha512::digest(b"abcdefgh")));
        assert!(
            mr_archive(&path, 16, 1024).await.is_err(),
            "the declared index budget must be real"
        );
    }
}
