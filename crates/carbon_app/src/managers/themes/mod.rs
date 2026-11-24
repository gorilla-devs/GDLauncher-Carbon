pub mod archive;
pub mod css;
pub mod error;
pub mod extras;
pub mod files;
pub mod manifest;
pub mod preferences;
pub mod season;
pub mod streams;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use carbon_repos::repos::app_configuration::AppConfigurationPatch;

use self::css::check_theme_css;
use self::error::ThemeError;
use self::files::{
    Failure, FileKind, ThemeLimits, content_type, file_kind, is_ignored_entry, is_valid_theme_id,
    normalize_rel_path, scan_theme_dir, scan_theme_dir_checked, shown,
};
use self::manifest::{ValidManifest, parse_builtin_manifest, parse_manifest};
use self::streams::AssetStreams;
use super::ManagerRef;
use crate::api::keys::settings::GET_SETTINGS;
use crate::api::keys::themes::{GET_THEME, LIST};

/// The most manifest warnings that reach the logs or the caller.
const MAX_WARNINGS: usize = 20;

/// The largest asset the route serves (the theme size limit).
const MAX_ASSET_BYTES: u64 = 52_428_800;

/// Where a theme's files are served from, and how.
pub struct AssetUrlBase {
    pub origin: String,
    pub token: Option<String>,
}

impl AssetUrlBase {
    pub fn from_server() -> Self {
        Self {
            origin: crate::api::auth::local_origin()
                .unwrap_or_else(|| "http://127.0.0.1:0".to_string()),
            token: crate::api::auth::expected_token().map(str::to_string),
        }
    }

    pub fn asset_url(&self, id: &str, rel: &str, version: u64) -> String {
        let path: String = url::form_urlencoded::byte_serialize(rel.as_bytes()).collect();
        let mut url = format!(
            "{}/theme/asset?id={id}&path={path}&v={version:x}",
            self.origin
        );
        if let Some(token) = &self.token {
            url.push_str("&_token=");
            url.push_str(token);
        }
        url
    }
}

pub struct LoadedTheme {
    pub manifest: ValidManifest,
    pub css: Option<String>,
    pub preview_url: Option<String>,
    /// Same order as `manifest.fonts`.
    pub font_urls: Vec<String>,
    /// Path to URL for every file the manifest names.
    pub asset_urls: std::collections::BTreeMap<String, String>,
    pub fingerprint: u64,
}

pub fn theme_dir(themes_root: &Path, id: &str) -> Result<PathBuf, ThemeError> {
    if !is_valid_theme_id(id) {
        return Err(ThemeError::Invalid(format!(
            "\"{}\" isn't a valid theme id",
            shown(id)
        )));
    }
    let dir = themes_root.join(id);
    match std::fs::symlink_metadata(&dir) {
        Ok(_) => Ok(dir),
        Err(_) => Err(ThemeError::NotFound(id.to_string())),
    }
}

/// A theme folder that is itself a link (symlink or junction) could point
/// anywhere, so it is refused.
fn refuse_linked_folder(dir: &Path) -> Result<(), ThemeError> {
    let err = || ThemeError::Invalid("the theme folder can't be a link".to_string());
    let meta = std::fs::symlink_metadata(dir)
        .map_err(|e| ThemeError::Invalid(format!("can't read the theme folder: {e}")))?;
    if meta.file_type().is_symlink() {
        return Err(err());
    }
    // Junctions on Windows: the folder resolves somewhere other than where it
    // sits.
    if let (Some(parent), Some(name)) = (dir.parent(), dir.file_name()) {
        if let (Ok(real), Ok(parent)) = (dunce::canonicalize(dir), dunce::canonicalize(parent)) {
            if real != parent.join(name) {
                return Err(err());
            }
        }
    }
    Ok(())
}

/// Warnings are capped before they reach the logs or the returned value.
fn cap_warnings(mut warnings: Vec<String>) -> Vec<String> {
    if warnings.len() > MAX_WARNINGS {
        let more = warnings.len() - MAX_WARNINGS;
        warnings.truncate(MAX_WARNINGS);
        warnings.push(format!("and {more} more"));
    }
    warnings
}

/// Identical warnings for the same theme and fingerprint are logged once, so
/// the watcher reloading doesn't repeat them.
fn log_warnings_once(id: &str, fingerprint: u64, warnings: &[String]) {
    static SEEN: OnceLock<Mutex<HashSet<(String, u64)>>> = OnceLock::new();
    if warnings.is_empty() {
        return;
    }
    let first = SEEN
        .get_or_init(Default::default)
        .lock()
        .map(|mut seen| seen.insert((id.to_string(), fingerprint)))
        .unwrap_or(true);
    if first {
        for w in warnings {
            tracing::warn!("theme \"{id}\": {w}");
        }
    }
}

pub fn load_theme_dir(
    dir: &Path,
    expected_id: &str,
    base: &AssetUrlBase,
    limits: &ThemeLimits,
) -> Result<LoadedTheme, ThemeError> {
    load_theme_dir_checked(dir, expected_id, base, limits, true).map_err(Failure::into_theme_error)
}

/// Like `load_theme_dir`, for the app's own themes: they keep their `season`.
#[cfg(test)]
fn load_builtin_theme_dir(
    dir: &Path,
    expected_id: &str,
    base: &AssetUrlBase,
    limits: &ThemeLimits,
) -> Result<LoadedTheme, ThemeError> {
    load_theme_dir_checked(dir, expected_id, base, limits, false).map_err(Failure::into_theme_error)
}

/// `load_theme_dir`, telling disk problems apart from theme defects.
fn load_theme_dir_checked(
    dir: &Path,
    expected_id: &str,
    base: &AssetUrlBase,
    limits: &ThemeLimits,
    installed: bool,
) -> Result<LoadedTheme, Failure> {
    let bad = |m: String| Failure::Theme(ThemeError::Invalid(m));
    refuse_linked_folder(dir)?;
    let files = scan_theme_dir_checked(dir, limits)?;

    let manifest_path = dir.join("manifest.json");
    // Text that isn't UTF-8 is the theme's fault; anything else is the disk's.
    let read_err = |what: &str, e: std::io::Error| {
        let msg = format!("can't read {what}: {}", e.kind());
        if e.kind() == std::io::ErrorKind::InvalidData {
            Failure::Theme(ThemeError::Invalid(msg))
        } else {
            Failure::Io(msg)
        }
    };
    let len = std::fs::metadata(&manifest_path)
        .map_err(|e| read_err("manifest.json", e))?
        .len();
    if len > limits.max_manifest_bytes {
        return Err(bad(format!(
            "manifest.json is larger than {} KB",
            limits.max_manifest_bytes / 1024
        )));
    }
    let bytes = std::fs::read(&manifest_path).map_err(|e| read_err("manifest.json", e))?;
    let mut manifest = if installed {
        parse_manifest(&bytes, &files, limits)?
    } else {
        parse_builtin_manifest(&bytes, &files, limits)?
    };

    if manifest.id != expected_id {
        return Err(bad(format!(
            "the folder name \"{}\" must match the theme id \"{}\"",
            shown(expected_id),
            shown(&manifest.id)
        )));
    }

    let id = manifest.id.clone();
    let fingerprint = files.fingerprint;
    let url_of = |rel: &str| base.asset_url(&id, rel, fingerprint);

    let css = match &manifest.css {
        Some(rel) => {
            let path = dir.join(rel);
            let len = std::fs::metadata(&path)
                .map_err(|e| read_err(rel, e))?
                .len();
            if len > limits.max_css_bytes {
                return Err(bad("theme.css is larger than 1 MB".to_string()));
            }
            let text = std::fs::read_to_string(&path).map_err(|e| read_err(rel, e))?;
            Some(check_theme_css(&text, &files, limits, &url_of)?)
        }
        None => None,
    };

    let preview_url = manifest.preview.as_deref().map(url_of);
    let font_urls = manifest.fonts.iter().map(|f| url_of(&f.src)).collect();

    let mut asset_urls = std::collections::BTreeMap::new();
    let named = manifest
        .preview
        .iter()
        .chain(manifest.fonts.iter().map(|f| &f.src))
        .chain(
            manifest
                .background
                .iter()
                .flat_map(|b| std::iter::once(&b.src).chain(b.motion.iter())),
        )
        .chain(manifest.decorations.iter().flat_map(|d| d.images.iter()))
        .chain(manifest.effects.iter().flat_map(|e| match e {
            extras::Effect::Particles { images, .. } => images.as_slice(),
            _ => &[],
        }))
        .chain(
            [
                &manifest.sounds.launch,
                &manifest.sounds.download_complete,
                &manifest.sounds.click,
                &manifest.sounds.notification,
                &manifest.sounds.error,
            ]
            .into_iter()
            .flatten(),
        );
    for rel in named {
        asset_urls.insert(rel.clone(), url_of(rel));
    }

    let animated =
        extras::animated_image_warnings(dir, manifest.background.as_ref(), &manifest.decorations);
    manifest.warnings.extend(animated);
    manifest.warnings = cap_warnings(std::mem::take(&mut manifest.warnings));
    log_warnings_once(&id, fingerprint, &manifest.warnings);

    Ok(LoadedTheme {
        manifest,
        css,
        preview_url,
        font_urls,
        asset_urls,
        fingerprint,
    })
}

/// Finds the file a `/theme/asset` request names, or says why it can't be
/// served.
pub fn resolve_asset_path(
    themes_root: &Path,
    id: &str,
    rel: &str,
) -> Result<(PathBuf, &'static str), ThemeError> {
    let dir = theme_dir(themes_root, id)?;
    refuse_linked_folder(&dir)?;
    let not_found = || ThemeError::NotFound(format!("{id}/{}", shown(rel)));
    let Some(rel) = normalize_rel_path(rel) else {
        return Err(ThemeError::Invalid("not a valid theme file path".into()));
    };
    if is_ignored_entry(&rel) || !file_kind(&rel).is_some_and(FileKind::is_asset) {
        return Err(ThemeError::Invalid(
            "that file isn't a theme asset".to_string(),
        ));
    }
    let content_type = content_type(&rel).ok_or_else(not_found)?;

    let path = dir.join(&rel);
    let meta = std::fs::symlink_metadata(&path).map_err(|_| not_found())?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err(ThemeError::Invalid(
            "that file isn't a regular theme file".to_string(),
        ));
    }
    let real_dir = dunce::canonicalize(&dir).map_err(|_| not_found())?;
    let real = dunce::canonicalize(&path).map_err(|_| not_found())?;
    if !real.starts_with(&real_dir) {
        return Err(ThemeError::Invalid(
            "that file is outside the theme".to_string(),
        ));
    }
    if meta.len() > MAX_ASSET_BYTES {
        return Err(ThemeError::Invalid("that file is too large".to_string()));
    }
    Ok((real, content_type))
}

/// Staging folders are dot-prefixed so they can never collide with a valid
/// theme id, and live under the themes root (not the temp folder, which is wiped
/// at startup) so renames stay on one volume and a kept backup survives a restart.
/// The list and the watcher skip dot-folders.
const STAGING_NEW: &str = ".staging-theme";
const STAGING_OLD: &str = ".staging-old";
/// Where the zip is extracted inside `STAGING_NEW`. Dot-prefixed too, so no
/// theme id can ever be named like it.
const STAGING_INCOMING: &str = ".incoming";
/// Where `inspect_theme_zip` unpacks. Dot-prefixed like the other staging names.
const STAGING_INSPECT: &str = ".staging-inspect";

/// Where a replace keeps the installed version of `id` until the new one is in
/// place (and where a failed replace leaves it).
fn backup_dir(themes_root: &Path, id: &str) -> PathBuf {
    themes_root.join(format!("{STAGING_OLD}-{id}"))
}

/// A `.gdltheme` larger than the unpacked limit plus 10% is refused without
/// being opened.
fn max_zip_bytes(limits: &ThemeLimits) -> u64 {
    limits.max_total_bytes + limits.max_total_bytes / 10
}

/// Runs `op`, trying again after `pause` when it is refused with
/// PermissionDenied, up to `tries` attempts in all (antivirus or another
/// program briefly holding a handle on Windows).
fn retry_denied<T>(
    tries: u32,
    pause: std::time::Duration,
    mut op: impl FnMut() -> std::io::Result<T>,
) -> std::io::Result<T> {
    let mut attempt = 1;
    loop {
        match op() {
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied && attempt < tries => {
                attempt += 1;
                std::thread::sleep(pause);
            }
            other => return other,
        }
    }
}

fn rename_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    let tries = if cfg!(windows) { 5 } else { 1 };
    retry_denied(tries, std::time::Duration::from_millis(100), || {
        std::fs::rename(from, to)
    })
}

/// What install and inspect both ask of the file first: a regular file (a
/// directory or a FIFO would fail, or block `File::open` for good) that isn't
/// larger than the limit.
async fn check_zip_file(zip: &Path, limits: &ThemeLimits) -> anyhow::Result<()> {
    use anyhow::Context;

    let meta = tokio::fs::metadata(zip)
        .await
        .context("couldn't read the theme file")?;
    if !meta.is_file() {
        anyhow::bail!("the theme path isn't a file");
    }
    if meta.len() > max_zip_bytes(limits) {
        return Err(ThemeError::Invalid(
            "the theme file is too large (the limit is 50 MB unpacked)".to_string(),
        )
        .into());
    }
    Ok(())
}

/// Unpacks, validates and installs a `.gdltheme`, replacing any theme with the
/// same id, and returns the id. The installed folder is only touched once the
/// whole theme has passed validation. Disk problems come back as plain errors;
/// `ThemeError` is reserved for defects in the theme.
///
/// The theme's asset streams (`streams`) are ended before its folder moves.
pub async fn install_theme_zip(
    zip: &Path,
    themes_root: &Path,
    limits: &ThemeLimits,
    streams: &Arc<AssetStreams>,
) -> anyhow::Result<String> {
    install_theme_zip_gated(zip, themes_root, limits, streams, || {}).await
}

/// `gate` runs first on the blocking thread, so a test can hold the work in
/// flight.
async fn install_theme_zip_gated<G: FnOnce() + Send + 'static>(
    zip: &Path,
    themes_root: &Path,
    limits: &ThemeLimits,
    streams: &Arc<AssetStreams>,
    gate: G,
) -> anyhow::Result<String> {
    check_zip_file(zip, limits).await?;

    let zip = zip.to_path_buf();
    let root = themes_root.to_path_buf();
    let limits = limits.clone();
    let streams = streams.clone();

    // All the file work lives inside the blocking task, so dropping this
    // future (a client that went away) can't leave it half done or delete the
    // folders the task is still using.
    tokio::task::spawn_blocking(move || {
        gate();
        stage_and_swap(&zip, &root, &limits, &streams)
    })
    .await?
}

fn remove_path(path: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Removes a scratch folder when dropped, so a panic can't leave it behind.
struct ScratchGuard<'a>(&'a Path);

impl Drop for ScratchGuard<'_> {
    fn drop(&mut self) {
        if let Err(e) = remove_path(self.0) {
            tracing::warn!("couldn't remove the theme scratch folder {:?}: {e}", self.0);
        }
    }
}

/// Runs `work` with a cleared `<root>/<name>` folder (creating the root), and
/// always removes the folder afterwards, a panic included.
fn in_scratch<T>(
    root: &Path,
    name: &str,
    work: impl FnOnce(&Path) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    use anyhow::Context;

    std::fs::create_dir_all(root).context("couldn't create the themes folder")?;
    let stage = root.join(name);
    remove_path(&stage).context("couldn't clear the previous run's leftovers")?;
    let _cleanup = ScratchGuard(&stage);
    work(&stage)
}

fn stage_and_swap(
    zip: &Path,
    root: &Path,
    limits: &ThemeLimits,
    streams: &AssetStreams,
) -> anyhow::Result<String> {
    in_scratch(root, STAGING_NEW, |stage| {
        stage_in(stage, zip, root, limits, streams)
    })
}

/// Extracts the zip into `<stage>/.incoming`, checks every file and the
/// manifest, moves it to `<stage>/<id>` and loads it exactly as an installed
/// theme is loaded. Install and inspect both run this, so they can't disagree.
/// Returns the id, the staged folder and what loaded.
fn stage_theme(
    stage: &Path,
    zip: &Path,
    limits: &ThemeLimits,
) -> anyhow::Result<(String, PathBuf, LoadedTheme)> {
    use anyhow::Context;

    let incoming = stage.join(STAGING_INCOMING);
    archive::extract_theme_zip(zip, &incoming, limits).map_err(Failure::into_anyhow)?;

    let files = scan_theme_dir_checked(&incoming, limits).map_err(Failure::into_anyhow)?;
    let bytes =
        std::fs::read(incoming.join("manifest.json")).context("couldn't read manifest.json")?;
    let id = parse_manifest(&bytes, &files, limits)?.id;

    let ready = stage.join(&id);
    rename_retry(&incoming, &ready).context("couldn't prepare the theme")?;
    // Validation only: the URLs are thrown away.
    let placeholder = AssetUrlBase {
        origin: "http://127.0.0.1:0".to_string(),
        token: None,
    };
    let loaded = load_theme_dir_checked(&ready, &id, &placeholder, limits, true)
        .map_err(Failure::into_anyhow)?;
    Ok((id, ready, loaded))
}

fn stage_in(
    stage: &Path,
    zip: &Path,
    root: &Path,
    limits: &ThemeLimits,
    streams: &AssetStreams,
) -> anyhow::Result<String> {
    use anyhow::Context;

    let (id, ready, _) = stage_theme(stage, zip, limits)?;

    let target = root.join(&id);
    let old = backup_dir(root, &id);
    // A streamed file (a video background) would keep the folder from moving
    // on Windows. Requests that arrive during the swap are turned away.
    let closing = streams.close(&id);
    // A backup kept by an earlier failed install of this theme: put it back if
    // the theme is gone, otherwise it is stale.
    if std::fs::symlink_metadata(&old).is_ok() {
        if std::fs::symlink_metadata(&target).is_err() {
            if let Err(e) = rename_retry(&old, &target) {
                tracing::warn!("couldn't put the kept backup {old:?} back: {e}");
            }
        } else if let Err(e) = remove_path(&old) {
            tracing::warn!("couldn't remove the stale backup {old:?}: {e}");
        }
    }
    let had_old = std::fs::symlink_metadata(&target).is_ok();
    if had_old {
        rename_retry(&target, &old).context("couldn't replace the installed theme")?;
    }
    if let Err(e) = rename_retry(&ready, &target) {
        if had_old {
            if let Err(restore) = rename_retry(&old, &target) {
                tracing::error!(
                    "theme \"{id}\": couldn't install it ({e}) or put the previous version back ({restore}); the previous version is kept at {old:?}"
                );
                anyhow::bail!(
                    "couldn't install the theme, and the previous version couldn't be put back; it was kept in the themes folder as \"{}\"",
                    old.file_name().unwrap_or_default().to_string_lossy()
                );
            }
        }
        return Err(anyhow::Error::new(e).context("couldn't install the theme"));
    }
    // The new folder is in place, so requests can be served again. Deleting the
    // replaced folder (slow for a big theme) doesn't need them turned away.
    drop(closing);
    if had_old {
        if let Err(e) = remove_path(&old) {
            tracing::warn!("couldn't remove the replaced theme {old:?}: {e}");
        }
    }
    Ok(id)
}

/// The largest preview image sent inline (the rspc payload stays small).
const MAX_PREVIEW_BYTES: u64 = 2_097_152;

#[derive(Debug)]
pub struct InspectedTheme {
    pub manifest: ValidManifest,
    /// `data:<content type>;base64,<bytes>`; none when the theme has no
    /// preview or it is over `MAX_PREVIEW_BYTES`.
    pub preview: Option<String>,
    /// A theme with this id is already installed.
    pub installed: bool,
}

/// Runs every check `install_theme_zip` runs, without touching the installed
/// themes. The staging folder is removed whatever the result.
pub async fn inspect_theme_zip(
    zip: &Path,
    themes_root: &Path,
    limits: &ThemeLimits,
) -> anyhow::Result<InspectedTheme> {
    check_zip_file(zip, limits).await?;

    let zip = zip.to_path_buf();
    let root = themes_root.to_path_buf();
    let limits = limits.clone();
    tokio::task::spawn_blocking(move || {
        in_scratch(&root, STAGING_INSPECT, |stage| {
            let (id, ready, loaded) = stage_theme(stage, &zip, &limits)?;
            let manifest = loaded.manifest;
            let preview = manifest
                .preview
                .as_deref()
                .and_then(|rel| inline_preview(&ready, rel));
            let installed = std::fs::symlink_metadata(root.join(&id)).is_ok();
            Ok(InspectedTheme {
                manifest,
                preview,
                installed,
            })
        })
    })
    .await?
}

fn inline_preview(dir: &Path, rel: &str) -> Option<String> {
    use base64::Engine;

    let path = dir.join(rel);
    let meta = std::fs::symlink_metadata(&path).ok()?;
    if !meta.is_file() || meta.len() > MAX_PREVIEW_BYTES {
        return None;
    }
    let kind = content_type(rel)?;
    let bytes = std::fs::read(&path).ok()?;
    Some(format!(
        "data:{kind};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

/// The folder to open in the file manager: the themes root (created), or an
/// installed theme's own folder. Only a real directory directly inside the
/// root qualifies, so a file, a symlink or a junction is refused.
///
/// A refusal is a `ThemeError`; a disk failure is a plain error.
pub(crate) fn folder_to_open(themes_root: &Path, id: Option<&str>) -> anyhow::Result<PathBuf> {
    use anyhow::Context;

    let Some(id) = id else {
        std::fs::create_dir_all(themes_root).context("couldn't create the themes folder")?;
        return Ok(themes_root.to_path_buf());
    };
    let dir = theme_dir(themes_root, id)?;
    refuse_linked_folder(&dir)?;
    match std::fs::symlink_metadata(&dir) {
        Ok(m) if m.is_dir() => Ok(dir),
        _ => Err(ThemeError::Invalid("the theme folder isn't a folder".to_string()).into()),
    }
}

/// Deletes an installed theme's folder (and any backup a failed replace kept)
/// after ending its asset streams.
pub async fn remove_theme_dir(
    themes_root: &Path,
    id: &str,
    streams: &Arc<AssetStreams>,
) -> anyhow::Result<()> {
    let dir = theme_dir(themes_root, id)?;
    let backup = backup_dir(themes_root, id);
    let (id, streams) = (id.to_string(), streams.clone());
    tokio::task::spawn_blocking(move || {
        use anyhow::Context;
        let _closing = streams.close(&id);
        // A link is unlinked, never followed.
        match std::fs::symlink_metadata(&dir) {
            Ok(m) if m.file_type().is_symlink() => std::fs::remove_file(&dir)
                .or_else(|_| std::fs::remove_dir(&dir))
                .context("couldn't remove the theme")?,
            _ => std::fs::remove_dir_all(&dir).context("couldn't remove the theme")?,
        }
        // A backup kept by a failed install would bring the theme back.
        if let Err(e) = remove_path(&backup) {
            tracing::warn!("couldn't remove the kept backup {backup:?}: {e}");
        }
        Ok(())
    })
    .await?
}

/// Every theme folder under the root, sorted by id. Hidden folders (staging,
/// dot-folders) and stray files are skipped.
pub fn list_theme_dirs(
    themes_root: &Path,
    base: &AssetUrlBase,
) -> anyhow::Result<Vec<(String, Result<LoadedTheme, ThemeError>)>> {
    let entries = match std::fs::read_dir(themes_root) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut ids = Vec::new();
    for entry in entries {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if name.starts_with('.') || !entry.path().is_dir() {
            continue;
        }
        ids.push(name);
    }
    ids.sort();
    Ok(ids
        .into_iter()
        .map(|id| {
            let loaded = load_theme_dir(&themes_root.join(&id), &id, base, &ThemeLimits::DEFAULT);
            (id, loaded)
        })
        .collect())
}

/// Sorted names of the non-hidden folders under the themes root; empty when
/// the root is missing.
pub(crate) fn folder_names(themes_root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(themes_root) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_string();
            (!name.starts_with('.') && e.path().is_dir()).then_some(name)
        })
        .collect();
    names.sort();
    names
}

/// One tick of the folder-list watch. False for the first call (the baseline),
/// then true only when the set of folders changed.
pub(crate) fn list_step(prev: &mut Option<Vec<String>>, now: Vec<String>) -> bool {
    match prev {
        None => {
            *prev = Some(now);
            false
        }
        Some(before) if *before == now => false,
        Some(before) => {
            *before = now;
            true
        }
    }
}

/// What a watcher tick invalidates. An edit to the active theme also refreshes
/// the list, so a typo shows on its tile.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct WatchInvalidations {
    pub list: bool,
    pub theme: bool,
}

pub(crate) fn watch_invalidations(
    active_changed: bool,
    folders_changed: bool,
) -> WatchInvalidations {
    WatchInvalidations {
        list: active_changed || folders_changed,
        theme: active_changed,
    }
}

/// What the watcher remembers between ticks.
#[derive(Default)]
pub(crate) struct WatchState {
    pub key: Option<String>,
    /// `Some(None)` is a baseline of "the folder can't be scanned".
    pub fingerprint: Option<Option<u64>>,
}

/// One watcher tick. True means the active installed theme changed since the
/// last tick; a new active theme only records a baseline (false).
pub(crate) fn watch_step(state: &mut WatchState, active: Option<&str>, themes_root: &Path) -> bool {
    let Some(id) = active
        .and_then(|k| k.strip_prefix("user:"))
        .filter(|id| is_valid_theme_id(id))
    else {
        *state = WatchState::default();
        return false;
    };
    let key = active.map(str::to_string);
    let current = scan_theme_dir(&themes_root.join(id), &ThemeLimits::DEFAULT)
        .ok()
        .map(|f| f.fingerprint);
    if state.key != key {
        state.key = key;
        state.fingerprint = Some(current);
        return false;
    }
    if state.fingerprint != Some(current) {
        state.fingerprint = Some(current);
        return true;
    }
    false
}

/// What the watcher loop carries from one tick to the next.
#[derive(Default)]
pub(crate) struct Watch {
    active: WatchState,
    folders: Option<Vec<String>>,
}

/// Reads a JSON frontend-preference row. A missing row, or one that isn't valid
/// JSON for `T`, reads as the default (the latter with a warning); a failed
/// read is an error, so no caller saves a default over the real row.
pub(crate) async fn load_json_pref<T: serde::de::DeserializeOwned + Default>(
    db: &carbon_repos::db_exec::Db,
    key: &str,
    what: &str,
) -> anyhow::Result<T> {
    let row = carbon_repos::repos::frontend_preference::get_preference(db, key).await?;
    Ok(match row {
        None => T::default(),
        Some(row) => serde_json::from_str(&row.value).unwrap_or_else(|e| {
            tracing::warn!("the {what} is unreadable, using the defaults: {e}");
            T::default()
        }),
    })
}

const WATCH_INTERVAL: std::time::Duration = std::time::Duration::from_millis(1500);

pub struct ThemeManager {
    active: Mutex<Option<String>>,
    ops: Arc<tokio::sync::Mutex<()>>,
    /// Held across every read-modify-write of the season state.
    season: Arc<tokio::sync::Mutex<()>>,
    /// Held across every read-modify-write of the theme preferences.
    prefs: Arc<tokio::sync::Mutex<()>>,
    /// The asset route's open streams, which install and remove end first.
    streams: Arc<AssetStreams>,
}

impl ThemeManager {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(None),
            ops: Arc::new(tokio::sync::Mutex::new(())),
            season: Arc::new(tokio::sync::Mutex::new(())),
            prefs: Arc::new(tokio::sync::Mutex::new(())),
            streams: Arc::default(),
        }
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagerRef<'_, ThemeManager> {
    pub fn themes_root(self) -> PathBuf {
        self.app
            .settings_manager()
            .runtime_path
            .get_themes()
            .to_path()
    }

    /// What the asset route takes a ticket from for every file it serves.
    pub fn asset_streams(self) -> Arc<AssetStreams> {
        self.streams.clone()
    }

    pub async fn get_theme(self, id: &str, base: &AssetUrlBase) -> anyhow::Result<LoadedTheme> {
        let dir = theme_dir(&self.themes_root(), id)?;
        let id = id.to_string();
        let base = AssetUrlBase {
            origin: base.origin.clone(),
            token: base.token.clone(),
        };
        let loaded = tokio::task::spawn_blocking(move || {
            load_theme_dir(&dir, &id, &base, &ThemeLimits::DEFAULT)
        })
        .await??;
        Ok(loaded)
    }

    pub async fn list(
        self,
        base: &AssetUrlBase,
    ) -> anyhow::Result<Vec<(String, Result<LoadedTheme, ThemeError>)>> {
        let root = self.themes_root();
        let base = AssetUrlBase {
            origin: base.origin.clone(),
            token: base.token.clone(),
        };
        tokio::task::spawn_blocking(move || list_theme_dirs(&root, &base)).await?
    }

    fn invalidate_themes(self) {
        self.app.invalidate(LIST, None);
        self.app.invalidate(GET_THEME, None);
    }

    /// Runs on its own task holding the ops lock, so a caller that goes away
    /// halfway (a reloaded renderer) can't leave the work running unlocked.
    pub async fn install(self, zip: PathBuf) -> anyhow::Result<String> {
        let app = self.app.clone();
        tokio::spawn(async move {
            let themes = app.theme_manager();
            let _ops = themes.ops.clone().lock_owned().await;
            let result = install_theme_zip(
                &zip,
                &themes.themes_root(),
                &ThemeLimits::DEFAULT,
                &themes.streams,
            )
            .await;
            themes.invalidate_themes();
            result
        })
        .await?
    }

    /// Checks a `.gdltheme` as install would, without installing it. Runs on
    /// its own task holding the ops lock, like `install`.
    pub async fn inspect(self, zip: PathBuf) -> anyhow::Result<InspectedTheme> {
        let app = self.app.clone();
        tokio::spawn(async move {
            let themes = app.theme_manager();
            let _ops = themes.ops.clone().lock_owned().await;
            inspect_theme_zip(&zip, &themes.themes_root(), &ThemeLimits::DEFAULT).await
        })
        .await?
    }

    /// Opens the themes folder (`None`) or one theme's folder.
    pub async fn open_folder(self, id: Option<String>) -> anyhow::Result<()> {
        let root = self.themes_root();
        tokio::task::spawn_blocking(move || {
            let dir = folder_to_open(&root, id.as_deref())?;
            opener::open(dir)?;
            Ok(())
        })
        .await?
    }

    pub async fn remove(self, id: &str) -> anyhow::Result<()> {
        let app = self.app.clone();
        let id = id.to_string();
        tokio::spawn(async move {
            let themes = app.theme_manager();
            let _ops = themes.ops.clone().lock_owned().await;
            // The saved setting is the authority on what is active.
            let active = app.settings_manager().get_settings().await?.theme;
            let was_active = active == format!("user:{id}");
            // The settings move to "main" before the folder goes, so a renderer
            // that refetches in between never asks for a theme that is gone.
            let result = async {
                if was_active {
                    app.settings_manager()
                        .set(AppConfigurationPatch {
                            theme: Some("main".to_string()),
                            ..Default::default()
                        })
                        .await?;
                    themes.set_active("main");
                    app.invalidate(GET_SETTINGS, None);
                }
                remove_theme_dir(&themes.themes_root(), &id, &themes.streams).await
            }
            .await;
            if was_active {
                // The active theme's own query follows the settings, not the list.
                app.invalidate(LIST, None);
            } else {
                themes.invalidate_themes();
            }
            result
        })
        .await?
    }

    pub fn set_active(self, key: &str) {
        if let Ok(mut active) = self.active.lock() {
            *active = Some(key.to_string());
        }
    }

    /// Starts the loop that watches the themes folder: an edit to the active
    /// installed theme reloads it and its list entry, and a theme folder added
    /// or removed reloads the list. It holds the app weakly and ends when the
    /// app is dropped.
    pub fn launch_watcher(self) {
        let app = Arc::downgrade(self.app);
        tokio::spawn(async move {
            let mut watch = Watch::default();
            loop {
                tokio::time::sleep(WATCH_INTERVAL).await;
                let Some(app) = app.upgrade() else { return };
                watch = match app.theme_manager().watch_tick(watch).await {
                    Ok(next) => next,
                    Err(e) => {
                        tracing::warn!("the theme watcher stopped: {e}");
                        return;
                    }
                };
            }
        });
    }

    /// One watcher tick. It is skipped while an install or remove holds `ops`:
    /// between the two renames of a replace the active theme's folder is
    /// briefly gone, which would fall back to main. The next tick catches up.
    pub(crate) async fn watch_tick(self, mut watch: Watch) -> anyhow::Result<Watch> {
        let unset = self.active.lock().map(|a| a.is_none()).unwrap_or(false);
        if unset {
            if let Ok(settings) = self.app.settings_manager().get_settings().await {
                if let Ok(mut a) = self.active.lock() {
                    a.get_or_insert(settings.theme);
                }
            }
        }
        let Ok(ops) = self.ops.clone().try_lock_owned() else {
            return Ok(watch);
        };
        let active = self.active.lock().ok().and_then(|a| a.clone());
        let root = self.themes_root();
        let (watch, changed, list_changed) = tokio::task::spawn_blocking(move || {
            let changed = watch_step(&mut watch.active, active.as_deref(), &root);
            let list_changed = list_step(&mut watch.folders, folder_names(&root));
            (watch, changed, list_changed)
        })
        .await?;
        drop(ops);
        let todo = watch_invalidations(changed, list_changed);
        if todo.list {
            self.app.invalidate(LIST, None);
        }
        if todo.theme {
            self.app.invalidate(GET_THEME, None);
        }
        Ok(watch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn base() -> AssetUrlBase {
        AssetUrlBase {
            origin: "http://127.0.0.1:4650".into(),
            token: Some("tok".into()),
        }
    }

    fn manifest(id: &str) -> String {
        format!(
            r#"{{"themeApi":1,"id":"{id}","name":"Demo","author":"me","version":"1.0.0",
            "preview":"assets/a.png","colors":{{"primary":"1 2 3"}},
            "fonts":[{{"family":"F","src":"assets/f.woff2"}}],"css":"theme.css"}}"#
        )
    }

    /// `root/themes/<folder>` holding a theme whose manifest id is `id`.
    fn theme(folder: &str, id: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join(folder);
        fs::create_dir_all(dir.join("assets")).unwrap();
        fs::write(dir.join("manifest.json"), manifest(id)).unwrap();
        fs::write(dir.join("theme.css"), "a{background:url(assets/a.png)}").unwrap();
        fs::write(dir.join("assets/a.png"), "png").unwrap();
        fs::write(dir.join("assets/f.woff2"), "font").unwrap();
        fs::write(dir.join("assets/My Snow ❄.svg"), "<svg/>").unwrap();
        root
    }

    #[test]
    fn loads_a_theme_folder() {
        let root = theme("demo", "demo");
        let t = load_theme_dir(
            &root.path().join("demo"),
            "demo",
            &base(),
            &ThemeLimits::DEFAULT,
        )
        .unwrap();
        let css = t.css.unwrap();
        assert!(
            css.contains("/theme/asset?id=demo&path=assets%2Fa.png&v="),
            "{css}"
        );
        assert!(css.contains("&_token=tok"), "{css}");
        assert!(t.font_urls[0].contains("path=assets%2Ff.woff2"));
        assert!(t.preview_url.is_some());
    }

    #[test]
    fn loaded_theme_maps_every_asset_to_a_url() {
        let root = theme("demo", "demo");
        let dir = root.path().join("demo");
        fs::write(dir.join("assets/smoke.svg"), "<svg/>").unwrap();
        fs::write(dir.join("assets/bg.png"), "png").unwrap();
        fs::write(dir.join("assets/d2.png"), "png").unwrap();
        let m = manifest("demo").replacen(
            r#""css":"theme.css""#,
            r#""css":"theme.css",
            "background":{"src":"assets/bg.png","motion":["assets/smoke.svg"]},
            "decorations":[{"anchor":"nav","edge":"top","height":5,
              "images":["assets/My Snow ❄.svg","assets/d2.png"]}]"#,
            1,
        );
        fs::write(dir.join("manifest.json"), m).unwrap();
        let t = load_theme_dir(&dir, "demo", &base(), &ThemeLimits::DEFAULT).unwrap();
        for key in [
            "assets/bg.png",
            "assets/smoke.svg",
            "assets/My Snow ❄.svg",
            "assets/d2.png",
        ] {
            let u = &t.asset_urls[key];
            assert!(u.contains("path=assets%2F"), "{u}");
        }
        assert!(t.asset_urls.contains_key("assets/f.woff2"));
        assert!(t.asset_urls.contains_key("assets/a.png"));
        assert_eq!(t.asset_urls.len(), 6);

        let fe = crate::api::themes::FEResolvedTheme::from(t);
        let bg = fe.background.unwrap();
        assert!(bg.src.contains("path=assets%2Fbg.png"), "{}", bg.src);
        assert!(bg.motion[0].contains("path=assets%2Fsmoke.svg"));
        for img in &fe.decorations[0].images {
            assert!(img.contains("path=assets%2F"), "{img}");
        }
    }

    #[test]
    fn loaded_theme_maps_sounds_and_particles_to_urls() {
        let root = theme("demo", "demo");
        let dir = root.path().join("demo");
        fs::write(dir.join("assets/launch.ogg"), "ogg").unwrap();
        fs::write(dir.join("assets/p1.png"), "png").unwrap();
        fs::write(dir.join("assets/p2.png"), "png").unwrap();
        let m = manifest("demo").replacen(
            r#""css":"theme.css""#,
            r#""css":"theme.css",
            "layout":{"nav":"rail","panels":false,"tileStyle":"caption","tileSize":3},
            "sounds":{"launch":"assets/launch.ogg"},
            "effects":[{"type":"particles","preset":"float","images":["assets/p1.png","assets/p2.png"]}]"#,
            1,
        );
        fs::write(dir.join("manifest.json"), m).unwrap();
        let t = load_theme_dir(&dir, "demo", &base(), &ThemeLimits::DEFAULT).unwrap();
        for key in ["assets/launch.ogg", "assets/p1.png", "assets/p2.png"] {
            let u = &t.asset_urls[key];
            assert!(u.contains("path=assets%2F"), "{u}");
        }
        let fe = crate::api::themes::FEResolvedTheme::from(t);
        // the whole manifest is accepted as written
        assert!(fe.warnings.is_empty(), "{:?}", fe.warnings);
        assert_eq!(fe.layout.nav.as_deref(), Some("rail"));
        assert_eq!(fe.layout.panels, Some(false));
        assert_eq!(fe.layout.tile_style.as_deref(), Some("caption"));
        assert_eq!(fe.layout.tile_size, Some(3));
        let json = serde_json::to_value(&fe.layout).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"nav":"rail","panels":false,"tileStyle":"caption","tileSize":3})
        );
        let launch = fe.sounds.launch.unwrap();
        assert!(launch.contains("path=assets%2Flaunch.ogg"), "{launch}");
        match &fe.effects[0] {
            crate::api::themes::FEThemeEffect::Particles { images, .. } => {
                assert_eq!(images.len(), 2);
                for img in images {
                    assert!(img.contains("path=assets%2Fp"), "{img}");
                }
            }
            other => panic!("expected particles, got {other:?}"),
        }
    }

    #[test]
    fn an_animated_svg_background_warns_when_the_theme_loads() {
        let root = theme("demo", "demo");
        let dir = root.path().join("demo");
        fs::write(
            dir.join("assets/bg.svg"),
            "<svg><animate attributeName='x'/></svg>",
        )
        .unwrap();
        let m = manifest("demo").replacen(
            r#""css":"theme.css""#,
            r#""css":"theme.css","background":{"src":"assets/bg.svg"}"#,
            1,
        );
        fs::write(dir.join("manifest.json"), m).unwrap();
        let t = load_theme_dir(&dir, "demo", &base(), &ThemeLimits::DEFAULT).unwrap();
        assert!(
            t.manifest
                .warnings
                .iter()
                .any(|w| w.contains("background.src") && w.contains("animated SVG")),
            "{:?}",
            t.manifest.warnings
        );
    }

    #[test]
    fn asset_urls_encode_paths() {
        assert_eq!(
            base().asset_url("demo", "assets/My Snow ❄.svg", 255),
            "http://127.0.0.1:4650/theme/asset?id=demo&path=assets%2FMy+Snow+%E2%9D%84.svg&v=ff&_token=tok"
        );
        let u = base().asset_url("demo", "assets/a#b&c?d%e.png", 1);
        assert!(u.contains("path=assets%2Fa%23b%26c%3Fd%25e.png&v=1"), "{u}");
        let no_token = AssetUrlBase {
            origin: "http://x".into(),
            token: None,
        };
        assert!(!no_token.asset_url("d", "a.png", 1).contains("_token"));
    }

    #[test]
    fn folder_must_match_the_manifest_id() {
        let root = theme("demo", "other");
        let r = load_theme_dir(
            &root.path().join("demo"),
            "demo",
            &base(),
            &ThemeLimits::DEFAULT,
        );
        match r {
            Err(ThemeError::Invalid(m)) => assert!(m.contains("must match"), "{m}"),
            other => panic!("expected Invalid, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn theme_dir_reports_missing_and_bad_ids() {
        let root = theme("demo", "demo");
        assert!(theme_dir(root.path(), "demo").is_ok());
        assert!(matches!(
            theme_dir(root.path(), "nope"),
            Err(ThemeError::NotFound(_))
        ));
        assert!(matches!(
            theme_dir(root.path(), "../demo"),
            Err(ThemeError::Invalid(_))
        ));
    }

    #[test]
    fn asset_route_refuses_paths_outside_the_theme() {
        let root = theme("demo", "demo");
        fs::create_dir_all(root.path().join("other")).unwrap();
        fs::write(root.path().join("other/manifest.json"), "{}").unwrap();
        fs::write(root.path().join("demo/.hidden.png"), "x").unwrap();
        fs::write(root.path().join("demo/OFL.txt"), "licence").unwrap();
        let r = root.path();
        for (id, rel) in [
            ("demo", "OFL.txt"),
            ("demo", "../other/manifest.json"),
            ("demo", "assets/../../x.png"),
            ("demo", "manifest.json"),
            ("demo", "theme.css"),
            ("demo", ".hidden.png"),
            ("../demo", "assets/a.png"),
        ] {
            assert!(resolve_asset_path(r, id, rel).is_err(), "{id} {rel}");
        }
        let (_, ct) = resolve_asset_path(r, "demo", "assets/a.png").unwrap();
        assert_eq!(ct, "image/png");
        let (_, ct) = resolve_asset_path(r, "demo", "assets/My Snow ❄.svg").unwrap();
        assert_eq!(ct, "image/svg+xml");
    }

    #[cfg(unix)]
    #[test]
    fn asset_route_refuses_symlinks() {
        use std::os::unix::fs::symlink;
        let root = theme("demo", "demo");
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret.png"), "x").unwrap();
        let demo = root.path().join("demo");
        symlink(
            outside.path().join("secret.png"),
            demo.join("assets/link.png"),
        )
        .unwrap();
        assert!(resolve_asset_path(root.path(), "demo", "assets/link.png").is_err());

        fs::create_dir(demo.join("assets2")).unwrap();
        fs::remove_dir(demo.join("assets2")).unwrap();
        symlink(outside.path(), demo.join("linked")).unwrap();
        assert!(resolve_asset_path(root.path(), "demo", "linked/secret.png").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_theme_folder_is_refused() {
        use std::os::unix::fs::symlink;
        let real = theme("real", "demo");
        let root = tempfile::tempdir().unwrap();
        symlink(real.path().join("real"), root.path().join("demo")).unwrap();
        let r = load_theme_dir(
            &root.path().join("demo"),
            "demo",
            &base(),
            &ThemeLimits::DEFAULT,
        );
        match r {
            Err(ThemeError::Invalid(m)) => assert!(m.contains("can't be a link"), "{m}"),
            other => panic!("expected Invalid, got {:?}", other.map(|_| ())),
        }
        assert!(resolve_asset_path(root.path(), "demo", "assets/a.png").is_err());
    }

    #[test]
    fn warnings_are_capped() {
        let many: Vec<String> = (0..25).map(|i| format!("w{i}")).collect();
        let capped = cap_warnings(many);
        assert_eq!(capped.len(), 21);
        assert_eq!(capped[20], "and 5 more");
        assert_eq!(cap_warnings(vec!["a".into()]), vec!["a".to_string()]);
    }

    #[test]
    fn manifest_warnings_reach_the_result_capped() {
        let root = theme("demo", "demo");
        let mut extra = String::new();
        for i in 0..30 {
            extra.push_str(&format!(r#","x{i}":1"#));
        }
        let m = manifest("demo");
        let m = m.trim_end().trim_end_matches('}').to_string() + &extra + "}";
        fs::write(root.path().join("demo/manifest.json"), m).unwrap();
        let t = load_theme_dir(
            &root.path().join("demo"),
            "demo",
            &base(),
            &ThemeLimits::DEFAULT,
        )
        .unwrap();
        assert_eq!(t.manifest.warnings.len(), 21);
        assert_eq!(t.manifest.warnings[20], "and 10 more");
    }

    // ---- install, remove, watch ----

    fn zip_of(entries: &[(&str, &[u8])]) -> (tempfile::TempDir, PathBuf) {
        use std::io::Write;
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("t.gdltheme");
        let mut z = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (n, b) in entries {
            z.start_file(*n, opts).unwrap();
            z.write_all(b).unwrap();
        }
        z.finish().unwrap();
        (d, path)
    }

    fn min_manifest(id: &str) -> String {
        format!(
            r#"{{"themeApi":1,"id":"{id}","name":"Demo","author":"me","version":"1.0.0","css":"theme.css"}}"#
        )
    }

    struct Env {
        _dir: tempfile::TempDir,
        root: PathBuf,
        streams: Arc<AssetStreams>,
    }

    fn env() -> Env {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("themes");
        Env {
            _dir: dir,
            root,
            streams: Arc::default(),
        }
    }

    async fn install(env: &Env, zip: &Path) -> anyhow::Result<String> {
        install_theme_zip(zip, &env.root, &ThemeLimits::DEFAULT, &env.streams).await
    }

    async fn remove(env: &Env, id: &str) -> anyhow::Result<()> {
        remove_theme_dir(&env.root, id, &env.streams).await
    }

    /// Hidden entries (staging folders and kept backups) under the themes root.
    fn staging(root: &Path) -> Vec<String> {
        fs::read_dir(root)
            .map(|d| {
                d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                    .filter(|n| n.starts_with('.'))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn leftovers(dir: &Path) -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect()
    }

    #[tokio::test]
    async fn installs_a_theme_zip() {
        let env = env();
        let m = min_manifest("demo");
        let (_d, zip) = zip_of(&[("manifest.json", m.as_bytes()), ("theme.css", b"a{}")]);
        assert_eq!(install(&env, &zip).await.unwrap(), "demo");
        assert!(env.root.join("demo/manifest.json").exists());
        assert!(staging(&env.root).is_empty());
    }

    #[tokio::test]
    async fn a_malformed_season_does_not_refuse_an_install() {
        let env = env();
        let m = min_manifest("demo").replacen(
            "{",
            r#"{"season":{"available":["13-45"],"art":"nope.png"},"#,
            1,
        );
        let (_d, zip) = zip_of(&[("manifest.json", m.as_bytes()), ("theme.css", b"a{}")]);
        assert_eq!(install(&env, &zip).await.unwrap(), "demo");
        let t = load_theme_dir(
            &env.root.join("demo"),
            "demo",
            &base(),
            &ThemeLimits::DEFAULT,
        )
        .unwrap();
        assert_eq!(
            t.manifest.warnings,
            vec!["season is only used by built-in themes and was ignored"]
        );
    }

    #[tokio::test]
    async fn installs_themes_named_like_the_staging_folders() {
        let env = env();
        for id in ["old", "theme", "incoming"] {
            let m = min_manifest(id);
            let (_d, zip) = zip_of(&[("manifest.json", m.as_bytes()), ("theme.css", b"a{}")]);
            assert_eq!(install(&env, &zip).await.unwrap(), id);
            // And again, so the replace path runs with the same names.
            assert_eq!(install(&env, &zip).await.unwrap(), id);
            assert!(env.root.join(id).join("manifest.json").exists());
        }
        assert!(staging(&env.root).is_empty());
    }

    #[test]
    fn staging_names_are_never_theme_ids() {
        for name in [
            STAGING_NEW.to_string(),
            STAGING_OLD.to_string(),
            backup_dir(Path::new(""), "demo")
                .to_string_lossy()
                .into_owned(),
            STAGING_INCOMING.to_string(),
            STAGING_INSPECT.to_string(),
        ] {
            assert!(!is_valid_theme_id(&name), "{name}");
        }
    }

    #[tokio::test]
    async fn a_stale_backup_is_deleted_when_its_theme_is_installed() {
        let env = env();
        let m = min_manifest("demo");
        let (_d, zip) = zip_of(&[("manifest.json", m.as_bytes()), ("theme.css", b"a{}")]);
        install(&env, &zip).await.unwrap();
        fs::create_dir_all(env.root.join(".staging-old-demo")).unwrap();
        fs::write(env.root.join(".staging-old-demo/marker"), "x").unwrap();
        install(&env, &zip).await.unwrap();
        assert!(!env.root.join(".staging-old-demo").exists());
        assert!(env.root.join("demo/manifest.json").exists());
    }

    #[tokio::test]
    async fn removing_a_theme_drops_its_kept_backup() {
        let env = env();
        let m = min_manifest("demo");
        let (_d, zip) = zip_of(&[("manifest.json", m.as_bytes()), ("theme.css", b"a{}")]);
        install(&env, &zip).await.unwrap();
        fs::create_dir_all(env.root.join(".staging-old-demo")).unwrap();
        remove(&env, "demo").await.unwrap();
        assert!(!env.root.join("demo").exists());
        assert!(!env.root.join(".staging-old-demo").exists());
    }

    /// A zipped theme `id` holding the asset `rel`, as `(its temp dir, the zip)`.
    fn theme_with(id: &str, rel: &str) -> (tempfile::TempDir, PathBuf) {
        let m = min_manifest(id);
        zip_of(&[
            ("manifest.json", m.as_bytes()),
            ("theme.css", b"a{}"),
            (rel, b"x"),
        ])
    }

    /// Holds a stream ticket for `id` until it is cancelled, then reports
    /// whether `still` existed at that moment and lets the ticket go.
    fn hold_stream(
        streams: &AssetStreams,
        id: &str,
        still: PathBuf,
    ) -> tokio::task::JoinHandle<bool> {
        let ticket = streams.ticket(id);
        tokio::spawn(async move {
            ticket.cancelled().await;
            still.exists()
        })
    }

    #[tokio::test]
    async fn replacing_a_theme_ends_its_streams_before_the_swap() {
        let env = env();
        let (_a, v1) = theme_with("demo", "assets/old.png");
        let (_b, v2) = theme_with("demo", "assets/new.png");
        install(&env, &v1).await.unwrap();
        let other = env.streams.ticket("other");
        let held = hold_stream(&env.streams, "demo", env.root.join("demo/assets/old.png"));

        install(&env, &v2).await.unwrap();
        let before_swap = tokio::time::timeout(std::time::Duration::from_secs(5), held)
            .await
            .expect("the stream was never ended")
            .unwrap();
        assert!(before_swap, "the stream ended before the old folder moved");
        assert!(env.root.join("demo/assets/new.png").exists());
        assert!(!other.is_cancelled());
        // Later streams of the theme get a fresh token.
        assert!(!env.streams.ticket("demo").is_cancelled());
    }

    #[tokio::test]
    async fn removing_a_theme_ends_its_streams_first() {
        let env = env();
        let (_a, zip) = theme_with("demo", "assets/a.png");
        install(&env, &zip).await.unwrap();
        let other = env.streams.ticket("other");
        let held = hold_stream(&env.streams, "demo", env.root.join("demo/assets/a.png"));

        remove(&env, "demo").await.unwrap();
        let before_delete = tokio::time::timeout(std::time::Duration::from_secs(5), held)
            .await
            .expect("the stream was never ended")
            .unwrap();
        assert!(
            before_delete,
            "the stream ended before the folder was deleted"
        );
        assert!(!env.root.join("demo").exists());
        assert!(!other.is_cancelled());
        assert!(!env.streams.ticket("demo").is_cancelled());
    }

    #[tokio::test]
    async fn the_manager_ends_the_routes_streams_on_remove() {
        let app = crate::setup_managers_for_test().await;
        let root = app.settings_manager().runtime_path.get_themes().to_path();
        let dir = root.join("demo");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("manifest.json"), min_manifest("demo")).unwrap();
        fs::write(dir.join("theme.css"), "a{}").unwrap();
        // The same streams the asset route takes its tickets from.
        let streams = app.theme_manager().asset_streams();
        let held = hold_stream(&streams, "demo", dir.clone());

        app.theme_manager().remove("demo").await.unwrap();
        let before_delete = tokio::time::timeout(std::time::Duration::from_secs(5), held)
            .await
            .expect("the stream was never ended")
            .unwrap();
        assert!(before_delete);
        assert!(!dir.exists());
    }

    #[tokio::test]
    async fn the_watcher_skips_a_tick_while_an_operation_runs() {
        let mut app = crate::setup_managers_for_test().await;
        let root = app.settings_manager().runtime_path.get_themes().to_path();
        fs::create_dir_all(&root).unwrap();
        let list_refreshed = |app: &mut crate::TestEnv| {
            let mut keys = Vec::new();
            while let Ok(event) = app.invalidation_recv.try_recv() {
                keys.push(event.key);
            }
            keys.contains(&LIST.full)
        };
        let watch = app
            .theme_manager()
            .watch_tick(Watch::default())
            .await
            .unwrap();
        fs::create_dir_all(root.join("added")).unwrap();

        // An install or remove is between its renames. (Holding ops also
        // keeps the app's own watcher from ticking in between.)
        let ops = app.theme_manager().ops.clone().lock_owned().await;
        while app.invalidation_recv.try_recv().is_ok() {}
        let watch = app.theme_manager().watch_tick(watch).await.unwrap();
        assert!(!list_refreshed(&mut app), "the tick was skipped");
        drop(ops);

        app.theme_manager().watch_tick(watch).await.unwrap();
        assert!(list_refreshed(&mut app), "the next tick caught up");
    }

    #[test]
    fn folder_names_skip_hidden_and_files() {
        let dir = tempfile::tempdir().unwrap();
        for d in ["b", "a", ".staging-theme"] {
            fs::create_dir_all(dir.path().join(d)).unwrap();
        }
        fs::write(dir.path().join("x.txt"), "x").unwrap();
        assert_eq!(folder_names(dir.path()), ["a", "b"]);
        assert!(folder_names(&dir.path().join("missing")).is_empty());
    }

    #[test]
    fn list_step_reports_changes_once() {
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let mut prev = None;
        assert!(!list_step(&mut prev, names(&["a"])));
        assert!(!list_step(&mut prev, names(&["a"])));
        assert!(list_step(&mut prev, names(&["a", "b"])));
        assert!(!list_step(&mut prev, names(&["a", "b"])));
    }

    #[test]
    fn an_active_theme_edit_also_refreshes_the_list() {
        assert_eq!(
            watch_invalidations(true, false),
            WatchInvalidations {
                list: true,
                theme: true
            }
        );
        assert_eq!(
            watch_invalidations(false, true),
            WatchInvalidations {
                list: true,
                theme: false
            }
        );
        assert_eq!(
            watch_invalidations(false, false),
            WatchInvalidations {
                list: false,
                theme: false
            }
        );
    }

    #[tokio::test]
    async fn reinstalling_replaces_the_old_files() {
        let env = env();
        let m = min_manifest("demo");
        let (_a, v1) = zip_of(&[
            ("manifest.json", m.as_bytes()),
            ("theme.css", b"a{}"),
            ("assets/old.png", b"x"),
        ]);
        let (_b, v2) = zip_of(&[
            ("manifest.json", m.as_bytes()),
            ("theme.css", b"a{}"),
            ("assets/new.png", b"x"),
        ]);
        install(&env, &v1).await.unwrap();
        install(&env, &v2).await.unwrap();
        assert!(!env.root.join("demo/assets/old.png").exists());
        assert!(env.root.join("demo/assets/new.png").exists());
    }

    #[tokio::test]
    async fn a_failed_install_leaves_the_installed_theme_alone() {
        let env = env();
        let m = min_manifest("demo");
        let (_a, good) = zip_of(&[("manifest.json", m.as_bytes()), ("theme.css", b"a{}")]);
        let (_b, bad) = zip_of(&[
            ("manifest.json", m.as_bytes()),
            ("theme.css", b"a{background:url(https://e.com/a.png)}"),
        ]);
        install(&env, &good).await.unwrap();
        let e = install(&env, &bad).await.unwrap_err();
        assert!(e.to_string().contains("theme.css"), "{e}");
        assert!(matches!(
            e.downcast_ref::<ThemeError>(),
            Some(ThemeError::Invalid(_))
        ));
        assert_eq!(
            fs::read_to_string(env.root.join("demo/theme.css")).unwrap(),
            "a{}"
        );
        assert!(staging(&env.root).is_empty());
    }

    #[tokio::test]
    async fn an_oversized_zip_is_refused_before_it_is_opened() {
        let env = env();
        let d = tempfile::tempdir().unwrap();
        let zip = d.path().join("big.gdltheme");
        let limits = ThemeLimits {
            max_total_bytes: 100,
            ..ThemeLimits::DEFAULT
        };
        fs::write(&zip, vec![0u8; 111]).unwrap();
        let e = install_theme_zip(&zip, &env.root, &limits, &env.streams)
            .await
            .unwrap_err();
        match e.downcast_ref::<ThemeError>() {
            Some(ThemeError::Invalid(m)) => assert!(m.contains("too large"), "{m}"),
            other => panic!("expected Invalid, got {other:?}"),
        }
        // Exactly at the cap is not "too large" (it fails later, as a bad zip).
        fs::write(&zip, vec![0u8; 110]).unwrap();
        let e = install_theme_zip(&zip, &env.root, &limits, &env.streams)
            .await
            .unwrap_err();
        assert!(!e.to_string().contains("too large"), "{e}");
    }

    #[tokio::test]
    async fn io_failures_are_not_reported_as_a_broken_theme() {
        let env = env();
        // The themes root can't be created because a file sits in its place.
        fs::write(&env.root, "not a folder").unwrap();
        let m = min_manifest("demo");
        let (_d, zip) = zip_of(&[("manifest.json", m.as_bytes()), ("theme.css", b"a{}")]);
        let e = install(&env, &zip).await.unwrap_err();
        assert!(
            !matches!(e.downcast_ref::<ThemeError>(), Some(ThemeError::Invalid(_))),
            "{e:?}"
        );
        assert!(staging(&env.root).is_empty());
    }

    #[tokio::test]
    async fn stale_staging_and_backups_are_cleaned_at_the_next_install() {
        let env = env();
        let m = min_manifest("demo");
        let (_d, zip) = zip_of(&[("manifest.json", m.as_bytes()), ("theme.css", b"a{}")]);
        fs::create_dir_all(env.root.join(".staging-theme/junk")).unwrap();
        // A stale backup of this theme (the theme is installed) and a kept
        // backup of another theme, which must be left alone.
        fs::create_dir_all(env.root.join(".staging-old-demo")).unwrap();
        fs::create_dir_all(env.root.join(".staging-old-other")).unwrap();
        install(&env, &zip).await.unwrap();
        install(&env, &zip).await.unwrap();
        assert_eq!(staging(&env.root), [".staging-old-other"]);
    }

    #[tokio::test]
    async fn a_kept_backup_is_put_back_when_its_theme_is_gone() {
        let env = env();
        let m = min_manifest("demo");
        let (_d, zip) = zip_of(&[("manifest.json", m.as_bytes()), ("theme.css", b"a{}")]);
        let old = env.root.join(".staging-old-demo");
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join("marker"), "x").unwrap();
        install(&env, &zip).await.unwrap();
        // The backup was restored and then replaced by the new install.
        assert!(env.root.join("demo/manifest.json").exists());
        assert!(!env.root.join("demo/marker").exists());
        assert!(staging(&env.root).is_empty());
    }

    #[tokio::test]
    async fn removes_an_installed_theme() {
        let env = env();
        let m = min_manifest("demo");
        let (_d, zip) = zip_of(&[("manifest.json", m.as_bytes()), ("theme.css", b"a{}")]);
        install(&env, &zip).await.unwrap();
        remove(&env, "demo").await.unwrap();
        assert!(!env.root.join("demo").exists());
    }

    #[tokio::test]
    async fn remove_unknown_theme_is_not_found() {
        let env = env();
        fs::create_dir_all(&env.root).unwrap();
        let e = remove(&env, "nope").await.unwrap_err();
        assert!(matches!(
            e.downcast_ref::<ThemeError>(),
            Some(ThemeError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn remove_rejects_bad_ids() {
        let env = env();
        fs::create_dir_all(env.root.join("x")).unwrap();
        for id in ["../x", "", "a/b", ".staging-theme"] {
            let e = remove(&env, id).await.unwrap_err();
            assert!(
                matches!(e.downcast_ref::<ThemeError>(), Some(ThemeError::Invalid(_))),
                "{id}"
            );
        }
        assert!(env.root.join("x").exists());
    }

    fn watched() -> (tempfile::TempDir, PathBuf) {
        let root = theme("demo", "demo");
        let path = root.path().to_path_buf();
        (root, path)
    }

    #[test]
    fn installed_themes_cannot_declare_a_season() {
        let root = theme("demo", "demo");
        let dir = root.path().join("demo");
        let m = manifest("demo").replacen(
            r#""css":"theme.css""#,
            r#""css":"theme.css","season":{"available":["12-01","01-31"]}"#,
            1,
        );
        fs::write(dir.join("manifest.json"), m).unwrap();
        let listed = list_theme_dirs(root.path(), &base()).unwrap();
        let t = listed[0].1.as_ref().unwrap();
        assert_eq!(t.manifest.season, None);
        assert_eq!(
            t.manifest.warnings,
            vec!["season is only used by built-in themes and was ignored"]
        );
    }

    #[test]
    fn built_in_style_folders_keep_their_season() {
        let root = theme("demo", "demo");
        let dir = root.path().join("demo");
        let m = manifest("demo").replacen(
            r#""css":"theme.css""#,
            r#""css":"theme.css","season":{"available":["12-01","01-31"]}"#,
            1,
        );
        fs::write(dir.join("manifest.json"), m).unwrap();
        let t = load_builtin_theme_dir(&dir, "demo", &base(), &ThemeLimits::DEFAULT).unwrap();
        assert_eq!(
            t.manifest.season.map(|s| s.available),
            Some(season::SeasonWindow {
                start: season::MonthDay { month: 12, day: 1 },
                end: season::MonthDay { month: 1, day: 31 },
            })
        );
        assert!(t.manifest.warnings.is_empty(), "{:?}", t.manifest.warnings);
        let t = load_theme_dir(&dir, "demo", &base(), &ThemeLimits::DEFAULT).unwrap();
        assert_eq!(t.manifest.season, None);
        assert_eq!(t.manifest.warnings.len(), 1);
    }

    #[test]
    fn a_malformed_season_does_not_refuse_an_installed_theme() {
        let root = theme("demo", "demo");
        let dir = root.path().join("demo");
        let m = manifest("demo").replacen(
            r#""css":"theme.css""#,
            r#""css":"theme.css","season":{"available":["13-45"],"art":"nope.png"}"#,
            1,
        );
        fs::write(dir.join("manifest.json"), m).unwrap();
        let t = load_theme_dir(&dir, "demo", &base(), &ThemeLimits::DEFAULT).unwrap();
        assert_eq!(t.manifest.season, None);
        assert_eq!(
            t.manifest.warnings,
            vec!["season is only used by built-in themes and was ignored"]
        );
        assert!(load_builtin_theme_dir(&dir, "demo", &base(), &ThemeLimits::DEFAULT).is_err());
    }

    #[test]
    fn watch_ignores_builtin_themes() {
        let (_r, root) = watched();
        let mut st = WatchState::default();
        assert!(!watch_step(&mut st, Some("main"), &root));
        assert!(!watch_step(&mut st, Some("main"), &root));
        assert!(!watch_step(&mut st, None, &root));
    }

    #[test]
    fn watch_reports_a_change_once() {
        let (_r, root) = watched();
        let mut st = WatchState::default();
        assert!(!watch_step(&mut st, Some("user:demo"), &root));
        assert!(!watch_step(&mut st, Some("user:demo"), &root));
        fs::write(root.join("demo/theme.css"), "a{color:red;color:blue}").unwrap();
        assert!(watch_step(&mut st, Some("user:demo"), &root));
        assert!(!watch_step(&mut st, Some("user:demo"), &root));
    }

    #[test]
    fn editor_side_files_dont_trigger_a_reload() {
        let (_r, root) = watched();
        let mut st = WatchState::default();
        assert!(!watch_step(&mut st, Some("user:demo"), &root));
        for name in [
            "theme.css~",
            "#theme.css#",
            ".#theme.css",
            "4913",
            "a.css___jb_tmp___",
        ] {
            fs::write(root.join("demo").join(name), "x").unwrap();
        }
        assert!(!watch_step(&mut st, Some("user:demo"), &root));
    }

    #[test]
    fn switching_themes_resets_the_baseline() {
        let (_r, root) = watched();
        let mut st = WatchState::default();
        assert!(!watch_step(&mut st, Some("user:demo"), &root));
        assert!(!watch_step(&mut st, Some("user:other"), &root));
    }

    #[test]
    fn deleting_the_folder_counts_as_a_change() {
        let (_r, root) = watched();
        let mut st = WatchState::default();
        assert!(!watch_step(&mut st, Some("user:demo"), &root));
        fs::remove_dir_all(root.join("demo")).unwrap();
        assert!(watch_step(&mut st, Some("user:demo"), &root));
        assert!(!watch_step(&mut st, Some("user:demo"), &root));
    }

    #[test]
    fn watch_ignores_unsafe_ids() {
        let (_r, root) = watched();
        let mut st = WatchState::default();
        assert!(!watch_step(&mut st, Some("user:../demo"), &root));
        assert!(!watch_step(&mut st, Some("user:../demo"), &root));
    }

    #[tokio::test]
    async fn lists_installed_themes_sorted_and_skips_hidden_folders() {
        let env = env();
        for id in ["zed", "alpha"] {
            let m = min_manifest(id);
            let (_d, zip) = zip_of(&[("manifest.json", m.as_bytes()), ("theme.css", b"a{}")]);
            install(&env, &zip).await.unwrap();
        }
        fs::create_dir_all(env.root.join(".staging-x")).unwrap();
        fs::create_dir_all(env.root.join("broken")).unwrap();
        fs::write(env.root.join("a-file.txt"), "x").unwrap();
        let listed = list_theme_dirs(&env.root, &base()).unwrap();
        let ids: Vec<_> = listed.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(ids, ["alpha", "broken", "zed"]);
        assert!(listed[0].1.is_ok());
        assert!(listed[1].1.is_err());
    }

    #[tokio::test]
    async fn removing_the_active_theme_switches_back_to_main() {
        use carbon_repos::repos::app_configuration::AppConfigurationPatch;
        let mut app = crate::setup_managers_for_test().await;
        let root = app.settings_manager().runtime_path.get_themes().to_path();
        let dir = root.join("demo");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("manifest.json"), min_manifest("demo")).unwrap();
        fs::write(dir.join("theme.css"), "a{}").unwrap();
        app.settings_manager()
            .set(AppConfigurationPatch {
                theme: Some("user:demo".into()),
                ..Default::default()
            })
            .await
            .unwrap();
        app.theme_manager().set_active("user:demo");
        while app.invalidation_recv.try_recv().is_ok() {}
        app.theme_manager().remove("demo").await.unwrap();
        let mut keys = Vec::new();
        while let Ok(event) = app.invalidation_recv.try_recv() {
            keys.push(event.key);
        }
        assert!(keys.contains(&GET_SETTINGS.full), "{keys:?}");
        assert!(keys.contains(&LIST.full), "{keys:?}");
        assert!(!keys.contains(&GET_THEME.full), "{keys:?}");
        assert_eq!(
            app.settings_manager().get_settings().await.unwrap().theme,
            "main"
        );
        assert!(!dir.exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn the_active_theme_is_switched_before_its_folder_is_deleted() {
        use carbon_repos::repos::app_configuration::AppConfigurationPatch;
        use std::os::unix::fs::PermissionsExt;
        let app = crate::setup_managers_for_test().await;
        let root = app.settings_manager().runtime_path.get_themes().to_path();
        let dir = root.join("demo");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("manifest.json"), min_manifest("demo")).unwrap();
        app.settings_manager()
            .set(AppConfigurationPatch {
                theme: Some("user:demo".into()),
                ..Default::default()
            })
            .await
            .unwrap();
        // The folder can't be deleted from a read-only parent.
        fs::set_permissions(&root, fs::Permissions::from_mode(0o555)).unwrap();
        let r = app.theme_manager().remove("demo").await;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        // Running as root deletes it anyway, which leaves nothing to check.
        if r.is_err() {
            assert!(dir.exists());
            assert_eq!(
                app.settings_manager().get_settings().await.unwrap().theme,
                "main"
            );
        }
    }

    fn is_theme_invalid(e: &anyhow::Error) -> bool {
        matches!(e.downcast_ref::<ThemeError>(), Some(ThemeError::Invalid(_)))
    }

    #[tokio::test]
    async fn dropping_an_install_midway_keeps_the_installed_theme_intact() {
        let env = env();
        let m = min_manifest("demo");
        let (_a, v1) = zip_of(&[
            ("manifest.json", m.as_bytes()),
            ("theme.css", b"a{}"),
            ("assets/old.png", b"x"),
        ]);
        let (_b, v2) = zip_of(&[
            ("manifest.json", m.as_bytes()),
            ("theme.css", b"a{}"),
            ("assets/new.png", b"x"),
        ]);
        install(&env, &v1).await.unwrap();

        let (started_tx, started_rx) = std::sync::mpsc::channel::<()>();
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let (root, v2, streams) = (env.root.clone(), v2.clone(), env.streams.clone());
        let task = tokio::spawn(async move {
            install_theme_zip_gated(&v2, &root, &ThemeLimits::DEFAULT, &streams, move || {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            })
            .await
        });
        tokio::task::spawn_blocking(move || started_rx.recv().unwrap())
            .await
            .unwrap();
        // The caller goes away while the blocking work is in flight.
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        release_tx.send(()).unwrap();

        // The work finishes on its own and cleans up after itself.
        for _ in 0..200 {
            if env.root.join("demo/assets/new.png").exists() && staging(&env.root).is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        assert!(staging(&env.root).is_empty());
        assert_eq!(leftovers(&env.root), ["demo"]);
        assert!(env.root.join("demo/assets/new.png").exists());
        assert!(!env.root.join("demo/assets/old.png").exists());
        assert!(env.root.join("demo/manifest.json").exists());
    }

    #[tokio::test]
    async fn a_folder_instead_of_a_file_is_a_plain_error() {
        let env = env();
        let d = tempfile::tempdir().unwrap();
        let e = install(&env, d.path()).await.unwrap_err();
        assert!(!is_theme_invalid(&e), "{e:?}");
        let e = install(&env, &d.path().join("missing.gdltheme"))
            .await
            .unwrap_err();
        assert!(!is_theme_invalid(&e), "{e:?}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn an_unreadable_zip_is_a_plain_error() {
        use std::os::unix::fs::PermissionsExt;
        let env = env();
        let m = min_manifest("demo");
        let (_d, zip) = zip_of(&[("manifest.json", m.as_bytes()), ("theme.css", b"a{}")]);
        fs::set_permissions(&zip, fs::Permissions::from_mode(0o000)).unwrap();
        let readable = fs::File::open(&zip).is_ok(); // running as root
        let r = install(&env, &zip).await;
        fs::set_permissions(&zip, fs::Permissions::from_mode(0o644)).unwrap();
        if !readable {
            let e = r.unwrap_err();
            assert!(!is_theme_invalid(&e), "{e:?}");
            assert!(!e.to_string().contains(&*zip.to_string_lossy()), "{e}");
        }
        assert!(staging(&env.root).is_empty());
    }

    #[tokio::test]
    async fn a_damaged_zip_is_a_broken_theme() {
        let env = env();
        let m = min_manifest("demo");
        let (_d, zip) = zip_of(&[("manifest.json", m.as_bytes()), ("theme.css", b"a{}")]);
        let good = fs::read(&zip).unwrap();

        // Truncated.
        fs::write(&zip, &good[..good.len() / 2]).unwrap();
        let e = install(&env, &zip).await.unwrap_err();
        assert!(is_theme_invalid(&e), "{e:?}");

        // A flipped byte in the data fails its checksum.
        let at = good.windows(3).position(|w| w == b"a{}").unwrap();
        let mut bad = good.clone();
        bad[at] = b'b';
        fs::write(&zip, bad).unwrap();
        let e = install(&env, &zip).await.unwrap_err();
        assert!(is_theme_invalid(&e), "{e:?}");
        assert!(e.to_string().contains("damaged"), "{e}");
        assert!(staging(&env.root).is_empty());
    }

    #[test]
    fn denied_renames_are_retried_a_few_times() {
        use std::io::{Error, ErrorKind};
        let pause = std::time::Duration::from_millis(1);
        let mut calls = 0;
        let r = retry_denied(5, pause, || {
            calls += 1;
            if calls < 3 {
                Err(Error::from(ErrorKind::PermissionDenied))
            } else {
                Ok(calls)
            }
        });
        assert_eq!(r.unwrap(), 3);

        let mut calls = 0;
        let r: std::io::Result<()> = retry_denied(5, pause, || {
            calls += 1;
            Err(Error::from(ErrorKind::PermissionDenied))
        });
        assert!(r.is_err());
        assert_eq!(calls, 5);

        // Other errors are not retried.
        let mut calls = 0;
        let r: std::io::Result<()> = retry_denied(5, pause, || {
            calls += 1;
            Err(Error::from(ErrorKind::NotFound))
        });
        assert!(r.is_err());
        assert_eq!(calls, 1);
    }

    fn builtin_themes_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/ui/themes")
    }

    #[test]
    fn builtin_themes_pass_the_core_checks() {
        let base = AssetUrlBase {
            origin: "http://127.0.0.1:0".into(),
            token: None,
        };
        let mut seen = 0;
        for entry in fs::read_dir(builtin_themes_root()).unwrap() {
            let entry = entry.unwrap();
            if !entry.file_type().unwrap().is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let t = load_builtin_theme_dir(&entry.path(), &name, &base, &ThemeLimits::DEFAULT)
                .unwrap_or_else(|e| panic!("{name}: {e:?}"));
            assert!(
                t.manifest.warnings.is_empty(),
                "{name}: {:?}",
                t.manifest.warnings
            );
            seen += 1;
        }
        assert_eq!(seen, 7);
    }

    // ---- inspect and open folder ----

    const PNG_1PX: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0x0d, 0x49, 0x48, 0x44, 0x52, 0,
        0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 0x1f, 0x15, 0xc4, 0x89, 0, 0, 0, 0x0a, 0x49, 0x44,
        0x41, 0x54, 0x78, 0x9c, 0x63, 0, 1, 0, 0, 5, 0, 1, 0x0d, 0x0a, 0x2d, 0xb4, 0, 0, 0, 0,
        0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    fn preview_manifest(id: &str) -> String {
        format!(
            r#"{{"themeApi":1,"id":"{id}","name":"Demo","author":"me","version":"1.0.0","preview":"assets/p.png"}}"#
        )
    }

    async fn inspect(env: &Env, zip: &Path) -> anyhow::Result<InspectedTheme> {
        inspect_theme_zip(zip, &env.root, &ThemeLimits::DEFAULT).await
    }

    #[tokio::test]
    async fn inspects_a_theme_without_installing_it() {
        let env = env();
        let m = preview_manifest("demo");
        let (_d, zip) = zip_of(&[("manifest.json", m.as_bytes()), ("assets/p.png", PNG_1PX)]);
        let t = inspect(&env, &zip).await.unwrap();
        assert_eq!(t.manifest.id, "demo");
        assert_eq!(t.manifest.name, "Demo");
        assert_eq!(t.manifest.author, "me");
        assert!(t.preview.unwrap().starts_with("data:image/png;base64,"));
        assert!(!t.installed);
        assert!(!env.root.join("demo").exists());
        assert!(staging(&env.root).is_empty());
    }

    #[tokio::test]
    async fn inspect_reports_an_installed_id() {
        let env = env();
        let m = min_manifest("demo");
        let (_d, zip) = zip_of(&[("manifest.json", m.as_bytes()), ("theme.css", b"a{}")]);
        install(&env, &zip).await.unwrap();
        assert!(inspect(&env, &zip).await.unwrap().installed);
    }

    #[tokio::test]
    async fn inspect_refuses_what_install_refuses() {
        let env = env();
        let m = min_manifest("demo");
        let (_a, bad) = zip_of(&[
            ("manifest.json", m.as_bytes()),
            ("theme.css", b"a{background:url(https://e.com/a.png)}"),
        ]);
        let e = inspect(&env, &bad).await.unwrap_err();
        assert!(e.to_string().contains("theme.css"), "{e}");
        assert!(matches!(
            e.downcast_ref::<ThemeError>(),
            Some(ThemeError::Invalid(_))
        ));
        let new = min_manifest("demo").replace("\"themeApi\":1", "\"themeApi\":2");
        let (_b, new_zip) = zip_of(&[("manifest.json", new.as_bytes()), ("theme.css", b"a{}")]);
        let e = inspect(&env, &new_zip).await.unwrap_err();
        assert!(matches!(
            e.downcast_ref::<ThemeError>(),
            Some(ThemeError::TooNew { .. })
        ));
        assert!(!env.root.join(".staging-inspect").exists());
    }

    #[tokio::test]
    async fn a_large_preview_is_left_out() {
        let env = env();
        let m = preview_manifest("demo");
        let big = vec![0u8; 3_000_000];
        let (_d, zip) = zip_of(&[("manifest.json", m.as_bytes()), ("assets/p.png", &big)]);
        let t = inspect(&env, &zip).await.unwrap();
        assert_eq!(t.preview, None);
    }

    #[test]
    fn folder_to_open_resolves_and_refuses() {
        use crate::error::FeErrorCode;
        let env = env();
        assert_eq!(folder_to_open(&env.root, None).unwrap(), env.root);
        assert!(env.root.is_dir());
        fs::create_dir_all(env.root.join("demo")).unwrap();
        assert_eq!(
            folder_to_open(&env.root, Some("demo")).unwrap(),
            env.root.join("demo")
        );
        let refused = |id: &str| folder_to_open(&env.root, Some(id)).unwrap_err();
        let e = refused("../x");
        assert!(matches!(
            e.downcast_ref::<ThemeError>(),
            Some(ThemeError::Invalid(_))
        ));
        assert_eq!(
            e.downcast_ref::<ThemeError>().unwrap().error_code(),
            "THEME_INVALID"
        );
        let e = refused("nope");
        assert!(matches!(
            e.downcast_ref::<ThemeError>(),
            Some(ThemeError::NotFound(_))
        ));
        assert_eq!(
            e.downcast_ref::<ThemeError>().unwrap().error_code(),
            "THEME_NOT_FOUND"
        );
    }

    #[test]
    fn a_failed_root_create_is_a_plain_error() {
        let env = env();
        fs::create_dir_all(&env.root).unwrap();
        fs::write(env.root.join("blocker"), "x").unwrap();
        let e = folder_to_open(&env.root.join("blocker").join("themes"), None).unwrap_err();
        assert!(e.downcast_ref::<ThemeError>().is_none(), "{e:#}");
    }

    #[test]
    fn the_scratch_folder_is_removed_after_a_panic() {
        let env = env();
        let root = env.root.clone();
        let r = std::panic::catch_unwind(|| {
            let _: anyhow::Result<()> = in_scratch(&root, ".staging-x", |stage| {
                fs::create_dir_all(stage).unwrap();
                fs::write(stage.join("f"), "x").unwrap();
                panic!("boom");
            });
        });
        assert!(r.is_err());
        assert!(staging(&env.root).is_empty());
    }

    #[test]
    fn folder_to_open_refuses_files_and_links() {
        let env = env();
        fs::create_dir_all(&env.root).unwrap();
        fs::write(env.root.join("file"), "x").unwrap();
        assert!(matches!(
            folder_to_open(&env.root, Some("file"))
                .unwrap_err()
                .downcast_ref::<ThemeError>(),
            Some(ThemeError::Invalid(_))
        ));
        #[cfg(unix)]
        {
            let outside = tempfile::tempdir().unwrap();
            std::os::unix::fs::symlink(outside.path(), env.root.join("link")).unwrap();
            assert!(matches!(
                folder_to_open(&env.root, Some("link"))
                    .unwrap_err()
                    .downcast_ref::<ThemeError>(),
                Some(ThemeError::Invalid(_))
            ));
        }
    }

    #[test]
    fn palette_matches_the_main_theme() {
        let text = fs::read_to_string(builtin_themes_root().join("main/manifest.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        let keys: HashSet<&str> = v["colors"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let palette: HashSet<&str> = manifest::PALETTE_TOKENS.iter().copied().collect();
        assert_eq!(keys, palette);
    }
}
