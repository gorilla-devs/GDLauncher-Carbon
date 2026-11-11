//! Server-specific planning uses the real server game/loader and persisted owners.
//!
//! What runs on the server is decided as on instances (see [`resolve`]):
//! - the version the user picks is installed whatever Minecraft versions it
//!   lists, as in 2.0.40; only a jar built for other loaders is refused, with
//!   a message naming both loaders. A jar that names no loader (most
//!   CurseForge files for 1.7.10 to 1.12.2) runs on any loader, Fabric mods
//!   run on Quilt and Forge mods on NeoForge 1.20.1;
//! - a dependency is downloaded at the newest release for the server's
//!   Minecraft version built for its loader, else at one that names no
//!   loader, never at one for other loaders. A Modrinth pin is trusted for
//!   the Minecraft version;
//! - a Modrinth project is refused only when its authors say it does not run
//!   on servers; most that leave it unknown are server mods;
//! - on Quilt, a missing Fabric API is installed as Quilted Fabric API, or as
//!   Fabric API when Quilted Fabric API has no version for the server.
//!
//! Required dependencies follow the rule of the instance installer
//! (`instance/installer/dependency.rs`), as in 2.0.40, Prism Launcher and the
//! Modrinth App:
//! - an enabled installed copy satisfies the requirement, whatever its version,
//!   next to any disabled or duplicate copy. It is never resolved, replaced or
//!   deleted, and its own dependencies are not walked. On Quilt, Quilted Fabric
//!   API provides Fabric API;
//! - a disabled-only copy (no enabled jar of its mod id, scanned or not)
//!   refuses a fresh install with a message naming it, but counts as present
//!   when the selected addon replaces an installed version of itself;
//! - a missing dependency that cannot be resolved is left out with a warning
//!   naming it; only the addon the user picked has to resolve. A connection
//!   problem stays fatal, so a transient failure never installs an addon
//!   without its dependency;
//! - a Modrinth pin that cannot be installed on the server is replaced by the
//!   same release built for it, else by the newest version for it, with a
//!   warning naming both versions when they are different releases;
//! - a dependency one dependant accepts at any version and another pins is
//!   planned at the pin, as if the pin had come first (see [`Repinned`]);
//! - a version change only adds the requirements the new version introduces;
//!   the ones the replaced version already declared are left as the user has
//!   them (see [`Previous`]);
//! - a downloaded dependency the server already has under its file name or its
//!   mod id (a copy the platforms do not identify, scanned or not) is not
//!   published again, and neither is what only that dependency required;
//! - a `mods/` folder linked to a folder shared between servers is read
//!   through, as publication writes through it (see [`occupant_at`]).
//!
//! Only the selected addon replaces an installed version of itself (GPT-0024),
//! whichever platform the installed copy came from; next to duplicates, the
//! copy the server loads (see [`replaced_copy`]). Another file where the
//! selected addon goes stops the installation before anything is published,
//! unless it already has the selected bytes (see [`check_destination`]).
use super::*;
use crate::domain::vtask::{DependencyPlatform, SkippedDependencyReason, TaskWarning};
use crate::managers::AppInner;
use crate::managers::instance::installer::dependency::{
    self as rule, Catalogue, FABRIC_API, LoaderFit, PIN_FREE_DEPENDENCIES, QUILTED_FABRIC_API,
};
use crate::managers::instance::installer::publication::{
    SERVER_STAGING_PREFIX, UnclearAddonFile, matches_download, selected_file, settle_staging,
};
use carbon_net::{Checksum, DownloadOptions, Downloadable};
use carbon_platforms::{
    ModChannel,
    curseforge::FileRelationType,
    modrinth::{
        project::{ProjectSupportRange, ProjectType, ProjectVersionsFilters},
        search::ProjectID,
        version::DependencyType,
    },
};
use sha2::{Digest, Sha512};
use std::{
    collections::{HashSet, VecDeque},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum Selection {
    Curseforge(u32, Option<u32>),
    Modrinth(String, Option<String>),
    /// The newest Modrinth version for the server, preferring this release
    /// (see `release_of`): what replaces a pin that is not for the server.
    ModrinthRelease(String, Vec<String>),
}

impl Selection {
    /// Planner key (`cf:<project>` / `mr:<project>`) and the pinned version.
    fn key(&self) -> (String, Option<String>) {
        match self {
            Selection::Curseforge(project, pin) => {
                (format!("cf:{project}"), pin.map(|pin| pin.to_string()))
            }
            Selection::Modrinth(project, pin) => (format!("mr:{project}"), pin.clone()),
            Selection::ModrinthRelease(project, _) => (format!("mr:{project}"), None),
        }
    }

    fn platform(&self) -> (DependencyPlatform, String) {
        match self {
            Selection::Curseforge(project, _) => {
                (DependencyPlatform::Curseforge, project.to_string())
            }
            Selection::Modrinth(project, _) | Selection::ModrinthRelease(project, _) => {
                (DependencyPlatform::Modrinth, project.clone())
            }
        }
    }

    /// Keys under which the version an update replaces may have declared
    /// this requirement (see [`Previous`]).
    fn declared_keys(&self) -> Vec<String> {
        let (key, pin) = self.key();
        let mut keys = vec![key];
        if let (Selection::Modrinth(..), Some(pin)) = (self, pin) {
            keys.push(format!("mr-version:{pin}"));
        }
        keys
    }
}

/// A requirement declared by a resolved artifact.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Child {
    Project(Selection),
    /// A Modrinth requirement declared by file name only: nothing on Modrinth
    /// provides it, only a copy with that file name does.
    File(String),
    /// A requirement the parent's metadata already shows cannot be installed.
    Unresolvable(Unresolvable),
}

impl From<Selection> for Child {
    fn from(selection: Selection) -> Self {
        Child::Project(selection)
    }
}

#[derive(Clone)]
struct Artifact {
    key: String,
    version: String,
    /// Project title, for messages.
    name: String,
    /// Version number as the platform shows it.
    version_name: String,
    filename: String,
    url: String,
    size: u64,
    checksum: Checksum,
    required: bool,
    /// The release a Modrinth version number names (see `release_of`).
    release: Vec<String>,
}
struct Prepared {
    artifact: Artifact,
    location: String,
    /// The installed copy the selected addon replaces.
    old: Option<(mfcdb::ServerAddonFullRow, Vec<u8>)>,
    /// The selected addon's destination already holds the bytes it publishes
    /// (a copy no platform identified as it, or one the scan has not
    /// recorded): the file is left as it is and only its source is recorded
    /// (see [`check_destination`]).
    adopt: Option<PathBuf>,
    download: Downloadable,
    reuse: Option<PathBuf>,
}

/// An addon that cannot be installed on this server. As a requirement it is
/// left out with a warning; as the selected addon it fails the install.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Unresolvable {
    /// Unknown when the platform no longer serves the project.
    name: Option<String>,
    platform: DependencyPlatform,
    /// A project id, or the file name of a requirement declared by file only.
    project_id: String,
    reason: SkippedDependencyReason,
}

impl Unresolvable {
    fn label(&self) -> String {
        rule::label(self.name.as_deref(), self.platform, &self.project_id)
    }

    /// The planner key of the project, or of the file for a requirement
    /// declared by file name only.
    fn key(&self) -> String {
        match self.platform {
            DependencyPlatform::Curseforge => format!("cf:{}", self.project_id),
            DependencyPlatform::Modrinth => format!("mr:{}", self.project_id),
        }
    }
}

impl std::fmt::Display for Unresolvable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} cannot be installed on this server: {}",
            self.label(),
            rule::describe(self.reason)
        )
    }
}

impl std::error::Error for Unresolvable {}

/// A pinned version that cannot be installed on this server: gone, of another
/// project, or a jar for other loaders. The Modrinth pin of a requirement is
/// then replaced by a version for the server, as on instances; the selected
/// addon fails with this message.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct UnusablePin {
    skip: Unresolvable,
    /// The pinned version, when it exists but is not for this server.
    version: Option<PinnedVersion>,
    /// Why the pinned version does not run on the server, naming what it is
    /// built for and what the server runs (see [`loader_mismatch`]).
    detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PinnedVersion {
    /// Version number as Modrinth shows it.
    number: String,
    /// The release it names (see `release_of`).
    release: Vec<String>,
}

impl std::fmt::Display for UnusablePin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(detail) => write!(
                f,
                "{} cannot be installed on this server: {detail}",
                self.skip.label()
            ),
            None => self.skip.fmt(f),
        }
    }
}

impl std::error::Error for UnusablePin {}

impl From<Unresolvable> for TaskWarning {
    fn from(value: Unresolvable) -> Self {
        TaskWarning::SkippedDependency {
            name: value.name,
            platform: value.platform,
            project_id: value.project_id,
            reason: value.reason,
        }
    }
}

/// Names a project while it is being resolved.
struct Naming {
    name: Option<String>,
    platform: DependencyPlatform,
    project_id: String,
}

impl Naming {
    fn unresolvable(&self, reason: SkippedDependencyReason) -> Unresolvable {
        Unresolvable {
            name: self.name.clone(),
            platform: self.platform,
            project_id: self.project_id.clone(),
            reason,
        }
    }

    fn because(&self, reason: SkippedDependencyReason) -> anyhow::Error {
        self.unresolvable(reason).into()
    }

    fn unusable_pin(
        &self,
        reason: SkippedDependencyReason,
        version: Option<PinnedVersion>,
    ) -> anyhow::Error {
        UnusablePin {
            skip: self.unresolvable(reason),
            version,
            detail: None,
        }
        .into()
    }

    /// A pinned jar built for other loaders than the server runs.
    fn foreign_pin(
        &self,
        tags: &[String],
        server_loader: Option<&str>,
        game_version: &str,
        version: Option<PinnedVersion>,
    ) -> anyhow::Error {
        UnusablePin {
            skip: self.unresolvable(SkippedDependencyReason::NoCompatibleVersion),
            version,
            detail: Some(loader_mismatch(tags, server_loader, game_version)),
        }
        .into()
    }

    /// A project, file or version the platform no longer serves cannot be
    /// installed. Any other lookup failure (offline, timeout, server error)
    /// stays an error, as on instances.
    fn lookup(&self, error: anyhow::Error) -> anyhow::Error {
        if rule::not_found(&error) {
            self.because(SkippedDependencyReason::Unavailable)
        } else {
            error.context(format!(
                "Could not look up {}. Check your connection and retry.",
                rule::label(self.name.as_deref(), self.platform, &self.project_id)
            ))
        }
    }
}

fn basename(filename: &str) -> anyhow::Result<()> {
    world::validate_relative(filename)?;
    if filename.contains('/') {
        bail!("Addon filename is not a basename");
    }
    Ok(())
}
pub(super) async fn digest(path: &Path) -> anyhow::Result<Vec<u8>> {
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path).await?;
    let mut digest = Sha512::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(digest.finalize().to_vec())
}
/// No-replace move through the shared publication helper. It copies when the
/// server folder is on another drive than staging, so it runs on the
/// blocking pool.
async fn move_owned(source: &Path, destination: &Path) -> anyhow::Result<()> {
    crate::util::publish::move_noclobber_async(source, destination).await?;
    Ok(())
}

/// The installed copy a new version replaces could not be moved aside
/// because a process holds it open: on Windows a running server keeps its
/// Fabric and Quilt mod jars and its datapack zips open. Nothing has changed
/// when this is returned.
#[derive(Debug)]
struct ReplacedCopyInUse {
    filename: String,
}

impl std::fmt::Display for ReplacedCopyInUse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} is in use and cannot be replaced", self.filename)
    }
}

/// Whether `error` says that another process holds the file open so that it
/// cannot be moved: a sharing or lock violation on Windows, a busy file
/// anywhere.
fn held_open(error: &anyhow::Error) -> bool {
    error
        .chain()
        .filter_map(|cause| cause.downcast_ref::<std::io::Error>())
        .any(|error| {
            error.kind() == std::io::ErrorKind::ResourceBusy
                || (cfg!(windows) && matches!(error.raw_os_error(), Some(32 | 33)))
        })
}

/// Names the addon and the way out in the error of an installation that
/// could not replace `addon`'s installed copy because something holds it
/// open (see [`ReplacedCopyInUse`]): stopping the server when it runs.
/// Other errors are returned as they are.
fn explain_in_use(error: anyhow::Error, addon: &str, server: &str, running: bool) -> anyhow::Error {
    let Some(in_use) = error.downcast_ref::<ReplacedCopyInUse>() else {
        return error;
    };
    let filename = &in_use.filename;
    let message = if running {
        format!(
            "Stop {server} to change the version of {addon}: the running server keeps {filename} open, so it cannot be replaced. Nothing was changed."
        )
    } else {
        format!(
            "{filename} is open in another program, so the version of {addon} could not be changed. Close that program, then try again. Nothing was changed."
        )
    };
    error.context(message)
}

// ---------------------------------------------------------------------------
// What the server already has.

/// One copy of an installed addon, as messages name it.
#[derive(Debug, Clone, PartialEq)]
struct InstalledCopy {
    name: String,
    filename: String,
    mr_version_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum Presence {
    /// At least one enabled copy: any version, next to any disabled or
    /// duplicate copy.
    Enabled(Vec<InstalledCopy>),
    /// Only disabled copies, sorted by file name.
    DisabledOnly(Vec<InstalledCopy>),
    Absent,
}

/// An addon the server loads: in `mods/` or in the selected world's datapacks.
#[derive(Debug, Clone)]
struct InstalledAddon {
    copy: InstalledCopy,
    enabled: bool,
    /// Planner keys of every platform project that publishes these bytes.
    keys: Vec<String>,
    /// The platform versions these bytes are known as, as exact selections.
    versions: Vec<Selection>,
    /// The jar's loader mod id, normalized.
    modid: Option<String>,
}

/// Projects whose installed copy also provides `key` on a server with
/// `loader`: on Quilt, Quilted Fabric API provides Fabric API, as in Prism's
/// getOverrideDeps.
pub(super) fn providers(key: &str, loader: Option<&str>) -> Vec<String> {
    let mut keys = vec![key.to_owned()];
    let fabric_api = [
        format!("mr:{}", FABRIC_API.0),
        format!("cf:{}", FABRIC_API.1),
    ];
    if loader == Some("quilt") && fabric_api.iter().any(|fabric| fabric == key) {
        keys.push(format!("mr:{}", QUILTED_FABRIC_API.0));
        keys.push(format!("cf:{}", QUILTED_FABRIC_API.1));
    }
    keys
}

/// What is downloaded for a required `selection` the server does not have,
/// when another project provides it (see [`providers`]): on Quilt, Quilted
/// Fabric API from the same platform for Fabric API, as Prism's
/// getOverrideDeps does. Fabric API itself also runs on Quilt, so it is
/// still installed when Quilted Fabric API has no version for the server.
fn stand_in(selection: &Selection, loader: Option<&str>) -> Option<Selection> {
    if loader != Some("quilt") {
        return None;
    }
    match selection {
        Selection::Modrinth(project, _) | Selection::ModrinthRelease(project, _)
            if project == FABRIC_API.0 =>
        {
            Some(Selection::Modrinth(QUILTED_FABRIC_API.0.to_owned(), None))
        }
        Selection::Curseforge(project, _) if i64::from(*project) == i64::from(FABRIC_API.1) => {
            Some(Selection::Curseforge(QUILTED_FABRIC_API.1 as u32, None))
        }
        _ => None,
    }
}

/// The mod id Fabric API's jar declares: a copy no platform identifies (a
/// jar dropped in by hand) still provides it.
const FABRIC_API_MOD_ID: &str = "fabric-api";

// ---------------------------------------------------------------------------
// What runs on the server.

/// Loader tags a jar may carry to run on the server: its loader, Fabric on
/// Quilt and Forge on NeoForge 1.20.1, as on instances (see
/// [`rule::accepted_loader_tags`]). None on a vanilla server.
fn accepted_loaders(server_loader: Option<&str>, game_version: &str) -> Vec<&'static str> {
    use crate::domain::instance::info::ModLoaderType;
    let loader = match server_loader.map(str::to_ascii_lowercase).as_deref() {
        Some("forge") => ModLoaderType::Forge,
        Some("neoforge") => ModLoaderType::Neoforge,
        Some("fabric") => ModLoaderType::Fabric,
        Some("quilt") => ModLoaderType::Quilt,
        _ => return Vec::new(),
    };
    rule::accepted_loader_tags(game_version, [loader])
}

/// How a jar's loader tags (Modrinth `loaders`, CurseForge `gameVersions`)
/// fit the server, with the rule instances use (see [`rule::loader_fit`]):
/// `Tagged` when they name the server's loader or one it also loads,
/// `Untagged` when they name no loader at all, as most CurseForge files for
/// 1.7.10 to 1.12.2 do (CurseForge counts those as Forge files), `Foreign`
/// when they name other loaders only. Every jar is foreign to a vanilla
/// server.
fn loader_fit(tags: &[String], server_loader: Option<&str>, game_version: &str) -> LoaderFit {
    rule::loader_fit(tags, &accepted_loaders(server_loader, game_version))
}

fn is_jar(filename: &str) -> bool {
    filename.to_ascii_lowercase().ends_with(".jar")
}

/// Whether a file runs on the server: anything that is not a jar (a
/// datapack), and a jar that is not built for other loaders only (see
/// [`loader_fit`]). `Err`: why not, naming both loaders.
fn validate_loader(
    filename: &str,
    tags: &[String],
    server_loader: Option<&str>,
    game_version: &str,
) -> Result<(), String> {
    if is_jar(filename) && loader_fit(tags, server_loader, game_version) == LoaderFit::Foreign {
        return Err(loader_mismatch(tags, server_loader, game_version));
    }
    Ok(())
}

/// How messages name a loader tag.
fn loader_name(tag: &str) -> String {
    match tag.to_ascii_lowercase().as_str() {
        "forge" => "Forge".to_owned(),
        "neoforge" => "NeoForge".to_owned(),
        "fabric" => "Fabric".to_owned(),
        "quilt" => "Quilt".to_owned(),
        "liteloader" => "LiteLoader".to_owned(),
        "babric" => "Babric".to_owned(),
        "bta-babric" => "BTA (Babric)".to_owned(),
        "legacy-fabric" => "Legacy Fabric".to_owned(),
        "nilloader" => "NilLoader".to_owned(),
        "optifine" => "OptiFine".to_owned(),
        "bungeecord" => "BungeeCord".to_owned(),
        tag => {
            let mut chars = tag.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        }
    }
}

/// "A", "A and B", "A, B and C" (with `or` for "A or B").
fn listed(names: &[String], conjunction: &str) -> String {
    match names {
        [] => String::new(),
        [name] => name.clone(),
        [rest @ .., last] => format!("{} {conjunction} {last}", rest.join(", ")),
    }
}

/// Why a jar built for other loaders cannot run on the server: what it is
/// built for, what the server runs, and what to do instead, naming every
/// loader whose builds the server runs, its own first ("Quilt or Fabric").
fn loader_mismatch(tags: &[String], server_loader: Option<&str>, game_version: &str) -> String {
    let mut built: Vec<String> = Vec::new();
    for tag in tags.iter().filter(|tag| rule::is_loader_tag(tag)) {
        let name = loader_name(tag);
        if !built.contains(&name) {
            built.push(name);
        }
    }
    let runs = |loader: &str| {
        let mut names = vec![loader_name(loader)];
        for tag in accepted_loaders(Some(loader), game_version) {
            let name = loader_name(tag);
            if !names.contains(&name) {
                names.push(name);
            }
        }
        listed(&names, "or")
    };
    match server_loader {
        Some(loader) if built.is_empty() => format!(
            "this version is not made for {}. Pick a version made for {}.",
            loader_name(loader),
            runs(loader)
        ),
        Some(loader) => format!(
            "this version is made for {}, and the server runs {}. Pick a version made for {}.",
            listed(&built, "and"),
            loader_name(loader),
            runs(loader)
        ),
        None if built.is_empty() => "this version is a mod, and the server runs vanilla Minecraft, which cannot load mods. Create a server with a mod loader to use it.".to_owned(),
        None => format!(
            "this version is a mod for {}, and the server runs vanilla Minecraft, which cannot load mods. Create a server with {} to use it.",
            listed(&built, "and"),
            listed(&built, "or")
        ),
    }
}

fn normalized_mod_id(modid: &str) -> Option<String> {
    let modid = modid.trim().to_lowercase();
    (!modid.is_empty()).then_some(modid)
}

/// Whether an addon file is there: enabled (`Some(true)`) or only as its
/// `.disabled` twin (`Some(false)`).
async fn occupant(path: &Path) -> std::io::Result<Option<bool>> {
    let mut disabled = path.as_os_str().to_owned();
    disabled.push(".disabled");
    for (candidate, enabled) in [(path.to_path_buf(), true), (PathBuf::from(disabled), false)] {
        match tokio::fs::symlink_metadata(&candidate).await {
            Ok(_) => return Ok(Some(enabled)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(None)
}

/// [`occupant`] of an addon location relative to the server's data folder.
/// A linked folder on the way (`mods/` as a symlink, or a junction on
/// Windows, shared between servers) is followed, as publication follows it:
/// this only reads, so it needs none of the ownership checks that guard
/// replacing or deleting a file.
async fn occupant_at(data: &Path, location: &str) -> anyhow::Result<Option<bool>> {
    world::validate_local(location)?;
    Ok(occupant(&data.join(location)).await?)
}

/// The server's addons as the dependency rule sees them. Every row is checked
/// against its file, so a jar deleted or toggled outside GDLauncher since the
/// last scan is seen as it is.
#[derive(Debug, Default)]
struct InstalledAddons {
    addons: Vec<InstalledAddon>,
    /// Files in `mods/` the scan has not recorded yet, with their state.
    unscanned: Vec<(String, bool)>,
    loader: Option<String>,
    /// The server's data folder, where `unscanned` is read.
    data: Option<PathBuf>,
    /// The jars among `unscanned` with the mod id each declares, read once
    /// and only when a requirement needs them (see [`unscanned_jars`]).
    jars: tokio::sync::OnceCell<Vec<InstalledAddon>>,
}

impl InstalledAddons {
    async fn read(data: &Path, rows: &[mfcdb::ServerAddonFullRow], loader: Option<&str>) -> Self {
        let world = world::selected(data).ok();
        let mut addons = Vec::new();
        let mut recorded = HashSet::new();
        for row in rows {
            let loaded = (row.addon_type == "mods"
                && row.location == format!("mods/{}", row.filename))
                || world.as_ref().is_some_and(|world| {
                    row.addon_type == "datapacks"
                        && row.location == format!("{world}/datapacks/{}", row.filename)
                });
            if !loaded {
                continue;
            }
            if row.addon_type == "mods" {
                recorded.insert(row.filename.clone());
            }
            if let Err(error) = world::validate_local(&row.location) {
                tracing::debug!(
                    filename = %row.filename,
                    "Server addon location is not usable: {error:#}"
                );
                continue;
            }
            let enabled = match occupant_at(data, &row.location).await {
                Ok(Some(enabled)) => enabled,
                // The file is gone; the row is stale.
                Ok(None) => continue,
                Err(error) => {
                    tracing::debug!(
                        filename = %row.filename,
                        "Server addon cannot be inspected, using its recorded state: {error:#}"
                    );
                    row.enabled
                }
            };
            let mut keys = Vec::new();
            for project in [row.cf_project_id, row.cache_cf_project_id]
                .into_iter()
                .flatten()
            {
                keys.push(format!("cf:{project}"));
            }
            for project in [&row.mr_project_id, &row.cache_mr_project_id]
                .into_iter()
                .flatten()
            {
                keys.push(format!("mr:{project}"));
            }
            keys.sort_unstable();
            keys.dedup();
            let mut versions = Vec::new();
            if let (Some(project), Some(file)) = (row.cf_project_id, row.cf_file_id) {
                if let (Ok(project), Ok(file)) = (u32::try_from(project), u32::try_from(file)) {
                    versions.push(Selection::Curseforge(project, Some(file)));
                }
            }
            if let (Some(project), Some(version)) = (&row.mr_project_id, &row.mr_version_id) {
                versions.push(Selection::Modrinth(project.clone(), Some(version.clone())));
            }
            addons.push(InstalledAddon {
                copy: InstalledCopy {
                    name: row
                        .meta_name
                        .as_deref()
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                        .unwrap_or(row.filename.as_str())
                        .to_owned(),
                    filename: row.filename.clone(),
                    mr_version_id: row.mr_version_id.clone(),
                },
                enabled,
                keys,
                versions,
                modid: (row.addon_type == "mods")
                    .then(|| row.modid.as_deref().and_then(normalized_mod_id))
                    .flatten(),
            });
        }
        let mut unscanned = Vec::new();
        if let Ok(mut entries) = tokio::fs::read_dir(data.join("mods")).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let Ok(name) = entry.file_name().into_string() else {
                    continue;
                };
                let (filename, enabled) = match name.strip_suffix(".disabled") {
                    Some(filename) => (filename.to_owned(), false),
                    None => (name, true),
                };
                if !recorded.contains(&filename) {
                    unscanned.push((filename, enabled));
                }
            }
        }
        Self {
            addons,
            unscanned,
            loader: loader.map(str::to_owned),
            data: Some(data.to_path_buf()),
            jars: tokio::sync::OnceCell::new(),
        }
    }

    /// The jars in `mods/` the scan has not recorded, with the mod id each
    /// declares, read on first use.
    async fn unscanned_jars(&self) -> &[InstalledAddon] {
        let Some(data) = &self.data else {
            return &[];
        };
        self.jars
            .get_or_init(|| unscanned_jars(data, &self.unscanned))
            .await
    }

    fn presence<'a>(copies: impl Iterator<Item = &'a InstalledAddon>) -> Presence {
        let (enabled, mut disabled): (Vec<_>, Vec<_>) = copies.partition(|addon| addon.enabled);
        if !enabled.is_empty() {
            return Presence::Enabled(enabled.into_iter().map(|a| a.copy.clone()).collect());
        }
        disabled.sort_by(|a, b| a.copy.filename.cmp(&b.copy.filename));
        disabled.dedup_by(|a, b| a.copy.filename == b.copy.filename);
        if disabled.is_empty() {
            Presence::Absent
        } else {
            Presence::DisabledOnly(disabled.into_iter().map(|a| a.copy.clone()).collect())
        }
    }

    /// What the server has of a platform project, among its scanned addons
    /// and, for the mod id of a disabled copy, `unscanned` (see
    /// [`InstalledAddons::requirement`]).
    fn project(&self, key: &str, unscanned: &[InstalledAddon]) -> Presence {
        let keys = providers(key, self.loader.as_deref());
        let claims = |addon: &&InstalledAddon| addon.keys.iter().any(|key| keys.contains(key));
        let found = Self::presence(self.addons.iter().filter(claims));
        let Presence::DisabledOnly(_) = &found else {
            return found;
        };
        // The project's copy is disabled next to an enabled jar with the same
        // mod id that no platform links to the project (a GitHub build, a
        // copy only the other platform knows, or a symlink the scan skips).
        // The server loads that jar, and enabling the disabled copy as well
        // would start it with two mods of one id.
        let disabled: HashSet<&str> = self
            .addons
            .iter()
            .filter(claims)
            .filter_map(|addon| addon.modid.as_deref())
            .collect();
        let provider = self
            .addons
            .iter()
            .chain(unscanned)
            .filter(|addon| addon.enabled)
            .filter(|addon| {
                addon
                    .modid
                    .as_deref()
                    .is_some_and(|modid| disabled.contains(modid))
            });
        match Self::presence(provider) {
            Presence::Enabled(copies) => Presence::Enabled(copies),
            _ => found,
        }
    }

    /// What the server has under an exact file name.
    fn file(&self, filename: &str) -> Presence {
        let found = Self::presence(
            self.addons
                .iter()
                .filter(|addon| addon.copy.filename == filename),
        );
        if found != Presence::Absent {
            return found;
        }
        let copy = InstalledCopy {
            name: filename.to_owned(),
            filename: filename.to_owned(),
            mr_version_id: None,
        };
        let states: Vec<bool> = self
            .unscanned
            .iter()
            .filter(|(name, _)| name == filename)
            .map(|(_, enabled)| *enabled)
            .collect();
        if states.contains(&true) {
            Presence::Enabled(vec![copy])
        } else if states.is_empty() {
            Presence::Absent
        } else {
            Presence::DisabledOnly(vec![copy])
        }
    }

    /// What the server has of a loader mod id, among its scanned addons and
    /// `unscanned`, the jars the scan has not recorded (see
    /// [`unscanned_jars`]).
    fn mod_id(&self, modid: &str, unscanned: &[InstalledAddon]) -> Presence {
        let Some(modid) = normalized_mod_id(modid) else {
            return Presence::Absent;
        };
        Self::presence(
            self.addons
                .iter()
                .chain(unscanned)
                .filter(|addon| addon.modid.as_deref() == Some(modid.as_str())),
        )
    }

    /// The installed version of the platform project `key` (on its own
    /// platform), which the selected addon replaces: the copy the server
    /// loads, next to disabled duplicates (see [`replaced_copy`]).
    fn version_of(&self, key: &str) -> Option<Selection> {
        let enabled = self.addons.iter().filter(|addon| addon.enabled);
        let disabled = self.addons.iter().filter(|addon| !addon.enabled);
        enabled
            .chain(disabled)
            .flat_map(|addon| &addon.versions)
            .find(|version| version.key().0 == key)
            .cloned()
    }

    /// What the server has of a requirement. A project it has only disabled
    /// is provided by an enabled jar with the disabled copy's mod id, scanned
    /// or not: the jars the scan has not recorded are read only then, so a
    /// refusal never asks the user to enable a second copy of a loaded mod.
    async fn requirement(&self, requirement: &Requirement) -> Presence {
        match requirement {
            Requirement::Project(key) => match self.project(key, &[]) {
                Presence::DisabledOnly(_) => self.project(key, self.unscanned_jars().await),
                found => found,
            },
            Requirement::File(filename) => self.file(filename),
        }
    }
}

/// The jars in `mods/` the scan has not recorded (on a server created moments
/// ago, a jar dropped in by hand, or a symlink, which the scan skips), with the
/// mod id each declares. Read only when a required project is there only
/// disabled, or a downloaded dependency's mod id matches nothing the scan
/// recorded (at publication, under the gate).
async fn unscanned_jars(data: &Path, unscanned: &[(String, bool)]) -> Vec<InstalledAddon> {
    let jars: Vec<(String, bool, PathBuf)> = unscanned
        .iter()
        .filter(|(filename, _)| filename.to_ascii_lowercase().ends_with(".jar"))
        .map(|(filename, enabled)| {
            let name = if *enabled {
                filename.clone()
            } else {
                format!("{filename}.disabled")
            };
            (filename.clone(), *enabled, data.join("mods").join(name))
        })
        .collect();
    if jars.is_empty() {
        return Vec::new();
    }
    tokio::task::spawn_blocking(move || {
        jars.into_iter()
            .filter_map(|(filename, enabled, path)| {
                // Follows a symlink to the jar it points at.
                let mut file = std::fs::File::open(&path).ok()?;
                if !file.metadata().ok()?.is_file() {
                    return None;
                }
                let modid = match crate::managers::metadata::mods::parse_metadata(&mut file) {
                    Ok(meta) => meta?.modid,
                    Err(error) => {
                        tracing::debug!(
                            %filename,
                            "Server addon has no readable mod metadata: {error:#}"
                        );
                        None
                    }
                };
                Some(InstalledAddon {
                    copy: InstalledCopy {
                        name: filename.clone(),
                        filename,
                        mr_version_id: None,
                    },
                    enabled,
                    keys: Vec::new(),
                    versions: Vec::new(),
                    modid: Some(modid.as_deref().and_then(normalized_mod_id)?),
                })
            })
            .collect()
    })
    .await
    .unwrap_or_default()
}

/// The refusal for a required dependency the server only has disabled. The
/// Addons tab cannot toggle a copy the scan has not recorded, so renaming the
/// file is offered too.
fn disabled_refusal(copies: &[InstalledCopy]) -> anyhow::Error {
    match copies {
        [copy] if copy.name == copy.filename => anyhow!(
            "Required dependency {0} is installed but disabled. Enable it in the server's Addons tab, or rename {0}.disabled to {0} in the server's mods folder, and retry.",
            copy.filename
        ),
        [copy] => anyhow!(
            "Required dependency {0} ({1}) is installed but disabled. Enable it in the server's Addons tab, or rename {1}.disabled to {1} in the server's mods folder, and retry.",
            copy.name,
            copy.filename
        ),
        _ => anyhow!(
            "Required dependency {} is installed but disabled ({}). Enable one of them in the server's Addons tab, or remove .disabled from the end of its name in the server's mods folder, and retry.",
            copies
                .first()
                .map(|copy| copy.name.as_str())
                .unwrap_or_default(),
            copies
                .iter()
                .map(|copy| copy.filename.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

// ---------------------------------------------------------------------------
// Planning.

#[derive(Debug, Clone, PartialEq)]
enum Requirement {
    /// A planner key (`cf:<project>` / `mr:<project>`).
    Project(String),
    File(String),
}

impl Requirement {
    /// The key the dependency graph knows it by.
    fn key(&self) -> String {
        match self {
            Requirement::Project(key) => key.clone(),
            Requirement::File(filename) => format!("file:{filename}"),
        }
    }
}

impl Child {
    /// The key the dependency graph knows it by.
    fn key(&self) -> String {
        match self {
            Child::Project(selection) => selection.key().0,
            Child::File(filename) => format!("file:{filename}"),
            Child::Unresolvable(skip) => skip.key(),
        }
    }
}

/// A required dependency the server already has. It is checked again before
/// publication, since the user can disable or delete it during the download.
#[derive(Debug, Clone, PartialEq)]
struct Present {
    requirement: Requirement,
    /// How the installed copy names itself.
    name: String,
    platform: DependencyPlatform,
    project_id: String,
}

/// An installed copy kept although a dependant pins another version of it.
#[derive(Debug, Clone, PartialEq)]
struct KeptPin {
    key: String,
    name: String,
    filename: String,
    pin: String,
}

/// What each planned addon requires, by graph key (`cf:<project>`,
/// `mr:<project>`, or `file:<name>` for a requirement declared by file name
/// only).
#[derive(Debug, Default)]
struct Graph {
    /// The selected addon.
    root: String,
    /// The requirements each planned addon declares.
    requires: HashMap<String, Vec<String>>,
}

/// What a plan publishes.
#[derive(Debug, Default)]
struct Reach {
    /// The planned addons that are published.
    published: HashSet<String>,
    /// The requirements of the published addons.
    needed: HashSet<String>,
}

impl Graph {
    /// What is published when the planned dependencies in `held` are not,
    /// because the server already has them. What only a held dependency
    /// requires is not needed: an installed dependency's own requirements are
    /// never walked, so the newer version planned in its place adds nothing.
    fn reach(&self, held: &HashSet<String>) -> Reach {
        let mut reach = Reach::default();
        reach.published.insert(self.root.clone());
        let mut stack = vec![self.root.as_str()];
        while let Some(key) = stack.pop() {
            for requirement in self.requires.get(key).into_iter().flatten() {
                reach.needed.insert(requirement.clone());
                if self.requires.contains_key(requirement)
                    && !held.contains(requirement)
                    && reach.published.insert(requirement.clone())
                {
                    stack.push(requirement);
                }
            }
        }
        reach
    }
}

/// What a warning is about. It is shown only while that holds once the plan
/// knows what it publishes.
#[derive(Debug, Clone, PartialEq)]
enum Concern {
    /// A requirement of a published addon.
    Needed(String),
    /// A planned addon that is published.
    Published(String),
}

impl Concern {
    fn holds(&self, reach: &Reach) -> bool {
        match self {
            Concern::Needed(key) => reach.needed.contains(key),
            Concern::Published(key) => reach.published.contains(key),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Warned {
    concern: Concern,
    warning: TaskWarning,
}

struct Planned {
    /// The selected addon first, then the dependencies to download.
    artifacts: Vec<Artifact>,
    /// The selected addon replaces an installed version of itself.
    replacing: bool,
    graph: Graph,
    /// Required dependencies the server has. Deeper in the graph, also the
    /// ones it only has disabled: those refuse the installation at
    /// publication if a published addon still needs them.
    present: Vec<Present>,
    kept_pins: Vec<KeptPin>,
    warned: Vec<Warned>,
}

impl Planned {
    fn warn(&mut self, concern: Concern, warning: TaskWarning) {
        if !self.warned.iter().any(|warned| warned.warning == warning) {
            self.warned.push(Warned { concern, warning });
        }
    }

    /// `parent` gets `stand_in` for its requirement `key` (see
    /// [`stand_in`]): what it requires is published with it.
    fn require_instead(&mut self, parent: &str, key: &str, stand_in: &str) {
        if let Some(requires) = self.graph.requires.get_mut(parent) {
            for requirement in requires
                .iter_mut()
                .filter(|requirement| *requirement == key)
            {
                *requirement = stand_in.to_owned();
            }
        }
    }

    /// Records what a planned addon requires, and queues it.
    fn declare(
        &mut self,
        key: &str,
        children: Vec<Child>,
        dependencies: bool,
        pending: &mut VecDeque<(String, Child)>,
    ) {
        let children = if dependencies { children } else { Vec::new() };
        self.graph
            .requires
            .insert(key.to_owned(), children.iter().map(Child::key).collect());
        pending.extend(children.into_iter().map(|child| (key.to_owned(), child)));
    }

    /// A pin replaced by `artifact`, the version for the server: reported
    /// unless both name the same release or the dependency runs with any
    /// build (Fabric API), as on instances.
    fn replaced_pin(&mut self, artifact: &Artifact, project_id: &str, pinned: &PinnedVersion) {
        if PIN_FREE_DEPENDENCIES.contains(&project_id) {
            return;
        }
        if pinned.release.is_empty() || pinned.release != artifact.release {
            self.warn(
                Concern::Published(artifact.key.clone()),
                TaskWarning::ReplacedPinnedVersion {
                    name: artifact.name.clone(),
                    project_id: project_id.to_owned(),
                    pinned: pinned.number.clone(),
                    installed: artifact.version_name.clone(),
                },
            );
        }
    }

    /// A required dependency the server only has disabled. It counts as
    /// present when the selected addon replaces itself. The selected addon's
    /// own requirement refuses the installation now; a deeper one is checked
    /// at publication, since the dependency that requires it may turn out to
    /// be installed already.
    fn disabled_only(
        &mut self,
        from_root: bool,
        present: Present,
        copies: &[InstalledCopy],
    ) -> anyhow::Result<()> {
        if self.replacing {
            tracing::info!(
                dependency = %present.name,
                "Replacing a server addon next to a required dependency that is only installed disabled"
            );
        } else if from_root {
            return Err(disabled_refusal(copies));
        } else {
            tracing::info!(
                dependency = %present.name,
                "A required dependency of a dependency is only installed disabled; checked again at publication"
            );
        }
        self.present.push(present);
        Ok(())
    }

    #[cfg(test)]
    fn warnings(&self) -> Vec<TaskWarning> {
        self.warned
            .iter()
            .map(|warned| warned.warning.clone())
            .collect()
    }
}

/// The server's resolution of a selection, behind a seam so the installation
/// can be tested without the platforms.
#[async_trait::async_trait]
trait Resolver: Send + Sync {
    async fn resolve(
        &self,
        server: &server_repo::ServerRow,
        selection: Selection,
        dependencies: bool,
    ) -> anyhow::Result<(Artifact, Vec<Child>)>;

    /// The graph keys of the requirements an exact version declares (see
    /// [`Previous`]).
    async fn declared(&self, _selection: Selection) -> anyhow::Result<HashSet<String>> {
        Ok(HashSet::new())
    }
}

/// CurseForge and Modrinth through a catalogue: the app's platform clients,
/// or a stand-in for them in tests.
struct Live(Arc<dyn Catalogue>);

#[async_trait::async_trait]
impl Resolver for Live {
    async fn resolve(
        &self,
        server: &server_repo::ServerRow,
        selection: Selection,
        dependencies: bool,
    ) -> anyhow::Result<(Artifact, Vec<Child>)> {
        resolve(&*self.0, server, selection, dependencies).await
    }

    async fn declared(&self, selection: Selection) -> anyhow::Result<HashSet<String>> {
        declared(&*self.0, selection).await
    }
}

async fn plan(
    app: &Arc<AppInner>,
    server: &server_repo::ServerRow,
    selection: Selection,
    installed: &InstalledAddons,
    resolver: &dyn Resolver,
) -> anyhow::Result<Planned> {
    let dependencies = app
        .settings_manager()
        .get_settings()
        .await?
        .download_dependencies;
    let previous = installed
        .version_of(&selection.key().0)
        .map(|replaced| Previous::new(resolver.declared(replaced)));
    plan_graph_with(selection, dependencies, installed, previous, |selection| {
        resolver.resolve(server, selection, dependencies)
    })
    .await
}

/// The requirements of the installed version the selected addon replaces.
/// Those were already the user's to manage: 2.0.40 never added any on a
/// version change, and the user may have removed one on purpose (a library
/// that only runs on clients), so only the requirements the new version
/// introduces are resolved, as on instances (GPT-0022). Looked up once, and
/// only when one of the selected addon's requirements is missing.
struct Previous<'a> {
    lookup: Option<
        std::pin::Pin<
            Box<dyn std::future::Future<Output = anyhow::Result<HashSet<String>>> + Send + 'a>,
        >,
    >,
    declared: HashSet<String>,
}

impl<'a> Previous<'a> {
    fn new(
        lookup: impl std::future::Future<Output = anyhow::Result<HashSet<String>>> + Send + 'a,
    ) -> Self {
        Self {
            lookup: Some(Box::pin(lookup)),
            declared: HashSet::new(),
        }
    }

    async fn declares(&mut self, keys: &[String]) -> anyhow::Result<bool> {
        if let Some(lookup) = self.lookup.take() {
            self.declared = match lookup.await {
                Ok(declared) => declared,
                // The old version is gone: every missing requirement is resolved.
                Err(error) if rule::not_found(&error) => HashSet::new(),
                Err(error) => {
                    return Err(error.context(
                        "Could not look up the version this addon replaces. Check your connection and retry.",
                    ));
                }
            };
        }
        Ok(keys.iter().any(|key| self.declared.contains(key)))
    }
}

async fn declared_before(
    previous: &mut Option<Previous<'_>>,
    keys: &[String],
) -> anyhow::Result<bool> {
    let Some(previous) = previous else {
        return Ok(false);
    };
    let declared = previous.declares(keys).await?;
    if declared {
        tracing::info!(
            dependency = %keys[0],
            "Changing a server addon's version without a requirement the replaced version already had"
        );
    }
    Ok(declared)
}

/// What the plan decided for a requirement.
enum Decision {
    /// To be downloaded at `version`.
    Planned {
        version: String,
        /// Chosen by a pin (or by the user), not as the newest version.
        pinned: bool,
        /// Pins this version replaces because they cannot be installed.
        replaced: Vec<(String, Option<PinnedVersion>)>,
    },
    /// The server already has it.
    Present,
    /// The version the selected addon replaces already required it.
    Previous,
    /// Left out with a warning.
    Skipped,
    /// Planned as the project under this key, which provides it (see
    /// [`stand_in`]).
    StandIn(String),
}

/// The reason a dependency cannot be installed, or the error that stops the
/// installation (a connection problem).
fn unresolvable(error: anyhow::Error) -> anyhow::Result<Unresolvable> {
    let error = match error.downcast::<UnusablePin>() {
        Ok(unusable) => return Ok(unusable.skip),
        Err(error) => error,
    };
    error.downcast::<Unresolvable>()
}

/// Checks that the platform returned the dependency asked for, with a usable
/// file name.
fn accept(
    mut artifact: Artifact,
    key: &str,
    pin: Option<&str>,
    platform: DependencyPlatform,
    project_id: &str,
) -> Result<Artifact, Unresolvable> {
    let naming = Naming {
        name: Some(artifact.name.clone()),
        platform,
        project_id: project_id.to_owned(),
    };
    if artifact.key != key || pin.is_some_and(|pin| pin != artifact.version) {
        tracing::warn!(
            dependency = %artifact.name,
            "Provider returned a different artifact than the required one"
        );
        return Err(naming.unresolvable(SkippedDependencyReason::Unavailable));
    }
    if let Err(error) = basename(&artifact.filename) {
        tracing::warn!(
            dependency = %artifact.name,
            "Required dependency has an unusable file name: {error:#}"
        );
        return Err(naming.unresolvable(SkippedDependencyReason::Unavailable));
    }
    artifact.required = true;
    Ok(artifact)
}

#[cfg(test)]
async fn plan_graph<F, Fut>(
    selection: Selection,
    dependencies: bool,
    installed: &InstalledAddons,
    resolve: F,
) -> anyhow::Result<Planned>
where
    F: FnMut(Selection) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<(Artifact, Vec<Child>)>>,
{
    plan_graph_with(selection, dependencies, installed, None, resolve).await
}

/// Resolves each selection once, so a planning that starts over (see
/// [`Repinned`]) asks the platforms nothing twice and sees the same versions.
struct Memo<F> {
    resolve: F,
    known: HashMap<Selection, Known>,
}

/// A resolution worth remembering. Any other error (a connection problem)
/// stops the planning.
#[derive(Clone)]
enum Known {
    Resolved(Artifact, Vec<Child>),
    Unusable(UnusablePin),
    Unresolvable(Unresolvable),
}

impl<F, Fut> Memo<F>
where
    F: FnMut(Selection) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<(Artifact, Vec<Child>)>>,
{
    fn new(resolve: F) -> Self {
        Self {
            resolve,
            known: HashMap::new(),
        }
    }

    async fn resolve(&mut self, selection: Selection) -> anyhow::Result<(Artifact, Vec<Child>)> {
        if let Some(known) = self.known.get(&selection) {
            return match known.clone() {
                Known::Resolved(artifact, children) => Ok((artifact, children)),
                Known::Unusable(unusable) => Err(unusable.into()),
                Known::Unresolvable(skip) => Err(skip.into()),
            };
        }
        let resolved = (self.resolve)(selection.clone()).await;
        let known = match &resolved {
            Ok((artifact, children)) => Some(Known::Resolved(artifact.clone(), children.clone())),
            Err(error) => error
                .downcast_ref::<UnusablePin>()
                .cloned()
                .map(Known::Unusable)
                .or_else(|| {
                    error
                        .downcast_ref::<Unresolvable>()
                        .cloned()
                        .map(Known::Unresolvable)
                }),
        };
        if let Some(known) = known {
            self.known.insert(selection, known);
        }
        resolved
    }
}

/// A dependency that a later dependant pins after an earlier one had it
/// planned at its newest version. The pin satisfies both, so the planning
/// starts over with it decided: nothing the replaced version's own
/// requirements chose (a version of a library, a warning) outlives it.
#[derive(Clone)]
struct Repinned {
    artifact: Artifact,
    children: Vec<Child>,
    /// Pins it replaces because they cannot be installed.
    replaced: Vec<(String, Option<PinnedVersion>)>,
}

/// Plans the selected addon and the required dependencies the server does not
/// already have. Only the selected addon has to resolve.
async fn plan_graph_with<F, Fut>(
    selection: Selection,
    dependencies: bool,
    installed: &InstalledAddons,
    mut previous: Option<Previous<'_>>,
    resolve: F,
) -> anyhow::Result<Planned>
where
    F: FnMut(Selection) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<(Artifact, Vec<Child>)>>,
{
    let mut resolve = Memo::new(resolve);
    let (key, pin) = selection.key();
    let (platform, project_id) = selection.platform();
    let (mut root, root_children) = resolve.resolve(selection).await?;
    let identity = if root.key != key || pin.as_ref().is_some_and(|pin| *pin != root.version) {
        Err(anyhow!("the platform returned another file"))
    } else {
        basename(&root.filename)
    };
    if let Err(error) = identity {
        tracing::warn!(
            addon = %root.name,
            filename = %root.filename,
            "The selected server addon cannot be installed: {error:#}"
        );
        return Err(Unresolvable {
            name: Some(root.name.clone()),
            platform,
            project_id,
            reason: SkippedDependencyReason::Unavailable,
        }
        .into());
    }
    root.required = false;
    let replacing = installed.project(&root.key, &[]) != Presence::Absent;
    // Each dependency is repinned at most once: from then on it is pinned,
    // and another pin only warns. So there is one pass more than repins, and
    // no more passes than a plan has dependencies.
    let mut repinned: HashMap<String, Repinned> = HashMap::new();
    let mut passes = 0;
    'passes: loop {
        passes += 1;
        if passes > 513 {
            bail!("Server dependency graph exceeds the supported bound");
        }
        let mut planned = Planned {
            replacing,
            artifacts: Vec::new(),
            graph: Graph {
                root: root.key.clone(),
                requires: HashMap::new(),
            },
            present: Vec::new(),
            kept_pins: Vec::new(),
            warned: Vec::new(),
        };
        let mut decided = HashMap::from([(
            root.key.clone(),
            Decision::Planned {
                version: root.version.clone(),
                pinned: true,
                replaced: Vec::new(),
            },
        )]);
        let mut pending = VecDeque::new();
        planned.declare(&root.key, root_children.clone(), dependencies, &mut pending);
        planned.artifacts.push(root.clone());
        while let Some((parent, child)) = pending.pop_front() {
            let from_root = parent == planned.graph.root;
            if !from_root && matches!(decided.get(&child.key()), Some(Decision::Previous)) {
                // A new dependency needs what the replaced version already
                // required: it is resolved for that one.
                decided.remove(&child.key());
            }
            let selection = match child {
                Child::Unresolvable(skip) => {
                    let key = skip.key();
                    if decided.contains_key(&key) {
                        continue;
                    }
                    let mut keys = vec![key.clone()];
                    if skip.platform == DependencyPlatform::Modrinth {
                        keys.push(format!("mr-version:{}", skip.project_id));
                    }
                    if from_root && declared_before(&mut previous, &keys).await? {
                        decided.insert(key, Decision::Previous);
                    } else {
                        decided.insert(key.clone(), Decision::Skipped);
                        planned.warn(Concern::Needed(key), skip.into());
                    }
                    continue;
                }
                Child::File(filename) => {
                    let key = format!("file:{filename}");
                    if decided.contains_key(&key) {
                        continue;
                    }
                    let requirement = Requirement::File(filename.clone());
                    match installed.requirement(&requirement).await {
                        Presence::Enabled(copies) => {
                            planned
                                .present
                                .push(present(requirement, &copies, &filename));
                            decided.insert(key, Decision::Present);
                        }
                        Presence::DisabledOnly(copies) => {
                            planned.disabled_only(
                                from_root,
                                present(requirement, &copies, &filename),
                                &copies,
                            )?;
                            decided.insert(key, Decision::Present);
                        }
                        Presence::Absent => {
                            if from_root && declared_before(&mut previous, &[key.clone()]).await? {
                                decided.insert(key, Decision::Previous);
                            } else {
                                decided.insert(key.clone(), Decision::Skipped);
                                planned.warn(
                                    Concern::Needed(key),
                                    Unresolvable {
                                        name: Some(filename.clone()),
                                        platform: DependencyPlatform::Modrinth,
                                        project_id: filename,
                                        reason: SkippedDependencyReason::Unavailable,
                                    }
                                    .into(),
                                );
                            }
                        }
                    }
                    continue;
                }
                Child::Project(selection) => selection,
            };
            let (key, pin) = selection.key();
            let (platform, project_id) = selection.platform();
            let repin = match decided.get(&key) {
                None => false,
                Some(Decision::Planned {
                    version,
                    pinned,
                    replaced,
                }) => {
                    let Some(pin) = pin.as_ref() else {
                        continue;
                    };
                    if pin == version || replaced.iter().any(|(replaced, _)| replaced == pin) {
                        continue;
                    }
                    if *pinned {
                        // Two dependants pin different versions. Only one can
                        // be installed; the first dependant's is kept.
                        let artifact = planned
                            .artifacts
                            .iter()
                            .find(|artifact| artifact.key == key)
                            .context("Planned dependency missing from the plan")?;
                        let warning = TaskWarning::ConflictingVersions {
                            name: artifact.name.clone(),
                            platform,
                            project_id,
                            installed: artifact.version_name.clone(),
                        };
                        planned.warn(Concern::Published(key), warning);
                        continue;
                    }
                    true
                }
                Some(Decision::Present) => {
                    if let Some(pin) = pin {
                        if let Presence::Enabled(copies) = installed
                            .requirement(&Requirement::Project(key.clone()))
                            .await
                        {
                            keep_pin(&mut planned.kept_pins, &key, &copies, &pin);
                        }
                    }
                    continue;
                }
                Some(Decision::Previous | Decision::Skipped) => continue,
                Some(Decision::StandIn(stand_in)) => {
                    let stand_in = stand_in.clone();
                    planned.require_instead(&parent, &key, &stand_in);
                    continue;
                }
            };
            if repin {
                // The newest version was planned for a dependant that accepts
                // any version; this one pins a version, which satisfies both.
                let pin = pin.context("A repinned dependency has a pin")?;
                let Some(Decision::Planned { mut replaced, .. }) = decided.remove(&key) else {
                    bail!("Planned dependency missing from the plan");
                };
                let current = planned
                    .artifacts
                    .iter()
                    .find(|artifact| artifact.key == key)
                    .context("Planned dependency missing from the plan")?
                    .clone();
                let pinned = match resolve.resolve(selection).await {
                    Ok((artifact, children)) => {
                        match accept(artifact, &key, Some(&pin), platform, &project_id) {
                            Ok(artifact) => Ok((artifact, children)),
                            Err(skip) => Err((skip, None)),
                        }
                    }
                    Err(error) => match error.downcast::<UnusablePin>() {
                        Ok(unusable) => Err((unusable.skip, unusable.version)),
                        Err(error) => Err((unresolvable(error)?, None)),
                    },
                };
                match pinned {
                    Ok((artifact, children)) => {
                        // Start over with the pinned version, so what the
                        // newest version required is never decided.
                        repinned.insert(
                            key,
                            Repinned {
                                artifact,
                                children,
                                replaced,
                            },
                        );
                        continue 'passes;
                    }
                    Err((skip, version)) => {
                        tracing::info!(
                            dependency = %skip.label(),
                            pin = %pin,
                            "A pinned required version cannot be installed; keeping the version planned for another dependant"
                        );
                        if let Some(version) = &version {
                            planned.replaced_pin(&current, &project_id, version);
                        }
                        replaced.push((pin, version));
                        decided.insert(
                            key,
                            Decision::Planned {
                                version: current.version,
                                pinned: false,
                                replaced,
                            },
                        );
                    }
                }
                continue;
            }
            let requirement = Requirement::Project(key.clone());
            match installed.requirement(&requirement).await {
                Presence::Enabled(copies) => {
                    if let Some(pin) = &pin {
                        keep_pin(&mut planned.kept_pins, &key, &copies, pin);
                    }
                    planned.present.push(Present {
                        requirement,
                        name: copies[0].name.clone(),
                        platform,
                        project_id,
                    });
                    decided.insert(key, Decision::Present);
                    continue;
                }
                Presence::DisabledOnly(copies) => {
                    let present = Present {
                        requirement,
                        name: copies[0].name.clone(),
                        platform,
                        project_id,
                    };
                    planned.disabled_only(from_root, present, &copies)?;
                    decided.insert(key, Decision::Present);
                    continue;
                }
                Presence::Absent => {}
            }
            if from_root && declared_before(&mut previous, &selection.declared_keys()).await? {
                decided.insert(key, Decision::Previous);
                continue;
            }
            if let Some(repin) = repinned.get(&key) {
                // Repinned in an earlier pass: planned at its pin, and this
                // requirement is checked against it like any later one.
                for version in repin
                    .replaced
                    .iter()
                    .filter_map(|(_, version)| version.as_ref())
                {
                    planned.replaced_pin(&repin.artifact, &project_id, version);
                }
                decided.insert(
                    key.clone(),
                    Decision::Planned {
                        version: repin.artifact.version.clone(),
                        pinned: true,
                        replaced: repin.replaced.clone(),
                    },
                );
                planned.declare(&key, repin.children.clone(), dependencies, &mut pending);
                planned.artifacts.push(repin.artifact.clone());
                pending.push_front((parent, Child::Project(selection)));
                continue;
            }
            if let Some(stand_in) = stand_in(&selection, installed.loader.as_deref()) {
                // Quilted Fabric API is planned in its place like any other
                // requirement, when it can be installed and no copy the
                // platforms do not identify provides Fabric API already (that
                // copy keeps Fabric API at publication).
                let stand_key = stand_in.key().0;
                let by_mod_id = installed
                    .mod_id(FABRIC_API_MOD_ID, installed.unscanned_jars().await)
                    != Presence::Absent;
                let usable = !by_mod_id
                    && match decided.get(&stand_key) {
                        Some(Decision::Planned { .. }) => true,
                        Some(_) => false,
                        None if repinned.contains_key(&stand_key) => true,
                        None => {
                            let (stand_platform, stand_project) = stand_in.platform();
                            let probed = match resolve.resolve(stand_in.clone()).await {
                                Ok((artifact, _)) => accept(
                                    artifact,
                                    &stand_key,
                                    None,
                                    stand_platform,
                                    &stand_project,
                                )
                                .map(|_| ()),
                                Err(error) => Err(unresolvable(error)?),
                            };
                            if let Err(skip) = &probed {
                                tracing::info!(
                                    dependency = %skip.label(),
                                    "Quilted Fabric API cannot be installed on this server ({skip}); installing Fabric API"
                                );
                            }
                            probed.is_ok()
                        }
                    };
                if usable {
                    planned.require_instead(&parent, &key, &stand_key);
                    decided.insert(key, Decision::StandIn(stand_key));
                    pending.push_front((parent, Child::Project(stand_in)));
                    continue;
                }
            }
            let has_pin = pin.is_some();
            let resolved = match resolve.resolve(selection).await {
                Ok((artifact, children)) => {
                    accept(artifact, &key, pin.as_deref(), platform, &project_id)
                        .map(|artifact| (artifact, children, Vec::new()))
                }
                Err(error) => match (error.downcast::<UnusablePin>(), pin) {
                    // A pin that cannot be installed on the server is replaced
                    // by the same release built for it, else by the newest
                    // version for it, as on instances.
                    (Ok(unusable), Some(pin)) if platform == DependencyPlatform::Modrinth => {
                        tracing::info!(
                            dependency = %unusable.skip.label(),
                            pin = %pin,
                            "A pinned required version cannot be installed on this server; choosing one for it"
                        );
                        let release = unusable
                            .version
                            .as_ref()
                            .map(|version| version.release.clone())
                            .unwrap_or_default();
                        match resolve
                            .resolve(Selection::ModrinthRelease(project_id.clone(), release))
                            .await
                        {
                            Ok((artifact, children)) => {
                                accept(artifact, &key, None, platform, &project_id).map(
                                    |artifact| (artifact, children, vec![(pin, unusable.version)]),
                                )
                            }
                            Err(error) => Err(unresolvable(error)?),
                        }
                    }
                    (Ok(unusable), _) => Err(unusable.skip),
                    (Err(error), _) => Err(unresolvable(error)?),
                },
            };
            match resolved {
                Ok((artifact, children, replaced)) => {
                    for version in replaced.iter().filter_map(|(_, version)| version.as_ref()) {
                        planned.replaced_pin(&artifact, &project_id, version);
                    }
                    decided.insert(
                        key.clone(),
                        Decision::Planned {
                            version: artifact.version.clone(),
                            pinned: has_pin && replaced.is_empty(),
                            replaced,
                        },
                    );
                    planned.declare(&key, children, dependencies, &mut pending);
                    planned.artifacts.push(artifact);
                    if planned.artifacts.len() > 512 {
                        bail!("Server dependency graph exceeds the supported bound");
                    }
                }
                Err(skip) => {
                    decided.insert(key.clone(), Decision::Skipped);
                    planned.warn(Concern::Needed(key), skip.into());
                }
            }
        }
        return Ok(planned);
    }
}

fn present(requirement: Requirement, copies: &[InstalledCopy], filename: &str) -> Present {
    Present {
        requirement,
        name: copies
            .first()
            .map_or_else(|| filename.to_owned(), |copy| copy.name.clone()),
        platform: DependencyPlatform::Modrinth,
        project_id: filename.to_owned(),
    }
}

/// A pin only chooses what to download; an installed copy of another version
/// is kept, as on instances, in Prism and in the Modrinth App.
fn keep_pin(kept: &mut Vec<KeptPin>, key: &str, copies: &[InstalledCopy], pin: &str) {
    if copies
        .iter()
        .any(|copy| copy.mr_version_id.as_deref() == Some(pin))
    {
        return;
    }
    let pin = KeptPin {
        key: key.to_owned(),
        name: copies[0].name.clone(),
        filename: copies[0].filename.clone(),
        pin: pin.to_owned(),
    };
    if !kept.contains(&pin) {
        kept.push(pin);
    }
}

/// Resolves a selection for the server.
///
/// A pinned version (the one the user picked, or a Modrinth dependency's pin)
/// is only checked for what it is: the project's own file, with a hash, a
/// download and a usable name, and not a jar built for other loaders only
/// (see [`validate_loader`]). Its Minecraft versions are not checked, as in
/// 2.0.40, Prism Launcher and the Modrinth App: authors often list only the
/// release (1.20 for 1.20.1) a file runs on, and the user or the dependant's
/// author picked it.
///
/// Without a pin, the newest release for the server's Minecraft version is
/// chosen among the files built for its loader, else among those that name
/// no loader (see [`rule::best_candidate`]).
async fn resolve(
    catalogue: &dyn Catalogue,
    server: &server_repo::ServerRow,
    selection: Selection,
    dependencies: bool,
) -> anyhow::Result<(Artifact, Vec<Child>)> {
    match selection {
        Selection::Curseforge(project, pin) => {
            resolve_curseforge(catalogue, server, project, pin).await
        }
        Selection::Modrinth(project, pin) => {
            resolve_modrinth(catalogue, server, project, pin, Vec::new(), dependencies).await
        }
        Selection::ModrinthRelease(project, release) => {
            resolve_modrinth(catalogue, server, project, None, release, dependencies).await
        }
    }
}

/// Resolves a CurseForge project for the server: the pinned file, or the
/// newest file for it (see [`resolve`]). Files are looked up by Minecraft
/// version only, since CurseForge's loader filter leaves out the files of
/// the loaders the server also loads (Fabric on Quilt).
async fn resolve_curseforge(
    catalogue: &dyn Catalogue,
    server: &server_repo::ServerRow,
    project: u32,
    pin: Option<u32>,
) -> anyhow::Result<(Artifact, Vec<Child>)> {
    let key = format!("cf:{project}");
    let mut naming = Naming {
        name: None,
        platform: DependencyPlatform::Curseforge,
        project_id: project.to_string(),
    };
    let Ok(project_id) = i32::try_from(project) else {
        return Err(naming.because(SkippedDependencyReason::Unavailable));
    };
    let metadata = catalogue
        .cf_mod(project_id)
        .await
        .map_err(|error| naming.lookup(error))?;
    naming.name = Some(metadata.name.clone());
    if metadata.id != project_id {
        return Err(naming.because(SkippedDependencyReason::Unavailable));
    }
    let loader = server.modloader_type.as_deref();
    let file = if let Some(pin) = pin {
        let Ok(file_id) = i32::try_from(pin) else {
            return Err(naming.unusable_pin(SkippedDependencyReason::Unavailable, None));
        };
        let file = catalogue
            .cf_file(project_id, file_id)
            .await
            .map_err(|error| naming.lookup(error))?;
        if file.mod_id != project_id || file.id != file_id {
            return Err(naming.unusable_pin(SkippedDependencyReason::Unavailable, None));
        }
        if let Err(detail) = validate_loader(
            &file.file_name,
            &file.game_versions,
            loader,
            &server.game_version,
        ) {
            tracing::info!(
                addon = %metadata.name,
                file = file.id,
                "Server addon file does not fit the server's loader: {detail}"
            );
            return Err(naming.foreign_pin(
                &file.game_versions,
                loader,
                &server.game_version,
                None,
            ));
        }
        file
    } else {
        let mut files = Vec::new();
        for page in 0..rule::PAGES {
            let response = catalogue
                .cf_files(project_id, &server.game_version, page * 50)
                .await
                .map_err(|error| naming.lookup(error))?;
            let done = response.data.len() < 50
                || response
                    .pagination
                    .as_ref()
                    .is_some_and(|p| p.index + p.result_count >= p.total_count);
            files.extend(response.data);
            if done {
                break;
            }
            // The newest files come first; the oldest are left out.
        }
        if files.iter().any(|file| file.mod_id != project_id) {
            return Err(naming.because(SkippedDependencyReason::Unavailable));
        }
        rule::best_candidate(&files, ModChannel::Stable, |file| {
            if !file.game_versions.contains(&server.game_version) {
                return None;
            }
            let fit = if is_jar(&file.file_name) {
                loader_fit(&file.game_versions, loader, &server.game_version)
            } else {
                LoaderFit::Tagged
            };
            Some((fit, ModChannel::from(file.release_type)))
        })
        .cloned()
        .ok_or_else(|| naming.because(SkippedDependencyReason::NoCompatibleVersion))?
    };
    let Some(hash) = file
        .hashes
        .iter()
        .find(|h| matches!(h.algo, carbon_platforms::curseforge::HashAlgo::Sha1))
        .or(file.hashes.first())
    else {
        return Err(naming.because(SkippedDependencyReason::Unavailable));
    };
    let checksum = match hash.algo {
        carbon_platforms::curseforge::HashAlgo::Sha1 => Checksum::Sha1(hash.value.clone()),
        carbon_platforms::curseforge::HashAlgo::Md5 => Checksum::Md5(hash.value.clone()),
    };
    let children = file
        .dependencies
        .iter()
        .filter(|d| matches!(d.relation_type, FileRelationType::RequiredDependency))
        .filter_map(|d| u32::try_from(d.mod_id).ok())
        .map(|project| Child::Project(Selection::Curseforge(project, None)))
        .collect::<Vec<_>>();
    let Some(url) = file.download_url else {
        return Err(naming.because(SkippedDependencyReason::NotDownloadable));
    };
    Ok((
        Artifact {
            key,
            version: file.id.to_string(),
            name: metadata.name,
            version_name: file.display_name,
            filename: file.file_name,
            url,
            size: file.file_length as u64,
            checksum,
            required: false,
            release: Vec::new(),
        },
        children,
    ))
}

/// The file a Modrinth version downloads.
fn primary_file(
    version: &carbon_platforms::modrinth::version::Version,
) -> Option<&carbon_platforms::modrinth::version::VersionFile> {
    version
        .files
        .iter()
        .find(|file| file.primary)
        .or(version.files.first())
}

/// Resolves a Modrinth project for the server: the pinned version, or the
/// newest version for it, of `release` when there is one (see [`resolve`]).
/// A pin that cannot be installed on the server is an [`UnusablePin`].
async fn resolve_modrinth(
    catalogue: &dyn Catalogue,
    server: &server_repo::ServerRow,
    project: String,
    pin: Option<String>,
    release: Vec<String>,
    dependencies: bool,
) -> anyhow::Result<(Artifact, Vec<Child>)> {
    let key = format!("mr:{project}");
    let mut naming = Naming {
        name: None,
        platform: DependencyPlatform::Modrinth,
        project_id: project.clone(),
    };
    let metadata = catalogue
        .mr_project(&project)
        .await
        .map_err(|error| naming.lookup(error))?;
    naming.name = Some(metadata.title.clone());
    // Only a project its authors say does not run on servers is refused.
    // Server search lists the projects that leave it unknown, and many of
    // them are server mods (SkinsRestorer).
    if matches!(metadata.server_side, ProjectSupportRange::Unsupported) {
        return Err(naming.because(SkippedDependencyReason::NotForServers));
    }
    let loader = server.modloader_type.as_deref();
    let accepted = accepted_loaders(loader, &server.game_version);
    let is_mod = metadata.project_type == ProjectType::Mod;
    let version = if let Some(pin) = &pin {
        let version = match catalogue.mr_version(pin).await {
            Ok(version) => version,
            Err(error) if rule::not_found(&error) => {
                return Err(naming.unusable_pin(SkippedDependencyReason::Unavailable, None));
            }
            Err(error) => return Err(naming.lookup(error)),
        };
        if version.project_id != project || version.id != *pin {
            return Err(naming.unusable_pin(SkippedDependencyReason::Unavailable, None));
        }
        version
    } else {
        // Asks for the loaders the server runs mods of (Fabric too on
        // Quilt); a vanilla server only takes a version that is not a jar.
        let versions = catalogue
            .mr_versions(ProjectVersionsFilters {
                project_id: ProjectID(project.clone()),
                game_versions: Some(vec![server.game_version.clone()]),
                loaders: (is_mod && !accepted.is_empty())
                    .then(|| accepted.iter().map(|loader| (*loader).to_owned()).collect()),
                limit: None,
                offset: None,
            })
            .await
            .map_err(|error| naming.lookup(error))?;
        if versions.iter().any(|version| version.project_id != project) {
            return Err(naming.because(SkippedDependencyReason::Unavailable));
        }
        let fit = |version: &carbon_platforms::modrinth::version::Version| {
            if !version.game_versions.contains(&server.game_version) {
                return None;
            }
            let fit = if is_jar(&primary_file(version)?.filename) {
                loader_fit(&version.loaders, loader, &server.game_version)
            } else {
                LoaderFit::Tagged
            };
            Some((fit, ModChannel::from(version.version_type)))
        };
        // The release a replaced pin names, built for this server, before
        // the newest version.
        let same_release: Vec<_> = if release.is_empty() {
            Vec::new()
        } else {
            versions
                .iter()
                .filter(|version| rule::release_of(version) == release)
                .cloned()
                .collect()
        };
        rule::best_candidate(&same_release, ModChannel::Stable, &fit)
            .or_else(|| rule::best_candidate(&versions, ModChannel::Stable, &fit))
            .cloned()
            .ok_or_else(|| naming.because(SkippedDependencyReason::NoCompatibleVersion))?
    };
    let Some(file) = primary_file(&version) else {
        return Err(naming.because(SkippedDependencyReason::Unavailable));
    };
    if let Err(detail) = validate_loader(
        &file.filename,
        &version.loaders,
        loader,
        &server.game_version,
    ) {
        tracing::info!(
            addon = %metadata.title,
            version = %version.version_number,
            "Server addon version does not fit the server's loader: {detail}"
        );
        return Err(if pin.is_some() {
            naming.foreign_pin(
                &version.loaders,
                loader,
                &server.game_version,
                Some(PinnedVersion {
                    number: version.version_number.clone(),
                    release: rule::release_of(&version),
                }),
            )
        } else {
            naming.because(SkippedDependencyReason::NoCompatibleVersion)
        });
    }
    let mut children = Vec::new();
    if dependencies {
        for dependency in version
            .dependencies
            .iter()
            .filter(|d| matches!(d.dependency_type, DependencyType::Required))
        {
            let child = match modrinth_requirement(dependency) {
                Some(Ok(child)) => child,
                // Declared by version only: the version names the project.
                Some(Err(pin)) => match catalogue.mr_version(&pin).await {
                    Ok(version) => {
                        Child::Project(Selection::Modrinth(version.project_id, Some(pin)))
                    }
                    Err(error) if rule::not_found(&error) => Child::Unresolvable(Unresolvable {
                        name: None,
                        platform: DependencyPlatform::Modrinth,
                        project_id: pin,
                        reason: SkippedDependencyReason::Unavailable,
                    }),
                    Err(error) => {
                        return Err(error.context(format!(
                            "Could not look up a required dependency of {}. Check your connection and retry.",
                            metadata.title
                        )));
                    }
                },
                None => {
                    tracing::debug!(
                        "Ignoring a required dependency that names no project, version or file"
                    );
                    continue;
                }
            };
            children.push(child);
        }
    }
    Ok((
        Artifact {
            key,
            version: version.id.clone(),
            name: metadata.title.clone(),
            version_name: version.version_number.clone(),
            filename: file.filename.clone(),
            url: file.url.clone(),
            size: file.size as u64,
            checksum: Checksum::Sha512(file.hashes.sha512.clone()),
            required: false,
            release: rule::release_of(&version),
        },
        children,
    ))
}

/// What a Modrinth requirement names: a project (with the version it pins),
/// or only a file name. `Err(version)`: only a version id, which names the
/// project once looked up. `None`: nothing at all.
fn modrinth_requirement(
    dependency: &carbon_platforms::modrinth::version::Dependency,
) -> Option<Result<Child, String>> {
    match (
        &dependency.project_id,
        &dependency.version_id,
        &dependency.file_name,
    ) {
        (Some(project), pin, _) => Some(Ok(Child::Project(Selection::Modrinth(
            project.clone(),
            pin.clone(),
        )))),
        (None, Some(pin), _) => Some(Err(pin.clone())),
        (None, None, Some(file_name)) if !file_name.trim().is_empty() => {
            Some(Ok(Child::File(file_name.clone())))
        }
        _ => None,
    }
}

/// The graph keys of the requirements an exact installed version declares
/// (see [`Previous`]); `mr-version:<id>` for one declared by version only.
async fn declared(
    catalogue: &dyn Catalogue,
    selection: Selection,
) -> anyhow::Result<HashSet<String>> {
    Ok(match selection {
        Selection::Curseforge(project, Some(file)) => catalogue
            .cf_file(i32::try_from(project)?, i32::try_from(file)?)
            .await?
            .dependencies
            .iter()
            .filter(|d| matches!(d.relation_type, FileRelationType::RequiredDependency))
            .map(|d| format!("cf:{}", d.mod_id))
            .collect(),
        Selection::Modrinth(_, Some(version)) => catalogue
            .mr_version(&version)
            .await?
            .dependencies
            .iter()
            .filter(|d| matches!(d.dependency_type, DependencyType::Required))
            .filter_map(|d| match modrinth_requirement(d)? {
                Ok(child) => Some(child.key()),
                Err(pin) => Some(format!("mr-version:{pin}")),
            })
            .collect(),
        _ => HashSet::new(),
    })
}

pub(super) async fn install(
    app: &Arc<AppInner>,
    id: ServerId,
    selection: Selection,
) -> anyhow::Result<VisualTaskId> {
    install_with(
        app,
        id,
        selection,
        Arc::new(Live(Arc::new(rule::Live(app.clone())))),
    )
    .await
}

async fn install_with(
    app: &Arc<AppInner>,
    id: ServerId,
    selection: Selection,
    resolver: Arc<dyn Resolver>,
) -> anyhow::Result<VisualTaskId> {
    let activity = app.runtime_lifecycle.admit_owned(
        format!("server {} addon installation", id.0),
        crate::managers::runtime_lifecycle::ActivityPolicy::Installer,
    )?;
    let app = app.clone();
    crate::managers::runtime_lifecycle::spawn_retained(activity, move |slot| {
        Box::pin(async move { install_owned(&app, id, selection, resolver, slot).await })
    })
    .await?
}

/// The loader mod id of a downloaded jar, while it is still privately staged.
async fn staged_mod_id(path: &Path) -> Option<String> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut file = std::fs::File::open(&path).ok()?;
        match crate::managers::metadata::mods::parse_metadata(&mut file) {
            Ok(meta) => meta?.modid,
            Err(error) => {
                tracing::debug!("Staged server addon has no readable mod metadata: {error:#}");
                None
            }
        }
    })
    .await
    .ok()
    .flatten()
    .as_deref()
    .and_then(normalized_mod_id)
}

/// Checks the dependencies the plan found installed once more, under the
/// publication gate and for those a published addon needs: one that is only
/// disabled (since the plan, or already then deeper in the graph) refuses a
/// fresh install, and one deleted meanwhile is reported as missing.
async fn recheck_present(
    present: &[Present],
    installed: &InstalledAddons,
    replacing: bool,
) -> anyhow::Result<Vec<TaskWarning>> {
    let mut warnings = Vec::new();
    for dependency in present {
        match installed.requirement(&dependency.requirement).await {
            Presence::Enabled(_) => {}
            Presence::DisabledOnly(_) if replacing => {}
            Presence::DisabledOnly(copies) => return Err(disabled_refusal(&copies)),
            Presence::Absent => warnings.push(
                Unresolvable {
                    name: Some(dependency.name.clone()),
                    platform: dependency.platform,
                    project_id: dependency.project_id.clone(),
                    reason: SkippedDependencyReason::RemovedDuringInstall,
                }
                .into(),
            ),
        }
    }
    Ok(warnings)
}

/// Which downloaded dependencies are published, and what that publishes.
struct Settled {
    publish: Vec<bool>,
    reach: Reach,
}

/// Decides, under the publication gate and before anything is published,
/// which downloaded dependencies to publish. One the server already has as a
/// copy no platform identifies (the same file name, enabled or as its
/// `.disabled` twin, or a jar with the same mod id, scanned or not) is kept as
/// it is: never overwritten and never published next to itself, so an
/// occupied destination never fails the installation. What only a kept
/// dependency required is not published either, since an installed copy's own
/// requirements are never walked. A copy that is there only disabled, with no
/// enabled jar of its mod id, refuses a fresh install as when the plan finds
/// one, and only while a published addon needs it.
async fn settle_dependencies(
    data: &Path,
    prepared: &[Prepared],
    modids: &[Option<String>],
    installed: &InstalledAddons,
    graph: &Graph,
    replacing: bool,
) -> anyhow::Result<Settled> {
    let mut held = HashSet::new();
    let mut refusals = HashMap::new();
    for (item, modid) in prepared.iter().zip(modids) {
        if !item.artifact.required {
            continue;
        }
        let name = &item.artifact.name;
        // Where publication writes it, through a linked `mods/` folder too.
        let occupied = match occupant_at(data, &item.location).await {
            Ok(state) => state,
            Err(error) => {
                tracing::warn!(
                    dependency = %name,
                    "Cannot inspect the destination of a required dependency, keeping what is there: {error:#}"
                );
                Some(true)
            }
        };
        let by_mod_id = match (occupied, modid) {
            (Some(true), _) | (_, None) => Presence::Absent,
            (_, Some(modid)) => match installed.mod_id(modid, &[]) {
                Presence::Enabled(copies) => Presence::Enabled(copies),
                _ => installed.mod_id(modid, installed.unscanned_jars().await),
            },
        };
        let presence = match occupied {
            Some(true) => Presence::Enabled(Vec::new()),
            // An enabled jar with the mod id is loaded; enabling the twin as
            // well would start the server with two mods of one id.
            Some(false) => match by_mod_id {
                Presence::Enabled(copies) => Presence::Enabled(copies),
                _ => Presence::DisabledOnly(vec![InstalledCopy {
                    name: name.clone(),
                    filename: item.artifact.filename.clone(),
                    mr_version_id: None,
                }]),
            },
            None => by_mod_id,
        };
        match presence {
            Presence::Absent => continue,
            Presence::DisabledOnly(copies) if !replacing => {
                refusals.insert(item.artifact.key.clone(), copies);
            }
            _ => tracing::info!(
                dependency = %name,
                filename = %item.artifact.filename,
                modid = ?modid,
                "The server already has this required dependency; keeping its copy"
            ),
        }
        held.insert(item.artifact.key.clone());
    }
    let mut reach = graph.reach(&held);
    // Two published jars with one mod id: the first is enough.
    let mut ids = HashSet::new();
    let mut duplicates = false;
    for (item, modid) in prepared.iter().zip(modids) {
        if !reach.published.contains(&item.artifact.key) {
            continue;
        }
        if let Some(modid) = modid {
            if !ids.insert(modid.clone()) && item.artifact.required {
                tracing::info!(
                    dependency = %item.artifact.name,
                    modid = %modid,
                    "Another addon of this installation has the mod id of this required dependency"
                );
                duplicates |= held.insert(item.artifact.key.clone());
            }
        }
    }
    if duplicates {
        reach = graph.reach(&held);
    }
    for item in prepared {
        if let Some(copies) = refusals.get(&item.artifact.key) {
            if reach.needed.contains(&item.artifact.key) {
                return Err(disabled_refusal(copies));
            }
        }
    }
    let publish = prepared
        .iter()
        .map(|item| reach.published.contains(&item.artifact.key))
        .collect();
    Ok(Settled { publish, reach })
}

async fn install_owned(
    app: &Arc<AppInner>,
    id: ServerId,
    selection: Selection,
    resolver: Arc<dyn Resolver>,
    activity: &mut Option<crate::managers::runtime_lifecycle::RuntimeActivity>,
) -> anyhow::Result<VisualTaskId> {
    let server = server_repo::get_server(&app.db, id.0)
        .await?
        .context("Server not found")?;
    // One installation per server at a time, so two plans never supersede
    // each other; the server gate stays free during metadata and downloads.
    let queue = app.server_manager.addon_queue(id);
    let task = crate::managers::vtask::VisualTask::new(
        crate::api::translation::Translation::ServerTaskInstallMod {
            mod_name: "Server addon and required dependencies".into(),
            server_name: server.name.clone(),
        },
    );
    let task_id = app.task_manager().spawn_task(&task).await;
    let activity = activity.take().unwrap();
    let app = app.clone();
    crate::managers::runtime_lifecycle::spawn_retained(activity, move |slot| {
        Box::pin(async move {
            let cancel = slot.as_ref().unwrap().cancellation();
            // This installation's private folder, once made (see below).
            let mut staging_dir = None;
            let result = async {
                // An Install clicked while another addon installs on this
                // server waits for it, then plans against what it published.
                let _turn = tokio::select! {
                    biased;
                    () = cancel.cancelled() => bail!("Server addon installation cancelled"),
                    turn = queue.lock_owned() => turn,
                };
                let server_path = app
                    .settings_manager()
                    .runtime_path
                    .get_servers()
                    .get_server_path(&server.shortpath);
                // Planned once an installation or stop of the server under
                // way has ended, against what it left.
                drop(
                    app.server_manager()
                        .addon_gate(id, &server.name, &server_path.get_root(), &cancel)
                        .await?,
                );
                let server = server_repo::get_server(&app.db, id.0)
                    .await?
                    .context("Server not found")?;
                let server_path = app
                    .settings_manager()
                    .runtime_path
                    .get_servers()
                    .get_server_path(&server.shortpath);
                let data = server_path.get_data_path();
                let loader = server.modloader_type.as_deref();
                let installed = InstalledAddons::read(
                    &data,
                    &mfcdb::get_server_mods_full(&app.db, id.0).await?,
                    loader,
                )
                .await;
                let Planned {
                    artifacts,
                    replacing,
                    graph,
                    present,
                    kept_pins,
                    warned,
                } = plan(&app, &server, selection, &installed, &*resolver).await?;
                for kept in &kept_pins {
                    tracing::warn!(
                        server = id.0,
                        dependency = %kept.name,
                        filename = %kept.filename,
                        pin = %kept.pin,
                        "Keeping the installed required dependency although a dependant pins another version"
                    );
                }
                let subject = artifacts
                    .first()
                    .map(|artifact| artifact.name.clone())
                    .unwrap_or_default();
                let root = app.settings_manager().runtime_path.to_path_buf();
                let staging = tempfile::Builder::new()
                    .prefix(SERVER_STAGING_PREFIX)
                    .tempdir_in(root)?
                    .keep();
                staging_dir = Some(staging.clone());
                let prepared = async {
                    let _guard = app
                        .server_manager()
                        .addon_gate(id, &server.name, &server_path.get_root(), &cancel)
                        .await?;
                    let cached = mfcdb::get_server_mods_full(&app.db, id.0).await?;
                    let mut prepared = Vec::new();
                    for (index, artifact) in artifacts.into_iter().enumerate() {
                        let folder = server_addon_dir(&server_path, &artifact.filename)?;
                        let location = folder
                            .join(&artifact.filename)
                            .strip_prefix(&data)?
                            .to_str()
                            .context("Invalid addon path")?
                            .replace(std::path::MAIN_SEPARATOR, "/");
                        let stage_path = staging.join(format!("{index}-{}", artifact.filename));
                        if let Some(item) = prepare_artifact(
                            &data,
                            &folder,
                            artifact,
                            location,
                            &cached,
                            &stage_path,
                        )
                        .await?
                        {
                            prepared.push(item);
                        }
                    }
                    Ok::<_, anyhow::Error>(prepared)
                }
                .await;
                // Refused before any download: copies of the selected addon
                // to choose between, or another file where it goes.
                let mut prepared = prepared?;
                for item in &prepared {
                    if let Some(source) = &item.reuse {
                        tokio::fs::copy(source, &item.download.path)
                            .await
                            .context("Satisfied bytes changed while staging")?;
                    }
                }
                let downloads: Vec<_> = prepared
                    .iter()
                    .filter(|item| item.reuse.is_none())
                    .map(|item| item.download.clone())
                    .collect();
                carbon_net::download_multiple(
                    &downloads,
                    DownloadOptions::builder()
                        .concurrency(4)
                        .cancel_token(cancel.clone())
                        .build(),
                )
                .await
                .context("Addon download failed")?;
                let mut modids = Vec::with_capacity(prepared.len());
                for item in &prepared {
                    modids.push(staged_mod_id(&item.download.path).await);
                }
                // A stop pressed meanwhile is waited for: the addon is
                // published once the server has stopped.
                let _guard = app
                    .server_manager()
                    .addon_gate(id, &server.name, &server_path.get_root(), &cancel)
                    .await?;
                // A running server may hold a copy a new version replaces.
                let running = app
                    .server_manager
                    .servers
                    .read()
                    .await
                    .get(&id)
                    .is_some_and(|server| matches!(server.state, ServerState::Running { .. }));
                anyhow::ensure!(
                    !cancel.is_cancelled(),
                    "Server addon installation cancelled before publication"
                );
                let current = server_repo::get_server(&app.db, id.0)
                    .await?
                    .context("Server was deleted during download")?;
                if current.shortpath != server.shortpath
                    || current.game_version != server.game_version
                    || current.modloader_type != server.modloader_type
                {
                    bail!("Server owner or version changed during download");
                }
                let rows = mfcdb::get_server_mods_full(&app.db, id.0).await?;
                // Nothing is published yet; the downloads are of no use.
                validate_plan(&data, &mut prepared, &rows).await?;
                let installed = InstalledAddons::read(&data, &rows, loader).await;
                let settled = match settle_dependencies(
                    &data, &prepared, &modids, &installed, &graph, replacing,
                )
                .await
                {
                    Ok(settled) => {
                        // Only what a published addon still needs.
                        let needed: Vec<Present> = present
                            .iter()
                            .filter(|present| {
                                settled.reach.needed.contains(&present.requirement.key())
                            })
                            .cloned()
                            .collect();
                        recheck_present(&needed, &installed, replacing)
                            .await
                            .map(|missing| (settled, missing))
                    }
                    Err(error) => Err(error),
                };
                let (publish, warnings) = match settled {
                    Ok((settled, missing)) => {
                        let warnings: Vec<TaskWarning> = warned
                            .into_iter()
                            .filter(|warned| warned.concern.holds(&settled.reach))
                            .map(|warned| warned.warning)
                            .chain(missing)
                            .collect();
                        (settled.publish, warnings)
                    }
                    // A refusal before anything is published (a required
                    // dependency is only there disabled): the downloads are
                    // of no use.
                    Err(error) => return Err(error),
                };
                for (index, (item, publish)) in prepared.into_iter().zip(publish).enumerate() {
                    if !publish {
                        if let Err(error) = tokio::fs::remove_file(&item.download.path).await {
                            if error.kind() != std::io::ErrorKind::NotFound {
                                return Err(error.into());
                            }
                        }
                        continue;
                    }
                    if !matches_download(&item.download.path, &item.download).await? {
                        bail!("Staged server addon failed verification");
                    }
                    let folder = server_addon_dir(&server_path, &item.artifact.filename)?;
                    let current_location = folder
                        .join(&item.artifact.filename)
                        .strip_prefix(&data)?
                        .to_str()
                        .context("Invalid addon path")?
                        .replace(std::path::MAIN_SEPARATOR, "/");
                    if current_location != item.location {
                        bail!("Selected world changed during download");
                    }
                    let kind = addon_kind(&item.artifact.filename);
                    if let Some(adopted) = &item.adopt {
                        // The destination already holds these bytes: the file
                        // is left as it is, and only its row and the selected
                        // source are recorded.
                        let (now, enabled) = selected_file(&folder, &item.artifact.filename)
                            .await?
                            .filter(|(now, _)| now == adopted)
                            .context("Server addon destination changed before publication")?;
                        if !matches_download(&now, &item.download).await? {
                            bail!("Server addon destination changed before publication");
                        }
                        let metadata_id = app
                            .meta_cache_manager()
                            .prepare_server_addon_metadata(
                                &item.download.path,
                                &item.artifact.filename,
                            )
                            .await?;
                        let identity_artifact = item.artifact.clone();
                        let location = item.location.clone();
                        app.db
                            .write(move |mut conn| {
                                Ok(publish_selection(
                                    &mut conn,
                                    id.0,
                                    kind,
                                    &location,
                                    enabled,
                                    &metadata_id,
                                    &identity_artifact,
                                    None,
                                ))
                            })
                            .await??;
                        tokio::fs::remove_file(&item.download.path).await?;
                        continue;
                    }
                    let mut old_path = None;
                    let mut enabled = true;
                    if let Some((row, hash)) = &item.old {
                        let now = mfcdb::get_server_mod_file_cache_by_id(&app.db, &row.id)
                            .await?
                            .context("Replacement owner disappeared")?;
                        if now.server_id != id.0
                            || now.location != row.location
                            || now.addon_type != row.addon_type
                        {
                            bail!("Replacement owner changed");
                        }
                        let physical = world::owned_addon(
                            &data,
                            &row.addon_type,
                            &row.location,
                            &row.filename,
                        )?;
                        if digest(&physical).await? != *hash {
                            bail!("Old addon changed during download");
                        }
                        enabled = !physical.to_string_lossy().ends_with(".disabled");
                        old_path = Some(physical);
                    }
                    selected_source(&item.artifact)?; // source feasibility before retiring any live bytes
                    let metadata_id = app
                        .meta_cache_manager()
                        .prepare_server_addon_metadata(&item.download.path, &item.artifact.filename)
                        .await?;
                    let identity_artifact = item.artifact.clone();
                    let location = item.location.clone();
                    let retired = item
                        .old
                        .as_ref()
                        .filter(|(row, _)| row.location != item.location)
                        .map(|(row, _)| row.id.clone());
                    let backup = staging.join(format!("previous-{index}"));
                    let db = &app.db;
                    publish_with_metadata(
                        &item.download,
                        &folder,
                        &item.artifact.filename,
                        old_path.as_deref(),
                        item.old.as_ref().map(|(_, hash)| hash.as_slice()),
                        enabled,
                        &backup,
                        async move {
                            db.write(move |mut conn| {
                                Ok(publish_selection(
                                    &mut conn,
                                    id.0,
                                    kind,
                                    &location,
                                    enabled,
                                    &metadata_id,
                                    &identity_artifact,
                                    retired.as_deref(),
                                ))
                            })
                            .await?
                        },
                    )
                    .await
                    .map_err(|error| {
                        explain_in_use(error, &item.artifact.name, &server.name, running)
                    })?;
                    if item.old.is_some() {
                        tokio::fs::remove_file(&backup).await?;
                    }
                }
                Ok::<_, anyhow::Error>((warnings, subject))
            }
            .await;
            // Whatever the outcome, the downloads are of no further use: a
            // failed or cancelled installation published none of them, or
            // only what it reports. Only a replaced addon that could not be
            // put back stays, where its error says (see `settle_staging`).
            if let Some(staging) = &staging_dir {
                settle_staging(staging).await;
            }
            let result = result.map(|(warnings, subject)| {
                if !warnings.is_empty() {
                    // The toast names the addon, not this task's generic name.
                    task.set_warning_subject(subject);
                }
                for warning in warnings {
                    tracing::warn!(
                        server = id.0,
                        ?warning,
                        "Server addon installed without a required dependency or with another version of one"
                    );
                    task.warn(warning);
                }
            });
            if result.is_ok() {
                app.meta_cache_manager()
                    .queue_caching(
                        crate::managers::metadata::cache::CacheEntityId::Server(id.0),
                        true,
                    )
                    .await;
            }
            retain_outcome(task, result).await;
            app.invalidate(GET_SERVER_ADDONS, None);
        })
    });
    Ok(task_id)
}

fn replacement_candidates<'a>(
    cached: &'a [mfcdb::ServerAddonFullRow],
    kind: &str,
    data: &Path,
    folder: &Path,
    artifact: &Artifact,
) -> Vec<&'a mfcdb::ServerAddonFullRow> {
    let Ok(namespace) = folder.strip_prefix(data) else {
        return Vec::new();
    };
    let namespace = namespace
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
    cached
        .iter()
        .filter(|row| {
            row.addon_type == kind
                && row
                    .location
                    .rsplit_once('/')
                    .is_some_and(|(parent, _)| parent == namespace)
                && same_project(row, artifact)
        })
        .collect()
}
/// Only the selected addon replaces an installed version of itself
/// (GPT-0024). A required dependency is planned only when the server does not
/// have its project, so it never takes over, replaces or deletes an installed
/// copy; one the planner could not identify is settled at publication.
async fn prepare_artifact(
    data: &Path,
    folder: &Path,
    artifact: Artifact,
    location: String,
    cached: &[mfcdb::ServerAddonFullRow],
    staging: &Path,
) -> anyhow::Result<Option<Prepared>> {
    let kind = addon_kind(&artifact.filename);
    let replaced = if artifact.required {
        None
    } else {
        let candidates = replacement_candidates(cached, kind, data, folder, &artifact);
        replaced_copy(data, &artifact, &location, candidates).await?
    };
    selected_source(&artifact)?;
    let download = Downloadable::new(&artifact.url, staging)
        .with_size(artifact.size)
        .with_checksum(Some(artifact.checksum.clone()));
    let mut reuse = None;
    let old = if let Some(row) = replaced {
        let physical = world::owned_addon(data, &row.addon_type, &row.location, &row.filename)?;
        let hash = digest(&physical).await?;
        if hash != row.sha512 {
            bail!("Installed addon changed since scan; refresh before replacing");
        }
        if matches_download(&physical, &download).await? {
            reuse = Some(physical);
        }
        Some((row.clone(), hash))
    } else {
        None
    };
    let mut item = Prepared {
        artifact,
        location,
        old,
        adopt: None,
        download,
        reuse,
    };
    // Before downloading: a file the selected addon cannot be published over
    // stops the installation now, and one with its bytes is not downloaded.
    check_destination(data, &mut item, cached).await?;
    if let Some(adopted) = &item.adopt {
        item.reuse = Some(adopted.clone());
    }
    Ok(Some(item))
}

fn addon_kind(filename: &str) -> &'static str {
    if filename.to_ascii_lowercase().ends_with(".zip") {
        "datapacks"
    } else {
        "mods"
    }
}

/// The installed copy the selected addon replaces, among the copies of its
/// project in its folder (on either platform) whose file is still there. With
/// several, as 2.0.40 left them when a version was picked next to another
/// (GPT-0024), the copy the server loads is replaced and disabled copies are
/// left as they are; when none is loaded, the one at the destination. Two
/// loaded copies, or several disabled ones elsewhere, are the user's to sort
/// out, and the refusal names them.
async fn replaced_copy<'a>(
    data: &Path,
    artifact: &Artifact,
    location: &str,
    candidates: Vec<&'a mfcdb::ServerAddonFullRow>,
) -> anyhow::Result<Option<&'a mfcdb::ServerAddonFullRow>> {
    let mut copies = Vec::new();
    for row in candidates {
        match occupant_at(data, &row.location).await {
            // The file is gone; the row is stale and the next scan drops it.
            Ok(None) => continue,
            Ok(Some(enabled)) => copies.push((row, enabled)),
            Err(error) => {
                tracing::debug!(
                    filename = %row.filename,
                    "Server addon cannot be inspected, counting it as loaded: {error:#}"
                );
                copies.push((row, true));
            }
        }
    }
    if copies.len() <= 1 {
        return Ok(copies.first().map(|(row, _)| *row));
    }
    let loaded: Vec<_> = copies.iter().filter(|(_, enabled)| *enabled).collect();
    match loaded.as_slice() {
        [(row, _)] => return Ok(Some(*row)),
        [] => {
            if let Some((row, _)) = copies.iter().find(|(row, _)| row.location == location) {
                return Ok(Some(*row));
            }
        }
        _ => {}
    }
    Err(installed_more_than_once(
        artifact,
        copies.iter().map(|(row, _)| row.filename.as_str()),
    ))
}

fn installed_more_than_once<'a>(
    artifact: &Artifact,
    filenames: impl Iterator<Item = &'a str>,
) -> anyhow::Error {
    let mut filenames: Vec<&str> = filenames.collect();
    filenames.sort_unstable();
    filenames.dedup();
    anyhow!(
        "{} is installed more than once ({}). Remove the copies you do not want from the server's Addons tab and retry.",
        artifact.name,
        filenames.join(", ")
    )
}

/// The selected addon is published under its file name, enabled or as its
/// `.disabled` twin, which has to be free or hold the copy it replaces. An
/// enabled file there with the bytes of the download is that version
/// already, installed by hand or from the other platform before a platform
/// identified it: it is adopted, so only its source is recorded and the file
/// is left as it is. The same bytes disabled are refused, since the server
/// would still not load them, and so are they when the plan replaces a copy
/// under another name, since the server would then have two. Any other file
/// stops the installation before anything is published, and the refusal
/// names it. Checked before the download and again under the publication
/// gate. A required dependency's destination is settled at publication
/// instead (see [`settle_dependencies`]).
async fn check_destination(
    data: &Path,
    item: &mut Prepared,
    rows: &[mfcdb::ServerAddonFullRow],
) -> anyhow::Result<()> {
    item.adopt = None;
    if item.artifact.required {
        return Ok(());
    }
    if item
        .old
        .as_ref()
        .is_some_and(|(row, _)| row.location == item.location)
    {
        // It replaces the copy at its destination.
        return Ok(());
    }
    world::validate_local(&item.location)?;
    let destination = data.join(&item.location);
    let folder = destination
        .parent()
        .context("Server addon destination has no folder")?;
    let filename = &item.artifact.filename;
    let name = &item.artifact.name;
    let folder_name = match addon_kind(filename) {
        "datapacks" => "datapacks folder",
        _ => "mods folder",
    };
    // An IO error is the reason; it is named without guessing a fix.
    let unreadable = |error: anyhow::Error| {
        error.context(format!(
            "Cannot install {name}: {filename} in the server's {folder_name} could not be read"
        ))
    };
    let occupant = match selected_file(folder, filename).await {
        Ok(occupant) => occupant,
        Err(error) => {
            return Err(match error.downcast_ref::<UnclearAddonFile>() {
                Some(UnclearAddonFile::Both) => anyhow!(
                    "Cannot install {name}: the server has both {filename} and {filename}.disabled. Delete one of them from the server's {folder_name} and retry."
                ),
                Some(UnclearAddonFile::NotAFile { enabled }) => anyhow!(
                    "Cannot install {name}: {filename}{} in the server's {folder_name} is a link or folder, not a file. Move it out of that folder and retry.",
                    if *enabled { "" } else { ".disabled" }
                ),
                None => unreadable(error),
            });
        }
    };
    let Some((occupant, enabled)) = occupant else {
        return Ok(());
    };
    let another_copy = rows.iter().find(|row| {
        row.location == item.location
            && row.addon_type == addon_kind(filename)
            && same_project(row, &item.artifact)
    });
    match &item.old {
        // A duplicate of the project sits where the copy it replaces goes.
        Some((old, _)) if another_copy.is_some() => {
            return Err(installed_more_than_once(
                &item.artifact,
                [old.filename.as_str(), filename.as_str()].into_iter(),
            ));
        }
        Some((old, _)) => {
            if matches_download(&occupant, &item.download)
                .await
                .map_err(unreadable)?
            {
                // The file names carry the versions; a CurseForge file's
                // display name usually starts with the project name too.
                return Err(anyhow!(
                    "The version of {name} you picked is already in the server's {folder_name} as {filename}{}, next to {}. Remove the copy you do not want from the server's {folder_name} and retry.",
                    if enabled { "" } else { ".disabled" },
                    old.filename
                ));
            }
        }
        None => {
            if matches_download(&occupant, &item.download)
                .await
                .map_err(unreadable)?
            {
                if !enabled {
                    return Err(anyhow!(
                        "{name} is already on the server but disabled ({filename}.disabled). Enable it in the server's Addons tab, or rename it to {filename} in the server's {folder_name}, and retry."
                    ));
                }
                tracing::info!(
                    addon = %item.artifact.name,
                    %filename,
                    "The server already has the selected addon's bytes at its destination; recording its source"
                );
                item.adopt = Some(occupant);
                return Ok(());
            }
        }
    }
    Err(anyhow!(
        "Cannot install {}: the server already has another file named {filename}{}. Remove it from the server's Addons tab and retry.",
        item.artifact.name,
        if enabled { "" } else { " (disabled)" }
    ))
}

/// Checks, under the publication gate and before anything is published, that
/// every copy the plan replaces is unchanged and that the selected addon's
/// destination can still take it (see [`check_destination`]).
async fn validate_plan(
    data: &Path,
    plan: &mut [Prepared],
    current: &[mfcdb::ServerAddonFullRow],
) -> anyhow::Result<()> {
    for item in plan.iter() {
        if let Some((row, expected)) = &item.old {
            let now = current
                .iter()
                .find(|now| now.id == row.id)
                .context("Replacement owner disappeared")?;
            if now.location != row.location || now.addon_type != row.addon_type {
                bail!("Replacement owner changed");
            }
            let physical = world::owned_addon(data, &row.addon_type, &row.location, &row.filename)?;
            if digest(&physical).await? != *expected {
                bail!("Old addon changed during download");
            }
        }
    }
    for item in plan.iter_mut() {
        check_destination(data, item, current).await?;
    }
    Ok(())
}
fn same_project(row: &mfcdb::ServerAddonFullRow, artifact: &Artifact) -> bool {
    row.cf_project_id.map(|id| format!("cf:{id}")) == Some(artifact.key.clone())
        || row.mr_project_id.as_ref().map(|id| format!("mr:{id}")) == Some(artifact.key.clone())
}

fn selected_source(artifact: &Artifact) -> anyhow::Result<(&str, &str)> {
    let (platform, project) = artifact
        .key
        .split_once(':')
        .context("Missing selected source")?;
    if !matches!(platform, "cf" | "mr") || project.is_empty() || artifact.version.is_empty() {
        bail!("Invalid selected source");
    }
    if platform == "cf" {
        project.parse::<u32>()?;
        artifact.version.parse::<u32>()?;
    }
    basename(&artifact.filename)?;
    Ok((platform, project))
}
fn publish_identity(
    conn: &impl carbon_repos::db_exec::WriteAccess,
    server_id: i32,
    kind: &str,
    location: &str,
    metadata_id: &str,
    artifact: &Artifact,
) -> anyhow::Result<()> {
    let (platform, project) = selected_source(artifact)?;
    let metadata = carbon_repos::repos::mod_metadata::get_metadata_by_id_conn(conn, metadata_id)?
        .context("Verified metadata disappeared")?;
    if mfcdb::publish_server_source_conn(
        conn,
        server_id,
        kind,
        location,
        platform,
        project,
        &artifact.version,
        &metadata.sha512,
        DbDateTime(Utc::now().fixed_offset()),
    )? != 1
    {
        bail!("Selected source no longer matches the current owner/hash");
    }
    Ok(())
}
fn publish_selection(
    conn: &mut carbon_repos::db_exec::WriteGuard<'_>,
    server_id: i32,
    kind: &str,
    location: &str,
    enabled: bool,
    metadata_id: &str,
    artifact: &Artifact,
    retired: Option<&str>,
) -> anyhow::Result<()> {
    selected_source(artifact)?;
    let transaction = conn.transaction()?;
    mfcdb::upsert_server_mod_file_cache_at_location_conn(
        &transaction,
        server_id,
        &artifact.filename,
        location,
        artifact.size.try_into()?,
        enabled,
        kind,
        metadata_id,
        DbDateTime(Utc::now().fixed_offset()),
    )?;
    publish_identity(
        &transaction,
        server_id,
        kind,
        location,
        metadata_id,
        artifact,
    )?;
    if let Some(retired) = retired {
        mfcdb::delete_server_mod_file_cache_by_id_conn(&transaction, retired)?;
    }
    transaction.commit()?;
    Ok(())
}
async fn publish_with_metadata<F>(
    download: &Downloadable,
    folder: &Path,
    filename: &str,
    old: Option<&Path>,
    expected_old: Option<&[u8]>,
    enabled: bool,
    backup: &Path,
    publication: F,
) -> anyhow::Result<()>
where
    F: std::future::Future<Output = anyhow::Result<()>>,
{
    publish_staged(
        download,
        folder,
        filename,
        old,
        expected_old,
        enabled,
        backup,
    )
    .await?;
    if let Err(error) = publication.await {
        let destination = folder.join(if enabled {
            filename.to_owned()
        } else {
            format!("{filename}.disabled")
        });
        let restored = async {
            if !matches_download(&destination, download).await? {
                bail!("Published bytes changed before recovery");
            }
            move_owned(&destination, &download.path).await?;
            if let Some(old) = old {
                move_owned(backup, old).await?;
            }
            Ok::<_, anyhow::Error>(())
        }
        .await;
        return match restored {
            Ok(()) => Err(error).context("Addon metadata publication failed; original files/rows restored and staged bytes retained"),
            Err(recovery) => Err(error).context(format!("Addon metadata publication failed; recovery unresolved at {}: {recovery:#}", backup.display())),
        };
    }
    Ok(())
}

async fn retain_outcome(task: crate::managers::vtask::VisualTask, result: anyhow::Result<()>) {
    if let Err(error) = result {
        task.fail(error).await;
    } else {
        drop(task);
    }
}
async fn publish_staged(
    download: &Downloadable,
    folder: &Path,
    filename: &str,
    old: Option<&Path>,
    expected_old: Option<&[u8]>,
    enabled: bool,
    backup: &Path,
) -> anyhow::Result<()> {
    basename(filename)?;
    if !matches_download(&download.path, download).await? {
        bail!("Staged server addon failed verification");
    }
    tokio::fs::create_dir_all(folder).await?;
    if let Some((occupied, _)) = selected_file(folder, filename).await? {
        if old != Some(occupied.as_path()) {
            bail!("Server addon destination collision");
        }
    }
    let destination = folder.join(if enabled {
        filename.to_owned()
    } else {
        format!("{filename}.disabled")
    });
    if let Some(old) = old {
        if Some(digest(old).await?.as_slice()) != expected_old {
            bail!("Old server addon changed before publication");
        }
        if let Err(error) = move_owned(old, backup).await {
            // Nothing has moved yet.
            if held_open(&error) {
                let filename = old
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| old.display().to_string());
                return Err(error.context(ReplacedCopyInUse { filename }));
            }
            return Err(error);
        }
        if Some(digest(backup).await?.as_slice()) != expected_old {
            let _ = move_owned(backup, old).await;
            bail!(
                "Old server addon changed at publication; recovery retained at {}",
                backup.display()
            );
        }
    }
    if let Err(error) = move_owned(&download.path, &destination).await {
        if let Some(old) = old {
            let _ = move_owned(backup, old).await;
        }
        return Err(error).context(format!(
            "Addon publication failed; recovery retained at {}",
            backup.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod task07_tests {
    use super::*;
    fn artifact(key: &str, version: &str) -> Artifact {
        Artifact {
            key: format!("mr:{key}"),
            version: version.into(),
            name: key.into(),
            version_name: version.into(),
            filename: format!("{key}.jar"),
            url: "https://fixture.invalid/never-requested".into(),
            size: 3,
            checksum: Checksum::Sha512(hex::encode(Sha512::digest(b"new"))),
            required: false,
            release: Vec::new(),
        }
    }
    fn review_fixture(
        data: &Path,
        location: &str,
        filename: &str,
        kind: &str,
    ) -> (rusqlite::Connection, mfcdb::ServerAddonFullRow) {
        use carbon_repos::db_exec::test_support::wg;
        let mut conn = rusqlite::Connection::open(data.join("fixture.db")).unwrap();
        carbon_repos::get_migrations()
            .0
            .to_latest(&mut conn)
            .unwrap();
        conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
        carbon_repos::repos::mod_metadata::insert_metadata_conn(
            &wg(&mut conn),
            "meta",
            1,
            &Sha512::digest(b"new"),
            &[0; 20],
            "forge",
            None,
            None,
            None,
            None,
            None,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .unwrap();
        mfcdb::upsert_server_mod_file_cache_at_location_conn(
            &wg(&mut conn),
            7,
            filename,
            location,
            3,
            true,
            kind,
            "meta",
            DbDateTime(Utc::now().fixed_offset()),
        )
        .unwrap();
        let mut selected = artifact("23", "1");
        selected.filename = filename.into();
        publish_identity(&wg(&mut conn), 7, kind, location, "meta", &selected).unwrap();
        let row = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7)
            .unwrap()
            .remove(0);
        (conn, row)
    }
    /// A dependency the server already has is never resolved, downloaded or
    /// replaced; it is only checked again under the publication gate. One
    /// disabled during the download still refuses a fresh install with the
    /// message that names it; one deleted during the download leaves the
    /// selected addon published with a warning that it is missing; other bytes
    /// are still an enabled copy, kept as they are.
    #[tokio::test]
    async fn task07r1_r2_satisfied_dependency_revalidated_after_operator_mutation() {
        use carbon_repos::db_exec::test_support::wg;
        for action in ["unchanged", "disable", "delete", "change"] {
            let root = tempfile::tempdir().unwrap();
            let folder = root.path().join("mods");
            std::fs::create_dir(&folder).unwrap();
            let file = folder.join("dependency.jar");
            std::fs::write(&file, b"new").unwrap();
            let (mut conn, row) =
                review_fixture(root.path(), "mods/dependency.jar", "dependency.jar", "mods");
            let installed =
                InstalledAddons::read(root.path(), std::slice::from_ref(&row), Some("forge")).await;
            // Production planner completes, then releases the gate for network.
            let gate = tokio::sync::Mutex::new(());
            let owner = gate.lock().await;
            let planned = plan_graph(
                Selection::Modrinth("root".into(), Some("1".into())),
                true,
                &installed,
                |selection| {
                    assert_eq!(
                        selection.key().0,
                        "mr:root",
                        "an installed dependency is never resolved"
                    );
                    std::future::ready(Ok((
                        artifact("root", "1"),
                        vec![Selection::Modrinth("23".into(), Some("other".into())).into()],
                    )))
                },
            )
            .await
            .unwrap();
            assert_eq!(
                planned.artifacts.len(),
                1,
                "only the selected addon is fetched"
            );
            assert_eq!(planned.present.len(), 1);
            drop(owner);
            tokio::task::yield_now().await;
            let operator = gate.lock().await;
            match action {
                "disable" => {
                    std::fs::rename(&file, folder.join("dependency.jar.disabled")).unwrap()
                }
                "delete" => std::fs::remove_file(&file).unwrap(),
                "change" => std::fs::write(&file, b"bad").unwrap(),
                _ => (),
            }
            if action == "disable" {
                mfcdb::update_server_mod_file_enabled_conn(
                    &wg(&mut conn),
                    &row.id,
                    false,
                    DbDateTime(Utc::now().fixed_offset()),
                )
                .unwrap();
            }
            if action == "delete" {
                mfcdb::delete_server_mod_file_cache_by_id_conn(&wg(&mut conn), &row.id).unwrap();
            }
            drop(operator);
            let publication = gate.lock().await;
            let current = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
            let now = InstalledAddons::read(root.path(), &current, Some("forge")).await;
            let result = recheck_present(&planned.present, &now, planned.replacing).await;
            match action {
                "disable" => {
                    let error = result.unwrap_err().to_string();
                    assert_eq!(
                        error,
                        "Required dependency dependency.jar is installed but disabled. Enable it in the server's Addons tab, or rename dependency.jar.disabled to dependency.jar in the server's mods folder, and retry."
                    );
                }
                "delete" => assert_eq!(
                    result.unwrap(),
                    vec![TaskWarning::SkippedDependency {
                        name: Some("dependency.jar".into()),
                        platform: DependencyPlatform::Modrinth,
                        project_id: "23".into(),
                        reason: SkippedDependencyReason::RemovedDuringInstall,
                    }],
                    "a deleted dependency is reported, the selected addon still installs"
                ),
                _ => assert!(result.unwrap().is_empty(), "{action}"),
            }
            drop(publication);
        }
    }
    #[test]
    fn task07r1_r3_inactive_world_is_not_replacement_candidate() {
        let root = tempfile::tempdir().unwrap();
        let (_conn, mut row) =
            review_fixture(root.path(), "A/datapacks/pack.zip", "pack.zip", "datapacks");
        let mut selected = artifact("23", "2");
        selected.filename = "pack.zip".into();
        let b = root.path().join("B/datapacks");
        assert_eq!(
            replacement_candidates(&[row.clone()], "datapacks", root.path(), &b, &selected).len(),
            0,
            "inactive A must not block first B installation"
        );
        let a = row.clone();
        row.id = "B-owner".into();
        row.location = "B/datapacks/pack.zip".into();
        let rows = [a, row];
        let found = replacement_candidates(&rows, "datapacks", root.path(), &b, &selected);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "B-owner");
    }
    #[tokio::test]
    async fn task07r1_r3_installs_and_replaces_b_while_a_owner_and_file_survive() {
        use carbon_repos::{db_exec::test_support::wg, repos::mod_metadata as metadata};
        let root = tempfile::tempdir().unwrap();
        for name in ["A", "B"] {
            std::fs::create_dir_all(root.path().join(format!("{name}/datapacks"))).unwrap();
        }
        std::fs::write(root.path().join("server.properties"), "level-name=B").unwrap();
        let a_path = root.path().join("A/datapacks/pack.zip");
        std::fs::write(&a_path, b"new").unwrap();
        let (mut conn, a) =
            review_fixture(root.path(), "A/datapacks/pack.zip", "pack.zip", "datapacks");
        let folder = root.path().join("B/datapacks");
        let mut b_id = None;
        for (version, bytes) in [("1", b"new"), ("2", b"two")] {
            let mut selected = artifact("23", version);
            selected.filename = "pack.zip".into();
            selected.checksum = Checksum::Sha512(hex::encode(Sha512::digest(bytes)));
            let cached = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
            let staged = root.path().join(format!("stage-{version}.zip"));
            let item = prepare_artifact(
                root.path(),
                &folder,
                selected.clone(),
                "B/datapacks/pack.zip".into(),
                &cached,
                &staged,
            )
            .await
            .unwrap()
            .unwrap();
            assert_eq!(item.old.is_some(), version == "2");
            std::fs::write(&staged, bytes).unwrap();
            validate_plan(root.path(), &mut [item], &cached)
                .await
                .unwrap();
            let meta = if version == "1" { "meta" } else { "two" };
            if version == "2" {
                metadata::insert_metadata_conn(
                    &wg(&mut conn),
                    meta,
                    2,
                    &Sha512::digest(bytes),
                    &[1; 20],
                    "",
                    None,
                    None,
                    None,
                    None,
                    None,
                    DbDateTime(Utc::now().fixed_offset()),
                )
                .unwrap();
            }
            let old = (version == "2").then(|| folder.join("pack.zip"));
            let old_hash = Sha512::digest(b"new");
            let download = Downloadable::new(&selected.url, &staged)
                .with_size(3)
                .with_checksum(Some(selected.checksum.clone()));
            publish_with_metadata(
                &download,
                &folder,
                "pack.zip",
                old.as_deref(),
                old.as_ref().map(|_| old_hash.as_slice()),
                true,
                &root.path().join(format!("backup-{version}")),
                async {
                    publish_selection(
                        &mut wg(&mut conn),
                        7,
                        "datapacks",
                        "B/datapacks/pack.zip",
                        true,
                        meta,
                        &selected,
                        None,
                    )
                },
            )
            .await
            .unwrap();
            let now = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
            assert_eq!(now.len(), 2);
            assert!(
                now.iter()
                    .any(|row| row.id == a.id && row.location == a.location)
            );
            let b = now
                .iter()
                .find(|row| row.location == "B/datapacks/pack.zip")
                .unwrap();
            if let Some(id) = &b_id {
                assert_eq!(&b.id, id);
            } else {
                b_id = Some(b.id.clone());
            }
            assert_eq!(std::fs::read(&a_path).unwrap(), b"new");
            assert_eq!(std::fs::read(folder.join("pack.zip")).unwrap(), bytes);
        }
        assert!(world::owned_addon(root.path(), "datapacks", &a.location, &a.filename).is_err());
    }

    #[test]
    fn task07r1_r4_identical_bytes_accept_distinct_selected_versions() {
        use carbon_repos::db_exec::test_support::wg;
        for provider in ["cf", "mr"] {
            let root = tempfile::tempdir().unwrap();
            let (mut conn, _row) = review_fixture(root.path(), "mods/one.jar", "one.jar", "mods");
            let mut first = artifact("23", "1");
            first.key = format!("{provider}:23");
            publish_identity(&wg(&mut conn), 7, "mods", "mods/one.jar", "meta", &first).unwrap();
            let mut second = first.clone();
            second.version = "2".into();
            assert!(
                publish_identity(&wg(&mut conn), 7, "mods", "mods/one.jar", "meta", &second)
                    .is_ok(),
                "same physical metadata must support a legitimate second selected version"
            );
        }
    }
    #[test]
    fn task07r1_r5_selected_jar_loader_is_independent_of_discovery_filter() {
        let forge = vec!["forge".into()];
        // Both specific/latest resolution branches converge on this selected-file validator.
        assert!(
            validate_loader("pack.jar", &forge, None, "1.20.1").is_err(),
            "vanilla cannot load a Forge jar"
        );
        assert!(validate_loader("pack.jar", &forge, Some("fabric"), "1.20.1").is_err());
        validate_loader("pack.jar", &forge, Some("forge"), "1.20.1").unwrap();
        validate_loader("pack.zip", &["datapack".into()], None, "1.20.1").unwrap();
    }

    #[tokio::test]
    async fn task07r1_r4_shared_hash_selection_and_sql_failure_restore_current_owner() {
        use carbon_repos::{db_exec::test_support::wg, repos::mod_metadata as metadata};
        for provider in ["cf", "mr"] {
            for same_name in [false, true] {
                for equal_bytes in [false, true] {
                    for enabled in [false, true] {
                        for fault in ["none", "row", "source"] {
                            let root = tempfile::tempdir().unwrap();
                            let folder = root.path().join("mods");
                            std::fs::create_dir(&folder).unwrap();
                            let mut conn =
                                rusqlite::Connection::open(root.path().join("fixture.db")).unwrap();
                            carbon_repos::get_migrations()
                                .0
                                .to_latest(&mut conn)
                                .unwrap();
                            conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
                            let old_bytes = if equal_bytes { b"new" } else { b"old" };
                            let old_meta = if equal_bytes { "shared" } else { "old" };
                            metadata::insert_metadata_conn(
                                &wg(&mut conn),
                                "shared",
                                1,
                                &Sha512::digest(b"new"),
                                &[0; 20],
                                "forge",
                                None,
                                None,
                                None,
                                None,
                                None,
                                DbDateTime(Utc::now().fixed_offset()),
                            )
                            .unwrap();
                            if !equal_bytes {
                                metadata::insert_metadata_conn(
                                    &wg(&mut conn),
                                    "old",
                                    2,
                                    &Sha512::digest(old_bytes),
                                    &[1; 20],
                                    "forge",
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    DbDateTime(Utc::now().fixed_offset()),
                                )
                                .unwrap();
                            }
                            // This is the actual hash lookup used by ensure_mod_metadata, with a
                            // pre-existing legitimate v3 owner. Identical bytes reuse that ID.
                            let shared = metadata::find_metadata_by_hashes_conn(
                                &wg(&mut conn),
                                &Sha512::digest(b"new"),
                                1,
                            )
                            .unwrap()
                            .unwrap();
                            assert_eq!(shared.id, "shared");
                            let mut first = artifact("23", "1");
                            first.key = format!("{provider}:23");
                            first.filename = "old.jar".into();
                            publish_selection(
                                &mut wg(&mut conn),
                                7,
                                "mods",
                                "mods/old.jar",
                                enabled,
                                old_meta,
                                &first,
                                None,
                            )
                            .unwrap();
                            let mut other = artifact("23", "3");
                            other.key = first.key.clone();
                            other.filename = "shared.jar".into();
                            publish_selection(
                                &mut wg(&mut conn),
                                8,
                                "mods",
                                "mods/shared.jar",
                                true,
                                "shared",
                                &other,
                                None,
                            )
                            .unwrap();
                            let old_path = folder.join(if enabled {
                                "old.jar"
                            } else {
                                "old.jar.disabled"
                            });
                            std::fs::write(&old_path, old_bytes).unwrap();
                            std::fs::write(root.path().join("other-server.jar"), b"new").unwrap();
                            let before = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7)
                                .unwrap()
                                .remove(0);
                            let other_before =
                                mfcdb::get_server_mod_files_by_server_conn(&wg(&mut conn), 8)
                                    .unwrap();
                            let mut selected = artifact("23", "2");
                            selected.key = first.key.clone();
                            selected.filename =
                                if same_name { "old.jar" } else { "new.jar" }.into();
                            let location = format!("mods/{}", selected.filename);
                            let stage = root.path().join("staged.jar");
                            let item = prepare_artifact(
                                root.path(),
                                &folder,
                                selected.clone(),
                                location.clone(),
                                &[before.clone()],
                                &stage,
                            )
                            .await
                            .unwrap()
                            .unwrap();
                            assert!(
                                item.old.is_some(),
                                "explicit same-byte selection must publish its new owner source"
                            );
                            if equal_bytes {
                                assert!(item.reuse.is_some());
                            }
                            std::fs::write(&stage, b"new").unwrap();
                            if fault == "row" {
                                conn.execute_batch("CREATE TRIGGER fail_row BEFORE INSERT ON ServerModFileCache WHEN NEW.serverId=7 BEGIN SELECT RAISE(ABORT,'injected row failure'); END").unwrap();
                            }
                            if fault == "source" {
                                conn.execute_batch("CREATE TRIGGER fail_source BEFORE UPDATE OF sourceVersionId ON ServerModFileCache WHEN NEW.serverId=7 BEGIN SELECT RAISE(ABORT,'injected source failure'); END").unwrap();
                            }
                            let backup = root.path().join("backup");
                            let expected = Sha512::digest(old_bytes);
                            let retired = (!same_name).then_some(before.id.as_str());
                            let result = publish_with_metadata(
                                &item.download,
                                &folder,
                                &selected.filename,
                                Some(&old_path),
                                Some(&expected),
                                enabled,
                                &backup,
                                async {
                                    publish_selection(
                                        &mut wg(&mut conn),
                                        7,
                                        "mods",
                                        &location,
                                        enabled,
                                        &shared.id,
                                        &selected,
                                        retired,
                                    )
                                },
                            )
                            .await;
                            let current =
                                mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
                            assert_eq!(current.len(), 1);
                            if fault == "none" {
                                result.unwrap();
                                assert_eq!(current[0].source_version_id.as_deref(), Some("2"));
                                assert_eq!(current[0].filename, selected.filename);
                                assert_eq!(current[0].enabled, enabled);
                                if same_name {
                                    assert_eq!(current[0].id, before.id);
                                }
                                assert_eq!(
                                    std::fs::read(folder.join(if enabled {
                                        selected.filename.clone()
                                    } else {
                                        format!("{}.disabled", selected.filename)
                                    }))
                                    .unwrap(),
                                    b"new"
                                );
                            } else {
                                assert!(result.is_err());
                                assert_eq!(current[0].id, before.id);
                                assert_eq!(current[0].source_version_id.as_deref(), Some("1"));
                                assert_eq!(std::fs::read(&old_path).unwrap(), old_bytes);
                                assert!(stage.exists());
                                assert!(!backup.exists());
                                if !same_name {
                                    assert!(!folder.join("new.jar").exists());
                                    assert!(!folder.join("new.jar.disabled").exists());
                                }
                            }
                            assert_eq!(
                                mfcdb::get_server_mod_files_by_server_conn(&wg(&mut conn), 8)
                                    .unwrap(),
                                other_before
                            );
                            assert_eq!(
                                std::fs::read(root.path().join("other-server.jar")).unwrap(),
                                b"new"
                            );
                        }
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn task07_immediate_versions_replace_without_background_enrichment() {
        use carbon_repos::{db_exec::test_support::wg, repos::mod_metadata as metadata};
        for provider in ["mr", "cf"] {
            for same_name in [false, true] {
                let root = tempfile::tempdir().unwrap();
                let mut conn = rusqlite::Connection::open(root.path().join("fixture.db")).unwrap();
                carbon_repos::get_migrations()
                    .0
                    .to_latest(&mut conn)
                    .unwrap();
                conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
                let folder = root.path().join("mods");
                std::fs::create_dir(&folder).unwrap();
                let mut previous_id = None;
                for (version, bytes) in [("1", b"one"), ("2", b"two")] {
                    let mut selected = artifact("23", version);
                    selected.key = format!("{provider}:23");
                    selected.filename = if same_name {
                        "same.jar".into()
                    } else {
                        format!("version-{version}.jar")
                    };
                    selected.checksum = Checksum::Sha512(hex::encode(Sha512::digest(bytes)));
                    let rows = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
                    let old = rows.iter().find(|row| same_project(row, &selected));
                    if version == "2" {
                        assert!(
                            old.is_some(),
                            "known source identity must be visible without any enrichment worker"
                        );
                    }
                    let old_path = old.map(|row| folder.join(format!("{}.disabled", row.filename)));
                    let staged = root.path().join(format!("stage-{version}"));
                    std::fs::write(&staged, bytes).unwrap();
                    let download = Downloadable::new(&selected.url, &staged)
                        .with_size(3)
                        .with_checksum(Some(selected.checksum.clone()));
                    let backup = root.path().join(format!("backup-{version}"));
                    publish_staged(
                        &download,
                        &folder,
                        &selected.filename,
                        old_path.as_deref(),
                        old.map(|row| row.sha512.as_slice()),
                        false,
                        &backup,
                    )
                    .await
                    .unwrap();
                    let meta_id = format!("meta-{version}");
                    metadata::insert_metadata_conn(
                        &wg(&mut conn),
                        &meta_id,
                        version.parse().unwrap(),
                        &Sha512::digest(bytes),
                        &[0; 20],
                        "forge",
                        None,
                        None,
                        None,
                        None,
                        None,
                        DbDateTime(Utc::now().fixed_offset()),
                    )
                    .unwrap();
                    let location = format!("mods/{}", selected.filename);
                    mfcdb::upsert_server_mod_file_cache_at_location_conn(
                        &wg(&mut conn),
                        7,
                        &selected.filename,
                        &location,
                        3,
                        false,
                        "mods",
                        &meta_id,
                        DbDateTime(Utc::now().fixed_offset()),
                    )
                    .unwrap();
                    publish_identity(&wg(&mut conn), 7, "mods", &location, &meta_id, &selected)
                        .unwrap();
                    if let Some(old) = old {
                        if old.location != location {
                            mfcdb::delete_server_mod_file_cache_by_id_conn(&wg(&mut conn), &old.id)
                                .unwrap();
                        }
                        std::fs::remove_file(backup).unwrap();
                    }
                    let current = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
                    assert_eq!(current.len(), 1);
                    assert!(!current[0].enabled);
                    assert!(same_project(&current[0], &selected));
                    if same_name && version == "2" {
                        assert_eq!(Some(&current[0].id), previous_id.as_ref());
                    }
                    previous_id = Some(current[0].id.clone());
                    assert_eq!(
                        std::fs::read(folder.join(format!("{}.disabled", selected.filename)))
                            .unwrap(),
                        bytes
                    );
                    // A different selected source belongs to this owner, without modifying shared hash metadata.
                    let mut conflict = selected.clone();
                    conflict.version = "3".into();
                    assert!(
                        publish_identity(&wg(&mut conn), 7, "mods", &location, &meta_id, &conflict)
                            .is_ok()
                    );
                    assert!(same_project(
                        &mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap()[0],
                        &selected
                    ));
                }
                assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 1);
                assert_eq!(
                    conn.query_row("SELECT count(*) FROM ModMetadata", [], |row| row
                        .get::<_, i64>(0))
                        .unwrap(),
                    2
                );
                let before_cf =
                    metadata::get_cf_cache_by_metadata_conn(&wg(&mut conn), "meta-2").unwrap();
                let before_mr =
                    metadata::get_mr_cache_by_metadata_conn(&wg(&mut conn), "meta-2").unwrap();
                metadata::insert_metadata_conn(
                    &wg(&mut conn),
                    "foreign",
                    99,
                    &[99; 64],
                    &[99; 20],
                    "forge",
                    None,
                    None,
                    None,
                    None,
                    None,
                    DbDateTime(Utc::now().fixed_offset()),
                )
                .unwrap();
                let mut colliding = artifact("23", "2");
                colliding.key = format!("{provider}:23");
                assert!(
                    publish_identity(
                        &wg(&mut conn),
                        7,
                        "mods",
                        "mods/missing.jar",
                        "foreign",
                        &colliding
                    )
                    .is_err()
                );
                assert_eq!(
                    metadata::get_cf_cache_by_metadata_conn(&wg(&mut conn), "meta-2").unwrap(),
                    before_cf
                );
                assert_eq!(
                    metadata::get_mr_cache_by_metadata_conn(&wg(&mut conn), "meta-2").unwrap(),
                    before_mr
                );
                assert_eq!(
                    mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7)
                        .unwrap()
                        .len(),
                    1
                );
            }
        }
    }

    #[tokio::test]
    async fn task07_required_graph_deduplicates_cycles_and_honors_setting() {
        let installed = InstalledAddons::default();
        let plan = plan_graph(
            Selection::Modrinth("a".into(), Some("v1".into())),
            true,
            &installed,
            |selection| async move {
                let Selection::Modrinth(key, _) = selection else {
                    unreachable!()
                };
                Ok((
                    artifact(&key, "v1"),
                    vec![
                        Selection::Modrinth(
                            if key == "a" { "b" } else { "a" }.into(),
                            Some("v1".into()),
                        )
                        .into(),
                    ],
                ))
            },
        )
        .await
        .unwrap();
        assert_eq!(
            plan.artifacts
                .iter()
                .map(|a| (a.key.as_str(), a.required))
                .collect::<Vec<_>>(),
            vec![("mr:a", false), ("mr:b", true)]
        );
        assert!(plan.warnings().is_empty());
        let plan = plan_graph(
            Selection::Modrinth("a".into(), None),
            false,
            &installed,
            |_| async {
                Ok((
                    artifact("a", "v1"),
                    vec![Selection::Modrinth("b".into(), None).into()],
                ))
            },
        )
        .await
        .unwrap();
        assert_eq!(plan.artifacts.len(), 1);
    }
    /// Two dependants pinning different versions of one project keep the
    /// first and name the dependency in a warning; the selected addon still
    /// installs. A lookup failure that is not "cannot be installed" (offline,
    /// a server error) stays an error, so a transient problem never installs
    /// an addon without its dependency.
    #[tokio::test]
    async fn task07_dependency_pin_conflict_warns_and_lookup_failure_is_an_error() {
        let installed = InstalledAddons::default();
        let plan = plan_graph(
            Selection::Modrinth("a".into(), Some("v1".into())),
            true,
            &installed,
            |_| async {
                Ok((
                    artifact("a", "v1"),
                    vec![Selection::Modrinth("a".into(), Some("v2".into())).into()],
                ))
            },
        )
        .await
        .unwrap();
        assert_eq!(plan.artifacts.len(), 1);
        assert_eq!(
            plan.warnings(),
            vec![TaskWarning::ConflictingVersions {
                name: "a".into(),
                platform: DependencyPlatform::Modrinth,
                project_id: "a".into(),
                installed: "v1".into(),
            }]
        );
        let result = plan_graph(
            Selection::Modrinth("a".into(), None),
            true,
            &installed,
            |selection| async move {
                let Selection::Modrinth(key, _) = selection else {
                    unreachable!()
                };
                if key == "b" {
                    bail!("fixture required metadata missing");
                }
                Ok((
                    artifact("a", "v1"),
                    vec![Selection::Modrinth("b".into(), None).into()],
                ))
            },
        )
        .await;
        assert!(
            result
                .err()
                .unwrap()
                .to_string()
                .contains("metadata missing")
        );
    }
    #[tokio::test]
    async fn task07_replacement_preserves_disabled_and_same_path_and_failure() {
        for same_path in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let folder = root.path().join("mods");
            std::fs::create_dir(&folder).unwrap();
            let old = folder.join("v1.jar.disabled");
            std::fs::write(&old, b"old").unwrap();
            let staged = root.path().join("staged");
            std::fs::write(&staged, b"bad").unwrap();
            let backup = root.path().join("backup");
            let download = Downloadable::new("https://fixture.invalid/not-requested", &staged)
                .with_size(3)
                .with_checksum(Some(Checksum::Sha512(hex::encode(Sha512::digest(b"new")))));
            let filename = if same_path { "v1.jar" } else { "v2.jar" };
            let expected = digest(&old).await.unwrap();
            assert!(
                publish_staged(
                    &download,
                    &folder,
                    filename,
                    Some(&old),
                    Some(&expected),
                    false,
                    &backup
                )
                .await
                .is_err()
            );
            assert_eq!(std::fs::read(&old).unwrap(), b"old");
            std::fs::write(&staged, b"new").unwrap();
            publish_staged(
                &download,
                &folder,
                filename,
                Some(&old),
                Some(&expected),
                false,
                &backup,
            )
            .await
            .unwrap();
            std::fs::remove_file(&backup).unwrap(); // DB publication has completed
            assert_eq!(
                std::fs::read(folder.join(format!("{filename}.disabled"))).unwrap(),
                b"new"
            );
            assert!(!folder.join(filename).exists());
            assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 1);
        }
    }
    #[tokio::test]
    async fn task07_replacement_rejects_changed_old_and_unrelated_collision() {
        let root = tempfile::tempdir().unwrap();
        let staged = root.path().join("staged");
        std::fs::write(&staged, b"new").unwrap();
        let download = Downloadable::new("https://fixture.invalid/not-requested", &staged)
            .with_size(3)
            .with_checksum(Some(Checksum::Sha512(hex::encode(Sha512::digest(b"new")))));
        let old = root.path().join("old.jar");
        std::fs::write(&old, b"old").unwrap();
        let expected = digest(&old).await.unwrap();
        std::fs::write(&old, b"edit").unwrap();
        assert!(
            publish_staged(
                &download,
                root.path(),
                "new.jar",
                Some(&old),
                Some(&expected),
                true,
                &root.path().join("backup")
            )
            .await
            .is_err()
        );
        std::fs::write(root.path().join("new.jar"), b"unrelated").unwrap();
        assert!(
            publish_staged(
                &download,
                root.path(),
                "new.jar",
                None,
                None,
                true,
                &root.path().join("backup")
            )
            .await
            .is_err()
        );
        assert_eq!(std::fs::read(&old).unwrap(), b"edit");
        assert_eq!(
            std::fs::read(root.path().join("new.jar")).unwrap(),
            b"unrelated"
        );
    }
    #[tokio::test]
    async fn task07_artifact_failure_is_retained_in_actual_visual_task() {
        let task = crate::managers::vtask::VisualTask::new(
            crate::api::translation::Translation::ServerTaskInstallMod {
                mod_name: "fixture".into(),
                server_name: "fixture".into(),
            },
        );
        let observer = task.clone();
        retain_outcome(task, Err(anyhow!("fixture artifact checksum failed"))).await;
        let actual = observer.make_domain_task(VisualTaskId(7)).await;
        assert!(
            matches!(actual.progress, crate::domain::vtask::Progress::Failed(error) if error.to_string().contains("checksum failed"))
        );
    }
}

/// Required dependencies follow the rule instances use: an installed copy is
/// kept as it is, a disabled-only copy refuses a fresh install, and a missing
/// dependency that cannot be resolved is skipped with a named warning.
#[cfg(test)]
mod dependency_tests {
    use super::*;
    use std::sync::Mutex;

    fn artifact(key: &str, version: &str, filename: &str, bytes: &[u8]) -> Artifact {
        let (_, project) = key.split_once(':').unwrap();
        Artifact {
            key: key.into(),
            version: version.into(),
            name: format!("{project} title"),
            version_name: format!("{project} {version}"),
            filename: filename.into(),
            url: "https://fixture.invalid/never-requested".into(),
            size: bytes.len() as u64,
            checksum: Checksum::Sha512(hex::encode(Sha512::digest(bytes))),
            required: false,
            release: Vec::new(),
        }
    }

    /// One installed addon, as the planner sees it.
    fn copy(filename: &str, enabled: bool, keys: &[&str], version: Option<&str>) -> InstalledAddon {
        InstalledAddon {
            copy: InstalledCopy {
                name: filename.trim_end_matches(".jar").into(),
                filename: filename.into(),
                mr_version_id: version.map(Into::into),
            },
            enabled,
            keys: keys.iter().map(|key| key.to_string()).collect(),
            versions: Vec::new(),
            modid: None,
        }
    }

    fn installed(addons: Vec<InstalledAddon>, loader: &str) -> InstalledAddons {
        InstalledAddons {
            addons,
            loader: Some(loader.into()),
            ..Default::default()
        }
    }

    type Resolved = std::future::Ready<anyhow::Result<(Artifact, Vec<Child>)>>;

    /// A fake platform: `graph` maps a planner key to the requirements of its
    /// artifact; any other project has no version for the server. Every call
    /// is recorded.
    fn platform<'a>(
        calls: &'a Mutex<Vec<String>>,
        graph: &'a [(&'a str, Vec<Child>)],
    ) -> impl FnMut(Selection) -> Resolved + 'a {
        move |selection| {
            let (key, pin) = selection.key();
            let (platform, project_id) = selection.platform();
            calls.lock().unwrap().push(key.clone());
            std::future::ready(match graph.iter().find(|(known, _)| *known == key) {
                Some((_, children)) => Ok((
                    artifact(
                        &key,
                        pin.as_deref().unwrap_or("latest"),
                        &format!("{project_id}.jar"),
                        b"bytes",
                    ),
                    children.clone(),
                )),
                None => Err(Unresolvable {
                    name: Some(format!("{project_id} title")),
                    platform,
                    project_id,
                    reason: SkippedDependencyReason::NoCompatibleVersion,
                }
                .into()),
            })
        }
    }

    fn mr(project: &str) -> Child {
        Selection::Modrinth(project.into(), None).into()
    }

    fn keys(planned: &Planned) -> Vec<&str> {
        planned
            .artifacts
            .iter()
            .map(|artifact| artifact.key.as_str())
            .collect()
    }

    fn root() -> Selection {
        Selection::Modrinth("addon".into(), Some("1".into()))
    }

    // (a) A required child installed at another version is kept: never
    // resolved, its own requirements never walked, never planned.
    #[tokio::test]
    async fn installed_dependency_at_another_version_is_kept_and_not_walked() {
        let calls = Mutex::new(Vec::new());
        let graph = [
            ("mr:addon", vec![mr("create")]),
            ("mr:create", vec![mr("flywheel")]),
        ];
        // Duplicates and a disabled sibling are all copies the server has.
        let server = installed(
            vec![
                copy(
                    "create-0.5.1.jar",
                    true,
                    &["mr:create", "cf:328085"],
                    Some("v051"),
                ),
                copy("create-0.5.0.jar", true, &["mr:create"], Some("v050")),
                copy("create-old.jar", false, &["mr:create"], None),
            ],
            "forge",
        );
        let planned = plan_graph(root(), true, &server, platform(&calls, &graph))
            .await
            .unwrap();
        assert_eq!(keys(&planned), vec!["mr:addon"]);
        assert_eq!(*calls.lock().unwrap(), vec!["mr:addon"]);
        assert_eq!(
            planned.present,
            vec![Present {
                requirement: Requirement::Project("mr:create".into()),
                name: "create-0.5.1".into(),
                platform: DependencyPlatform::Modrinth,
                project_id: "create".into(),
            }]
        );
        assert!(planned.warnings().is_empty());
        assert!(planned.kept_pins.is_empty());
        assert!(!planned.replacing);
    }

    // (b) A disabled-only child refuses a fresh install with the named
    // message; an enabled copy next to it satisfies it; when the selected
    // addon replaces an installed version of itself it counts as present.
    #[tokio::test]
    async fn disabled_only_dependency_refuses_a_fresh_install_but_not_a_replacement() {
        let graph = [("mr:addon", vec![mr("lib")]), ("mr:lib", vec![])];
        let disabled = || copy("lib.jar", false, &["mr:lib"], None);

        let calls = Mutex::new(Vec::new());
        let refused = plan_graph(
            root(),
            true,
            &installed(vec![disabled()], "fabric"),
            platform(&calls, &graph),
        )
        .await
        .err()
        .expect("a fresh install refuses a disabled-only dependency");
        assert_eq!(
            refused.to_string(),
            "Required dependency lib (lib.jar) is installed but disabled. Enable it in the server's Addons tab, or rename lib.jar.disabled to lib.jar in the server's mods folder, and retry."
        );
        assert_eq!(
            *calls.lock().unwrap(),
            vec!["mr:addon"],
            "nothing else is fetched"
        );

        let two = plan_graph(
            root(),
            true,
            &installed(
                vec![disabled(), {
                    let mut second = copy("lib-2.jar", false, &["mr:lib"], None);
                    second.copy.name = "lib".into();
                    second
                }],
                "fabric",
            ),
            platform(&Mutex::new(Vec::new()), &graph),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(
            two.to_string(),
            "Required dependency lib is installed but disabled (lib-2.jar, lib.jar). Enable one of them in the server's Addons tab, or remove .disabled from the end of its name in the server's mods folder, and retry."
        );

        let calls = Mutex::new(Vec::new());
        let enabled_next_to_it = plan_graph(
            root(),
            true,
            &installed(
                vec![disabled(), copy("lib-new.jar", true, &["mr:lib"], None)],
                "fabric",
            ),
            platform(&calls, &graph),
        )
        .await
        .unwrap();
        assert_eq!(keys(&enabled_next_to_it), vec!["mr:addon"]);
        assert_eq!(enabled_next_to_it.present.len(), 1);
        assert_eq!(*calls.lock().unwrap(), vec!["mr:addon"]);

        let calls = Mutex::new(Vec::new());
        let replacing = plan_graph(
            root(),
            true,
            &installed(
                vec![
                    disabled(),
                    copy("addon-0.jar", true, &["mr:addon"], Some("0")),
                ],
                "fabric",
            ),
            platform(&calls, &graph),
        )
        .await
        .unwrap();
        assert!(replacing.replacing);
        assert_eq!(keys(&replacing), vec!["mr:addon"]);
        assert_eq!(
            replacing.present.len(),
            1,
            "the disabled copy counts as present"
        );
        assert!(replacing.warnings().is_empty());
        assert_eq!(*calls.lock().unwrap(), vec!["mr:addon"]);
    }

    // A jar the scan cannot link to the project, enabled with the mod id of
    // the project's disabled copy, provides it (the Duplicate mods resolver
    // leaves exactly this behind).
    #[tokio::test]
    async fn enabled_jar_with_the_disabled_copys_mod_id_provides_it() {
        let graph = [("mr:addon", vec![mr("lib")]), ("mr:lib", vec![])];
        let mut disabled = copy("lib.jar", false, &["mr:lib"], None);
        disabled.modid = Some("lib".into());
        let mut build = copy("lib-dev.jar", true, &[], None);
        build.modid = Some("lib".into());
        let calls = Mutex::new(Vec::new());
        let planned = plan_graph(
            root(),
            true,
            &installed(vec![disabled, build], "fabric"),
            platform(&calls, &graph),
        )
        .await
        .unwrap();
        assert_eq!(keys(&planned), vec!["mr:addon"]);
        assert_eq!(planned.present[0].name, "lib-dev");
    }

    // (c) A Modrinth pin other than the installed version keeps the installed
    // copy and is logged.
    #[tokio::test]
    async fn pinned_dependency_keeps_the_installed_version_and_logs_it() {
        let graph = [(
            "mr:addon",
            vec![Selection::Modrinth("cobblemon".into(), Some("v150".into())).into()],
        )];
        let server = installed(
            vec![copy("cobblemon.jar", true, &["mr:cobblemon"], Some("v152"))],
            "fabric",
        );
        let calls = Mutex::new(Vec::new());
        let planned = plan_graph(root(), true, &server, platform(&calls, &graph))
            .await
            .unwrap();
        assert_eq!(keys(&planned), vec!["mr:addon"]);
        assert_eq!(*calls.lock().unwrap(), vec!["mr:addon"]);
        assert_eq!(
            planned.kept_pins,
            vec![KeptPin {
                key: "mr:cobblemon".into(),
                name: "cobblemon".into(),
                filename: "cobblemon.jar".into(),
                pin: "v150".into(),
            }]
        );
        assert!(
            planned.warnings().is_empty(),
            "never a reason to stop the server"
        );

        let same = [(
            "mr:addon",
            vec![Selection::Modrinth("cobblemon".into(), Some("v152".into())).into()],
        )];
        let planned = plan_graph(root(), true, &server, platform(&Mutex::default(), &same))
            .await
            .unwrap();
        assert!(planned.kept_pins.is_empty());
    }

    // (d) A missing child that cannot be resolved leaves a plan with the
    // selected addon and a warning naming it; the selected addon itself has
    // to resolve.
    #[tokio::test]
    async fn unresolvable_dependency_is_skipped_with_a_named_warning() {
        let graph = [
            ("mr:addon", vec![mr("missing"), mr("lib")]),
            ("mr:lib", vec![mr("missing")]),
        ];
        let calls = Mutex::new(Vec::new());
        let planned = plan_graph(
            root(),
            true,
            &installed(Vec::new(), "fabric"),
            platform(&calls, &graph),
        )
        .await
        .unwrap();
        assert_eq!(keys(&planned), vec!["mr:addon", "mr:lib"]);
        assert_eq!(
            planned.warnings(),
            vec![TaskWarning::SkippedDependency {
                name: Some("missing title".into()),
                platform: DependencyPlatform::Modrinth,
                project_id: "missing".into(),
                reason: SkippedDependencyReason::NoCompatibleVersion,
            }],
            "named once, although two addons need it"
        );
        assert_eq!(
            *calls.lock().unwrap(),
            vec!["mr:addon", "mr:missing", "mr:lib"]
        );

        let failed = plan_graph(
            Selection::Modrinth("gone".into(), Some("1".into())),
            true,
            &installed(Vec::new(), "fabric"),
            platform(&Mutex::default(), &graph),
        )
        .await
        .err()
        .expect("the selected addon has to resolve");
        assert_eq!(
            failed.to_string(),
            "gone title (gone) cannot be installed on this server: it has no version for this Minecraft version and loader"
        );
    }

    // A requirement found unresolvable while reading its parent (a version id
    // Modrinth no longer serves) is a warning too.
    #[tokio::test]
    async fn requirement_unresolvable_inside_the_parent_is_a_warning() {
        let gone = Unresolvable {
            name: None,
            platform: DependencyPlatform::Modrinth,
            project_id: "VERSIONID".into(),
            reason: SkippedDependencyReason::Unavailable,
        };
        let graph = [("mr:addon", vec![Child::Unresolvable(gone.clone())])];
        let planned = plan_graph(
            root(),
            true,
            &installed(Vec::new(), "fabric"),
            platform(&Mutex::default(), &graph),
        )
        .await
        .unwrap();
        assert_eq!(keys(&planned), vec!["mr:addon"]);
        assert_eq!(planned.warnings(), vec![TaskWarning::from(gone)]);
    }

    // (e) A Modrinth dependency declared by file name only is a warning, not
    // an error; a copy with that file name, scanned or not, provides it.
    #[tokio::test]
    async fn dependency_declared_by_file_name_only_is_a_warning_not_an_error() {
        use carbon_platforms::modrinth::version::Dependency;
        let declared = Dependency {
            version_id: None,
            project_id: None,
            file_name: Some("library.jar".into()),
            dependency_type: DependencyType::Required,
        };
        assert_eq!(
            modrinth_requirement(&declared),
            Some(Ok(Child::File("library.jar".into())))
        );
        let by_version = Dependency {
            version_id: Some("V".into()),
            ..declared.clone()
        };
        assert_eq!(modrinth_requirement(&by_version), Some(Err("V".into())));
        let graph = [("mr:addon", vec![Child::File("library.jar".into())])];

        let planned = plan_graph(
            root(),
            true,
            &installed(Vec::new(), "fabric"),
            platform(&Mutex::default(), &graph),
        )
        .await
        .unwrap();
        assert_eq!(keys(&planned), vec!["mr:addon"]);
        assert_eq!(
            planned.warnings(),
            vec![TaskWarning::SkippedDependency {
                name: Some("library.jar".into()),
                platform: DependencyPlatform::Modrinth,
                project_id: "library.jar".into(),
                reason: SkippedDependencyReason::Unavailable,
            }]
        );

        let recorded = plan_graph(
            root(),
            true,
            &installed(vec![copy("library.jar", true, &[], None)], "fabric"),
            platform(&Mutex::default(), &graph),
        )
        .await
        .unwrap();
        assert!(recorded.warnings().is_empty());
        assert_eq!(recorded.present.len(), 1);

        let mut unscanned = installed(Vec::new(), "fabric");
        unscanned.unscanned.push(("library.jar".into(), true));
        let planned = plan_graph(
            root(),
            true,
            &unscanned,
            platform(&Mutex::default(), &graph),
        )
        .await
        .unwrap();
        assert!(planned.warnings().is_empty());

        let mut disabled = installed(Vec::new(), "fabric");
        disabled.unscanned.push(("library.jar".into(), false));
        assert!(
            plan_graph(root(), true, &disabled, platform(&Mutex::default(), &graph))
                .await
                .is_err()
        );
    }

    // (f) On Quilt, an installed Quilted Fabric API provides Fabric API, from
    // either platform; on Fabric it does not.
    #[tokio::test]
    async fn quilted_fabric_api_provides_fabric_api_on_quilt() {
        let qfapi = || copy("qfapi.jar", true, &["mr:qvIfYCYJ", "cf:634179"], None);
        for (root, fabric_api) in [
            (root(), Selection::Modrinth("P7dR8mSH".into(), None)),
            (
                Selection::Curseforge(1, Some(1)),
                Selection::Curseforge(306612, None),
            ),
        ] {
            let root_key = root.key().0;
            let fabric_key = fabric_api.key().0;
            let graph = [
                (root_key.as_str(), vec![fabric_api.clone().into()]),
                (fabric_key.as_str(), vec![]),
            ];
            let calls = Mutex::new(Vec::new());
            let planned = plan_graph(
                root.clone(),
                true,
                &installed(vec![qfapi()], "quilt"),
                platform(&calls, &graph),
            )
            .await
            .unwrap();
            assert_eq!(keys(&planned), vec![root_key.as_str()]);
            assert!(planned.warnings().is_empty());
            assert_eq!(*calls.lock().unwrap(), vec![root_key.clone()]);

            let planned = plan_graph(
                root,
                true,
                &installed(vec![qfapi()], "fabric"),
                platform(&Mutex::default(), &graph),
            )
            .await
            .unwrap();
            assert_eq!(keys(&planned), vec![root_key.as_str(), fabric_key.as_str()]);
        }
    }

    fn fixture_db(data: &Path) -> rusqlite::Connection {
        let mut conn = rusqlite::Connection::open(data.join("fixture.db")).unwrap();
        carbon_repos::get_migrations()
            .0
            .to_latest(&mut conn)
            .unwrap();
        conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
        conn
    }

    /// A jar in `mods/` with its scan row; `source` is the platform it was
    /// installed from (`None` for a copy no platform identifies).
    fn installed_jar(
        conn: &mut rusqlite::Connection,
        data: &Path,
        filename: &str,
        bytes: &[u8],
        modid: Option<&str>,
        source: Option<(&str, &str)>,
    ) {
        use carbon_repos::db_exec::test_support::wg;
        std::fs::create_dir_all(data.join("mods")).unwrap();
        std::fs::write(data.join("mods").join(filename), bytes).unwrap();
        let now = DbDateTime(Utc::now().fixed_offset());
        carbon_repos::repos::mod_metadata::insert_metadata_conn(
            &wg(conn),
            filename,
            1,
            &Sha512::digest(bytes),
            &[0; 20],
            "forge",
            None,
            modid,
            None,
            None,
            None,
            now,
        )
        .unwrap();
        let location = format!("mods/{filename}");
        mfcdb::upsert_server_mod_file_cache_at_location_conn(
            &wg(conn),
            7,
            filename,
            &location,
            bytes.len() as i32,
            true,
            "mods",
            filename,
            now,
        )
        .unwrap();
        if let Some((platform, project)) = source {
            mfcdb::publish_server_source_conn(
                &wg(conn),
                7,
                "mods",
                &location,
                platform,
                project,
                "1",
                &Sha512::digest(bytes),
                now,
            )
            .unwrap();
        }
    }

    // (g) A copy installed from Modrinth whose bytes CurseForge also knows
    // satisfies a CurseForge requirement.
    #[tokio::test]
    async fn modrinth_sourced_copy_known_to_curseforge_satisfies_a_curseforge_requirement() {
        use carbon_repos::db_exec::test_support::wg;
        let root = tempfile::tempdir().unwrap();
        let mut conn = fixture_db(root.path());
        installed_jar(
            &mut conn,
            root.path(),
            "fabric-api.jar",
            b"fabric api",
            Some("fabric-api"),
            Some(("mr", "P7dR8mSH")),
        );
        conn.execute(
            "INSERT INTO CurseForgeModCache
               (metadataId, murmur2, projectId, fileId, name, version, urlslug, summary, authors, releaseType, updatePaths, cachedAt)
             VALUES ('fabric-api.jar', 1, 306612, 5, 'Fabric API', '0.92', 'fabric-api', '', '', 1, '', 0)",
            [],
        )
        .unwrap();
        let rows = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
        assert_eq!(rows[0].source_platform.as_deref(), Some("mr"));
        let server = InstalledAddons::read(root.path(), &rows, Some("fabric")).await;
        let graph = [
            ("cf:1", vec![Selection::Curseforge(306612, None).into()]),
            ("cf:306612", vec![]),
        ];
        let calls = Mutex::new(Vec::new());
        let planned = plan_graph(
            Selection::Curseforge(1, Some(1)),
            true,
            &server,
            platform(&calls, &graph),
        )
        .await
        .unwrap();
        assert_eq!(keys(&planned), vec!["cf:1"], "no second Fabric API");
        assert_eq!(*calls.lock().unwrap(), vec!["cf:1"]);
        assert_eq!(planned.present[0].name, "fabric-api.jar");
    }

    async fn prepare(
        data: &Path,
        artifact: Artifact,
        rows: &[mfcdb::ServerAddonFullRow],
    ) -> Prepared {
        let folder = data.join("mods");
        let location = format!("mods/{}", artifact.filename);
        let stage = data.join(format!("stage-{}", artifact.filename));
        prepare_artifact(data, &folder, artifact, location, rows, &stage)
            .await
            .unwrap()
            .unwrap()
    }

    // (h) A downloaded dependency the server already has as a copy no
    // platform identifies, by mod id or by file name, is kept: no second jar
    // and no "destination collision".
    #[tokio::test]
    async fn dependency_with_an_unidentified_installed_copy_is_not_published() {
        use carbon_repos::db_exec::test_support::wg;
        let root = tempfile::tempdir().unwrap();
        let data = root.path();
        let mut conn = fixture_db(data);
        installed_jar(
            &mut conn,
            data,
            "create-ci.jar",
            b"ci build",
            Some("create"),
            None,
        );
        // On disk, not scanned yet.
        std::fs::write(data.join("mods/existing.jar"), b"existing").unwrap();
        std::fs::write(data.join("mods/off.jar.disabled"), b"off").unwrap();
        let rows = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
        let server = InstalledAddons::read(data, &rows, Some("forge")).await;
        assert_eq!(server.unscanned.len(), 2);

        let mut items =
            vec![prepare(data, artifact("mr:addon", "1", "addon.jar", b"a"), &rows).await];
        let mut modids = vec![Some("addon".to_owned())];
        for (key, filename, modid) in [
            ("mr:create", "create-6.0.jar", Some("create")),
            ("mr:existing", "existing.jar", None),
            ("mr:lib", "lib.jar", Some("lib")),
            ("mr:lib-twin", "lib-twin.jar", Some("lib")),
        ] {
            let mut dependency = artifact(key, "1", filename, b"d");
            dependency.required = true;
            items.push(prepare(data, dependency, &rows).await);
            modids.push(modid.map(Into::into));
        }
        assert!(items.iter().all(|item| item.old.is_none()));
        let publish = settle_dependencies(data, &items, &modids, &server, &flat(&items), false)
            .await
            .unwrap()
            .publish;
        assert_eq!(publish, vec![true, false, false, true, false]);

        // A disabled twin of the file refuses a fresh install by name, and is
        // left as it is when the selected addon replaces itself.
        let mut off = artifact("mr:off", "1", "off.jar", b"o");
        off.required = true;
        let items = vec![
            prepare(data, artifact("mr:addon", "1", "addon.jar", b"a"), &rows).await,
            prepare(data, off, &rows).await,
        ];
        let refused =
            settle_dependencies(data, &items, &[None, None], &server, &flat(&items), false)
                .await
                .err()
                .expect("a disabled twin refuses a fresh install");
        assert_eq!(
            refused.to_string(),
            "Required dependency off title (off.jar) is installed but disabled. Enable it in the server's Addons tab, or rename off.jar.disabled to off.jar in the server's mods folder, and retry."
        );
        assert_eq!(
            settle_dependencies(data, &items, &[None, None], &server, &flat(&items), true)
                .await
                .unwrap()
                .publish,
            vec![true, false]
        );
        assert_eq!(
            std::fs::read(data.join("mods/existing.jar")).unwrap(),
            b"existing"
        );
        assert_eq!(
            std::fs::read(data.join("mods/create-ci.jar")).unwrap(),
            b"ci build"
        );
        assert_eq!(std::fs::read_dir(data.join("mods")).unwrap().count(), 3);
    }

    /// The graph of a plan whose selected addon (the first item) requires
    /// every other item directly.
    fn flat(items: &[Prepared]) -> Graph {
        let root = items[0].artifact.key.clone();
        let mut requires: HashMap<String, Vec<String>> = items[1..]
            .iter()
            .map(|item| (item.artifact.key.clone(), Vec::new()))
            .collect();
        requires.insert(
            root.clone(),
            items[1..]
                .iter()
                .map(|item| item.artifact.key.clone())
                .collect(),
        );
        Graph { root, requires }
    }

    /// A jar whose only entry is the mod metadata file `path`.
    fn jar(path: &str, content: &str) -> Vec<u8> {
        use std::io::Write;
        use zip::{CompressionMethod, ZipWriter, write::FileOptions};
        let mut bytes = Vec::new();
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut bytes));
        zip.start_file(
            path,
            FileOptions::<()>::default().compression_method(CompressionMethod::Stored),
        )
        .unwrap();
        zip.write_all(content.as_bytes()).unwrap();
        zip.finish().unwrap();
        bytes
    }

    fn fabric_jar(modid: &str) -> Vec<u8> {
        jar(
            "fabric.mod.json",
            &format!(r#"{{"schemaVersion": 1, "id": "{modid}", "version": "1.0.0"}}"#),
        )
    }

    /// Forge 1.12.2 and older declare the mod id in mcmod.info.
    fn mcmod_jar(modid: &str) -> Vec<u8> {
        jar(
            "mcmod.info",
            &format!(
                r#"{{"modListVersion": 2, "modList": [{{"modid": "{modid}", "name": "{modid}", "version": "1.5.2"}}]}}"#
            ),
        )
    }

    // A downloaded dependency whose mod id a jar the scan has not recorded
    // declares (a server created from a pack moments ago, a jar dropped in by
    // hand, a symlink) is kept as that jar, whatever its file name; a disabled
    // twin of the download is kept when an enabled jar has its mod id; only a
    // copy that is there disabled, with no enabled one, refuses.
    #[tokio::test]
    async fn unscanned_jar_with_the_mod_id_keeps_the_dependency() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path();
        let mods = data.join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        let pack: Vec<(&str, Vec<u8>)> = vec![
            ("fabric-api-0.92.2+1.20.1.jar", fabric_jar("fabric-api")),
            ("Baubles-1.12-1.5.2.jar", mcmod_jar("Baubles")),
            ("lib-1.1.jar", fabric_jar("lib")),
            ("lib-1.0.jar.disabled", fabric_jar("lib")),
            ("geckolib-old.jar.disabled", fabric_jar("geckolib")),
            ("readme.txt", b"not a jar".to_vec()),
        ];
        for (filename, bytes) in &pack {
            std::fs::write(mods.join(filename), bytes).unwrap();
        }
        #[cfg(unix)]
        {
            std::fs::write(data.join("shared.jar"), fabric_jar("shared")).unwrap();
            std::os::unix::fs::symlink(data.join("shared.jar"), mods.join("shared.jar")).unwrap();
        }
        // No scan rows yet.
        let server = InstalledAddons::read(data, &[], Some("fabric")).await;
        let mut items =
            vec![prepare(data, artifact("mr:addon", "1", "addon.jar", b"a"), &[]).await];
        let mut modids = vec![Some("addon".to_owned())];
        let mut expected = vec![true];
        let mut dependencies = vec![
            (
                "mr:P7dR8mSH",
                "fabric-api-0.100.0+1.20.1.jar",
                "fabric-api",
                false,
            ),
            ("cf:225643", "Baubles-1.12-1.5.3.jar", "baubles", false),
            ("mr:lib", "lib-1.0.jar", "lib", false),
            ("mr:new", "new.jar", "new", true),
        ];
        if cfg!(unix) {
            dependencies.push(("mr:shared", "shared-2.jar", "shared", false));
            // A symlink with the download's own file name is kept as well,
            // without failing the installation.
            dependencies.push(("mr:shared-link", "shared.jar", "shared", false));
        }
        for (key, filename, modid, published) in dependencies {
            let mut dependency = artifact(key, "1", filename, b"d");
            dependency.required = true;
            items.push(prepare(data, dependency, &[]).await);
            modids.push(Some(modid.to_owned()));
            expected.push(published);
        }
        let settled = settle_dependencies(data, &items, &modids, &server, &flat(&items), false)
            .await
            .unwrap();
        assert_eq!(settled.publish, expected);

        let mut geckolib = artifact("mr:geckolib", "1", "geckolib-4.jar", b"g");
        geckolib.required = true;
        let items = vec![
            prepare(data, artifact("mr:addon", "1", "addon.jar", b"a"), &[]).await,
            prepare(data, geckolib, &[]).await,
        ];
        let modids = [Some("addon".to_owned()), Some("geckolib".to_owned())];
        let refused = settle_dependencies(data, &items, &modids, &server, &flat(&items), false)
            .await
            .err()
            .expect("a disabled jar with the mod id refuses a fresh install");
        assert_eq!(
            refused.to_string(),
            "Required dependency geckolib-old.jar is installed but disabled. Enable it in the server's Addons tab, or rename geckolib-old.jar.disabled to geckolib-old.jar in the server's mods folder, and retry."
        );
        assert_eq!(
            settle_dependencies(data, &items, &modids, &server, &flat(&items), true)
                .await
                .unwrap()
                .publish,
            vec![true, false],
            "it counts as present when the selected addon replaces itself"
        );
        for (filename, bytes) in &pack {
            assert_eq!(std::fs::read(mods.join(filename)).unwrap(), *bytes);
        }
    }

    // An old Fabric API, identified and disabled, next to an enabled Fabric
    // API the scan has not recorded (a symlink to a library shared between
    // servers, which the scan skips): the enabled jar provides it, when the
    // plan finds it and again at publication, rather than a refusal telling
    // the user to enable a second copy of a loaded mod.
    #[tokio::test]
    async fn an_unscanned_enabled_jar_provides_a_project_installed_disabled() {
        use carbon_repos::db_exec::test_support::wg;
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let mut conn = fixture_db(data);
        let old = fabric_jar("fabric-api");
        installed_jar(
            &mut conn,
            data,
            "fabric-api-0.80.0.jar",
            &old,
            Some("fabric-api"),
            Some(("mr", FABRIC_API.0)),
        );
        let mods = data.join("mods");
        std::fs::rename(
            mods.join("fabric-api-0.80.0.jar"),
            mods.join("fabric-api-0.80.0.jar.disabled"),
        )
        .unwrap();
        let rows = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
        let graph = [
            ("mr:addon", vec![mr(FABRIC_API.0)]),
            ("mr:P7dR8mSH", vec![]),
        ];
        let refusal = "Required dependency fabric-api-0.80.0.jar is installed but disabled. Enable it in the server's Addons tab, or rename fabric-api-0.80.0.jar.disabled to fabric-api-0.80.0.jar in the server's mods folder, and retry.";

        // Only the disabled copy: refused.
        let server = InstalledAddons::read(data, &rows, Some("fabric")).await;
        let refused = plan_graph(root(), true, &server, platform(&Mutex::default(), &graph))
            .await
            .err()
            .expect("a disabled-only dependency refuses a fresh install");
        assert_eq!(refused.to_string(), refusal);

        // An enabled jar of another mod id changes nothing.
        std::fs::write(mods.join("other.jar"), fabric_jar("other")).unwrap();
        let server = InstalledAddons::read(data, &rows, Some("fabric")).await;
        let refused = plan_graph(root(), true, &server, platform(&Mutex::default(), &graph))
            .await
            .err()
            .unwrap();
        assert_eq!(refused.to_string(), refusal);

        let shared = data.join("shared");
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::write(
            shared.join("fabric-api-0.92.2.jar"),
            fabric_jar("fabric-api"),
        )
        .unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            shared.join("fabric-api-0.92.2.jar"),
            mods.join("fabric-api.jar"),
        )
        .unwrap();
        #[cfg(not(unix))]
        std::fs::copy(
            shared.join("fabric-api-0.92.2.jar"),
            mods.join("fabric-api.jar"),
        )
        .unwrap();
        let server = InstalledAddons::read(data, &rows, Some("fabric")).await;
        let calls = Mutex::new(Vec::new());
        let planned = plan_graph(root(), true, &server, platform(&calls, &graph))
            .await
            .unwrap();
        assert_eq!(keys(&planned), vec!["mr:addon"]);
        assert_eq!(*calls.lock().unwrap(), vec!["mr:addon"]);
        assert_eq!(planned.present.len(), 1);
        assert_eq!(planned.present[0].name, "fabric-api.jar");
        assert!(planned.warnings().is_empty());
        let now = InstalledAddons::read(data, &rows, Some("fabric")).await;
        assert_eq!(
            recheck_present(&planned.present, &now, false)
                .await
                .unwrap(),
            Vec::<TaskWarning>::new(),
            "not refused at publication either"
        );
        assert_eq!(
            std::fs::read(mods.join("fabric-api-0.80.0.jar.disabled")).unwrap(),
            old
        );
    }

    // L 1.x is installed but known only on CurseForge, so a Modrinth
    // requirement plans L 2.x, which requires M, a library that is gone and
    // one the server has only disabled. Once the downloaded L 2.x turns out to
    // be the installed L (same mod id), none of that is published, reported
    // or refused: the installed L's own requirements are never walked.
    #[tokio::test]
    async fn what_only_a_kept_dependency_requires_is_not_published() {
        use carbon_repos::db_exec::test_support::wg;
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let mut conn = fixture_db(data);
        installed_jar(
            &mut conn,
            data,
            "lib-1.jar",
            b"lib 1",
            Some("lib"),
            Some(("cf", "99")),
        );
        let rows = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
        let off = || copy("off.jar", false, &["mr:off"], None);
        let mut server = InstalledAddons::read(data, &rows, Some("forge")).await;
        server.addons.push(off());
        // The add-on and `other` pin different versions of L.
        let pinned = |pin: &str| Child::from(Selection::Modrinth("lib".into(), Some(pin.into())));
        let graph = [
            ("mr:addon", vec![pinned("2.0"), mr("other")]),
            ("mr:lib", vec![mr("m"), mr("gone"), mr("off")]),
            ("mr:other", vec![pinned("2.1")]),
            ("mr:m", vec![]),
        ];
        let conflict = TaskWarning::ConflictingVersions {
            name: "lib title".into(),
            platform: DependencyPlatform::Modrinth,
            project_id: "lib".into(),
            installed: "lib 2.0".into(),
        };
        let gone = TaskWarning::SkippedDependency {
            name: Some("gone title".into()),
            platform: DependencyPlatform::Modrinth,
            project_id: "gone".into(),
            reason: SkippedDependencyReason::NoCompatibleVersion,
        };
        let modids = [
            Some("addon".to_owned()),
            Some("lib".to_owned()),
            Some("other".to_owned()),
            Some("m".to_owned()),
        ];

        let planned = plan_graph(root(), true, &server, platform(&Mutex::default(), &graph))
            .await
            .expect("a disabled requirement of a dependency does not refuse yet");
        assert_eq!(
            keys(&planned),
            vec!["mr:addon", "mr:lib", "mr:other", "mr:m"]
        );
        assert_eq!(planned.warnings(), vec![gone.clone(), conflict]);
        assert_eq!(planned.present.len(), 1);
        let mut items = Vec::new();
        for artifact in &planned.artifacts {
            items.push(prepare(data, artifact.clone(), &rows).await);
        }
        let settled = settle_dependencies(
            data,
            &items,
            &modids,
            &server,
            &planned.graph,
            planned.replacing,
        )
        .await
        .unwrap();
        assert_eq!(settled.publish, vec![true, false, true, false]);
        assert!(
            planned
                .warned
                .iter()
                .all(|warned| !warned.concern.holds(&settled.reach)),
            "nothing is reported about what nothing published needs"
        );
        assert!(
            planned
                .present
                .iter()
                .all(|present| !settled.reach.needed.contains(&present.requirement.key())),
            "the disabled library is not checked again"
        );

        // Without L installed, L 2.x is published with all it requires, and
        // the library the server has only disabled refuses at publication.
        let bare = installed(vec![off()], "forge");
        let planned = plan_graph(root(), true, &bare, platform(&Mutex::default(), &graph))
            .await
            .unwrap();
        let settled = settle_dependencies(
            data,
            &items,
            &modids,
            &bare,
            &planned.graph,
            planned.replacing,
        )
        .await
        .unwrap();
        assert_eq!(settled.publish, vec![true, true, true, true]);
        assert!(
            planned
                .warned
                .iter()
                .all(|warned| warned.concern.holds(&settled.reach))
        );
        let needed: Vec<Present> = planned
            .present
            .iter()
            .filter(|present| settled.reach.needed.contains(&present.requirement.key()))
            .cloned()
            .collect();
        assert_eq!(
            recheck_present(&needed, &bare, false)
                .await
                .err()
                .expect("a needed disabled library refuses")
                .to_string(),
            "Required dependency off (off.jar) is installed but disabled. Enable it in the server's Addons tab, or rename off.jar.disabled to off.jar in the server's mods folder, and retry."
        );
    }

    // A version change leaves out the requirements the replaced version
    // already declared (the user may have removed one that only runs on
    // clients); a new dependency that needs one still gets it.
    #[tokio::test]
    async fn a_version_change_only_adds_the_requirements_the_new_version_introduces() {
        use carbon_repos::db_exec::test_support::wg;
        use std::sync::atomic::{AtomicUsize, Ordering};
        let dir = tempfile::tempdir().unwrap();
        let mut conn = fixture_db(dir.path());
        installed_jar(
            &mut conn,
            dir.path(),
            "addon-0.jar",
            b"addon 0",
            None,
            Some(("mr", "addon")),
        );
        let rows = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
        let server = InstalledAddons::read(dir.path(), &rows, Some("forge")).await;
        assert_eq!(
            server.version_of("mr:addon"),
            Some(Selection::Modrinth("addon".into(), Some("1".into())))
        );
        assert_eq!(server.version_of("cf:1"), None);
        let lookups = AtomicUsize::new(0);
        let previous = |declared: anyhow::Result<HashSet<String>>| {
            let lookups = &lookups;
            Some(Previous::new(async move {
                lookups.fetch_add(1, Ordering::SeqCst);
                declared
            }))
        };
        let declared = || {
            Ok(HashSet::from([
                "mr:client-lib".to_owned(),
                "file:old.jar".to_owned(),
            ]))
        };

        let graph = [
            (
                "mr:addon",
                vec![
                    mr("client-lib"),
                    mr("new-lib"),
                    Child::File("old.jar".into()),
                ],
            ),
            ("mr:new-lib", vec![]),
            ("mr:client-lib", vec![]),
        ];
        let calls = Mutex::new(Vec::new());
        let planned = plan_graph_with(
            root(),
            true,
            &server,
            previous(declared()),
            platform(&calls, &graph),
        )
        .await
        .unwrap();
        assert!(planned.replacing);
        assert_eq!(keys(&planned), vec!["mr:addon", "mr:new-lib"]);
        assert_eq!(*calls.lock().unwrap(), vec!["mr:addon", "mr:new-lib"]);
        assert!(planned.warnings().is_empty(), "{:?}", planned.warnings());
        assert_eq!(lookups.swap(0, Ordering::SeqCst), 1, "looked up once");

        let needs_it = [
            ("mr:addon", vec![mr("client-lib"), mr("new-lib")]),
            ("mr:new-lib", vec![mr("client-lib")]),
            ("mr:client-lib", vec![]),
        ];
        let planned = plan_graph_with(
            root(),
            true,
            &server,
            previous(declared()),
            platform(&Mutex::default(), &needs_it),
        )
        .await
        .unwrap();
        assert_eq!(
            keys(&planned),
            vec!["mr:addon", "mr:new-lib", "mr:client-lib"]
        );
        assert_eq!(lookups.swap(0, Ordering::SeqCst), 1);

        // Nothing is missing: the replaced version is not looked up.
        let present = installed(
            vec![copy("client-lib.jar", true, &["mr:client-lib"], None)],
            "forge",
        );
        let planned = plan_graph_with(
            root(),
            true,
            &present,
            previous(declared()),
            platform(&Mutex::default(), &[("mr:addon", vec![mr("client-lib")])]),
        )
        .await
        .unwrap();
        assert_eq!(keys(&planned), vec!["mr:addon"]);
        assert_eq!(lookups.swap(0, Ordering::SeqCst), 0);

        // The replaced version is gone from Modrinth: every missing
        // requirement is resolved. A connection problem stops the change.
        let gone = || -> anyhow::Error {
            use crate::error::request::{RequestContext, RequestError, RequestErrorDetails};
            RequestError {
                context: RequestContext::none(),
                error: RequestErrorDetails::UnexpectedStatus {
                    status: reqwest::StatusCode::NOT_FOUND,
                    details: None,
                },
            }
            .into()
        };
        let planned = plan_graph_with(
            root(),
            true,
            &server,
            previous(Err(gone())),
            platform(&Mutex::default(), &graph),
        )
        .await
        .unwrap();
        assert_eq!(
            keys(&planned),
            vec!["mr:addon", "mr:client-lib", "mr:new-lib"]
        );
        let offline = plan_graph_with(
            root(),
            true,
            &server,
            previous(Err(anyhow!("connection reset"))),
            platform(&Mutex::default(), &graph),
        )
        .await
        .err()
        .expect("a connection problem stops the change");
        assert_eq!(
            offline.to_string(),
            "Could not look up the version this addon replaces. Check your connection and retry."
        );
    }

    // A Modrinth pin that cannot be installed on the server is replaced by a
    // version for it (the same release when there is one), with a warning
    // when the releases differ, except for Fabric API, whose builds each name
    // their own Minecraft version; a second dependant pinning it adds nothing.
    #[tokio::test]
    async fn a_pin_not_for_the_server_is_replaced_by_a_version_for_it() {
        let unusable = |project: &str, version: Option<(&str, &str)>| -> anyhow::Error {
            UnusablePin {
                skip: Unresolvable {
                    name: Some(format!("{project} title")),
                    platform: DependencyPlatform::Modrinth,
                    project_id: project.into(),
                    reason: SkippedDependencyReason::NoCompatibleVersion,
                },
                version: version.map(|(number, release)| PinnedVersion {
                    number: number.into(),
                    release: vec![release.into()],
                }),
                detail: None,
            }
            .into()
        };
        let calls = Mutex::new(Vec::new());
        let resolve = |selection: Selection| {
            calls.lock().unwrap().push(format!("{selection:?}"));
            let (key, _) = selection.key();
            let (_, project) = selection.platform();
            std::future::ready(match selection {
                Selection::Modrinth(project, Some(pin))
                    if project == "addon" || project == "other" =>
                {
                    let child = |project: &str, pin: &str| {
                        Child::from(Selection::Modrinth(project.into(), Some(pin.into())))
                    };
                    Ok((
                        artifact(&key, &pin, &format!("{project}.jar"), b"a"),
                        if project == "addon" {
                            vec![
                                child("sodium", "fabric-0.5.11"),
                                child("iris", "fabric-1.6.4"),
                                child("gone", "deleted"),
                                child("none", "fabric-1"),
                                child("P7dR8mSH", "fabric-api-1.20.1"),
                                Selection::Modrinth("other".into(), Some("1".into())).into(),
                            ]
                        } else {
                            vec![child("sodium", "fabric-0.5.11")]
                        },
                    ))
                }
                Selection::Modrinth(project, Some(_)) => Err(match project.as_str() {
                    "sodium" => unusable(&project, Some(("mc1.20.1-0.5.11-fabric", "0.5.11"))),
                    "iris" => unusable(&project, Some(("1.6.4+fabric", "1.6.4"))),
                    "none" => unusable(&project, Some(("1+fabric", "1"))),
                    "P7dR8mSH" => unusable(&project, Some(("0.86.1+1.20.1", "0.86.1"))),
                    _ => unusable(&project, None),
                }),
                Selection::ModrinthRelease(_, release) if project == "none" => {
                    assert_eq!(release, vec!["1".to_owned()]);
                    Err(Unresolvable {
                        name: Some("none title".into()),
                        platform: DependencyPlatform::Modrinth,
                        project_id: project,
                        reason: SkippedDependencyReason::NoCompatibleVersion,
                    }
                    .into())
                }
                Selection::ModrinthRelease(_, release) => {
                    let mut artifact =
                        artifact(&key, "for-server", &format!("{project}.jar"), b"d");
                    artifact.release = match project.as_str() {
                        // Built for the server from the pinned release.
                        "sodium" => release,
                        _ => vec!["newer".into()],
                    };
                    artifact.version_name = format!("{project} for the server");
                    Ok((artifact, Vec::new()))
                }
                _ => unreachable!("{key}"),
            })
        };
        let planned = plan_graph(root(), true, &installed(Vec::new(), "neoforge"), resolve)
            .await
            .unwrap();
        assert_eq!(
            keys(&planned),
            vec![
                "mr:addon",
                "mr:sodium",
                "mr:iris",
                "mr:gone",
                "mr:P7dR8mSH",
                "mr:other"
            ]
        );
        assert!(
            planned.artifacts[1..5]
                .iter()
                .all(|artifact| artifact.version == "for-server" && artifact.required)
        );
        assert_eq!(
            planned.warnings(),
            vec![
                TaskWarning::ReplacedPinnedVersion {
                    name: "iris title".into(),
                    project_id: "iris".into(),
                    pinned: "1.6.4+fabric".into(),
                    installed: "iris for the server".into(),
                },
                TaskWarning::SkippedDependency {
                    name: Some("none title".into()),
                    platform: DependencyPlatform::Modrinth,
                    project_id: "none".into(),
                    reason: SkippedDependencyReason::NoCompatibleVersion,
                },
            ]
        );
        assert_eq!(
            calls
                .lock()
                .unwrap()
                .iter()
                .filter(|call| call.contains("sodium"))
                .count(),
            2,
            "the second dependant's identical pin is not looked up again"
        );

        // The pin the user picked is never replaced.
        let failed = plan_graph(
            Selection::Modrinth("sodium".into(), Some("fabric-0.5.11".into())),
            true,
            &installed(Vec::new(), "neoforge"),
            |_| std::future::ready(Err(unusable("sodium", Some(("x", "0.5.11"))))),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(
            failed.to_string(),
            "sodium title (sodium) cannot be installed on this server: it has no version for this Minecraft version and loader"
        );
    }

    // A dependant that pins a version after another took the newest one gets
    // its pin, which satisfies both, and what only the newest version
    // required is dropped; two different pins still warn.
    #[tokio::test]
    async fn a_later_pin_replaces_the_newest_version_planned_for_an_earlier_dependant() {
        let resolve = |selection: Selection| {
            let (key, pin) = selection.key();
            let (_, project) = selection.platform();
            let children: Vec<Child> = match (project.as_str(), pin.as_deref()) {
                ("addon", _) => vec![mr("x"), mr("y"), mr("z")],
                ("x", None) => vec![mr("only-latest")],
                ("y", _) => vec![Selection::Modrinth("x".into(), Some("p".into())).into()],
                ("z", _) => vec![Selection::Modrinth("x".into(), Some("q".into())).into()],
                _ => Vec::new(),
            };
            std::future::ready(Ok((
                artifact(
                    &key,
                    pin.as_deref().unwrap_or("latest"),
                    &format!("{project}.jar"),
                    b"b",
                ),
                children,
            )))
        };
        let planned = plan_graph(root(), true, &installed(Vec::new(), "fabric"), resolve)
            .await
            .unwrap();
        assert_eq!(keys(&planned), vec!["mr:addon", "mr:x", "mr:y", "mr:z"]);
        assert_eq!(planned.artifacts[1].version, "p");
        assert!(!planned.graph.requires.contains_key("mr:only-latest"));
        assert_eq!(
            planned.warnings(),
            vec![TaskWarning::ConflictingVersions {
                name: "x title".into(),
                platform: DependencyPlatform::Modrinth,
                project_id: "x".into(),
                installed: "x p".into(),
            }]
        );
    }

    // What the newest version of a repinned dependency decided goes with it:
    // X's newest version pins W to w2 and pins Z to a version not for the
    // server, X at the pin Y asks for pins W to w1 and accepts any Z. The
    // server gets W w1 and Z's newest version, with no warning about pins no
    // installed addon declares, and nothing is looked up twice.
    #[tokio::test]
    async fn a_repinned_dependency_drops_what_its_newest_version_decided() {
        let calls = Mutex::new(Vec::new());
        let resolve = |selection: Selection| {
            calls.lock().unwrap().push(format!("{selection:?}"));
            let (key, pin) = selection.key();
            let (_, project) = selection.platform();
            let pinned = |project: &str, pin: &str| {
                Child::from(Selection::Modrinth(project.into(), Some(pin.into())))
            };
            std::future::ready(match (&selection, project.as_str(), pin.as_deref()) {
                (_, "z", Some("z-old")) => Err(UnusablePin {
                    skip: Unresolvable {
                        name: Some("z title".into()),
                        platform: DependencyPlatform::Modrinth,
                        project_id: "z".into(),
                        reason: SkippedDependencyReason::NoCompatibleVersion,
                    },
                    version: Some(PinnedVersion {
                        number: "z 1.0".into(),
                        release: vec!["1.0".into()],
                    }),
                    detail: None,
                }
                .into()),
                (Selection::ModrinthRelease(..), "z", _) => {
                    let mut artifact = artifact(&key, "for-server", "z.jar", b"z");
                    artifact.release = vec!["2.0".into()];
                    Ok((artifact, Vec::new()))
                }
                _ => {
                    let children = match (project.as_str(), pin.as_deref()) {
                        ("addon", _) => vec![mr("x"), mr("y")],
                        ("y", _) => vec![pinned("x", "p")],
                        ("x", None) => vec![pinned("w", "w2"), pinned("z", "z-old")],
                        ("x", Some("p")) => vec![pinned("w", "w1"), mr("z")],
                        _ => Vec::new(),
                    };
                    Ok((
                        artifact(
                            &key,
                            pin.as_deref().unwrap_or("latest"),
                            &format!("{project}.jar"),
                            b"b",
                        ),
                        children,
                    ))
                }
            })
        };
        let planned = plan_graph(root(), true, &installed(Vec::new(), "fabric"), resolve)
            .await
            .unwrap();
        assert_eq!(
            planned
                .artifacts
                .iter()
                .map(|artifact| (artifact.key.as_str(), artifact.version.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("mr:addon", "1"),
                ("mr:x", "p"),
                ("mr:y", "latest"),
                ("mr:w", "w1"),
                ("mr:z", "latest"),
            ]
        );
        assert!(planned.warnings().is_empty(), "{:?}", planned.warnings());
        let calls = calls.into_inner().unwrap();
        for selection in [
            Selection::Modrinth("x".into(), None),
            Selection::Modrinth("w".into(), Some("w2".into())),
            Selection::Modrinth("z".into(), Some("z-old".into())),
            Selection::Modrinth("x".into(), Some("p".into())),
        ] {
            let call = format!("{selection:?}");
            assert_eq!(
                calls.iter().filter(|made| **made == call).count(),
                1,
                "{call} is looked up once"
            );
        }
    }

    // A dependency never takes over an installed row through the selected
    // addon's replacement path, even when a stale row claims its project.
    #[tokio::test]
    async fn dependency_never_replaces_an_installed_owner() {
        use carbon_repos::db_exec::test_support::wg;
        let root = tempfile::tempdir().unwrap();
        let data = root.path();
        let mut conn = fixture_db(data);
        installed_jar(
            &mut conn,
            data,
            "lib.jar",
            b"lib",
            None,
            Some(("mr", "lib")),
        );
        installed_jar(
            &mut conn,
            data,
            "lib-2.jar",
            b"lib 2",
            None,
            Some(("mr", "lib")),
        );
        let rows = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
        let mut dependency = artifact("mr:lib", "9", "lib-9.jar", b"lib 9");
        dependency.required = true;
        let item = prepare(data, dependency, &rows).await;
        assert!(
            item.old.is_none(),
            "never 'Multiple installed owners' for a dependency"
        );
        // The selected addon still refuses to pick one of two owners.
        let folder = data.join("mods");
        assert!(
            prepare_artifact(
                data,
                &folder,
                artifact("mr:lib", "9", "lib-9.jar", b"lib 9"),
                "mods/lib-9.jar".into(),
                &rows,
                &data.join("stage"),
            )
            .await
            .is_err()
        );
    }

    /// The platforms of the integration tests: the add-on requires Create,
    /// which the server has, a library the server has as an unscanned jar of
    /// the same name, and a library that has no version for the server.
    /// `meanwhile` runs once the plan has found Create installed, as a user
    /// would act on the server while the installation runs.
    struct Catalogue {
        base: String,
        calls: Mutex<Vec<String>>,
        meanwhile: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    }

    impl Catalogue {
        fn new(base: String) -> Arc<Self> {
            Arc::new(Self {
                base,
                calls: Mutex::new(Vec::new()),
                meanwhile: Mutex::new(None),
            })
        }
    }

    #[async_trait::async_trait]
    impl Resolver for Catalogue {
        async fn resolve(
            &self,
            _server: &server_repo::ServerRow,
            selection: Selection,
            _dependencies: bool,
        ) -> anyhow::Result<(Artifact, Vec<Child>)> {
            let (key, _) = selection.key();
            let (platform, project_id) = selection.platform();
            self.calls.lock().unwrap().push(key.clone());
            let download = |filename: &str, bytes: &[u8]| {
                let mut artifact = artifact(&key, "1", filename, bytes);
                artifact.url = format!("{}/{filename}", self.base);
                artifact
            };
            match key.as_str() {
                "mr:rails" => Ok((
                    download("steam-n-rails.jar", b"rails"),
                    vec![mr("LNytGWDc"), mr("lib"), mr("gone")],
                )),
                "mr:lib" => {
                    if let Some(meanwhile) = self.meanwhile.lock().unwrap().take() {
                        meanwhile();
                    }
                    Ok((download("lib.jar", b"downloaded lib"), vec![]))
                }
                _ => Err(Unresolvable {
                    name: Some("Gone Library".into()),
                    platform,
                    project_id,
                    reason: SkippedDependencyReason::NoCompatibleVersion,
                }
                .into()),
            }
        }
    }

    /// A Forge 1.20.1 server from a pack: Create 0.5.1, identified on
    /// Modrinth, and `lib`, a jar the scan has not recorded.
    async fn pack_server(
        app: &crate::TestEnv,
        lib: &str,
    ) -> (i32, PathBuf, Vec<mfcdb::ServerAddonFullRow>) {
        use crate::domain::server::ServerState;
        use carbon_repos::repos::mod_metadata as metadata;
        let group = app.server_manager().get_default_group().await.unwrap();
        let id = server_repo::insert_server(
            &app.db,
            "Pack".into(),
            "pack".into(),
            0,
            group.0,
            "1.20.1".into(),
            25565,
            "modded".into(),
            Some("forge".into()),
            Some("47.2.0".into()),
            None,
            None,
            None,
            None,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await
        .unwrap() as i32;
        app.server_manager.servers.write().await.insert(
            ServerId(id),
            ServerData {
                shortpath: "pack".into(),
                state: ServerState::Stopped { failed_task: None },
                handle: None,
                last_log_id: None,
            },
        );
        let data = app
            .settings_manager()
            .runtime_path
            .get_servers()
            .get_server_path("pack")
            .get_data_path();
        let mods = data.join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::write(mods.join("create-1.20.1-0.5.1.jar"), CREATE).unwrap();
        std::fs::write(mods.join(lib), b"pack lib").unwrap();
        app.db
            .write(move |conn| {
                let now = DbDateTime(Utc::now().fixed_offset());
                let sha = Sha512::digest(CREATE);
                metadata::insert_metadata_conn(
                    &conn,
                    "create",
                    1,
                    &sha,
                    &[0; 20],
                    "forge",
                    Some("Create"),
                    Some("create"),
                    None,
                    None,
                    None,
                    now,
                )?;
                mfcdb::upsert_server_mod_file_cache_at_location_conn(
                    &conn,
                    id,
                    "create-1.20.1-0.5.1.jar",
                    "mods/create-1.20.1-0.5.1.jar",
                    CREATE.len() as i32,
                    true,
                    "mods",
                    "create",
                    now,
                )?;
                mfcdb::publish_server_source_conn(
                    &conn,
                    id,
                    "mods",
                    "mods/create-1.20.1-0.5.1.jar",
                    "mr",
                    "LNytGWDc",
                    "create-051",
                    &sha,
                    now,
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let before = mfcdb::get_server_mods_full(&app.db, id).await.unwrap();
        assert_eq!(before.len(), 1);
        (id, mods, before)
    }

    const CREATE: &[u8] = b"create 0.5.1 from the pack";

    /// Installs Steam 'n' Rails on the pack server and waits for the outcome.
    async fn install_rails(
        app: &crate::TestEnv,
        id: i32,
        catalogue: Arc<Catalogue>,
    ) -> (VisualTaskId, crate::managers::vtask::TaskOutcome) {
        run_install(
            app,
            id,
            Selection::Modrinth("rails".into(), Some("1".into())),
            catalogue,
        )
        .await
    }

    /// Installs `selection` on the server and waits for the outcome.
    async fn run_install(
        app: &crate::TestEnv,
        id: i32,
        selection: Selection,
        resolver: Arc<dyn Resolver>,
    ) -> (VisualTaskId, crate::managers::vtask::TaskOutcome) {
        let task = install_with(app, ServerId(id), selection, resolver)
            .await
            .unwrap();
        let mut outcome = app.task_manager().get_task_outcome(task).await;
        for _ in 0..200 {
            if !matches!(outcome, crate::managers::vtask::TaskOutcome::Running(_)) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            outcome = app.task_manager().get_task_outcome(task).await;
        }
        (task, outcome)
    }

    fn files(mods: &Path) -> Vec<String> {
        let mut files: Vec<_> = std::fs::read_dir(mods)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        files.sort();
        files
    }

    fn staging_left(app: &crate::TestEnv) -> usize {
        std::fs::read_dir(app.settings_manager().runtime_path.to_path_buf())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".server-addon-")
            })
            .count()
    }

    fn failure(outcome: &crate::managers::vtask::TaskOutcome) -> String {
        match outcome {
            crate::managers::vtask::TaskOutcome::Failed(task) => match &task.progress {
                crate::domain::vtask::Progress::Failed(error) => error.to_string(),
                progress => panic!("failed without an error: {progress:?}"),
            },
            _ => panic!("the installation did not fail"),
        }
    }

    fn gone_library() -> TaskWarning {
        TaskWarning::SkippedDependency {
            name: Some("Gone Library".into()),
            platform: DependencyPlatform::Modrinth,
            project_id: "gone".into(),
            reason: SkippedDependencyReason::NoCompatibleVersion,
        }
    }

    // (i) A Forge server with Create 0.5.1 installed: installing an add-on
    // that requires Create leaves Create's bytes and row as they were and
    // publishes the add-on, keeps an unscanned library of the same file name
    // and reports the dependency it could not install.
    #[tokio::test]
    async fn installing_an_addon_keeps_the_servers_create_and_publishes_the_addon() {
        let app = crate::setup_managers_for_test().await;
        let (id, mods, before) = pack_server(&app, "lib.jar").await;
        let mut platform = mockito::Server::new_async().await;
        let rails = platform
            .mock("GET", "/steam-n-rails.jar")
            .with_body("rails")
            .create_async()
            .await;
        let _lib = platform
            .mock("GET", "/lib.jar")
            .with_body("downloaded lib")
            .create_async()
            .await;
        let catalogue = Catalogue::new(platform.url());
        let (task, outcome) = install_rails(&app, id, catalogue.clone()).await;
        if let crate::managers::vtask::TaskOutcome::Failed(task) = &outcome {
            panic!("installation failed: {:?}", task.progress);
        }
        assert!(matches!(
            outcome,
            crate::managers::vtask::TaskOutcome::Succeeded
        ));
        rails.assert_async().await;
        assert!(
            !catalogue
                .calls
                .lock()
                .unwrap()
                .contains(&"mr:LNytGWDc".to_owned()),
            "the installed Create is never resolved"
        );

        assert_eq!(
            std::fs::read(mods.join("create-1.20.1-0.5.1.jar")).unwrap(),
            CREATE
        );
        assert_eq!(
            std::fs::read(mods.join("steam-n-rails.jar")).unwrap(),
            b"rails"
        );
        assert_eq!(std::fs::read(mods.join("lib.jar")).unwrap(), b"pack lib");
        assert_eq!(
            files(&mods),
            vec!["create-1.20.1-0.5.1.jar", "lib.jar", "steam-n-rails.jar"]
        );
        let after = mfcdb::get_server_mods_full(&app.db, id).await.unwrap();
        let create_row = after
            .iter()
            .find(|row| row.id == before[0].id)
            .expect("Create keeps its row");
        assert_eq!(
            (
                &create_row.filename,
                &create_row.location,
                &create_row.sha512,
                create_row.enabled,
                &create_row.source_project_id,
                &create_row.source_version_id,
            ),
            (
                &before[0].filename,
                &before[0].location,
                &before[0].sha512,
                before[0].enabled,
                &before[0].source_project_id,
                &before[0].source_version_id,
            )
        );
        let rails_row = after
            .iter()
            .find(|row| row.filename == "steam-n-rails.jar")
            .expect("the add-on is published");
        assert_eq!(rails_row.source_project_id.as_deref(), Some("rails"));

        let warnings = app.task_manager().get_task_warnings().await;
        let warned = warnings
            .iter()
            .find(|entry| entry.id == task)
            .expect("the skipped dependency is reported");
        assert_eq!(warned.subject.as_deref(), Some("rails title"));
        assert_eq!(warned.warnings, vec![gone_library()]);
        assert_eq!(staging_left(&app), 0, "staging is cleaned up");
    }

    // A download that fails (here the library's) publishes nothing and leaves
    // no staging folder behind: what did download is of no use to anyone.
    #[tokio::test]
    async fn a_failed_download_leaves_no_staging_in_the_data_folder() {
        let app = crate::setup_managers_for_test().await;
        let (id, mods, _) = pack_server(&app, "other.jar").await;
        let mut platform = mockito::Server::new_async().await;
        let _rails = platform
            .mock("GET", "/steam-n-rails.jar")
            .with_body("rails")
            .create_async()
            .await;
        let _lib = platform
            .mock("GET", "/lib.jar")
            .with_status(404)
            .create_async()
            .await;
        let (_, outcome) = install_rails(&app, id, Catalogue::new(platform.url())).await;
        assert!(
            matches!(outcome, crate::managers::vtask::TaskOutcome::Failed(_)),
            "the installation fails"
        );
        assert_eq!(files(&mods), vec!["create-1.20.1-0.5.1.jar", "other.jar"]);
        assert_eq!(staging_left(&app), 0, "the downloads are removed");
    }

    // (k) Create deleted while the installation runs: the add-on is still
    // published, and the toast says Create was removed.
    #[tokio::test]
    async fn a_dependency_deleted_during_the_installation_is_reported_missing() {
        let app = crate::setup_managers_for_test().await;
        let (id, mods, _) = pack_server(&app, "lib.jar").await;
        let mut platform = mockito::Server::new_async().await;
        let _rails = platform
            .mock("GET", "/steam-n-rails.jar")
            .with_body("rails")
            .create_async()
            .await;
        let _lib = platform
            .mock("GET", "/lib.jar")
            .with_body("downloaded lib")
            .create_async()
            .await;
        let catalogue = Catalogue::new(platform.url());
        let create = mods.join("create-1.20.1-0.5.1.jar");
        *catalogue.meanwhile.lock().unwrap() =
            Some(Box::new(move || std::fs::remove_file(create).unwrap()));
        let (task, outcome) = install_rails(&app, id, catalogue).await;
        if let crate::managers::vtask::TaskOutcome::Failed(task) = &outcome {
            panic!("installation failed: {:?}", task.progress);
        }
        assert_eq!(files(&mods), vec!["lib.jar", "steam-n-rails.jar"]);
        let warnings = app.task_manager().get_task_warnings().await;
        let warned = warnings
            .iter()
            .find(|entry| entry.id == task)
            .expect("the removed dependency is reported");
        assert_eq!(
            warned.warnings,
            vec![
                gone_library(),
                TaskWarning::SkippedDependency {
                    name: Some("Create".into()),
                    platform: DependencyPlatform::Modrinth,
                    project_id: "LNytGWDc".into(),
                    reason: SkippedDependencyReason::RemovedDuringInstall,
                },
            ]
        );
        assert_eq!(staging_left(&app), 0);
    }

    // (k) Create disabled while the installation runs, or a downloaded
    // library whose file the server has only disabled: the installation is
    // refused with the message naming it, nothing is published and the
    // downloads are removed.
    #[tokio::test]
    async fn a_disabled_dependency_refuses_the_installation_and_publishes_nothing() {
        for (lib, disable_create, message) in [
            (
                "lib.jar",
                true,
                "Required dependency Create (create-1.20.1-0.5.1.jar) is installed but disabled. Enable it in the server's Addons tab, or rename create-1.20.1-0.5.1.jar.disabled to create-1.20.1-0.5.1.jar in the server's mods folder, and retry.",
            ),
            (
                "lib.jar.disabled",
                false,
                "Required dependency lib title (lib.jar) is installed but disabled. Enable it in the server's Addons tab, or rename lib.jar.disabled to lib.jar in the server's mods folder, and retry.",
            ),
        ] {
            let app = crate::setup_managers_for_test().await;
            let (id, mods, before) = pack_server(&app, lib).await;
            let mut platform = mockito::Server::new_async().await;
            let _rails = platform
                .mock("GET", "/steam-n-rails.jar")
                .with_body("rails")
                .create_async()
                .await;
            let _lib = platform
                .mock("GET", "/lib.jar")
                .with_body("downloaded lib")
                .create_async()
                .await;
            let catalogue = Catalogue::new(platform.url());
            let create = mods.join("create-1.20.1-0.5.1.jar");
            if disable_create {
                *catalogue.meanwhile.lock().unwrap() = Some(Box::new(move || {
                    let mut disabled = create.clone().into_os_string();
                    disabled.push(".disabled");
                    std::fs::rename(&create, disabled).unwrap();
                }));
            }
            let (_, outcome) = install_rails(&app, id, catalogue.clone()).await;
            assert_eq!(failure(&outcome), message);
            assert!(
                catalogue
                    .calls
                    .lock()
                    .unwrap()
                    .contains(&"mr:lib".to_owned()),
                "refused after the plan"
            );
            let expected = if disable_create {
                vec!["create-1.20.1-0.5.1.jar.disabled", "lib.jar"]
            } else {
                vec!["create-1.20.1-0.5.1.jar", "lib.jar.disabled"]
            };
            assert_eq!(files(&mods), expected, "nothing is published");
            assert_eq!(std::fs::read(mods.join(lib)).unwrap(), b"pack lib");
            let after = mfcdb::get_server_mods_full(&app.db, id).await.unwrap();
            assert_eq!(
                after.iter().map(|row| &row.id).collect::<Vec<_>>(),
                before.iter().map(|row| &row.id).collect::<Vec<_>>(),
                "no row is added"
            );
            assert_eq!(staging_left(&app), 0, "the downloads are removed");
        }
    }

    // A server whose `mods/` folder is a link to a folder shared between
    // servers (a symlink; a junction on Windows behaves the same): Create,
    // recorded by an earlier scan, still satisfies the add-on's requirement,
    // the library already in the shared folder is kept, and the add-on is
    // published into the shared folder as 2.0.40 did.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_linked_mods_folder_keeps_its_libraries_and_publishes_the_addon() {
        let app = crate::setup_managers_for_test().await;
        let (id, mods, before) = pack_server(&app, "lib.jar").await;
        let shared = app
            .settings_manager()
            .runtime_path
            .to_path_buf()
            .join("shared-mods");
        std::fs::rename(&mods, &shared).unwrap();
        std::os::unix::fs::symlink(&shared, &mods).unwrap();
        let mut platform = mockito::Server::new_async().await;
        let rails = platform
            .mock("GET", "/steam-n-rails.jar")
            .with_body("rails")
            .create_async()
            .await;
        let _lib = platform
            .mock("GET", "/lib.jar")
            .with_body("downloaded lib")
            .create_async()
            .await;
        let catalogue = Catalogue::new(platform.url());
        let (task, outcome) = install_rails(&app, id, catalogue.clone()).await;
        if let crate::managers::vtask::TaskOutcome::Failed(task) = &outcome {
            panic!("installation failed: {:?}", task.progress);
        }
        assert!(matches!(
            outcome,
            crate::managers::vtask::TaskOutcome::Succeeded
        ));
        rails.assert_async().await;
        assert!(
            !catalogue
                .calls
                .lock()
                .unwrap()
                .contains(&"mr:LNytGWDc".to_owned()),
            "the installed Create is never resolved"
        );
        assert!(
            std::fs::symlink_metadata(&mods)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            files(&shared),
            vec!["create-1.20.1-0.5.1.jar", "lib.jar", "steam-n-rails.jar"]
        );
        assert_eq!(
            std::fs::read(shared.join("create-1.20.1-0.5.1.jar")).unwrap(),
            CREATE
        );
        assert_eq!(std::fs::read(shared.join("lib.jar")).unwrap(), b"pack lib");
        let after = mfcdb::get_server_mods_full(&app.db, id).await.unwrap();
        let create_row = after
            .iter()
            .find(|row| row.id == before[0].id)
            .expect("Create keeps its row");
        assert_eq!(create_row.sha512, before[0].sha512);
        assert!(after.iter().any(|row| row.filename == "steam-n-rails.jar"));
        let warnings = app.task_manager().get_task_warnings().await;
        let warned = warnings
            .iter()
            .find(|entry| entry.id == task)
            .expect("the skipped dependency is reported");
        assert_eq!(warned.warnings, vec![gone_library()]);
        assert_eq!(staging_left(&app), 0, "staging is cleaned up");
    }

    // -----------------------------------------------------------------------
    // A copy installed from one platform, seen from the other, and what the
    // selected addon's destination already holds.

    /// A planner key, the version and file it resolves to, and the
    /// requirements that version declares.
    type ShelfFile = (
        &'static str,
        &'static str,
        &'static str,
        &'static [u8],
        Vec<Child>,
    );

    /// The platforms of the tests below: each planner key on the shelf
    /// resolves to one file served by `base`; any other project has no version
    /// for the server. Every call is recorded.
    struct Shelf {
        base: String,
        files: Vec<ShelfFile>,
        calls: Mutex<Vec<String>>,
    }

    impl Shelf {
        fn new(base: String, files: Vec<ShelfFile>) -> Arc<Self> {
            Arc::new(Self {
                base,
                files,
                calls: Mutex::new(Vec::new()),
            })
        }
    }

    #[async_trait::async_trait]
    impl Resolver for Shelf {
        async fn resolve(
            &self,
            _server: &server_repo::ServerRow,
            selection: Selection,
            _dependencies: bool,
        ) -> anyhow::Result<(Artifact, Vec<Child>)> {
            let (key, _) = selection.key();
            let (platform, project_id) = selection.platform();
            self.calls.lock().unwrap().push(key.clone());
            let Some((_, version, filename, bytes, children)) =
                self.files.iter().find(|(known, ..)| *known == key)
            else {
                return Err(Unresolvable {
                    name: None,
                    platform,
                    project_id,
                    reason: SkippedDependencyReason::NoCompatibleVersion,
                }
                .into());
            };
            let mut artifact = artifact(&key, version, filename, bytes);
            artifact.url = format!("{}/{filename}", self.base);
            Ok((artifact, children.clone()))
        }
    }

    /// A Fabric 1.20.1 server: its id and its `mods/` folder.
    async fn fabric_server(app: &crate::TestEnv) -> (i32, PathBuf) {
        use crate::domain::server::ServerState;
        let group = app.server_manager().get_default_group().await.unwrap();
        let id = server_repo::insert_server(
            &app.db,
            "Fabric".into(),
            "fabric".into(),
            0,
            group.0,
            "1.20.1".into(),
            25565,
            "modded".into(),
            Some("fabric".into()),
            Some("0.15.11".into()),
            None,
            None,
            None,
            None,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await
        .unwrap() as i32;
        app.server_manager.servers.write().await.insert(
            ServerId(id),
            ServerData {
                shortpath: "fabric".into(),
                state: ServerState::Stopped { failed_task: None },
                handle: None,
                last_log_id: None,
            },
        );
        let mods = app
            .settings_manager()
            .runtime_path
            .get_servers()
            .get_server_path("fabric")
            .get_data_path()
            .join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        (id, mods)
    }

    /// A jar in a server's `mods/`.
    #[derive(Clone, Copy, Default)]
    struct Jar {
        filename: &'static str,
        bytes: &'static [u8],
        disabled: bool,
        /// Only on disk: the scan has not recorded it.
        unscanned: bool,
        /// The CurseForge project and file whose hash cache matches the bytes.
        cf: Option<(i32, i32)>,
        /// The Modrinth project and version whose hash cache matches the bytes.
        mr: Option<(&'static str, &'static str)>,
        /// The platform, project and version it was installed from.
        source: Option<(&'static str, &'static str, &'static str)>,
    }

    async fn put(app: &crate::TestEnv, id: i32, mods: &Path, jar: Jar) {
        use carbon_repos::repos::mod_metadata as metadata;
        let on_disk = if jar.disabled {
            format!("{}.disabled", jar.filename)
        } else {
            jar.filename.to_owned()
        };
        std::fs::write(mods.join(on_disk), jar.bytes).unwrap();
        if jar.unscanned {
            return;
        }
        app.db
            .write(move |conn| {
                let now = DbDateTime(Utc::now().fixed_offset());
                let sha = Sha512::digest(jar.bytes);
                let location = format!("mods/{}", jar.filename);
                metadata::insert_metadata_conn(
                    &conn,
                    jar.filename,
                    1,
                    &sha,
                    &[0; 20],
                    "fabric",
                    None,
                    None,
                    None,
                    None,
                    None,
                    now,
                )?;
                mfcdb::upsert_server_mod_file_cache_at_location_conn(
                    &conn,
                    id,
                    jar.filename,
                    &location,
                    jar.bytes.len() as i32,
                    !jar.disabled,
                    "mods",
                    jar.filename,
                    now,
                )?;
                if let Some((project, file)) = jar.cf {
                    metadata::upsert_cf_mod_cache_conn(
                        &conn,
                        1,
                        project,
                        file,
                        "title",
                        "version",
                        "slug",
                        "",
                        "",
                        1,
                        "",
                        now,
                        jar.filename,
                    )?;
                }
                if let Some((project, version)) = jar.mr {
                    metadata::upsert_mr_mod_cache_conn(
                        &conn,
                        &hex::encode(sha),
                        project,
                        version,
                        "title",
                        "version",
                        "slug",
                        "",
                        "",
                        1,
                        "",
                        jar.filename,
                        "https://fixture.invalid/not-requested",
                        now,
                        jar.filename,
                    )?;
                }
                if let Some((platform, project, version)) = jar.source {
                    mfcdb::publish_server_source_conn(
                        &conn, id, "mods", &location, platform, project, version, &sha, now,
                    )?;
                }
                Ok(())
            })
            .await
            .unwrap();
    }

    async fn rows(app: &crate::TestEnv, id: i32) -> Vec<mfcdb::ServerAddonFullRow> {
        mfcdb::get_server_mods_full(&app.db, id).await.unwrap()
    }

    fn succeeded(outcome: &crate::managers::vtask::TaskOutcome) {
        if let crate::managers::vtask::TaskOutcome::Failed(task) = outcome {
            panic!("installation failed: {:?}", task.progress);
        }
        assert!(matches!(
            outcome,
            crate::managers::vtask::TaskOutcome::Succeeded
        ));
    }

    /// (platform, project, version) the row records as its source.
    fn source(row: &mfcdb::ServerAddonFullRow) -> (Option<&str>, Option<&str>, Option<&str>) {
        (
            row.source_platform.as_deref(),
            row.source_project_id.as_deref(),
            row.source_version_id.as_deref(),
        )
    }

    // (5) Fabric API installed from Modrinth meets a CurseForge mod's
    // requirement of CurseForge's Fabric API, under the same file name: it is
    // neither resolved, downloaded nor published again, and the server's addon
    // list shows it under both platforms, so neither browser offers it again.
    #[tokio::test]
    async fn fabric_api_installed_from_modrinth_meets_a_curseforge_requirement() {
        let app = crate::setup_managers_for_test().await;
        let (id, mods) = fabric_server(&app).await;
        put(
            &app,
            id,
            &mods,
            Jar {
                filename: "fabric-api-0.92.2.jar",
                bytes: b"fabric api 0.92.2",
                cf: Some((306612, 5)),
                mr: Some(("P7dR8mSH", "mr-0922")),
                source: Some(("mr", "P7dR8mSH", "mr-0922")),
                ..Default::default()
            },
        )
        .await;
        let before = rows(&app, id).await;
        let mut platform = mockito::Server::new_async().await;
        let addon = platform
            .mock("GET", "/addon.jar")
            .with_body("addon")
            .create_async()
            .await;
        let fabric_api = platform
            .mock("GET", "/fabric-api-0.92.2.jar")
            .expect(0)
            .create_async()
            .await;
        let shelf = Shelf::new(
            platform.url(),
            vec![
                (
                    "cf:1",
                    "1",
                    "addon.jar",
                    b"addon",
                    vec![Selection::Curseforge(306612, None).into()],
                ),
                (
                    "cf:306612",
                    "6",
                    "fabric-api-0.92.2.jar",
                    b"fabric api from curseforge",
                    vec![],
                ),
            ],
        );
        let (_, outcome) =
            run_install(&app, id, Selection::Curseforge(1, Some(1)), shelf.clone()).await;
        succeeded(&outcome);
        addon.assert_async().await;
        fabric_api.assert_async().await;
        assert_eq!(*shelf.calls.lock().unwrap(), vec!["cf:1"]);
        assert_eq!(files(&mods), vec!["addon.jar", "fabric-api-0.92.2.jar"]);
        assert_eq!(
            std::fs::read(mods.join("fabric-api-0.92.2.jar")).unwrap(),
            b"fabric api 0.92.2"
        );
        let after = rows(&app, id).await;
        assert_eq!(
            after.iter().find(|row| row.id == before[0].id),
            Some(&before[0]),
            "Fabric API keeps its row and source"
        );
        let listed = app
            .server_manager()
            .list_server_addons(ServerId(id))
            .await
            .unwrap();
        let fabric_api = listed
            .iter()
            .find(|addon| addon.filename == "fabric-api-0.92.2.jar")
            .unwrap();
        assert_eq!(
            (
                fabric_api.curseforge_project_id,
                fabric_api.modrinth_project_id.as_deref()
            ),
            (Some(306612), Some("P7dR8mSH")),
            "installed on both platforms"
        );
    }

    // (7) A version picked from the other platform's page replaces the copy
    // installed from the first: one jar and one row, carrying the new source.
    #[tokio::test]
    async fn a_version_picked_from_the_other_platform_replaces_the_installed_copy() {
        for installed_from in ["mr", "cf"] {
            let app = crate::setup_managers_for_test().await;
            let (id, mods) = fabric_server(&app).await;
            put(
                &app,
                id,
                &mods,
                Jar {
                    filename: "sodium-1.0.jar",
                    bytes: b"sodium 1.0",
                    cf: Some((123, 1)),
                    mr: Some(("abc", "mr-1")),
                    source: Some(if installed_from == "mr" {
                        ("mr", "abc", "mr-1")
                    } else {
                        ("cf", "123", "1")
                    }),
                    ..Default::default()
                },
            )
            .await;
            let mut platform = mockito::Server::new_async().await;
            let download = platform
                .mock("GET", "/sodium-2.0.jar")
                .with_body("sodium 2.0")
                .create_async()
                .await;
            let shelf = Shelf::new(
                platform.url(),
                vec![
                    ("cf:123", "2", "sodium-2.0.jar", b"sodium 2.0", vec![]),
                    ("mr:abc", "mr-2", "sodium-2.0.jar", b"sodium 2.0", vec![]),
                ],
            );
            let (picked, expected) = if installed_from == "mr" {
                (Selection::Curseforge(123, Some(2)), ("cf", "123", "2"))
            } else {
                (
                    Selection::Modrinth("abc".into(), Some("mr-2".into())),
                    ("mr", "abc", "mr-2"),
                )
            };
            let (_, outcome) = run_install(&app, id, picked, shelf).await;
            succeeded(&outcome);
            download.assert_async().await;
            assert_eq!(
                files(&mods),
                vec!["sodium-2.0.jar"],
                "the copy installed from {installed_from} is replaced"
            );
            let after = rows(&app, id).await;
            assert_eq!(after.len(), 1);
            assert_eq!(after[0].filename, "sodium-2.0.jar");
            assert_eq!(
                source(&after[0]),
                (Some(expected.0), Some(expected.1), Some(expected.2))
            );
        }
    }

    // (6) Another file where the selected addon goes (one no platform knows,
    // scanned or not, enabled or disabled) stops the installation before
    // anything is downloaded. The file and the rows stay as they are, and the
    // message names the file.
    #[tokio::test]
    async fn another_file_where_the_selected_addon_goes_stops_the_installation() {
        for (unscanned, disabled) in [(false, false), (true, false), (false, true)] {
            let app = crate::setup_managers_for_test().await;
            let (id, mods) = fabric_server(&app).await;
            put(
                &app,
                id,
                &mods,
                Jar {
                    filename: "addon.jar",
                    bytes: b"a build made by hand",
                    disabled,
                    unscanned,
                    ..Default::default()
                },
            )
            .await;
            let before = rows(&app, id).await;
            let mut platform = mockito::Server::new_async().await;
            let download = platform
                .mock("GET", "/addon.jar")
                .expect(0)
                .create_async()
                .await;
            let shelf = Shelf::new(
                platform.url(),
                vec![("mr:addon", "1", "addon.jar", b"addon", vec![])],
            );
            let (_, outcome) = run_install(
                &app,
                id,
                Selection::Modrinth("addon".into(), Some("1".into())),
                shelf,
            )
            .await;
            assert_eq!(
                failure(&outcome),
                format!(
                    "Cannot install addon title: the server already has another file named addon.jar{}. Remove it from the server's Addons tab and retry.",
                    if disabled { " (disabled)" } else { "" }
                )
            );
            download.assert_async().await;
            let on_disk = if disabled {
                "addon.jar.disabled"
            } else {
                "addon.jar"
            };
            assert_eq!(files(&mods), vec![on_disk]);
            assert_eq!(
                std::fs::read(mods.join(on_disk)).unwrap(),
                b"a build made by hand"
            );
            assert_eq!(rows(&app, id).await, before, "no row changes");
            assert_eq!(staging_left(&app), 0);
        }
    }

    // (6) A file put where the selected addon goes while it downloads is
    // checked again under the publication gate: other bytes stop the
    // installation with nothing published or recorded; the selected addon's
    // own bytes are adopted.
    #[tokio::test]
    async fn a_file_put_where_the_selected_addon_goes_during_the_download_is_checked() {
        for (meanwhile, refused) in [(&b"another build"[..], true), (&b"addon"[..], false)] {
            let app = crate::setup_managers_for_test().await;
            let (id, mods) = fabric_server(&app).await;
            let destination = mods.join("addon.jar");
            let mut platform = mockito::Server::new_async().await;
            let written = destination.clone();
            let download = platform
                .mock("GET", "/addon.jar")
                .with_body_from_request(move |_| {
                    std::fs::write(&written, meanwhile).unwrap();
                    b"addon".to_vec()
                })
                .create_async()
                .await;
            let shelf = Shelf::new(
                platform.url(),
                vec![("mr:addon", "1", "addon.jar", b"addon", vec![])],
            );
            let (_, outcome) = run_install(
                &app,
                id,
                Selection::Modrinth("addon".into(), Some("1".into())),
                shelf,
            )
            .await;
            download.assert_async().await;
            assert_eq!(files(&mods), vec!["addon.jar"]);
            assert_eq!(std::fs::read(&destination).unwrap(), meanwhile);
            let after = rows(&app, id).await;
            if refused {
                assert_eq!(
                    failure(&outcome),
                    "Cannot install addon title: the server already has another file named addon.jar. Remove it from the server's Addons tab and retry."
                );
                assert!(after.is_empty(), "nothing is recorded");
            } else {
                succeeded(&outcome);
                assert_eq!(after.len(), 1);
                assert_eq!(source(&after[0]), (Some("mr"), Some("addon"), Some("1")));
            }
            assert_eq!(staging_left(&app), 0);
        }
    }

    // (6) The selected addon's own bytes already where it goes (put there by
    // hand, or scanned before any platform identified them): nothing is
    // downloaded or moved, and its row, the same one when the scan recorded
    // it, records the source. The same bytes disabled are refused, naming the
    // file and both ways to enable it, since the server would not load them.
    #[tokio::test]
    async fn the_selected_addons_own_bytes_where_it_goes_are_adopted() {
        for (unscanned, disabled) in [(false, false), (true, false), (false, true), (true, true)] {
            let app = crate::setup_managers_for_test().await;
            let (id, mods) = fabric_server(&app).await;
            put(
                &app,
                id,
                &mods,
                Jar {
                    filename: "addon.jar",
                    bytes: b"addon",
                    disabled,
                    unscanned,
                    ..Default::default()
                },
            )
            .await;
            let on_disk = if disabled {
                "addon.jar.disabled"
            } else {
                "addon.jar"
            };
            #[cfg(unix)]
            let inode = {
                use std::os::unix::fs::MetadataExt;
                std::fs::metadata(mods.join(on_disk)).unwrap().ino()
            };
            let before = rows(&app, id).await;
            let mut platform = mockito::Server::new_async().await;
            let download = platform
                .mock("GET", "/addon.jar")
                .expect(0)
                .create_async()
                .await;
            let shelf = Shelf::new(
                platform.url(),
                vec![("mr:addon", "1", "addon.jar", b"addon", vec![])],
            );
            let (_, outcome) = run_install(
                &app,
                id,
                Selection::Modrinth("addon".into(), Some("1".into())),
                shelf,
            )
            .await;
            download.assert_async().await;
            assert_eq!(files(&mods), vec![on_disk]);
            assert_eq!(std::fs::read(mods.join(on_disk)).unwrap(), b"addon");
            if disabled {
                assert_eq!(
                    failure(&outcome),
                    "addon title is already on the server but disabled (addon.jar.disabled). Enable it in the server's Addons tab, or rename it to addon.jar in the server's mods folder, and retry."
                );
                assert_eq!(rows(&app, id).await, before, "no row changes");
                assert_eq!(staging_left(&app), 0);
                continue;
            }
            succeeded(&outcome);
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                assert_eq!(
                    std::fs::metadata(mods.join(on_disk)).unwrap().ino(),
                    inode,
                    "the file is left where it is"
                );
            }
            let after = rows(&app, id).await;
            assert_eq!(after.len(), 1);
            if !unscanned {
                assert_eq!(after[0].id, before[0].id, "the same row");
            }
            assert_eq!(after[0].location, "mods/addon.jar");
            assert!(after[0].enabled);
            assert_eq!(source(&after[0]), (Some("mr"), Some("addon"), Some("1")));
            assert_eq!(staging_left(&app), 0);
        }
    }

    // A server 2.0.40 left with two copies of Fabric API (GPT-0024): picking
    // a version replaces the copy the server loads and leaves a disabled one
    // as it is. When both are loaded the message names them, and nothing is
    // downloaded or changed.
    #[tokio::test]
    async fn a_version_change_next_to_a_duplicate_replaces_the_loaded_copy() {
        for older_disabled in [true, false] {
            let app = crate::setup_managers_for_test().await;
            let (id, mods) = fabric_server(&app).await;
            for (filename, bytes, version, disabled) in [
                (
                    "fabric-api-0.80.jar",
                    &b"fabric api 0.80"[..],
                    "v080",
                    older_disabled,
                ),
                (
                    "fabric-api-0.90.jar",
                    &b"fabric api 0.90"[..],
                    "v090",
                    false,
                ),
            ] {
                put(
                    &app,
                    id,
                    &mods,
                    Jar {
                        filename,
                        bytes,
                        disabled,
                        source: Some(("mr", "P7dR8mSH", version)),
                        ..Default::default()
                    },
                )
                .await;
            }
            let before = rows(&app, id).await;
            let mut platform = mockito::Server::new_async().await;
            let download = platform
                .mock("GET", "/fabric-api-0.92.jar")
                .with_body("fabric api 0.92")
                .expect(usize::from(older_disabled))
                .create_async()
                .await;
            let shelf = Shelf::new(
                platform.url(),
                vec![(
                    "mr:P7dR8mSH",
                    "v092",
                    "fabric-api-0.92.jar",
                    b"fabric api 0.92",
                    vec![],
                )],
            );
            let (_, outcome) = run_install(
                &app,
                id,
                Selection::Modrinth("P7dR8mSH".into(), Some("v092".into())),
                shelf,
            )
            .await;
            download.assert_async().await;
            let after = rows(&app, id).await;
            if older_disabled {
                succeeded(&outcome);
                assert_eq!(
                    files(&mods),
                    vec!["fabric-api-0.80.jar.disabled", "fabric-api-0.92.jar"]
                );
                let kept = before
                    .iter()
                    .find(|row| row.filename == "fabric-api-0.80.jar")
                    .unwrap();
                assert_eq!(after.iter().find(|row| row.id == kept.id), Some(kept));
                assert_eq!(after.len(), 2);
                let published = after
                    .iter()
                    .find(|row| row.filename == "fabric-api-0.92.jar")
                    .unwrap();
                assert!(published.enabled);
                assert_eq!(
                    source(published),
                    (Some("mr"), Some("P7dR8mSH"), Some("v092"))
                );
            } else {
                assert_eq!(
                    failure(&outcome),
                    "P7dR8mSH title is installed more than once (fabric-api-0.80.jar, fabric-api-0.90.jar). Remove the copies you do not want from the server's Addons tab and retry."
                );
                assert_eq!(
                    files(&mods),
                    vec!["fabric-api-0.80.jar", "fabric-api-0.90.jar"]
                );
                assert_eq!(after, before);
                assert_eq!(staging_left(&app), 0);
            }
        }
    }

    // Among copies of the selected addon's project: with none loaded, the one
    // where it goes is replaced and the other disabled copies stay; a row
    // whose file is gone is not a copy; a disabled duplicate where the loaded
    // copy's new version goes is named with it.
    #[tokio::test]
    async fn the_copy_a_version_change_replaces_among_duplicates() {
        use carbon_repos::db_exec::test_support::wg;
        let root = tempfile::tempdir().unwrap();
        let data = root.path();
        let folder = data.join("mods");
        let mut conn = fixture_db(data);
        for (filename, bytes) in [
            ("lib-1.jar", &b"lib 1"[..]),
            ("lib-2.jar", &b"lib 2"[..]),
            ("lib-gone.jar", &b"lib gone"[..]),
        ] {
            installed_jar(&mut conn, data, filename, bytes, None, Some(("mr", "lib")));
        }
        std::fs::remove_file(folder.join("lib-gone.jar")).unwrap();
        let toggle = |conn: &mut rusqlite::Connection, filename: &str, enabled: bool| {
            let (from, to) = if enabled {
                (format!("{filename}.disabled"), filename.to_owned())
            } else {
                (filename.to_owned(), format!("{filename}.disabled"))
            };
            std::fs::rename(folder.join(from), folder.join(to)).unwrap();
            let row = mfcdb::get_server_mods_full_conn(&wg(conn), 7)
                .unwrap()
                .into_iter()
                .find(|row| row.filename == filename)
                .unwrap();
            mfcdb::update_server_mod_file_enabled_conn(
                &wg(conn),
                &row.id,
                enabled,
                DbDateTime(Utc::now().fixed_offset()),
            )
            .unwrap();
        };
        toggle(&mut conn, "lib-1.jar", false);
        toggle(&mut conn, "lib-2.jar", false);
        let pick = |filename: &'static str, rows: Vec<mfcdb::ServerAddonFullRow>| {
            let folder = folder.clone();
            async move {
                prepare_artifact(
                    data,
                    &folder,
                    artifact("mr:lib", "9", filename, b"lib 9"),
                    format!("mods/{filename}"),
                    &rows,
                    &data.join("stage"),
                )
                .await
                .map(|item| item.unwrap().old.map(|(row, _)| row.filename))
            }
        };
        let duplicates = "lib title is installed more than once (lib-1.jar, lib-2.jar). Remove the copies you do not want from the server's Addons tab and retry.";

        // None loaded: the copy where the new version goes is replaced; with
        // none there, the user chooses.
        let rows = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(
            pick("lib-2.jar", rows.clone()).await.unwrap().as_deref(),
            Some("lib-2.jar")
        );
        assert_eq!(
            pick("lib-9.jar", rows).await.unwrap_err().to_string(),
            duplicates
        );

        // lib-1 loaded: it is replaced, unless the disabled lib-2 sits where
        // the new version goes.
        toggle(&mut conn, "lib-1.jar", true);
        let rows = mfcdb::get_server_mods_full_conn(&wg(&mut conn), 7).unwrap();
        assert_eq!(
            pick("lib-9.jar", rows.clone()).await.unwrap().as_deref(),
            Some("lib-1.jar")
        );
        assert_eq!(
            pick("lib-2.jar", rows).await.unwrap_err().to_string(),
            duplicates
        );
        assert_eq!(
            files(&folder),
            vec!["lib-1.jar", "lib-2.jar.disabled"],
            "nothing is touched before publication"
        );
    }

    // A destination that holds two copies, or a link or folder, refuses with
    // what is there and what to do; an IO error is named, not given a fix
    // (see the next test).
    #[tokio::test]
    async fn an_unclear_destination_names_what_is_there() {
        let refusal = |data: std::path::PathBuf| async move {
            prepare_artifact(
                &data,
                &data.join("mods"),
                artifact("mr:addon", "1", "addon.jar", b"addon"),
                "mods/addon.jar".into(),
                &[],
                &data.join("stage"),
            )
            .await
            .err()
            .expect("refused")
            .to_string()
        };
        let root = tempfile::tempdir().unwrap();
        let mods = root.path().join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::write(mods.join("addon.jar"), b"one").unwrap();
        std::fs::write(mods.join("addon.jar.disabled"), b"two").unwrap();
        assert_eq!(
            refusal(root.path().into()).await,
            "Cannot install addon title: the server has both addon.jar and addon.jar.disabled. Delete one of them from the server's mods folder and retry."
        );

        let root = tempfile::tempdir().unwrap();
        let mods = root.path().join("mods");
        std::fs::create_dir_all(mods.join("addon.jar.disabled")).unwrap();
        assert_eq!(
            refusal(root.path().into()).await,
            "Cannot install addon title: addon.jar.disabled in the server's mods folder is a link or folder, not a file. Move it out of that folder and retry."
        );
    }

    // A mods folder or file that cannot be read stops the installation with
    // the OS error, under a line naming the addon and the file.
    #[cfg(unix)]
    #[tokio::test]
    async fn an_unreadable_destination_names_the_addon_and_the_file() {
        use std::os::unix::fs::PermissionsExt;
        let failure = |data: std::path::PathBuf| async move {
            let error = prepare_artifact(
                &data,
                &data.join("mods"),
                artifact("mr:addon", "1", "addon.jar", b"addon"),
                "mods/addon.jar".into(),
                &[],
                &data.join("stage"),
            )
            .await
            .err()
            .expect("refused");
            (error.to_string(), format!("{error:#}"))
        };
        // The folder cannot be listed, or the file (the download's size, so
        // it is opened to compare its bytes) cannot be opened.
        for locked in ["mods", "mods/addon.jar"] {
            let root = tempfile::tempdir().unwrap();
            let mods = root.path().join("mods");
            std::fs::create_dir_all(&mods).unwrap();
            std::fs::write(mods.join("addon.jar"), b"other").unwrap();
            let locked = root.path().join(locked);
            let mode = std::fs::metadata(&locked).unwrap().permissions().mode();
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
            // Permissions do not stop root.
            let result = if std::fs::File::open(mods.join("addon.jar")).is_err() {
                Some(failure(root.path().into()).await)
            } else {
                None
            };
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(mode)).unwrap();
            let Some((headline, chain)) = result else {
                continue;
            };
            assert_eq!(
                headline,
                "Cannot install addon title: addon.jar in the server's mods folder could not be read"
            );
            assert!(chain.contains("Permission denied"), "{chain}");
            assert_eq!(
                std::fs::read(mods.join("addon.jar")).unwrap(),
                b"other",
                "nothing is touched"
            );
        }
    }

    // A file with the selected addon's bytes next to the copy it replaces
    // would leave two copies of the addon: the refusal names both.
    #[tokio::test]
    async fn the_selected_bytes_next_to_the_replaced_copy_name_both_copies() {
        let app = crate::setup_managers_for_test().await;
        let (id, mods) = fabric_server(&app).await;
        put(
            &app,
            id,
            &mods,
            Jar {
                filename: "addon-0.jar",
                bytes: b"old addon",
                source: Some(("mr", "addon", "0")),
                ..Default::default()
            },
        )
        .await;
        put(
            &app,
            id,
            &mods,
            Jar {
                filename: "addon.jar",
                bytes: b"addon",
                unscanned: true,
                ..Default::default()
            },
        )
        .await;
        let before = rows(&app, id).await;
        let mut platform = mockito::Server::new_async().await;
        let download = platform
            .mock("GET", "/addon.jar")
            .expect(0)
            .create_async()
            .await;
        let shelf = Shelf::new(
            platform.url(),
            vec![("mr:addon", "1", "addon.jar", b"addon", vec![])],
        );
        let (_, outcome) = run_install(
            &app,
            id,
            Selection::Modrinth("addon".into(), Some("1".into())),
            shelf,
        )
        .await;
        download.assert_async().await;
        assert_eq!(
            failure(&outcome),
            "The version of addon title you picked is already in the server's mods folder as addon.jar, next to addon-0.jar. Remove the copy you do not want from the server's mods folder and retry."
        );
        assert_eq!(files(&mods), vec!["addon-0.jar", "addon.jar"]);
        assert_eq!(rows(&app, id).await, before, "no row changes");
    }

    // The selected addon's destination is checked again under the
    // publication gate: adopted bytes removed meanwhile are published as
    // downloaded, other bytes refuse, its own bytes are adopted.
    #[tokio::test]
    async fn the_destination_is_checked_again_under_the_gate() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path();
        let destination = data.join("mods/addon.jar");
        std::fs::create_dir_all(data.join("mods")).unwrap();
        std::fs::write(&destination, b"addon").unwrap();
        let mut items =
            vec![prepare(data, artifact("mr:addon", "1", "addon.jar", b"addon"), &[]).await];
        assert_eq!(items[0].adopt.as_deref(), Some(destination.as_path()));
        assert_eq!(
            items[0].reuse.as_deref(),
            Some(destination.as_path()),
            "not downloaded"
        );
        std::fs::remove_file(&destination).unwrap();
        validate_plan(data, &mut items, &[]).await.unwrap();
        assert_eq!(items[0].adopt, None);
        std::fs::write(&destination, b"other").unwrap();
        assert_eq!(
            validate_plan(data, &mut items, &[])
                .await
                .unwrap_err()
                .to_string(),
            "Cannot install addon title: the server already has another file named addon.jar. Remove it from the server's Addons tab and retry."
        );
        assert_eq!(std::fs::read(&destination).unwrap(), b"other");
        std::fs::write(&destination, b"addon").unwrap();
        validate_plan(data, &mut items, &[]).await.unwrap();
        assert_eq!(items[0].adopt.as_deref(), Some(destination.as_path()));
    }

    // -----------------------------------------------------------------------
    // What a server takes (server-addon-selection-strict-tags): a file built
    // for its loader or naming none, whatever Minecraft versions the picked
    // file lists; Modrinth projects whose server side is unknown; Fabric mods
    // and Quilted Fabric API on Quilt.

    /// CurseForge and Modrinth as the catalogue answers them: files and
    /// versions newest first, and 404 for anything else. Every call is
    /// recorded.
    #[derive(Default)]
    struct Platforms {
        cf_mods: HashMap<i32, carbon_platforms::curseforge::Mod>,
        cf_files: HashMap<i32, Vec<carbon_platforms::curseforge::File>>,
        mr_projects: HashMap<String, carbon_platforms::modrinth::project::Project>,
        mr_versions: Vec<carbon_platforms::modrinth::version::Version>,
        calls: Mutex<Vec<String>>,
    }

    impl Platforms {
        fn answer<T: Clone>(&self, call: String, value: Option<&T>) -> anyhow::Result<T> {
            use crate::error::request::{RequestContext, RequestError, RequestErrorDetails};
            self.calls.lock().unwrap().push(call);
            value.cloned().ok_or_else(|| {
                RequestError {
                    context: RequestContext::none(),
                    error: RequestErrorDetails::UnexpectedStatus {
                        status: reqwest::StatusCode::NOT_FOUND,
                        details: None,
                    },
                }
                .into()
            })
        }

        fn cf(
            mut self,
            name: &str,
            project: i32,
            files: Vec<carbon_platforms::curseforge::File>,
        ) -> Self {
            self.cf_mods.insert(project, cf_mod(project, name));
            self.cf_files.insert(project, files);
            self
        }

        fn mr(
            mut self,
            project: carbon_platforms::modrinth::project::Project,
            versions: Vec<carbon_platforms::modrinth::version::Version>,
        ) -> Self {
            self.mr_projects.insert(project.id.clone(), project);
            self.mr_versions.extend(versions);
            self
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl rule::Catalogue for Platforms {
        async fn cf_mod(&self, project: i32) -> anyhow::Result<carbon_platforms::curseforge::Mod> {
            self.answer(format!("cf_mod:{project}"), self.cf_mods.get(&project))
        }
        async fn cf_file(
            &self,
            project: i32,
            file: i32,
        ) -> anyhow::Result<carbon_platforms::curseforge::File> {
            let found = self
                .cf_files
                .get(&project)
                .and_then(|files| files.iter().find(|f| f.id == file));
            self.answer(format!("cf_file:{project}:{file}"), found)
        }
        async fn cf_files(
            &self,
            project: i32,
            release: &str,
            index: i32,
        ) -> anyhow::Result<
            carbon_platforms::curseforge::CurseForgeResponse<
                Vec<carbon_platforms::curseforge::File>,
            >,
        > {
            // CurseForge matches the game version among every tag.
            let files: Vec<_> = self
                .cf_files
                .get(&project)
                .into_iter()
                .flatten()
                .filter(|file| file.game_versions.iter().any(|v| v == release))
                .skip(index as usize)
                .take(50)
                .cloned()
                .collect();
            self.answer(format!("cf_files:{project}:{release}"), Some(&files))
                .map(|data| carbon_platforms::curseforge::CurseForgeResponse {
                    data,
                    pagination: None,
                })
        }
        async fn mr_version(
            &self,
            version: &str,
        ) -> anyhow::Result<carbon_platforms::modrinth::version::Version> {
            let found = self.mr_versions.iter().find(|v| v.id == version);
            self.answer(format!("mr_version:{version}"), found)
        }
        async fn mr_project(
            &self,
            project: &str,
        ) -> anyhow::Result<carbon_platforms::modrinth::project::Project> {
            self.answer(
                format!("mr_project:{project}"),
                self.mr_projects.get(project),
            )
        }
        async fn mr_versions(
            &self,
            filters: ProjectVersionsFilters,
        ) -> anyhow::Result<Vec<carbon_platforms::modrinth::version::Version>> {
            let versions: Vec<_> = self
                .mr_versions
                .iter()
                .filter(|v| v.project_id == *filters.project_id)
                .filter(|v| {
                    filters
                        .game_versions
                        .as_ref()
                        .is_none_or(|games| v.game_versions.iter().any(|g| games.contains(g)))
                })
                .filter(|v| {
                    filters
                        .loaders
                        .as_ref()
                        .is_none_or(|loaders| v.loaders.iter().any(|l| loaders.contains(l)))
                })
                .cloned()
                .collect();
            self.answer(
                format!("mr_versions:{}:{:?}", *filters.project_id, filters.loaders),
                Some(&versions),
            )
        }
    }

    fn cf_mod(id: i32, name: &str) -> carbon_platforms::curseforge::Mod {
        serde_json::from_value(serde_json::json!({
            "id": id, "gameId": 432, "name": name, "slug": name.to_lowercase(), "summary": "", "status": 4,
            "links": {"websiteUrl": null, "wikiUrl": null, "issuesUrl": null, "sourceUrl": null},
            "downloadCount": 0, "isFeatured": false, "primaryCategoryId": 0, "categories": [], "classId": 6,
            "authors": [], "logo": null, "screenshots": [], "mainFileId": 0, "latestFiles": [], "latestFilesIndexes": [],
            "dateCreated": "2026-01-01T00:00:00Z", "dateModified": "2026-01-01T00:00:00Z", "dateReleased": "2026-01-01T00:00:00Z",
            "allowModDistribution": true, "gamePopularityRank": 0, "isAvailable": true, "thumbsUpCount": 0
        }))
        .unwrap()
    }

    /// A CurseForge file of `project`, served by `base` (`None`: its author
    /// only allows downloads from the website), requiring `requires`.
    /// `release_type`: 1 release, 2 beta, 3 alpha.
    fn cf_file(
        project: i32,
        id: i32,
        filename: &str,
        tags: &[&str],
        release_type: u8,
        base: Option<&str>,
        requires: &[i32],
    ) -> carbon_platforms::curseforge::File {
        let bytes = filename.as_bytes();
        let dependencies: Vec<_> = requires
            .iter()
            .map(|project| serde_json::json!({"modId": project, "relationType": 3}))
            .collect();
        serde_json::from_value(serde_json::json!({
            "id": id, "gameId": 432, "modId": project, "isAvailable": true,
            "displayName": filename, "fileName": filename, "releaseType": release_type,
            "fileStatus": 4, "fileDate": "2026-01-01T00:00:00Z", "fileLength": bytes.len(), "downloadCount": 0,
            "hashes": [{"value": hex::encode(<sha1::Sha1 as sha1::Digest>::digest(bytes)), "algo": 1}],
            "downloadUrl": base.map(|base| format!("{base}/{filename}")),
            "gameVersions": tags, "sortableGameVersions": [], "dependencies": dependencies, "fileFingerprint": 0
        }))
        .unwrap()
    }

    fn mr_project(
        id: &str,
        title: &str,
        server_side: &str,
    ) -> carbon_platforms::modrinth::project::Project {
        serde_json::from_value(serde_json::json!({
            "slug": id.to_lowercase(), "title": title, "description": "", "categories": [], "client_side": "optional", "server_side": server_side,
            "body": "", "additional_categories": [], "donation_urls": [], "project_type": "mod", "downloads": 0, "id": id, "team": "fixture",
            "published": "2026-01-01T00:00:00Z", "updated": "2026-01-01T00:00:00Z", "followers": 0, "status": "approved",
            "license": {"id": "MIT", "name": "MIT", "url": null}, "versions": [], "game_versions": [], "loaders": []
        }))
        .unwrap()
    }

    /// A Modrinth release of `project` whose jar is named after the version.
    fn mr_version(
        project: &str,
        id: &str,
        games: &[&str],
        loaders: &[&str],
        dependencies: serde_json::Value,
        base: &str,
    ) -> carbon_platforms::modrinth::version::Version {
        let filename = format!("{id}.jar");
        let bytes = filename.as_bytes();
        serde_json::from_value(serde_json::json!({
            "name": id, "version_number": id, "dependencies": dependencies, "game_versions": games,
            "version_type": "release", "loaders": loaders, "featured": false, "id": id, "project_id": project,
            "author_id": "fixture", "date_published": "2026-01-01T00:00:00Z", "downloads": 0,
            "files": [{"hashes": {"sha1": hex::encode(<sha1::Sha1 as sha1::Digest>::digest(bytes)), "sha512": hex::encode(Sha512::digest(bytes))},
                "url": format!("{base}/{filename}"), "filename": filename, "primary": true, "size": bytes.len()}]
        }))
        .unwrap()
    }

    fn server_row(game_version: &str, loader: Option<&str>) -> server_repo::ServerRow {
        let now = Utc::now().fixed_offset();
        server_repo::ServerRow {
            id: 1,
            name: "Server".into(),
            shortpath: "server".into(),
            favorite: false,
            index: 0,
            library_position: None,
            group_id: 1,
            server_type: "modded".into(),
            game_version: game_version.into(),
            port: 25565,
            motd: String::new(),
            max_players: 20,
            online_mode: true,
            xmx: 2048,
            xms: 1024,
            extra_java_args: String::new(),
            auto_restart: false,
            date_created: now,
            last_started: None,
            provider_type: "local".into(),
            hosted_server_id: None,
            icon_revision: None,
            modloader_type: loader.map(Into::into),
            modloader_version: None,
            modpack_platform: None,
            modpack_project_id: None,
            modpack_file_id: None,
        }
    }

    /// Plans `selection` on a server running `loader` at `game_version`
    /// against `platforms`, as an installation does.
    async fn plan_on(
        game_version: &str,
        loader: Option<&str>,
        platforms: &Arc<Platforms>,
        selection: Selection,
    ) -> anyhow::Result<Planned> {
        let installed = InstalledAddons {
            loader: loader.map(Into::into),
            ..Default::default()
        };
        plan_with(game_version, &installed, platforms, selection).await
    }

    /// [`plan_on`] a server that already has `installed`.
    async fn plan_with(
        game_version: &str,
        installed: &InstalledAddons,
        platforms: &Arc<Platforms>,
        selection: Selection,
    ) -> anyhow::Result<Planned> {
        let server = server_row(game_version, installed.loader.as_deref());
        let live = Live(platforms.clone());
        let (live, server) = (&live, &server);
        plan_graph(selection, true, installed, move |selection| {
            live.resolve(server, selection, true)
        })
        .await
    }

    /// What the plan publishes when the server has none of it.
    fn published(planned: &Planned) -> Vec<&str> {
        let reach = planned.graph.reach(&HashSet::new());
        planned
            .artifacts
            .iter()
            .map(|artifact| artifact.key.as_str())
            .filter(|key| reach.published.contains(*key))
            .collect()
    }

    /// A modded server: its id and its `mods/` folder.
    async fn modded_server(
        app: &crate::TestEnv,
        shortpath: &str,
        game_version: &str,
        loader: &str,
    ) -> (i32, PathBuf) {
        use crate::domain::server::ServerState;
        let group = app.server_manager().get_default_group().await.unwrap();
        let id = server_repo::insert_server(
            &app.db,
            shortpath.into(),
            shortpath.into(),
            0,
            group.0,
            game_version.into(),
            25565,
            "modded".into(),
            Some(loader.into()),
            Some("1".into()),
            None,
            None,
            None,
            None,
            DbDateTime(Utc::now().fixed_offset()),
        )
        .await
        .unwrap() as i32;
        app.server_manager.servers.write().await.insert(
            ServerId(id),
            ServerData {
                shortpath: shortpath.into(),
                state: ServerState::Stopped { failed_task: None },
                handle: None,
                last_log_id: None,
            },
        );
        let mods = app
            .settings_manager()
            .runtime_path
            .get_servers()
            .get_server_path(shortpath)
            .get_data_path()
            .join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        (id, mods)
    }

    /// Serves each file named after its URL path with its own name as bytes.
    async fn serve(platform: &mut mockito::Server, filenames: &[&str]) -> Vec<mockito::Mock> {
        let mut mocks = Vec::new();
        for filename in filenames {
            mocks.push(
                platform
                    .mock("GET", format!("/{filename}").as_str())
                    .with_body(*filename)
                    .create_async()
                    .await,
            );
        }
        mocks
    }

    const BAUBLES: i32 = 227083;
    const THAUMCRAFT: i32 = 223628;

    /// Baubles as CurseForge lists it for 1.12.2: no loader tag, as most
    /// files for 1.7.10 to 1.12.2.
    fn baubles(base: &str) -> Vec<carbon_platforms::curseforge::File> {
        vec![cf_file(
            BAUBLES,
            2518667,
            "Baubles-1.12-1.5.2.jar",
            &["1.12.2"],
            1,
            Some(base),
            &[],
        )]
    }

    #[test]
    fn loader_fit_names_what_runs_on_the_server() {
        let tags = |tags: &[&str]| tags.iter().map(|tag| tag.to_string()).collect::<Vec<_>>();
        for (file, loader, game, fit) in [
            (
                &["1.12.2"][..],
                Some("forge"),
                "1.12.2",
                LoaderFit::Untagged,
            ),
            (
                &["Forge", "1.12.2"],
                Some("forge"),
                "1.12.2",
                LoaderFit::Tagged,
            ),
            (
                &["Fabric", "1.20.1"],
                Some("forge"),
                "1.20.1",
                LoaderFit::Foreign,
            ),
            (
                &["Fabric", "1.20.1"],
                Some("quilt"),
                "1.20.1",
                LoaderFit::Tagged,
            ),
            (
                &["Forge", "1.20.1"],
                Some("neoforge"),
                "1.20.1",
                LoaderFit::Tagged,
            ),
            (
                &["Forge", "1.21.1"],
                Some("neoforge"),
                "1.21.1",
                LoaderFit::Foreign,
            ),
            (
                &["Quilt", "1.20.1"],
                Some("fabric"),
                "1.20.1",
                LoaderFit::Foreign,
            ),
            (&["Forge", "1.20.1"], None, "1.20.1", LoaderFit::Foreign),
            (&["1.20.1"], None, "1.20.1", LoaderFit::Foreign),
        ] {
            assert_eq!(
                loader_fit(&tags(file), loader, game),
                fit,
                "{file:?} on {loader:?} {game}"
            );
        }

        // Only a jar for other loaders, and any jar on a vanilla server, is
        // refused, naming both loaders and what to do instead.
        validate_loader("Baubles.jar", &tags(&["1.12.2"]), Some("forge"), "1.12.2").unwrap();
        assert_eq!(
            validate_loader("sodium.jar", &tags(&["Fabric", "Quilt", "1.20.1"]), Some("forge"), "1.20.1"),
            Err("this version is made for Fabric and Quilt, and the server runs Forge. Pick a version made for Forge.".into())
        );
        assert_eq!(
            validate_loader("create.jar", &tags(&["forge"]), Some("neoforge"), "1.21.1"),
            Err("this version is made for Forge, and the server runs NeoForge. Pick a version made for NeoForge.".into())
        );
        assert_eq!(
            validate_loader("sodium.jar", &tags(&["fabric", "quilt"]), None, "1.20.1"),
            Err("this version is a mod for Fabric and Quilt, and the server runs vanilla Minecraft, which cannot load mods. Create a server with Fabric or Quilt to use it.".into())
        );
        assert_eq!(
            validate_loader("Baubles.jar", &tags(&["1.12.2"]), None, "1.12.2"),
            Err("this version is a mod, and the server runs vanilla Minecraft, which cannot load mods. Create a server with a mod loader to use it.".into())
        );
        // The way out names every loader whose builds the server runs.
        assert_eq!(
            validate_loader("jei.jar", &tags(&["Forge", "1.20.1"]), Some("quilt"), "1.20.1"),
            Err("this version is made for Forge, and the server runs Quilt. Pick a version made for Quilt or Fabric.".into())
        );
        assert_eq!(
            validate_loader("sodium.jar", &tags(&["fabric"]), Some("neoforge"), "1.20.1"),
            Err("this version is made for Fabric, and the server runs NeoForge. Pick a version made for NeoForge or Forge.".into())
        );
        validate_loader("pack.zip", &tags(&["datapack"]), Some("forge"), "1.20.1").unwrap();
        validate_loader("pack.zip", &tags(&["datapack"]), None, "1.20.1").unwrap();
    }

    // A dependency is downloaded at a file built for the server's loader
    // before a newer one that names no loader, at a release before a newer
    // beta, at an untagged file when nothing names the loader, and never at a
    // file for other loaders.
    #[tokio::test]
    async fn a_dependency_is_chosen_tagged_then_release_and_never_foreign() {
        let resolve_lib = |files: Vec<carbon_platforms::curseforge::File>| async move {
            let platforms = Arc::new(Platforms::default().cf("Lib", 7, files));
            resolve(
                &*platforms,
                &server_row("1.12.2", Some("forge")),
                Selection::Curseforge(7, None),
                true,
            )
            .await
        };
        let file = |id: i32, tags: &[&str], release_type: u8| {
            cf_file(
                7,
                id,
                &format!("lib-{id}.jar"),
                tags,
                release_type,
                Some("https://fixture.invalid"),
                &[],
            )
        };
        // Newest first, as CurseForge answers.
        let (chosen, _) = resolve_lib(vec![
            file(6, &["Fabric", "1.12.2"], 1),
            file(5, &["Forge", "1.12.2"], 2),
            file(4, &["1.12.2"], 1),
            file(3, &["Forge", "1.12.2"], 1),
            file(2, &["Forge", "1.12.1"], 1),
        ])
        .await
        .unwrap();
        assert_eq!(chosen.version, "3");

        let (chosen, _) = resolve_lib(vec![
            file(6, &["Fabric", "1.12.2"], 1),
            file(5, &["1.12.2"], 2),
            file(4, &["1.12.2"], 1),
        ])
        .await
        .unwrap();
        assert_eq!(
            chosen.version, "4",
            "an untagged release when nothing is tagged"
        );

        let (chosen, _) = resolve_lib(vec![file(5, &["Forge", "1.12.2"], 2)])
            .await
            .unwrap();
        assert_eq!(chosen.version, "5", "a beta when there is no release");

        let foreign = resolve_lib(vec![file(6, &["Fabric", "1.12.2"], 1)])
            .await
            .err()
            .unwrap();
        assert_eq!(
            unresolvable(foreign).unwrap().reason,
            SkippedDependencyReason::NoCompatibleVersion
        );

        // The shared rule, as instances apply it.
        let candidates = [
            (LoaderFit::Untagged, ModChannel::Stable),
            (LoaderFit::Foreign, ModChannel::Stable),
            (LoaderFit::Tagged, ModChannel::Beta),
            (LoaderFit::Tagged, ModChannel::Stable),
        ];
        let best = rule::best_candidate(&candidates, ModChannel::Stable, |c| Some(*c));
        assert_eq!(best, Some(&candidates[3]));
        assert_eq!(
            rule::best_candidate(&candidates[..2], ModChannel::Stable, |c| Some(*c)),
            Some(&candidates[0])
        );
        assert_eq!(
            rule::best_candidate(&candidates[1..2], ModChannel::Stable, |c| Some(*c)),
            None
        );
    }

    // A Modrinth project whose server side is unknown installs, picked or
    // required; one its authors say does not run on servers is refused when
    // picked and left out with a warning when required.
    #[tokio::test]
    async fn modrinth_server_side_unknown_installs_and_unsupported_is_refused_or_skipped() {
        let base = "https://fixture.invalid";
        let platforms = Arc::new(
            Platforms::default()
                .mr(
                    mr_project("skins", "SkinsRestorer", "unknown"),
                    vec![mr_version(
                        "skins",
                        "skins-15",
                        &["1.20.1"],
                        &["fabric"],
                        serde_json::json!([
                            {"project_id": "konfig", "dependency_type": "required"},
                            {"project_id": "toggle", "dependency_type": "required"}
                        ]),
                        base,
                    )],
                )
                .mr(
                    mr_project("konfig", "Konfig", "unknown"),
                    vec![mr_version(
                        "konfig",
                        "konfig-1",
                        &["1.20.1"],
                        &["fabric"],
                        serde_json::json!([]),
                        base,
                    )],
                )
                .mr(
                    mr_project("toggle", "Shader Toggle", "unsupported"),
                    vec![mr_version(
                        "toggle",
                        "toggle-1",
                        &["1.20.1"],
                        &["fabric"],
                        serde_json::json!([]),
                        base,
                    )],
                ),
        );
        let planned = plan_on(
            "1.20.1",
            Some("fabric"),
            &platforms,
            Selection::Modrinth("skins".into(), Some("skins-15".into())),
        )
        .await
        .unwrap();
        assert_eq!(keys(&planned), vec!["mr:skins", "mr:konfig"]);
        assert_eq!(
            planned.warnings(),
            vec![TaskWarning::SkippedDependency {
                name: Some("Shader Toggle".into()),
                platform: DependencyPlatform::Modrinth,
                project_id: "toggle".into(),
                reason: SkippedDependencyReason::NotForServers,
            }]
        );

        let refused = plan_on(
            "1.20.1",
            Some("fabric"),
            &platforms,
            Selection::Modrinth("toggle".into(), Some("toggle-1".into())),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(
            refused.to_string(),
            "Shader Toggle (toggle) cannot be installed on this server: it does not run on servers"
        );
    }

    // A version the user picks that lists only the release (1.20 on a 1.20.1
    // server), or a dependency's pin that does, is installed as picked.
    #[tokio::test]
    async fn a_picked_version_listing_only_the_release_installs_on_a_patch_version() {
        let base = "https://fixture.invalid";
        let platforms = Arc::new(
            Platforms::default()
                .cf(
                    "Addon",
                    1,
                    vec![cf_file(1, 10, "addon-1.20.jar", &["Forge", "1.20"], 1, Some(base), &[])],
                )
                .mr(
                    mr_project("addon", "Addon", "required"),
                    vec![mr_version(
                        "addon",
                        "addon-2",
                        &["1.20"],
                        &["forge"],
                        serde_json::json!([{"project_id": "lib", "version_id": "lib-1", "dependency_type": "required"}]),
                        base,
                    )],
                )
                .mr(
                    mr_project("lib", "Lib", "required"),
                    vec![mr_version("lib", "lib-1", &["1.20"], &["forge"], serde_json::json!([]), base)],
                ),
        );
        let planned = plan_on(
            "1.20.1",
            Some("forge"),
            &platforms,
            Selection::Curseforge(1, Some(10)),
        )
        .await
        .unwrap();
        assert_eq!(keys(&planned), vec!["cf:1"]);
        assert_eq!(planned.artifacts[0].filename, "addon-1.20.jar");

        let planned = plan_on(
            "1.20.1",
            Some("forge"),
            &platforms,
            Selection::Modrinth("addon".into(), Some("addon-2".into())),
        )
        .await
        .unwrap();
        assert_eq!(keys(&planned), vec!["mr:addon", "mr:lib"]);
        assert_eq!(
            planned.artifacts[1].version, "lib-1",
            "the dependant's pin is kept"
        );
        assert!(planned.warnings().is_empty());
    }

    // A jar built only for another loader, picked for a Forge server, is
    // still refused, with a message naming both loaders.
    #[tokio::test]
    async fn a_fabric_only_jar_picked_for_a_forge_server_is_refused_naming_both_loaders() {
        let base = "https://fixture.invalid";
        let platforms = Arc::new(
            Platforms::default()
                .cf(
                    "Sodium",
                    394468,
                    vec![cf_file(
                        394468,
                        5,
                        "sodium-fabric.jar",
                        &["Fabric", "1.20.1"],
                        1,
                        Some(base),
                        &[],
                    )],
                )
                .mr(
                    mr_project("AANobbMI", "Sodium", "optional"),
                    vec![mr_version(
                        "AANobbMI",
                        "sodium-fabric",
                        &["1.20.1"],
                        &["fabric", "quilt"],
                        serde_json::json!([]),
                        base,
                    )],
                ),
        );
        let curseforge = plan_on(
            "1.20.1",
            Some("forge"),
            &platforms,
            Selection::Curseforge(394468, Some(5)),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(
            curseforge.to_string(),
            "Sodium (394468) cannot be installed on this server: this version is made for Fabric, and the server runs Forge. Pick a version made for Forge."
        );
        let modrinth = plan_on(
            "1.20.1",
            Some("forge"),
            &platforms,
            Selection::Modrinth("AANobbMI".into(), Some("sodium-fabric".into())),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(
            modrinth.to_string(),
            "Sodium (AANobbMI) cannot be installed on this server: this version is made for Fabric and Quilt, and the server runs Forge. Pick a version made for Forge."
        );
        // On Quilt, the same Fabric build is taken.
        let quilt = plan_on(
            "1.20.1",
            Some("quilt"),
            &platforms,
            Selection::Modrinth("AANobbMI".into(), Some("sodium-fabric".into())),
        )
        .await
        .unwrap();
        assert_eq!(keys(&quilt), vec!["mr:AANobbMI"]);
    }

    // A required CurseForge file its author only lets download from the
    // website is reported by name; the picked addon is still installed.
    #[tokio::test]
    async fn a_dependency_only_downloadable_from_curseforge_is_reported_and_the_addon_installs() {
        let base = "https://fixture.invalid";
        let platforms = Arc::new(
            Platforms::default()
                .cf(
                    "Addon",
                    1,
                    vec![cf_file(
                        1,
                        10,
                        "addon.jar",
                        &["Forge", "1.20.1"],
                        1,
                        Some(base),
                        &[2],
                    )],
                )
                .cf(
                    "Website Only",
                    2,
                    vec![cf_file(
                        2,
                        20,
                        "website-only.jar",
                        &["Forge", "1.20.1"],
                        1,
                        None,
                        &[],
                    )],
                ),
        );
        let planned = plan_on(
            "1.20.1",
            Some("forge"),
            &platforms,
            Selection::Curseforge(1, Some(10)),
        )
        .await
        .unwrap();
        assert_eq!(keys(&planned), vec!["cf:1"]);
        assert_eq!(
            planned.warnings(),
            vec![TaskWarning::SkippedDependency {
                name: Some("Website Only".into()),
                platform: DependencyPlatform::Curseforge,
                project_id: "2".into(),
                reason: SkippedDependencyReason::NotDownloadable,
            }]
        );
    }

    // On Quilt, a requirement of Fabric API gets Quilted Fabric API from the
    // same platform, published with the addon; when Quilted Fabric API has
    // no version for the server, Fabric API itself, which Quilt also loads.
    #[tokio::test]
    async fn on_quilt_a_fabric_api_requirement_gets_quilted_fabric_api() {
        let base = "https://fixture.invalid";
        let requires_fabric_api =
            serde_json::json!([{"project_id": FABRIC_API.0, "dependency_type": "required"}]);
        let platforms = |qfapi_game: &str| {
            Arc::new(
                Platforms::default()
                    .mr(
                        mr_project("lhGA9TYQ", "Architectury API", "required"),
                        vec![mr_version(
                            "lhGA9TYQ",
                            "arch-9",
                            &["1.20.1"],
                            &["fabric"],
                            requires_fabric_api.clone(),
                            base,
                        )],
                    )
                    .mr(
                        mr_project(FABRIC_API.0, "Fabric API", "required"),
                        vec![mr_version(
                            FABRIC_API.0,
                            "fapi-0.92",
                            &["1.20.1"],
                            &["fabric"],
                            serde_json::json!([]),
                            base,
                        )],
                    )
                    .mr(
                        mr_project(QUILTED_FABRIC_API.0, "QFAPI/QSL", "required"),
                        vec![mr_version(
                            QUILTED_FABRIC_API.0,
                            "qfapi-7",
                            &[qfapi_game],
                            &["quilt"],
                            serde_json::json!([]),
                            base,
                        )],
                    )
                    .cf(
                        "Architectury API",
                        419699,
                        vec![cf_file(
                            419699,
                            1,
                            "arch-cf.jar",
                            &["Fabric", "1.20.1"],
                            1,
                            Some(base),
                            &[FABRIC_API.1],
                        )],
                    )
                    .cf(
                        "Fabric API",
                        FABRIC_API.1,
                        vec![cf_file(
                            FABRIC_API.1,
                            2,
                            "fapi-cf.jar",
                            &["Fabric", "1.20.1"],
                            1,
                            Some(base),
                            &[],
                        )],
                    )
                    .cf(
                        "QFAPI",
                        QUILTED_FABRIC_API.1,
                        vec![cf_file(
                            QUILTED_FABRIC_API.1,
                            3,
                            "qfapi-cf.jar",
                            &["Quilt", qfapi_game],
                            1,
                            Some(base),
                            &[],
                        )],
                    ),
            )
        };
        let architectury = || Selection::Modrinth("lhGA9TYQ".into(), Some("arch-9".into()));
        let with_qfapi = platforms("1.20.1");
        let planned = plan_on("1.20.1", Some("quilt"), &with_qfapi, architectury())
            .await
            .unwrap();
        assert_eq!(keys(&planned), vec!["mr:lhGA9TYQ", "mr:qvIfYCYJ"]);
        assert_eq!(published(&planned), vec!["mr:lhGA9TYQ", "mr:qvIfYCYJ"]);
        assert!(planned.warnings().is_empty());
        assert!(
            !with_qfapi
                .calls()
                .iter()
                .any(|call| call.contains(FABRIC_API.0)),
            "Fabric API is never looked up: {:?}",
            with_qfapi.calls()
        );

        let planned = plan_on(
            "1.20.1",
            Some("quilt"),
            &with_qfapi,
            Selection::Curseforge(419699, Some(1)),
        )
        .await
        .unwrap();
        assert_eq!(keys(&planned), vec!["cf:419699", "cf:634179"]);
        assert_eq!(published(&planned), vec!["cf:419699", "cf:634179"]);

        // Quilted Fabric API has no version for the server: Fabric API.
        let without = platforms("1.19.2");
        let planned = plan_on("1.20.1", Some("quilt"), &without, architectury())
            .await
            .unwrap();
        assert_eq!(keys(&planned), vec!["mr:lhGA9TYQ", "mr:P7dR8mSH"]);
        assert_eq!(published(&planned), vec!["mr:lhGA9TYQ", "mr:P7dR8mSH"]);
        assert!(planned.warnings().is_empty());

        // On Fabric, Fabric API.
        let planned = plan_on("1.20.1", Some("fabric"), &with_qfapi, architectury())
            .await
            .unwrap();
        assert_eq!(keys(&planned), vec!["mr:lhGA9TYQ", "mr:P7dR8mSH"]);
    }

    // On Quilt, a Fabric API that no platform identifies (a scan row with its
    // mod id only, or a jar the scan has not recorded) keeps Quilted Fabric
    // API out: Fabric API is planned, and publication keeps that jar for it.
    // Both side by side would load two copies of the Fabric API.
    #[tokio::test]
    async fn an_unidentified_fabric_api_on_quilt_keeps_quilted_fabric_api_out() {
        let base = "https://fixture.invalid";
        let platforms = Arc::new(
            Platforms::default()
                .mr(
                    mr_project("lhGA9TYQ", "Architectury API", "required"),
                    vec![mr_version(
                        "lhGA9TYQ",
                        "arch-9",
                        &["1.20.1"],
                        &["fabric"],
                        serde_json::json!([{"project_id": FABRIC_API.0, "dependency_type": "required"}]),
                        base,
                    )],
                )
                .mr(
                    mr_project(FABRIC_API.0, "Fabric API", "required"),
                    vec![mr_version(
                        FABRIC_API.0,
                        "fapi-0.92",
                        &["1.20.1"],
                        &["fabric"],
                        serde_json::json!([]),
                        base,
                    )],
                )
                .mr(
                    mr_project(QUILTED_FABRIC_API.0, "QFAPI/QSL", "required"),
                    vec![mr_version(
                        QUILTED_FABRIC_API.0,
                        "qfapi-7",
                        &["1.20.1"],
                        &["quilt"],
                        serde_json::json!([]),
                        base,
                    )],
                ),
        );
        let architectury = || Selection::Modrinth("lhGA9TYQ".into(), Some("arch-9".into()));
        let mut scanned = copy("fabric-api-0.92.jar", true, &[], None);
        scanned.modid = Some(FABRIC_API_MOD_ID.into());
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("mods")).unwrap();
        std::fs::write(
            root.path().join("mods/fabric-api-0.92.jar"),
            fabric_jar(FABRIC_API_MOD_ID),
        )
        .unwrap();
        let unscanned = InstalledAddons::read(root.path(), &[], Some("quilt")).await;
        for server in [installed(vec![scanned], "quilt"), unscanned] {
            let planned = plan_with("1.20.1", &server, &platforms, architectury())
                .await
                .unwrap();
            assert_eq!(keys(&planned), vec!["mr:lhGA9TYQ", "mr:P7dR8mSH"]);
            assert!(
                !published(&planned)
                    .iter()
                    .any(|key| *key == "mr:qvIfYCYJ" || *key == "cf:634179")
            );
            assert!(planned.warnings().is_empty());
        }
        assert!(
            !platforms
                .calls()
                .iter()
                .any(|call| call.contains(QUILTED_FABRIC_API.0)),
            "Quilted Fabric API is never looked up: {:?}",
            platforms.calls()
        );

        // Without that copy, Quilted Fabric API stands in.
        let planned = plan_on("1.20.1", Some("quilt"), &platforms, architectury())
            .await
            .unwrap();
        assert_eq!(keys(&planned), vec!["mr:lhGA9TYQ", "mr:qvIfYCYJ"]);
    }

    // On Quilt, a Quilt mod that pins Quilted Fabric API gets its pin, which
    // also stands in for another addon's Fabric API, whichever comes first.
    #[tokio::test]
    async fn a_pinned_quilted_fabric_api_also_stands_in_for_fabric_api() {
        let base = "https://fixture.invalid";
        let qfapi = |id: &str| {
            mr_version(
                QUILTED_FABRIC_API.0,
                id,
                &["1.20.1"],
                &["quilt"],
                serde_json::json!([]),
                base,
            )
        };
        let fabric_api =
            serde_json::json!({"project_id": FABRIC_API.0, "dependency_type": "required"});
        let quilt_mod =
            serde_json::json!({"project_id": "quiltmod", "dependency_type": "required"});
        for (requirements, expected) in [
            (
                serde_json::json!([fabric_api, quilt_mod]),
                vec!["mr:addon", "mr:qvIfYCYJ", "mr:quiltmod"],
            ),
            (
                serde_json::json!([quilt_mod, fabric_api]),
                vec!["mr:addon", "mr:quiltmod", "mr:qvIfYCYJ"],
            ),
        ] {
            let platforms = Arc::new(
                Platforms::default()
                    .mr(
                        mr_project("addon", "Addon", "required"),
                        vec![mr_version(
                            "addon",
                            "addon-1",
                            &["1.20.1"],
                            &["fabric"],
                            requirements,
                            base,
                        )],
                    )
                    .mr(
                        mr_project("quiltmod", "Quilt Mod", "required"),
                        vec![mr_version(
                            "quiltmod",
                            "quiltmod-1",
                            &["1.20.1"],
                            &["quilt"],
                            serde_json::json!([{
                                "project_id": QUILTED_FABRIC_API.0,
                                "version_id": "qfapi-6",
                                "dependency_type": "required"
                            }]),
                            base,
                        )],
                    )
                    .mr(
                        mr_project(QUILTED_FABRIC_API.0, "QFAPI/QSL", "required"),
                        vec![qfapi("qfapi-7"), qfapi("qfapi-6")],
                    ),
            );
            let planned = plan_on(
                "1.20.1",
                Some("quilt"),
                &platforms,
                Selection::Modrinth("addon".into(), Some("addon-1".into())),
            )
            .await
            .unwrap();
            assert_eq!(keys(&planned), expected);
            assert_eq!(published(&planned), expected);
            let qfapi = planned
                .artifacts
                .iter()
                .find(|artifact| artifact.key == "mr:qvIfYCYJ")
                .unwrap();
            assert_eq!(qfapi.version, "qfapi-6");
            assert!(planned.warnings().is_empty());
        }
    }

    // A Forge 1.12.2 server: Baubles, whose files list only "1.12.2", is
    // installed when picked.
    #[tokio::test]
    async fn a_legacy_forge_file_without_a_loader_tag_installs_on_a_1_12_2_server() {
        let app = crate::setup_managers_for_test().await;
        let (id, mods) = modded_server(&app, "legacy", "1.12.2", "forge").await;
        let mut platform = mockito::Server::new_async().await;
        let served = serve(&mut platform, &["Baubles-1.12-1.5.2.jar"]).await;
        let platforms =
            Arc::new(Platforms::default().cf("Baubles", BAUBLES, baubles(&platform.url())));
        let (_, outcome) = run_install(
            &app,
            id,
            Selection::Curseforge(BAUBLES as u32, Some(2518667)),
            Arc::new(Live(platforms)),
        )
        .await;
        succeeded(&outcome);
        served[0].assert_async().await;
        assert_eq!(files(&mods), vec!["Baubles-1.12-1.5.2.jar"]);
        let rows = rows(&app, id).await;
        assert_eq!(
            source(&rows[0]),
            (Some("cf"), Some("227083"), Some("2518667"))
        );
    }

    // A Forge 1.12.2 server: Thaumcraft and Baubles, which it requires, both
    // untagged, are both installed.
    #[tokio::test]
    async fn thaumcraft_and_its_untagged_baubles_dependency_are_both_installed() {
        let app = crate::setup_managers_for_test().await;
        let (id, mods) = modded_server(&app, "legacy", "1.12.2", "forge").await;
        let mut platform = mockito::Server::new_async().await;
        let served = serve(
            &mut platform,
            &["Thaumcraft-1.12.2-6.1.BETA26.jar", "Baubles-1.12-1.5.2.jar"],
        )
        .await;
        let platforms = Arc::new(
            Platforms::default()
                .cf(
                    "Thaumcraft",
                    THAUMCRAFT,
                    vec![cf_file(
                        THAUMCRAFT,
                        2629023,
                        "Thaumcraft-1.12.2-6.1.BETA26.jar",
                        &["1.12.2"],
                        2,
                        Some(&platform.url()),
                        &[BAUBLES],
                    )],
                )
                .cf("Baubles", BAUBLES, baubles(&platform.url())),
        );
        let (task, outcome) = run_install(
            &app,
            id,
            Selection::Curseforge(THAUMCRAFT as u32, Some(2629023)),
            Arc::new(Live(platforms.clone())),
        )
        .await;
        succeeded(&outcome);
        for mock in &served {
            mock.assert_async().await;
        }
        assert!(
            platforms
                .calls()
                .contains(&"cf_files:227083:1.12.2".to_owned()),
            "the dependency is looked up by Minecraft version: {:?}",
            platforms.calls()
        );
        assert_eq!(
            files(&mods),
            vec!["Baubles-1.12-1.5.2.jar", "Thaumcraft-1.12.2-6.1.BETA26.jar"]
        );
        assert!(
            app.task_manager()
                .get_task_warnings()
                .await
                .iter()
                .all(|entry| entry.id != task),
            "nothing is reported"
        );
    }

    // A Quilt 1.20.1 server: Architectury, which requires Fabric API, is
    // installed with Quilted Fabric API.
    #[tokio::test]
    async fn architectury_installs_with_quilted_fabric_api_on_a_quilt_server() {
        let app = crate::setup_managers_for_test().await;
        let (id, mods) = modded_server(&app, "quilt", "1.20.1", "quilt").await;
        let mut platform = mockito::Server::new_async().await;
        let served = serve(&mut platform, &["arch-9.jar", "qfapi-7.jar"]).await;
        let fabric_api = platform
            .mock("GET", "/fapi-0.92.jar")
            .expect(0)
            .create_async()
            .await;
        let base = platform.url();
        let platforms = Arc::new(
            Platforms::default()
                .mr(
                    mr_project("lhGA9TYQ", "Architectury API", "required"),
                    vec![mr_version(
                        "lhGA9TYQ",
                        "arch-9",
                        &["1.20.1"],
                        &["fabric"],
                        serde_json::json!([{"project_id": FABRIC_API.0, "dependency_type": "required"}]),
                        &base,
                    )],
                )
                .mr(
                    mr_project(FABRIC_API.0, "Fabric API", "required"),
                    vec![mr_version(FABRIC_API.0, "fapi-0.92", &["1.20.1"], &["fabric"], serde_json::json!([]), &base)],
                )
                .mr(
                    mr_project(QUILTED_FABRIC_API.0, "QFAPI/QSL", "required"),
                    vec![mr_version(QUILTED_FABRIC_API.0, "qfapi-7", &["1.20.1"], &["quilt"], serde_json::json!([]), &base)],
                ),
        );
        let (_, outcome) = run_install(
            &app,
            id,
            Selection::Modrinth("lhGA9TYQ".into(), Some("arch-9".into())),
            Arc::new(Live(platforms)),
        )
        .await;
        succeeded(&outcome);
        for mock in &served {
            mock.assert_async().await;
        }
        fabric_api.assert_async().await;
        assert_eq!(files(&mods), vec!["arch-9.jar", "qfapi-7.jar"]);
    }

    // -----------------------------------------------------------------------
    // When an installation may write the server's files.

    /// The outcome of `task` once it has ended, or `Running` after 10 s.
    async fn outcome_of(
        app: &crate::TestEnv,
        task: VisualTaskId,
    ) -> crate::managers::vtask::TaskOutcome {
        let mut outcome = app.task_manager().get_task_outcome(task).await;
        for _ in 0..200 {
            if !matches!(outcome, crate::managers::vtask::TaskOutcome::Running(_)) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            outcome = app.task_manager().get_task_outcome(task).await;
        }
        outcome
    }

    async fn set_state(app: &crate::TestEnv, id: i32, state: crate::domain::server::ServerState) {
        app.server_manager
            .servers
            .write()
            .await
            .get_mut(&ServerId(id))
            .unwrap()
            .state = state;
    }

    fn running_state() -> crate::domain::server::ServerState {
        crate::domain::server::ServerState::Running {
            start_time: Utc::now(),
            log_id: ServerLogId(1),
            process_id: 1,
        }
    }

    /// A [`Shelf`] that holds the resolution of `held` until released, as a
    /// slow platform lookup would.
    struct Held {
        shelf: Arc<Shelf>,
        held: &'static str,
        entered: tokio::sync::Notify,
        release: tokio::sync::Semaphore,
    }

    #[async_trait::async_trait]
    impl Resolver for Held {
        async fn resolve(
            &self,
            server: &server_repo::ServerRow,
            selection: Selection,
            dependencies: bool,
        ) -> anyhow::Result<(Artifact, Vec<Child>)> {
            if selection.key().0 == self.held {
                self.entered.notify_one();
                self.release.acquire().await.unwrap().forget();
            }
            self.shelf.resolve(server, selection, dependencies).await
        }
    }

    // A second Install on the same server while the first is still planning
    // is accepted, not refused: it waits for the first to publish, plans
    // against what it published, and both addons are installed.
    #[tokio::test]
    async fn a_second_install_waits_for_the_first_and_both_are_installed() {
        let app = crate::setup_managers_for_test().await;
        let (id, mods) = fabric_server(&app).await;
        let mut platform = mockito::Server::new_async().await;
        let _a = platform
            .mock("GET", "/a.jar")
            .with_body("a")
            .create_async()
            .await;
        let _b = platform
            .mock("GET", "/b.jar")
            .with_body("b")
            .create_async()
            .await;
        let shelf = Shelf::new(
            platform.url(),
            vec![
                ("mr:a", "1", "a.jar", b"a", vec![]),
                ("mr:b", "1", "b.jar", b"b", vec![]),
            ],
        );
        let held = Arc::new(Held {
            shelf: shelf.clone(),
            held: "mr:a",
            entered: tokio::sync::Notify::new(),
            release: tokio::sync::Semaphore::new(0),
        });
        let first = install_with(
            &app,
            ServerId(id),
            Selection::Modrinth("a".into(), Some("1".into())),
            held.clone(),
        )
        .await
        .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(10), held.entered.notified())
            .await
            .expect("the first installation plans");
        let second = install_with(
            &app,
            ServerId(id),
            Selection::Modrinth("b".into(), Some("1".into())),
            shelf.clone(),
        )
        .await
        .expect("the second Install is accepted while the first runs");
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        assert!(
            shelf.calls.lock().unwrap().is_empty(),
            "the second waits for the first before it plans"
        );
        assert!(matches!(
            app.task_manager().get_task_outcome(second).await,
            crate::managers::vtask::TaskOutcome::Running(_)
        ));
        held.release.add_permits(1);
        succeeded(&outcome_of(&app, first).await);
        succeeded(&outcome_of(&app, second).await);
        assert_eq!(*shelf.calls.lock().unwrap(), vec!["mr:a", "mr:b"]);
        assert_eq!(files(&mods), vec!["a.jar", "b.jar"]);
        assert_eq!(rows(&app, id).await.len(), 2);
        assert_eq!(staging_left(&app), 0);
    }

    // An addon picked while the server installs (its loader, or a modpack)
    // or stops is installed once that has ended, as 2.0.40 installed it,
    // instead of failing; nothing is planned or written meanwhile.
    #[tokio::test]
    async fn an_install_waits_while_the_server_installs_or_stops() {
        use crate::domain::server::ServerState;
        for busy in [
            ServerState::Installing(VisualTaskId(99)),
            ServerState::Stopping,
        ] {
            let app = crate::setup_managers_for_test().await;
            let (id, mods) = fabric_server(&app).await;
            set_state(&app, id, busy.clone()).await;
            let mut platform = mockito::Server::new_async().await;
            let _addon = platform
                .mock("GET", "/addon.jar")
                .with_body("addon")
                .create_async()
                .await;
            let shelf = Shelf::new(
                platform.url(),
                vec![("mr:addon", "1", "addon.jar", b"addon", vec![])],
            );
            let task = install_with(
                &app,
                ServerId(id),
                Selection::Modrinth("addon".into(), Some("1".into())),
                shelf.clone(),
            )
            .await
            .unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
            assert!(
                matches!(
                    app.task_manager().get_task_outcome(task).await,
                    crate::managers::vtask::TaskOutcome::Running(_)
                ),
                "{busy:?}: the installation waits"
            );
            assert!(shelf.calls.lock().unwrap().is_empty(), "{busy:?}");
            assert!(files(&mods).is_empty(), "{busy:?}");
            set_state(&app, id, ServerState::Stopped { failed_task: None }).await;
            succeeded(&outcome_of(&app, task).await);
            assert_eq!(files(&mods), vec!["addon.jar"], "{busy:?}");
            assert_eq!(staging_left(&app), 0);
        }
    }

    // Stop pressed on a running server while an addon downloads: the addon is
    // published once the server has stopped, instead of failing and leaving
    // its download behind.
    #[tokio::test]
    async fn a_stop_pressed_during_the_download_is_waited_for() {
        use crate::domain::server::ServerState;
        let app = crate::setup_managers_for_test().await;
        let (id, mods) = fabric_server(&app).await;
        set_state(&app, id, running_state()).await;
        let inner = crate::managers::App::clone(&app);
        let mut platform = mockito::Server::new_async().await;
        let _addon = platform
            .mock("GET", "/addon.jar")
            .with_body_from_request(move |_| {
                inner
                    .server_manager
                    .servers
                    .try_write()
                    .unwrap()
                    .get_mut(&ServerId(id))
                    .unwrap()
                    .state = ServerState::Stopping;
                b"addon".to_vec()
            })
            .create_async()
            .await;
        let shelf = Shelf::new(
            platform.url(),
            vec![("mr:addon", "1", "addon.jar", b"addon", vec![])],
        );
        let task = install_with(
            &app,
            ServerId(id),
            Selection::Modrinth("addon".into(), Some("1".into())),
            shelf,
        )
        .await
        .unwrap();
        let mut stopping = false;
        for _ in 0..100 {
            stopping = matches!(
                app.server_manager.servers.read().await[&ServerId(id)].state,
                ServerState::Stopping
            );
            if stopping {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert!(stopping, "the addon was downloaded");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        assert!(matches!(
            app.task_manager().get_task_outcome(task).await,
            crate::managers::vtask::TaskOutcome::Running(_)
        ));
        assert!(
            files(&mods).is_empty(),
            "nothing is published while it stops"
        );
        set_state(&app, id, ServerState::Stopped { failed_task: None }).await;
        succeeded(&outcome_of(&app, task).await);
        assert_eq!(files(&mods), vec!["addon.jar"]);
        assert_eq!(rows(&app, id).await.len(), 1);
        assert_eq!(staging_left(&app), 0);
    }

    // A server being deleted gets no addon, and the message says why.
    #[tokio::test]
    async fn an_install_on_a_server_being_deleted_says_so() {
        use crate::domain::server::ServerState;
        let app = crate::setup_managers_for_test().await;
        let (id, mods) = fabric_server(&app).await;
        set_state(&app, id, ServerState::Deleting).await;
        let shelf = Shelf::new(
            "https://fixture.invalid".into(),
            vec![("mr:addon", "1", "addon.jar", b"addon", vec![])],
        );
        let (_, outcome) = run_install(
            &app,
            id,
            Selection::Modrinth("addon".into(), Some("1".into())),
            shelf,
        )
        .await;
        assert_eq!(
            failure(&outcome),
            "Fabric is being deleted, so the addon was not installed."
        );
        assert!(files(&mods).is_empty());
        assert_eq!(staging_left(&app), 0);
    }

    // An installation, start or stop that does not end within the limit
    // fails the installation with what the server is doing and the way out;
    // a quit cancels the wait.
    #[tokio::test]
    async fn a_wait_that_does_not_end_says_what_the_server_is_doing() {
        use crate::domain::server::ServerState;
        let app = crate::setup_managers_for_test().await;
        let (id, _) = fabric_server(&app).await;
        let root = app
            .settings_manager()
            .runtime_path
            .get_servers()
            .get_server_path("fabric")
            .get_root();
        for (state, message) in [
            (
                ServerState::Installing(VisualTaskId(99)),
                "Fabric is still installing, so the addon was not installed. Install it again once the server has finished installing.",
            ),
            (
                ServerState::Stopping,
                "Fabric is still stopping, so the addon was not installed. Install it again once the server has stopped.",
            ),
        ] {
            set_state(&app, id, state).await;
            let error = app
                .server_manager()
                .addon_gate_within(
                    ServerId(id),
                    "Fabric",
                    &root,
                    &tokio_util::sync::CancellationToken::new(),
                    std::time::Duration::from_millis(300),
                )
                .await
                .unwrap_err();
            assert_eq!(error.to_string(), message);
        }
        let cancel = tokio_util::sync::CancellationToken::new();
        cancel.cancel();
        let error = app
            .server_manager()
            .addon_gate_within(
                ServerId(id),
                "Fabric",
                &root,
                &cancel,
                std::time::Duration::from_secs(60),
            )
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "Server addon installation cancelled");
        set_state(&app, id, ServerState::Stopped { failed_task: None }).await;
        app.server_manager()
            .addon_gate(
                ServerId(id),
                "Fabric",
                &root,
                &tokio_util::sync::CancellationToken::new(),
            )
            .await
            .expect("a stopped server's files may change");
    }

    /// The error a held-open file gives a move: busy, or on Windows a sharing
    /// violation, as when a running server's JVM keeps a mod jar open.
    const HELD_OPEN: i32 = if cfg!(windows) { 32 } else { 16 };

    // A replaced copy that cannot be moved aside because a process holds it
    // open stops the publication before anything moves, and says so.
    #[tokio::test]
    async fn a_replaced_copy_held_open_is_left_as_it_is() {
        use crate::util::publish::fault::{Step, inject, pending};
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("mods");
        std::fs::create_dir(&folder).unwrap();
        let old = folder.join("sodium-1.0.jar");
        std::fs::write(&old, b"old").unwrap();
        let staged = root.path().join("staged");
        std::fs::write(&staged, b"new").unwrap();
        let backup = root.path().join("previous-0");
        let download = Downloadable::new("https://fixture.invalid/not-requested", &staged)
            .with_size(3)
            .with_checksum(Some(Checksum::Sha512(hex::encode(Sha512::digest(b"new")))));
        let expected = digest(&old).await.unwrap();
        inject(Step::NoReplace, &backup, HELD_OPEN);
        let error = publish_staged(
            &download,
            &folder,
            "sodium-2.0.jar",
            Some(&old),
            Some(&expected),
            true,
            &backup,
        )
        .await
        .unwrap_err();
        assert!(!pending(Step::NoReplace, &backup), "the move was tried");
        assert_eq!(
            error
                .downcast_ref::<ReplacedCopyInUse>()
                .map(|in_use| in_use.filename.as_str()),
            Some("sodium-1.0.jar")
        );
        assert_eq!(std::fs::read(&old).unwrap(), b"old");
        assert_eq!(std::fs::read(&staged).unwrap(), b"new");
        assert!(!backup.exists());
        assert!(!folder.join("sodium-2.0.jar").exists());
        // Any other failure is not taken for a file in use.
        assert!(!held_open(&anyhow::Error::from(std::io::Error::from(
            std::io::ErrorKind::PermissionDenied
        ))));
    }

    // Picking another version of an addon whose installed copy a process
    // holds open (on Windows, a running Fabric server's mod jar) fails
    // naming the addon, the file and the way out: stop the server when it
    // runs. The installed copy, its row and the data folder stay as they were.
    #[tokio::test]
    async fn a_version_change_of_a_copy_held_open_names_the_way_out() {
        use crate::domain::server::ServerState;
        use crate::util::publish::fault::inject;
        for (running, message) in [
            (
                true,
                "Stop Fabric to change the version of abc title: the running server keeps sodium-1.0.jar open, so it cannot be replaced. Nothing was changed.",
            ),
            (
                false,
                "sodium-1.0.jar is open in another program, so the version of abc title could not be changed. Close that program, then try again. Nothing was changed.",
            ),
        ] {
            let app = crate::setup_managers_for_test().await;
            let (id, mods) = fabric_server(&app).await;
            put(
                &app,
                id,
                &mods,
                Jar {
                    filename: "sodium-1.0.jar",
                    bytes: b"sodium 1.0",
                    mr: Some(("abc", "mr-1")),
                    source: Some(("mr", "abc", "mr-1")),
                    ..Default::default()
                },
            )
            .await;
            if running {
                set_state(&app, id, running_state()).await;
            }
            let before = rows(&app, id).await;
            let runtime = app.settings_manager().runtime_path.to_path_buf();
            let mut platform = mockito::Server::new_async().await;
            let _download = platform
                .mock("GET", "/sodium-2.0.jar")
                .with_body_from_request(move |_| {
                    // The running server holds the installed copy open.
                    for entry in std::fs::read_dir(&runtime).unwrap() {
                        let entry = entry.unwrap();
                        if entry
                            .file_name()
                            .to_string_lossy()
                            .starts_with(SERVER_STAGING_PREFIX)
                        {
                            inject(
                                crate::util::publish::fault::Step::NoReplace,
                                &entry.path().join("previous-0"),
                                HELD_OPEN,
                            );
                        }
                    }
                    b"sodium 2.0".to_vec()
                })
                .create_async()
                .await;
            let shelf = Shelf::new(
                platform.url(),
                vec![("mr:abc", "mr-2", "sodium-2.0.jar", b"sodium 2.0", vec![])],
            );
            let (_, outcome) = run_install(
                &app,
                id,
                Selection::Modrinth("abc".into(), Some("mr-2".into())),
                shelf,
            )
            .await;
            assert_eq!(failure(&outcome), message);
            assert_eq!(files(&mods), vec!["sodium-1.0.jar"]);
            assert_eq!(
                std::fs::read(mods.join("sodium-1.0.jar")).unwrap(),
                b"sodium 1.0"
            );
            assert_eq!(rows(&app, id).await, before);
            assert_eq!(staging_left(&app), 0, "the download is removed");
            if running {
                assert!(matches!(
                    app.server_manager.servers.read().await[&ServerId(id)].state,
                    ServerState::Running { .. }
                ));
            }
        }
    }
}
