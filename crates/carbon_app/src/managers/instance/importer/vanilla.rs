//! Minecraft Launcher, Lunar, Badlion and Feather importer; all share the `.minecraft` directory.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tracing::{trace, warn};

use super::{
    GET_IMPORT_SCAN_STATUS, ImportScanStatus, ImportableInstance, ImporterState, InstanceImporter,
    InternalImportEntry,
    common::{
        DeclaredModLoader, ImportedSettings, ParsedVersionId, apply_imported_settings,
        copy_game_dir, copy_optional_game_dir, dedup_paths, first_existing_dir, group_for_import,
        icon_from_data_url, json_i64, json_positive_u32, json_string, json_trimmed_string,
        looks_like_minecraft_version, parse_version_id, read_json_value, resolve_standard_version,
    },
    technic::{is_release_id, versions_from_version_json},
};
use crate::{
    api::translation::Translation,
    domain::{
        instance::info::{GameVersion, StandardVersion},
        vtask::VisualTaskId,
    },
    managers::{AppInner, instance::InstanceVersionSource},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaFlavor {
    Minecraft,
    Lunar,
    Badlion,
    Feather,
}

impl VanillaFlavor {
    fn label(self) -> &'static str {
        match self {
            Self::Minecraft => "Minecraft",
            Self::Lunar => "Lunar Client",
            Self::Badlion => "Badlion Client",
            Self::Feather => "Feather Client",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Minecraft => "minecraft",
            Self::Lunar => "lunar",
            Self::Badlion => "badlion",
            Self::Feather => "feather",
        }
    }

    fn version_markers(self) -> &'static [&'static str] {
        match self {
            Self::Minecraft => &[],
            Self::Lunar => &["lunar"],
            Self::Badlion => &["badlion", "blclient"],
            Self::Feather => &["feather"],
        }
    }
}

/// `latest-*` needs GDL's version manifest (network), so it's resolved at import time.
#[derive(Debug, Clone, PartialEq, Eq)]
enum VersionRef {
    Exact {
        minecraft: String,
        loaders: Vec<DeclaredModLoader>,
    },
    LatestRelease,
    LatestSnapshot,
    /// An installed version folder whose Minecraft version cannot be told.
    /// Importing it as some other version would drop its mod loader.
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum IconSource {
    File(PathBuf),
    DataUrl(String),
}

#[derive(Debug, Clone)]
struct Importable {
    filename: String,
    game_dir: PathBuf,
    name: String,
    icon: Option<IconSource>,
    version: VersionRef,
    last_played: Option<DateTime<Utc>>,
    settings: ImportedSettings,
    /// A profile's own game folder that did not exist at scan time: the
    /// official launcher creates it on first launch, so it imports empty.
    game_dir_missing: bool,
}

impl From<Importable> for ImportableInstance {
    fn from(value: Importable) -> Self {
        let mut warnings = value.settings.warnings();
        if value.game_dir_missing {
            warnings.push(Translation::InstanceImportGameDirMissing);
        }
        Self {
            filename: value.filename,
            instance_name: value.name,
            warnings,
        }
    }
}

#[derive(Debug)]
pub struct VanillaImporter {
    flavor: VanillaFlavor,
    state: RwLock<ImporterState<Importable>>,
}

impl VanillaImporter {
    pub fn new(flavor: VanillaFlavor) -> Self {
        Self {
            flavor,
            state: RwLock::new(ImporterState::NoResults),
        }
    }

    pub async fn get_default_scan_path(flavor: VanillaFlavor) -> anyhow::Result<Option<PathBuf>> {
        let mut candidates = Vec::new();

        // A custom game directory in the client's settings beats the stock location.
        for client_dir in client_dir_candidates(flavor) {
            if let Some(game_dir) = game_dir_hint_from_settings(&client_dir).await {
                candidates.push(game_dir);
            }
        }

        candidates.extend(minecraft_dir_candidates());

        Ok(first_existing_dir(candidates))
    }

    async fn scan_entries(&self, game_dir: &Path) -> anyhow::Result<Vec<Importable>> {
        let installed = read_installed_versions(game_dir).await?;

        let mut entries = match self.flavor {
            VanillaFlavor::Minecraft => launcher_profile_entries(game_dir, &installed).await,
            flavor => client_entries(flavor, game_dir, &installed).await,
        };

        if entries.is_empty() {
            let version = newest_installed_release(&installed)
                .map(|mc| VersionRef::Exact {
                    minecraft: mc,
                    loaders: Vec::new(),
                })
                .unwrap_or(VersionRef::LatestRelease);
            entries.push(Importable {
                filename: self.flavor.key().to_string(),
                game_dir: game_dir.to_path_buf(),
                name: self.flavor.label().to_string(),
                icon: None,
                version,
                last_played: None,
                settings: ImportedSettings::default(),
                game_dir_missing: false,
            });
        }

        Ok(entries)
    }
}

#[async_trait::async_trait]
impl InstanceImporter for VanillaImporter {
    async fn scan(&self, app: &Arc<AppInner>, scan_path: PathBuf) -> anyhow::Result<()> {
        let scan_path = if scan_path.is_file() {
            match scan_path.parent() {
                Some(parent) => parent.to_path_buf(),
                None => return Ok(()),
            }
        } else {
            scan_path
        };

        let Some(game_dir) = resolve_game_dir(&scan_path) else {
            trace!("{scan_path:?} does not look like a Minecraft game directory");
            return Ok(());
        };

        let entries = self.scan_entries(&game_dir).await?;

        if entries.len() == 1 {
            let entry = entries.into_iter().next().unwrap();
            self.state
                .write()
                .await
                .set_single(profile_entry(entry))
                .await;
            app.invalidate(GET_IMPORT_SCAN_STATUS, None);
            return Ok(());
        }

        for entry in entries {
            self.state
                .write()
                .await
                .push_multi(profile_entry(entry))
                .await;
            app.invalidate(GET_IMPORT_SCAN_STATUS, None);
        }

        Ok(())
    }

    async fn get_status(&self) -> ImportScanStatus {
        self.state.read().await.clone().into()
    }

    async fn begin_import(
        &self,
        app: &Arc<AppInner>,
        index: u32,
        name: Option<String>,
    ) -> anyhow::Result<VisualTaskId> {
        trace!(?index, ?name, "Beginning {} import", self.flavor.label());

        let instance = self
            .state
            .read()
            .await
            .get(index)
            .await
            .cloned()
            .ok_or_else(|| anyhow!("invalid importable instance index {index}"))?;

        let icon = match &instance.icon {
            Some(IconSource::File(path)) => {
                app.instance_manager().load_icon(path.clone()).await.ok()
            }
            Some(IconSource::DataUrl(url)) => icon_from_data_url(url),
            None => None,
        };

        let standard_version = match &instance.version {
            VersionRef::Exact { minecraft, loaders } => {
                resolve_standard_version(app, minecraft, loaders).await
            }
            VersionRef::Unresolved => {
                anyhow::bail!("Cannot tell which Minecraft version this profile uses; rescan")
            }
            VersionRef::LatestRelease | VersionRef::LatestSnapshot => {
                let manifest = app.minecraft_manager().get_minecraft_manifest().await?;
                let release = if instance.version == VersionRef::LatestSnapshot {
                    manifest.latest.snapshot
                } else {
                    manifest.latest.release
                };
                StandardVersion {
                    release,
                    modloaders: Default::default(),
                }
            }
        };

        let instance_version_source =
            InstanceVersionSource::Version(GameVersion::Standard(standard_version));

        let initializer = |instance_path: PathBuf| {
            let instance = &instance;
            async move {
                let filter = |path: &Path| !is_launcher_managed(path);
                if instance.game_dir_missing {
                    copy_optional_game_dir(&instance.game_dir, &instance_path, filter).await?;
                } else {
                    copy_game_dir(&instance.game_dir, &instance_path, filter).await?;
                }
                Ok(())
            }
        };

        let group = group_for_import(app, instance.settings.group.as_deref()).await?;

        let id = app
            .instance_manager()
            .create_instance_ext(
                group,
                name.unwrap_or_else(|| instance.name.clone()),
                icon,
                None,
                instance.last_played,
                instance_version_source,
                instance.settings.notes.clone().unwrap_or_default(),
                initializer,
            )
            .await?;

        apply_imported_settings(app, id, &instance.settings).await;

        app.instance_manager()
            .prepare_game(id, None, None, true)
            .await
            .map(|r| r.1)
    }
}

pub(super) fn is_launcher_managed(relative: &Path) -> bool {
    let Some(first) = relative.components().next() else {
        return false;
    };
    let first = first.as_os_str().to_string_lossy();
    let lower = first.to_ascii_lowercase();

    matches!(
        lower.as_str(),
        "versions"
            | "libraries"
            | "assets"
            | "runtime"
            | "natives"
            | "bin"
            | "logs"
            | "crash-reports"
            | "webcache"
            | "webcache2"
            | "cache"
            | "tmp"
            | "temp"
            | "blclient"
            | "feather"
            | ".feather"
            | ".fabric"
            | ".mixin.out"
            | "tcnodetracker"
            | "usercache.json"
            | "usernamecache.json"
            | "clientid.txt"
            | "launcher.jar"
            | "launcher.pack.lzma"
    ) || lower.starts_with("launcher_")
        || lower.starts_with("hs_err_pid")
        || lower.ends_with(".log")
}

fn resolve_game_dir(path: &Path) -> Option<PathBuf> {
    if !path.is_dir() {
        return None;
    }

    if looks_like_game_dir(path) {
        return Some(path.to_path_buf());
    }

    for name in [".minecraft", "minecraft"] {
        let nested = path.join(name);
        if looks_like_game_dir(&nested) {
            return Some(nested);
        }
    }

    // Nothing recognisable is still accepted (the user picked it) unless it's obviously something else.
    Some(path.to_path_buf())
}

fn looks_like_game_dir(path: &Path) -> bool {
    path.is_dir()
        && [
            "launcher_profiles.json",
            "launcher_profiles_microsoft_store.json",
            "options.txt",
            "usercache.json",
        ]
        .iter()
        .any(|f| path.join(f).is_file())
        || ["versions", "saves", "resourcepacks"]
            .iter()
            .any(|d| path.join(d).is_dir())
}

/// `versions/` only helps tell profiles' versions apart, so a folder that
/// cannot be listed costs that help, not the scan: profiles still come from
/// launcher_profiles.json, and ids not read here are parsed from the id.
async fn read_installed_versions(
    game_dir: &Path,
) -> anyhow::Result<HashMap<String, ParsedVersionId>> {
    let mut installed = HashMap::new();
    let mut parents = HashMap::new();
    let mut unread = HashSet::new();

    let versions_dir = game_dir.join("versions");
    let mut dir = match super::common::scan_directory(&versions_dir).await {
        Ok(dir) => dir,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(installed),
        Err(e) => {
            warn!("Cannot list {versions_dir:?}, reading versions from their ids: {e}");
            return Ok(installed);
        }
    };

    loop {
        let entry = match dir.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            Err(e) => {
                warn!("Cannot finish listing {versions_dir:?}: {e}");
                break;
            }
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();

        let json = read_json_value(&path.join(format!("{id}.json"))).await;
        let mut parsed = match &json {
            Some(json) => versions_from_version_json(json),
            None => parse_version_id(&id),
        };

        match &json {
            Some(json) => {
                if parsed.minecraft.is_none() {
                    parsed.minecraft = minecraft_from_version_json_hints(json);
                }
                if let Some(parent) = json_trimmed_string(json, "inheritsFrom") {
                    parents.insert(id.clone(), parent);
                }
            }
            None => {
                unread.insert(id.clone());
            }
        }

        // A json that only names itself is a vanilla version.
        if parsed.minecraft.is_none()
            && parsed.loaders.is_empty()
            && !parents.contains_key(&id)
            && (looks_like_minecraft_version(&id)
                || json
                    .as_ref()
                    .is_some_and(|json| is_mojang_version(json, &id)))
        {
            parsed.minecraft = Some(id.clone());
        }

        installed.insert(id, parsed);
    }

    inherit_from_installed_parents(&mut installed, &parents, &unread);

    Ok(installed)
}

/// Experimental snapshots Mojang published outside the version list
/// (`1.19_deep_dark_experimental_snapshot-1`) carry their own client and Mojang's
/// main class under their real id. These ids start with a digit, which tells them
/// from a renamed copy (`MyVanilla`).
fn is_mojang_version(json: &serde_json::Value, id: &str) -> bool {
    id.starts_with(|c: char| c.is_ascii_digit())
        && json_trimmed_string(json, "id").as_deref() == Some(id)
        && json.pointer("/downloads/client").is_some()
        && json_trimmed_string(json, "mainClass").as_deref()
            == Some("net.minecraft.client.main.Main")
}

/// Loader installers name the release in places other than `inheritsFrom`:
/// Forge and NeoForge pass `--fml.mcVersion`, older jsons carry `jar`, and
/// some launchers write `clientVersion`.
fn minecraft_from_version_json_hints(json: &serde_json::Value) -> Option<String> {
    let fml = json
        .pointer("/arguments/game")
        .and_then(|args| args.as_array())
        .and_then(|args| {
            args.iter()
                .skip_while(|arg| arg.as_str() != Some("--fml.mcVersion"))
                .nth(1)
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
        });
    fml.into_iter()
        .chain(json_trimmed_string(json, "clientVersion"))
        .chain(json_trimmed_string(json, "jar"))
        .find(|value| is_release_id(value))
}

/// A renamed pack folder (`ATM10`) usually inherits from the loader's own
/// folder (`neoforge-21.1.77`), which names the release. Follow `inheritsFrom`
/// through the installed folders and keep every loader found along the way,
/// also when the folder already names its release (`"jar": "1.21.1"`): that
/// says nothing about the loader its parent brings.
fn inherit_from_installed_parents(
    installed: &mut HashMap<String, ParsedVersionId>,
    parents: &HashMap<String, String>,
    unread: &HashSet<String>,
) {
    let mut resolved = Vec::new();
    for (id, parsed) in installed.iter() {
        let mut minecraft = parsed.minecraft.clone();
        let mut loaders = parsed.loaders.clone();
        let mut unknown_parent = false;
        let mut seen = HashSet::from([id.as_str()]);
        let mut current = parents.get(id);
        while let Some(parent) = current {
            if !seen.insert(parent.as_str()) {
                break;
            }
            let read_parent = installed.get(parent).filter(|_| !unread.contains(parent));
            let from_id;
            let (parent_minecraft, parent_loaders) = match read_parent {
                Some(parsed) => (parsed.minecraft.clone(), &parsed.loaders),
                // A parent whose json cannot be read can only be judged by its id.
                None => {
                    from_id = parse_version_id(parent);
                    let release = from_id.minecraft.clone().filter(|m| is_release_id(m));
                    unknown_parent = !is_release_id(parent) && from_id.loaders.is_empty();
                    (release, &from_id.loaders)
                }
            };
            for loader in parent_loaders {
                if !loaders.iter().any(|l| l.type_ == loader.type_) {
                    loaders.push(loader.clone());
                }
            }
            minecraft = minecraft.or(parent_minecraft);
            current = parents.get(parent);
        }
        if parents.contains_key(id) {
            // Whether that parent brings a loader cannot be told, and guessing
            // "none" would import a modded profile as vanilla.
            if unknown_parent && loaders.is_empty() {
                minecraft = None;
            }
            resolved.push((id.clone(), minecraft, loaders));
        }
    }
    for (id, minecraft, loaders) in resolved {
        if let Some(parsed) = installed.get_mut(&id) {
            parsed.minecraft = minecraft;
            parsed.loaders = loaders;
        }
    }
}

fn release_sort_key(id: &str) -> Option<Vec<u32>> {
    let parts: Option<Vec<u32>> = id.split('.').map(|p| p.parse().ok()).collect();
    parts.filter(|p| (2..=3).contains(&p.len()) && p[0] == 1)
}

/// Folders named by release id, not the patched ones a client or loader installed.
fn newest_installed_release(installed: &HashMap<String, ParsedVersionId>) -> Option<String> {
    installed
        .keys()
        .filter_map(|id| release_sort_key(id).map(|key| (key, id.clone())))
        .max()
        .map(|(_, id)| id)
}

fn version_ref(version_id: &str, installed: &HashMap<String, ParsedVersionId>) -> VersionRef {
    match version_id.trim() {
        "" | "latest-release" => return VersionRef::LatestRelease,
        "latest-snapshot" => return VersionRef::LatestSnapshot,
        _ => {}
    }

    let parsed = match installed.get(version_id.trim()) {
        // An installed folder was read and still names no release (a renamed
        // or self-contained modded version). Falling back to the latest
        // release would silently drop its loader.
        Some(parsed) if parsed.minecraft.is_none() => {
            warn!("Cannot tell which Minecraft version the installed `{version_id}` is");
            return VersionRef::Unresolved;
        }
        Some(parsed) => parsed.clone(),
        None => parse_version_id(version_id),
    };

    match parsed.minecraft {
        Some(minecraft) => VersionRef::Exact {
            minecraft,
            loaders: parsed.loaders,
        },
        None => {
            warn!(
                "Cannot tell which Minecraft version `{version_id}` is, using the latest release"
            );
            VersionRef::LatestRelease
        }
    }
}

/// Both the classic and the Microsoft Store profile files; a machine can have either or both.
async fn launcher_profile_entries(
    game_dir: &Path,
    installed: &HashMap<String, ParsedVersionId>,
) -> Vec<Importable> {
    let mut entries: Vec<Importable> = Vec::new();

    for file in [
        "launcher_profiles.json",
        "launcher_profiles_microsoft_store.json",
    ] {
        let Some(json) = read_json_value(&game_dir.join(file)).await else {
            continue;
        };
        let Some(profiles) = json.get("profiles").and_then(|p| p.as_object()) else {
            continue;
        };

        for (key, profile) in profiles {
            if !profile.is_object() {
                continue;
            }

            let profile_type = json_string(profile, "type").unwrap_or_default();

            let name = json_trimmed_string(profile, "name").unwrap_or_else(|| {
                match profile_type.as_str() {
                    "latest-release" => "Latest release".to_string(),
                    "latest-snapshot" => "Latest snapshot".to_string(),
                    _ => key.clone(),
                }
            });

            let version_id =
                json_trimmed_string(profile, "lastVersionId").unwrap_or_else(|| match profile_type
                    .as_str()
                {
                    "latest-snapshot" => "latest-snapshot".to_string(),
                    _ => "latest-release".to_string(),
                });

            let profile_game_dir = json_trimmed_string(profile, "gameDir")
                .map(|d| {
                    let d = PathBuf::from(d);
                    if d.is_absolute() { d } else { game_dir.join(d) }
                })
                .unwrap_or_else(|| game_dir.to_path_buf());

            let icon = json_string(profile, "icon")
                .filter(|i| i.starts_with("data:"))
                .map(IconSource::DataUrl);

            let last_played = json_string(profile, "lastUsed").and_then(|s| {
                DateTime::parse_from_rfc3339(s.trim())
                    .ok()
                    .map(|d| d.with_timezone(&Utc))
                    .filter(|d| d.timestamp() > 0)
            });

            // The key doubles as the row's secondary line, so prefer the version over the opaque profile id.
            let mut filename = version_id.clone();
            if entries.iter().any(|e| e.filename == filename) {
                filename = format!("{version_id} ({key})");
            }

            let mut settings = ImportedSettings {
                java_path: json_trimmed_string(profile, "javaDir"),
                game_resolution: profile.get("resolution").and_then(|r| {
                    Some((
                        json_positive_u32(r, "width")?,
                        json_positive_u32(r, "height")?,
                    ))
                }),
                ..Default::default()
            };
            if let Some(args) = json_string(profile, "javaArgs") {
                settings.absorb_jvm_args(&args);
            }

            entries.push(Importable {
                filename,
                game_dir: profile_game_dir,
                name,
                icon,
                version: version_ref(&version_id, installed),
                last_played,
                settings,
                game_dir_missing: false,
            });
        }
    }

    entries
}

async fn client_entries(
    flavor: VanillaFlavor,
    game_dir: &Path,
    installed: &HashMap<String, ParsedVersionId>,
) -> Vec<Importable> {
    let mut versions: Vec<(String, Vec<DeclaredModLoader>)> = Vec::new();

    if flavor == VanillaFlavor::Lunar {
        for client_dir in client_dir_candidates(flavor) {
            for mc in
                lunar_versions_from_multiver(&client_dir.join("offline").join("multiver")).await
            {
                if !versions.iter().any(|(v, _)| *v == mc) {
                    versions.push((mc, Vec::new()));
                }
            }
        }
    }

    for (id, parsed) in installed {
        let lower = id.to_ascii_lowercase();
        if !flavor.version_markers().iter().any(|m| lower.contains(m)) {
            continue;
        }
        let Some(mc) = parsed.minecraft.clone() else {
            continue;
        };
        if !versions.iter().any(|(v, _)| *v == mc) {
            versions.push((mc, parsed.loaders.clone()));
        }
    }

    versions.sort_by(|a, b| release_sort_key(&b.0).cmp(&release_sort_key(&a.0)));

    versions
        .into_iter()
        .map(|(mc, loaders)| Importable {
            filename: format!("{}-{mc}", flavor.key()),
            game_dir: game_dir.to_path_buf(),
            name: format!("{} {mc}", flavor.label()),
            icon: None,
            version: VersionRef::Exact {
                minecraft: mc,
                loaders,
            },
            last_played: None,
            settings: ImportedSettings::default(),
            game_dir_missing: false,
        })
        .collect()
}

/// `offline/multiver/<thing>-<mc version>-<build>.jar` names the versions Lunar was launched with.
async fn lunar_versions_from_multiver(multiver: &Path) -> Vec<String> {
    let mut versions = Vec::new();

    let Ok(mut dir) = tokio::fs::read_dir(multiver).await else {
        return versions;
    };

    while let Ok(Some(entry)) = dir.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        for token in name.split(|c: char| !c.is_ascii_digit() && c != '.') {
            let token = token.trim_matches('.');
            if release_sort_key(token).is_some() && !versions.contains(&token.to_string()) {
                versions.push(token.to_string());
            }
        }
    }

    versions.sort_by(|a, b| release_sort_key(b).cmp(&release_sort_key(a)));
    versions
}

pub(super) fn minecraft_dir_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let Some(basedirs) = directories::BaseDirs::new() else {
        return candidates;
    };
    let home = basedirs.home_dir().to_path_buf();

    #[cfg(target_os = "macos")]
    candidates.push(home.join("Library/Application Support/minecraft"));
    #[cfg(target_os = "windows")]
    candidates.push(basedirs.data_dir().join(".minecraft"));

    candidates.push(home.join(".minecraft"));
    candidates.push(basedirs.data_dir().join(".minecraft"));
    candidates.push(home.join(".var/app/com.mojang.Minecraft/.minecraft"));
    candidates.push(home.join("snap/minecraft-launcher/current/.minecraft"));

    dedup_paths(candidates)
}

fn client_dir_candidates(flavor: VanillaFlavor) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let Some(basedirs) = directories::BaseDirs::new() else {
        return candidates;
    };
    let home = basedirs.home_dir().to_path_buf();
    let data = basedirs.data_dir().to_path_buf();

    match flavor {
        VanillaFlavor::Minecraft => {}
        VanillaFlavor::Lunar => {
            candidates.push(home.join(".lunarclient"));
            candidates.push(data.join(".lunarclient"));
            candidates.push(data.join("lunarclient"));
            candidates.push(home.join("Library/Application Support/lunarclient"));
        }
        VanillaFlavor::Badlion => {
            candidates.push(data.join("Badlion Client"));
            candidates.push(home.join(".badlion"));
            candidates.push(home.join("Library/Application Support/Badlion Client"));
        }
        VanillaFlavor::Feather => {
            candidates.push(data.join(".feather"));
            candidates.push(data.join("feather"));
            candidates.push(home.join(".feather"));
        }
    }

    dedup_paths(candidates)
        .into_iter()
        .filter(|p| p.is_dir())
        .collect()
}

/// None of the clients documents where a custom game directory is stored; look through their small json files for one.
/// Clients record a relocated game directory in a settings file a couple of
/// levels under their own folder, so the search has to cover more than one fixed
/// name. It must not cover the whole install: `offline/`, `jre/` and `textures/`
/// hold thousands of files between them, and reading every JSON in there cost
/// ~400ms per candidate directory. Only config-shaped subdirectories are opened.
async fn game_dir_hint_from_settings(client_dir: &Path) -> Option<PathBuf> {
    const MAX_FILE_SIZE: u64 = 1024 * 1024;
    const SETTINGS_SUBDIRS: [&str; 5] = ["settings", "config", "configs", "launcher", "profiles"];
    const MAX_JSON_FILES: usize = 64;

    let mut json_files = Vec::new();
    let mut dirs = vec![(client_dir.to_path_buf(), 0u8)];

    while let Some((dir, depth)) = dirs.pop() {
        let Ok(mut entries) = tokio::fs::read_dir(&dir).await else {
            continue;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let Ok(meta) = entry.metadata().await else {
                continue;
            };
            if meta.is_dir() {
                // Below the client folder the subtrees are small, so only the
                // first level needs gating.
                let worth_opening = depth > 0
                    || SETTINGS_SUBDIRS.iter().any(|name| {
                        path.file_name()
                            .is_some_and(|n| n.eq_ignore_ascii_case(name))
                    });
                if depth < 2 && worth_opening {
                    dirs.push((path, depth + 1));
                }
            } else if meta.len() <= MAX_FILE_SIZE && path.extension().is_some_and(|e| e == "json") {
                json_files.push(path);
                if json_files.len() >= MAX_JSON_FILES {
                    break;
                }
            }
        }

        if json_files.len() >= MAX_JSON_FILES {
            break;
        }
    }

    for file in json_files {
        let Some(json) = read_json_value(&file).await else {
            continue;
        };
        if let Some(dir) = find_game_dir_string(&json) {
            return Some(dir);
        }
    }

    None
}

fn find_game_dir_string(value: &serde_json::Value) -> Option<PathBuf> {
    match value {
        serde_json::Value::String(s) => {
            let path = PathBuf::from(s.trim());
            (path.is_absolute() && looks_like_game_dir(&path)).then_some(path)
        }
        serde_json::Value::Array(items) => items.iter().find_map(find_game_dir_string),
        serde_json::Value::Object(map) => map.values().find_map(find_game_dir_string),
        _ => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn minecraft_dir_candidates_are_unique() {
        super::super::common::assert_paths_unique(&minecraft_dir_candidates());
    }
    use crate::domain::instance::info::ModLoaderType;

    fn write_version(game_dir: &Path, id: &str, json: serde_json::Value) {
        let dir = game_dir.join("versions").join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{id}.json")), json.to_string()).unwrap();
    }

    #[tokio::test]
    async fn launcher_profiles_become_instances() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path();
        std::fs::create_dir_all(game_dir.join("saves")).unwrap();
        write_version(game_dir, "1.20.1", serde_json::json!({ "id": "1.20.1" }));
        write_version(
            game_dir,
            "1.20.1-forge-47.2.0",
            serde_json::json!({
                "id": "1.20.1-forge-47.2.0",
                "inheritsFrom": "1.20.1",
                "libraries": [{ "name": "net.minecraftforge:forge:1.20.1-47.2.0" }]
            }),
        );
        let custom_dir = tmp.path().join("custom");
        std::fs::create_dir_all(&custom_dir).unwrap();

        std::fs::write(
            game_dir.join("launcher_profiles.json"),
            serde_json::json!({
                "profiles": {
                    "abc": {
                        "name": "Forge fun",
                        "type": "custom",
                        "lastVersionId": "1.20.1-forge-47.2.0",
                        "gameDir": custom_dir.to_string_lossy(),
                        "icon": "data:image/png;base64,AAAA",
                        "lastUsed": "2024-01-02T03:04:05.000Z",
                        "created": "2023-01-01T00:00:00.000Z",
                        "javaArgs": "-Xmx6G -XX:+UseG1GC",
                        "javaDir": "/opt/jdk/bin/java",
                        "resolution": { "width": 1920, "height": 1080 }
                    },
                    "def": {
                        "type": "latest-release",
                        "icon": "Grass",
                        "lastVersionId": "latest-release",
                        "lastUsed": "1970-01-01T00:00:00.000Z"
                    },
                    "ghi": {
                        "name": "Snapshots",
                        "type": "latest-snapshot",
                        "lastVersionId": "latest-snapshot"
                    }
                },
                "settings": {},
                "version": 3
            })
            .to_string(),
        )
        .unwrap();

        let importer = VanillaImporter::new(VanillaFlavor::Minecraft);
        let mut entries = importer.scan_entries(game_dir).await.unwrap();
        entries.sort_by(|a, b| a.filename.cmp(&b.filename));
        assert_eq!(entries.len(), 3);

        let forge = &entries[0];
        assert_eq!(forge.filename, "1.20.1-forge-47.2.0");
        assert_eq!(forge.name, "Forge fun");
        assert_eq!(forge.game_dir, custom_dir);
        assert_eq!(
            forge.version,
            VersionRef::Exact {
                minecraft: "1.20.1".into(),
                loaders: vec![DeclaredModLoader::new(ModLoaderType::Forge, "47.2.0")],
            }
        );
        assert_eq!(
            forge.icon,
            Some(IconSource::DataUrl("data:image/png;base64,AAAA".into()))
        );
        assert_eq!(
            forge.last_played.map(|d| d.timestamp()),
            Some(1_704_164_645)
        );
        assert_eq!(forge.settings.memory(), Some((6144, 6144)));
        assert_eq!(
            forge.settings.extra_java_args.as_deref(),
            Some("-XX:+UseG1GC")
        );
        assert_eq!(
            forge.settings.java_path.as_deref(),
            Some("/opt/jdk/bin/java")
        );
        assert_eq!(forge.settings.game_resolution, Some((1920, 1080)));

        let latest = &entries[1];
        assert_eq!(latest.name, "Latest release");
        assert_eq!(latest.version, VersionRef::LatestRelease);
        assert_eq!(latest.icon, None);
        assert_eq!(latest.last_played, None);
        assert_eq!(latest.game_dir, game_dir);
        assert_eq!(latest.settings, ImportedSettings::default());

        assert_eq!(entries[2].version, VersionRef::LatestSnapshot);
    }

    #[tokio::test]
    async fn game_dir_without_profiles_becomes_one_instance() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path();
        write_version(game_dir, "1.8.9", serde_json::json!({ "id": "1.8.9" }));
        write_version(game_dir, "1.20.4", serde_json::json!({ "id": "1.20.4" }));
        write_version(game_dir, "24w10a", serde_json::json!({ "id": "24w10a" }));

        let importer = VanillaImporter::new(VanillaFlavor::Minecraft);
        let entries = importer.scan_entries(game_dir).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Minecraft");
        assert_eq!(
            entries[0].version,
            VersionRef::Exact {
                minecraft: "1.20.4".into(),
                loaders: vec![],
            }
        );

        let empty = tempfile::tempdir().unwrap();
        let entries = importer.scan_entries(empty.path()).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].version, VersionRef::LatestRelease);
    }

    #[tokio::test]
    async fn client_versions_are_detected() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path();
        write_version(game_dir, "1.8.9", serde_json::json!({ "id": "1.8.9" }));
        write_version(
            game_dir,
            "Feather-1.20.1-fabric",
            serde_json::json!({
                "id": "Feather-1.20.1-fabric",
                "inheritsFrom": "1.20.1",
                "libraries": [{ "name": "net.fabricmc:fabric-loader:0.15.0" }]
            }),
        );
        write_version(
            game_dir,
            "BadlionClient-1.16.5",
            serde_json::json!({ "id": "BadlionClient-1.16.5", "inheritsFrom": "1.16.5" }),
        );

        let installed = read_installed_versions(game_dir).await.unwrap();

        let feather = client_entries(VanillaFlavor::Feather, game_dir, &installed).await;
        assert_eq!(feather.len(), 1);
        assert_eq!(feather[0].name, "Feather Client 1.20.1");
        assert_eq!(feather[0].filename, "feather-1.20.1");
        assert_eq!(
            feather[0].version,
            VersionRef::Exact {
                minecraft: "1.20.1".into(),
                loaders: vec![DeclaredModLoader::new(ModLoaderType::Fabric, "0.15.0")],
            }
        );

        let badlion = client_entries(VanillaFlavor::Badlion, game_dir, &installed).await;
        assert_eq!(badlion.len(), 1);
        assert_eq!(badlion[0].name, "Badlion Client 1.16.5");

        let importer = VanillaImporter::new(VanillaFlavor::Lunar);
        let entries = importer.scan_entries(game_dir).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Lunar Client");
        assert_eq!(
            entries[0].version,
            VersionRef::Exact {
                minecraft: "1.8.9".into(),
                loaders: vec![],
            }
        );
    }

    #[tokio::test]
    async fn lunar_versions_come_from_multiver_jars() {
        let tmp = tempfile::tempdir().unwrap();
        let multiver = tmp.path().join("offline").join("multiver");
        std::fs::create_dir_all(&multiver).unwrap();
        for name in [
            "lunar-prod-optifine-1.8.9-v2.jar",
            "1.20.1-lunar-assets.jar",
            "natives-1.20.1-windows.zip",
            "OptiFine_1.16.5_HD_U_G8.jar",
            "vpatcher-prod.jar",
        ] {
            std::fs::write(multiver.join(name), b"").unwrap();
        }

        assert_eq!(
            lunar_versions_from_multiver(&multiver).await,
            vec![
                "1.20.1".to_string(),
                "1.16.5".to_string(),
                "1.8.9".to_string()
            ]
        );
        assert!(
            lunar_versions_from_multiver(&tmp.path().join("nope"))
                .await
                .is_empty()
        );
    }

    #[test]
    fn version_refs_resolve_against_installed_versions() {
        let mut installed = HashMap::new();
        installed.insert(
            "fabric-loader-0.15.0-1.20.1".to_string(),
            ParsedVersionId {
                minecraft: Some("1.20.1".into()),
                loaders: vec![DeclaredModLoader::new(ModLoaderType::Fabric, "0.15.0")],
            },
        );

        assert_eq!(
            version_ref("latest-release", &installed),
            VersionRef::LatestRelease
        );
        assert_eq!(version_ref("", &installed), VersionRef::LatestRelease);
        assert_eq!(
            version_ref("latest-snapshot", &installed),
            VersionRef::LatestSnapshot
        );
        assert_eq!(
            version_ref("fabric-loader-0.15.0-1.20.1", &installed),
            VersionRef::Exact {
                minecraft: "1.20.1".into(),
                loaders: vec![DeclaredModLoader::new(ModLoaderType::Fabric, "0.15.0")],
            }
        );
        assert_eq!(
            version_ref("1.19.2", &installed),
            VersionRef::Exact {
                minecraft: "1.19.2".into(),
                loaders: vec![],
            }
        );
        assert_eq!(
            version_ref("forge-47.2.0", &installed),
            VersionRef::LatestRelease
        );
    }

    #[test]
    fn newest_release_is_picked_numerically() {
        let mut installed = HashMap::new();
        for id in ["1.8.9", "1.20.4", "1.9", "24w10a", "1.20.1-forge-47.2.0"] {
            installed.insert(id.to_string(), parse_version_id(id));
        }
        assert_eq!(
            newest_installed_release(&installed).as_deref(),
            Some("1.20.4")
        );
        assert_eq!(newest_installed_release(&HashMap::new()), None);
    }

    #[test]
    fn game_dirs_are_resolved_and_launcher_files_excluded() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let mc = root.join(".minecraft");
        std::fs::create_dir_all(mc.join("saves")).unwrap();

        assert_eq!(resolve_game_dir(&mc), Some(mc.clone()));
        assert_eq!(resolve_game_dir(root), Some(mc.clone()));
        assert_eq!(resolve_game_dir(&root.join("missing")), None);

        assert!(is_launcher_managed(Path::new("versions/1.20.1/1.20.1.jar")));
        assert!(is_launcher_managed(Path::new("launcher_profiles.json")));
        assert!(is_launcher_managed(Path::new(
            "launcher_accounts_microsoft_store.json"
        )));
        assert!(is_launcher_managed(Path::new("blclient/settings.json")));
        assert!(is_launcher_managed(Path::new("hs_err_pid1234.log")));
        assert!(!is_launcher_managed(Path::new("saves/world/level.dat")));
        assert!(!is_launcher_managed(Path::new("options.txt")));
        assert!(!is_launcher_managed(Path::new("mods/sodium.jar")));
        assert!(!is_launcher_managed(Path::new(
            "config/lunar/profile_manager.json"
        )));
    }

    #[tokio::test]
    async fn custom_game_dir_is_found_in_client_settings() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("games").join("mc");
        std::fs::create_dir_all(game.join("saves")).unwrap();
        let client = tmp.path().join("client");
        std::fs::create_dir_all(client.join("settings").join("launcher")).unwrap();
        std::fs::write(
            client
                .join("settings")
                .join("launcher")
                .join("settings.json"),
            serde_json::json!({ "theme": "dark", "game": { "directory": game.to_string_lossy() } })
                .to_string(),
        )
        .unwrap();
        std::fs::write(client.join("junk.json"), "not json").unwrap();

        assert_eq!(game_dir_hint_from_settings(&client).await, Some(game));
        assert_eq!(
            game_dir_hint_from_settings(&tmp.path().join("nope")).await,
            None
        );
    }

    #[tokio::test]
    async fn settings_search_skips_bulk_subdirectories() {
        let tmp = tempfile::tempdir().unwrap();
        let client = tmp.path().join("client");
        let decoy = tmp.path().join("decoy");
        std::fs::create_dir_all(decoy.join("saves")).unwrap();

        // A real install keeps thousands of files under offline/ and jre/. A game
        // directory recorded in there must not be found, because finding it would
        // mean the whole subtree was read.
        for sub in ["offline/multiver", "jre/zulu17"] {
            std::fs::create_dir_all(client.join(sub)).unwrap();
            std::fs::write(
                client.join(sub).join("cache.json"),
                serde_json::json!({ "game": { "directory": decoy.to_string_lossy() } }).to_string(),
            )
            .unwrap();
        }

        assert_eq!(game_dir_hint_from_settings(&client).await, None);
    }
}

#[cfg(test)]
mod task13_tests {
    use super::*;
    #[tokio::test]
    async fn task13_unavailable_explicit_profile_keeps_source() {
        let t = tempfile::tempdir().unwrap();
        let missing = t.path().join("detached");
        std::fs::write(t.path().join("launcher_profiles.json"), serde_json::json!({"profiles":{"custom":{"name":"Synthetic custom", "gameDir":missing,"lastVersionId":"1.20.1"},"default":{"lastVersionId":"1.20.1"}}}).to_string()).unwrap();
        let entries = launcher_profile_entries(t.path(), &HashMap::new()).await;
        assert_eq!(
            entries
                .iter()
                .find(|e| e.name == "Synthetic custom")
                .unwrap()
                .game_dir,
            missing
        );
        assert_eq!(
            entries
                .iter()
                .find(|e| e.name == "default")
                .unwrap()
                .game_dir,
            t.path()
        );
        // A profile never started in the official launcher has no game folder
        // yet: it imports empty with its version, and the row says so.
        let InternalImportEntry::Valid(custom) = profile_entry(
            entries
                .iter()
                .find(|e| e.name == "Synthetic custom")
                .unwrap()
                .clone(),
        ) else {
            panic!("a profile without its game folder yet was refused")
        };
        assert!(custom.game_dir_missing);
        assert_eq!(
            ImportableInstance::from(custom).warnings,
            [Translation::InstanceImportGameDirMissing]
        );
        // Something that is there but is not a folder stays unavailable.
        std::fs::write(&missing, b"not a folder").unwrap();
        assert!(matches!(
            profile_entry(
                entries
                    .iter()
                    .find(|e| e.name == "Synthetic custom")
                    .unwrap()
                    .clone()
            ),
            InternalImportEntry::Invalid(_)
        ));
        assert!(matches!(
            profile_entry(
                entries
                    .iter()
                    .find(|e| e.name == "default")
                    .unwrap()
                    .clone()
            ),
            InternalImportEntry::Valid(_)
        ));
    }
}

fn profile_entry(mut entry: Importable) -> InternalImportEntry<Importable> {
    entry.settings.skip_missing_java_path();
    if entry.version == VersionRef::Unresolved {
        return InternalImportEntry::Invalid(super::InvalidImportEntry {
            name: format!("{} ({})", entry.name, entry.filename),
            reason: Translation::InstanceImportUnknownMinecraftVersion,
        });
    }
    match std::fs::metadata(&entry.game_dir) {
        Ok(metadata) if metadata.is_dir() => InternalImportEntry::Valid(entry),
        // The official launcher creates a profile's game folder on first launch,
        // so a profile that was never started has none: import it empty. A
        // folder whose drive is not there at all stays unavailable instead.
        Err(e)
            if e.kind() == std::io::ErrorKind::NotFound && volume_is_present(&entry.game_dir) =>
        {
            entry.game_dir_missing = true;
            InternalImportEntry::Valid(entry)
        }
        _ => InternalImportEntry::Invalid(super::InvalidImportEntry {
            name: format!("{} ({})", entry.name, entry.game_dir.display()),
            reason: Translation::InstanceImportSourceUnavailable,
        }),
    }
}

/// Whether the drive or share a path lives on is reachable. A link whose target
/// is gone counts as not reachable: it points at a folder that existed.
fn volume_is_present(path: &Path) -> bool {
    let dangling_link = path
        .ancestors()
        .find(|ancestor| ancestor.symlink_metadata().is_ok())
        .is_some_and(|ancestor| !ancestor.exists());
    !dangling_link && volume_root(path).is_dir()
}

/// The folder that only exists while a path's drive is connected: `E:\` or
/// `\\server\share` on Windows, and the usual removable-drive mount points on
/// macOS and Linux. Anywhere else the drive cannot be told from the path, so `/`
/// is all there is to check.
fn volume_root(path: &Path) -> &Path {
    let root = path.ancestors().last().unwrap_or(path);
    let mut components = path.components();
    if components.next() != Some(std::path::Component::RootDir) {
        return root;
    }
    let names: Vec<&str> = components
        .map(|name| name.as_os_str().to_str().unwrap_or_default())
        .collect();
    let depth = match names.as_slice() {
        ["Volumes" | "mnt", _, ..] | ["media", _] => 2,
        // `/media/<user>/<drive>` and `/run/media/<user>/<drive>`.
        ["media", _, _, ..] => 3,
        ["run", "media", _, _, ..] => 4,
        _ => return root,
    };
    path.ancestors().nth(names.len() - depth).unwrap_or(root)
}

#[cfg(test)]
mod installed_version_tests {
    use super::*;
    use crate::domain::instance::info::ModLoaderType;

    fn write_version(game_dir: &Path, id: &str, json: serde_json::Value) {
        let dir = game_dir.join("versions").join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{id}.json")), json.to_string()).unwrap();
    }

    fn neoforge(minecraft: &str, loader: &str) -> VersionRef {
        VersionRef::Exact {
            minecraft: minecraft.into(),
            loaders: vec![DeclaredModLoader::new(ModLoaderType::Neoforge, loader)],
        }
    }

    /// Renamed or self-contained pack folders used to be imported as a vanilla
    /// "latest release" instance, silently dropping their mod loader.
    #[tokio::test]
    async fn custom_version_folders_keep_their_release_and_loader() {
        let t = tempfile::tempdir().unwrap();
        let game_dir = t.path();
        write_version(game_dir, "1.21.1", serde_json::json!({ "id": "1.21.1" }));
        write_version(
            game_dir,
            "neoforge-21.1.77",
            serde_json::json!({
                "id": "neoforge-21.1.77",
                "inheritsFrom": "1.21.1",
                "libraries": [{ "name": "net.neoforged:neoforge:21.1.77" }]
            }),
        );
        // Inherits from the loader's folder rather than from a release.
        write_version(
            game_dir,
            "ATM10",
            serde_json::json!({ "id": "ATM10", "inheritsFrom": "neoforge-21.1.77" }),
        );
        write_version(
            game_dir,
            "1.21.1-ATM10",
            serde_json::json!({ "id": "1.21.1-ATM10", "inheritsFrom": "ATM10" }),
        );
        // Self-contained: no inheritsFrom, the release only in the FML arguments.
        write_version(
            game_dir,
            "Merged Pack",
            serde_json::json!({
                "id": "Merged Pack",
                "libraries": [{ "name": "net.neoforged:neoforge:21.1.80" }],
                "arguments": { "game": ["--fml.neoForgeVersion", "21.1.80", "--fml.mcVersion", "1.21.1"] }
            }),
        );
        // Nothing names a release.
        write_version(
            game_dir,
            "Mystery",
            serde_json::json!({
                "id": "Mystery",
                "libraries": [{ "name": "net.neoforged:neoforge:21.1.80" }]
            }),
        );
        let installed = read_installed_versions(game_dir).await.unwrap();
        assert_eq!(
            version_ref("ATM10", &installed),
            neoforge("1.21.1", "21.1.77")
        );
        assert_eq!(
            version_ref("1.21.1-ATM10", &installed),
            neoforge("1.21.1", "21.1.77")
        );
        assert_eq!(
            version_ref("Merged Pack", &installed),
            neoforge("1.21.1", "21.1.80")
        );
        assert_eq!(version_ref("Mystery", &installed), VersionRef::Unresolved);
        // Ids that are not installed keep the existing handling.
        assert_eq!(
            version_ref("1.20.4", &installed),
            VersionRef::Exact {
                minecraft: "1.20.4".into(),
                loaders: vec![],
            }
        );

        std::fs::write(
            game_dir.join("launcher_profiles.json"),
            serde_json::json!({"profiles":{"m":{"name":"Mystery pack","lastVersionId":"Mystery"}}})
                .to_string(),
        )
        .unwrap();
        let entries = VanillaImporter::new(VanillaFlavor::Minecraft)
            .scan_entries(game_dir)
            .await
            .unwrap();
        let [entry] = entries.as_slice() else {
            panic!("expected one profile")
        };
        assert!(matches!(
            profile_entry(entry.clone()),
            InternalImportEntry::Invalid(super::super::InvalidImportEntry {
                reason: Translation::InstanceImportUnknownMinecraftVersion,
                ..
            })
        ));
    }

    /// A release hint (`jar`, `clientVersion`, `--fml.mcVersion`) says nothing
    /// about the loader a parent brings; stopping there imported these as vanilla.
    #[tokio::test]
    async fn a_release_hint_keeps_the_loader_of_its_parents() {
        let t = tempfile::tempdir().unwrap();
        let game_dir = t.path();
        write_version(game_dir, "1.21.1", serde_json::json!({ "id": "1.21.1" }));
        write_version(
            game_dir,
            "neoforge-21.1.77",
            serde_json::json!({
                "id": "neoforge-21.1.77",
                "inheritsFrom": "1.21.1",
                "libraries": [{ "name": "net.neoforged:neoforge:21.1.77" }]
            }),
        );
        for (id, hint) in [
            ("Jar", serde_json::json!({ "jar": "1.21.1" })),
            (
                "ClientVersion",
                serde_json::json!({ "clientVersion": "1.21.1" }),
            ),
            (
                "Fml",
                serde_json::json!({ "arguments": { "game": ["--fml.mcVersion", "1.21.1"] } }),
            ),
        ] {
            let mut json = serde_json::json!({ "id": id, "inheritsFrom": "neoforge-21.1.77" });
            json.as_object_mut()
                .unwrap()
                .extend(hint.as_object().unwrap().clone());
            write_version(game_dir, id, json);
        }
        // One step further from the loader.
        write_version(
            game_dir,
            "Chained",
            serde_json::json!({ "id": "Chained", "inheritsFrom": "Jar", "jar": "1.21.1" }),
        );
        // The parent is not installed; its id still names the loader.
        write_version(
            game_dir,
            "MyLite",
            serde_json::json!({
                "id": "MyLite",
                "inheritsFrom": "1.12.2-forge-14.23.5.2860",
                "jar": "1.12.2"
            }),
        );
        write_version(
            game_dir,
            "MissingLoaderFolder",
            serde_json::json!({ "id": "MissingLoaderFolder", "inheritsFrom": "neoforge-21.1.80" }),
        );
        write_version(
            game_dir,
            "NoHintNoFolder",
            serde_json::json!({
                "id": "NoHintNoFolder",
                "inheritsFrom": "1.20.1-forge-47.2.0"
            }),
        );
        // Nothing tells what these parents bring, so vanilla would be a guess.
        write_version(
            game_dir,
            "UnknownParent",
            serde_json::json!({ "id": "UnknownParent", "inheritsFrom": "SomePack", "jar": "1.21.1" }),
        );
        std::fs::create_dir_all(game_dir.join("versions").join("NoJson")).unwrap();
        write_version(
            game_dir,
            "UnreadParent",
            serde_json::json!({ "id": "UnreadParent", "inheritsFrom": "NoJson", "jar": "1.21.1" }),
        );
        // Plain inheritance from a release stays vanilla.
        write_version(
            game_dir,
            "Plain",
            serde_json::json!({ "id": "Plain", "inheritsFrom": "1.21.1", "jar": "1.21.1" }),
        );
        write_version(
            game_dir,
            "Detached",
            serde_json::json!({ "id": "Detached", "inheritsFrom": "1.20.4", "jar": "1.20.4" }),
        );

        let installed = read_installed_versions(game_dir).await.unwrap();
        for id in ["Jar", "ClientVersion", "Fml", "Chained"] {
            assert_eq!(
                version_ref(id, &installed),
                neoforge("1.21.1", "21.1.77"),
                "{id}"
            );
        }
        assert_eq!(
            version_ref("MyLite", &installed),
            VersionRef::Exact {
                minecraft: "1.12.2".into(),
                loaders: vec![DeclaredModLoader::new(ModLoaderType::Forge, "14.23.5.2860")],
            }
        );
        assert_eq!(
            version_ref("NoHintNoFolder", &installed),
            VersionRef::Exact {
                minecraft: "1.20.1".into(),
                loaders: vec![DeclaredModLoader::new(ModLoaderType::Forge, "47.2.0")],
            }
        );
        for id in ["MissingLoaderFolder", "UnknownParent", "UnreadParent"] {
            assert_eq!(version_ref(id, &installed), VersionRef::Unresolved, "{id}");
        }
        for (id, minecraft) in [("Plain", "1.21.1"), ("Detached", "1.20.4")] {
            assert_eq!(
                version_ref(id, &installed),
                VersionRef::Exact {
                    minecraft: minecraft.into(),
                    loaders: vec![],
                },
                "{id}"
            );
        }
    }

    /// Published versions outside the release grammar were refused with "can't
    /// tell which version this profile uses", with their id right there.
    #[tokio::test]
    async fn mojang_versions_with_unusual_ids_keep_their_id() {
        let t = tempfile::tempdir().unwrap();
        let game_dir = t.path();
        let vanilla = |id: &str| {
            serde_json::json!({
                "id": id,
                "mainClass": "net.minecraft.client.main.Main",
                "downloads": { "client": { "url": "https://example.invalid/client.jar" } }
            })
        };
        let ids = [
            "22w13oneblockatatime",
            "3D Shareware v1.34",
            "1.19_deep_dark_experimental_snapshot-1",
        ];
        for id in ids {
            write_version(game_dir, id, vanilla(id));
        }
        // A renamed copy, and a modded json with the client it patches.
        write_version(game_dir, "MyVanilla", vanilla("MyVanilla"));
        write_version(
            game_dir,
            "1.20.1_tweaked_version_folder",
            serde_json::json!({
                "id": "1.20.1_tweaked_version_folder",
                "mainClass": "net.minecraft.launchwrapper.Launch",
                "downloads": { "client": { "url": "https://example.invalid/client.jar" } }
            }),
        );
        let installed = read_installed_versions(game_dir).await.unwrap();
        for id in ids {
            assert_eq!(
                version_ref(id, &installed),
                VersionRef::Exact {
                    minecraft: id.into(),
                    loaders: vec![],
                },
                "{id}"
            );
        }
        for id in ["MyVanilla", "1.20.1_tweaked_version_folder"] {
            assert_eq!(version_ref(id, &installed), VersionRef::Unresolved, "{id}");
        }
    }

    fn profile_with_game_dir(game_dir: PathBuf) -> Importable {
        Importable {
            filename: "1.20.1".into(),
            game_dir,
            name: "Profile".into(),
            icon: None,
            version: VersionRef::Exact {
                minecraft: "1.20.1".into(),
                loaders: vec![],
            },
            last_played: None,
            settings: ImportedSettings::default(),
            game_dir_missing: false,
        }
    }

    fn assert_unavailable(game_dir: PathBuf) {
        assert!(!volume_is_present(&game_dir), "{game_dir:?}");
        assert!(
            matches!(
                profile_entry(profile_with_game_dir(game_dir.clone())),
                InternalImportEntry::Invalid(super::super::InvalidImportEntry {
                    reason: Translation::InstanceImportSourceUnavailable,
                    ..
                })
            ),
            "{game_dir:?}"
        );
    }

    #[test]
    fn removable_drive_mount_points_are_the_volume() {
        for (path, volume) in [
            ("/Volumes/USB/mc", "/Volumes/USB"),
            ("/Volumes/USB", "/Volumes/USB"),
            ("/mnt/usb/games/mc", "/mnt/usb"),
            ("/media/me/USB/mc", "/media/me/USB"),
            ("/media/usb", "/media/usb"),
            ("/run/media/me/USB/mc", "/run/media/me/USB"),
            ("/home/me/mc", "/"),
            ("/opt/games/mc", "/"),
            ("/media", "/"),
            ("/run/user/1000/mc", "/"),
        ] {
            assert_eq!(volume_root(Path::new(path)), Path::new(volume), "{path}");
        }
    }

    /// An unplugged drive must not look like a game folder that was never
    /// created: the row asks to reconnect it instead of importing nothing.
    #[test]
    fn a_game_folder_on_a_missing_drive_stays_unavailable() {
        for path in [
            "/Volumes/gdl-no-such-drive/mc",
            "/mnt/gdl-no-such-drive/mc",
            "/media/gdl-no-such-user/gdl-no-such-drive/mc",
            "/run/media/gdl-no-such-user/gdl-no-such-drive/mc",
        ] {
            assert_unavailable(PathBuf::from(path));
        }
    }

    /// A game folder that may simply not have been created yet still imports.
    #[test]
    fn a_never_created_game_folder_on_a_present_drive_imports_empty() {
        let t = tempfile::tempdir().unwrap();
        let InternalImportEntry::Valid(entry) =
            profile_entry(profile_with_game_dir(t.path().join("never-launched")))
        else {
            panic!("a never-created game folder was refused")
        };
        assert!(entry.game_dir_missing);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_a_folder_that_is_gone_stays_unavailable() {
        let t = tempfile::tempdir().unwrap();
        let link = t.path().join("mc");
        std::os::unix::fs::symlink("/mnt/gdl-no-such-drive/mc", &link).unwrap();
        assert_unavailable(link.clone());
        assert_unavailable(link.join("nested"));
    }

    #[cfg(windows)]
    #[test]
    fn a_game_folder_on_an_unused_drive_letter_stays_unavailable() {
        let unused = ('D'..='Z')
            .rev()
            .map(|letter| PathBuf::from(format!("{letter}:\\")))
            .find(|root| !root.exists())
            .expect("every drive letter is in use");
        assert_eq!(volume_root(&unused.join("mc")), unused.as_path());
        assert_unavailable(unused.join("mc"));
    }

    /// A Java picked by hand that is gone since would stop every launch.
    #[test]
    fn a_profile_with_a_missing_java_imports_without_it() {
        let t = tempfile::tempdir().unwrap();
        let mut profile = profile_with_game_dir(t.path().to_path_buf());
        profile.settings.java_path = Some(
            t.path()
                .join("jre1.8.0_51")
                .join("javaw.exe")
                .display()
                .to_string(),
        );
        let InternalImportEntry::Valid(entry) = profile_entry(profile) else {
            panic!("a profile with a stale Java path was refused")
        };
        assert_eq!(entry.settings.java_path, None);
        assert_eq!(
            ImportableInstance::from(entry).warnings,
            [Translation::InstanceImportJavaPathSkipped]
        );
    }

    /// An unreadable versions/ folder used to fail the whole scan with no rows.
    #[tokio::test]
    async fn an_unreadable_versions_folder_still_lists_profiles() {
        let t = tempfile::tempdir().unwrap();
        let game_dir = t.path();
        write_version(game_dir, "1.20.1", serde_json::json!({ "id": "1.20.1" }));
        std::fs::write(
            game_dir.join("launcher_profiles.json"),
            serde_json::json!({"profiles":{"a":{"name":"Plain","lastVersionId":"1.20.1"}}})
                .to_string(),
        )
        .unwrap();
        let versions = game_dir.join("versions");
        super::super::common::fail_scan_directory(versions.clone());
        let entries = VanillaImporter::new(VanillaFlavor::Minecraft)
            .scan_entries(game_dir)
            .await;
        super::super::common::clear_scan_failure(&versions);
        let entries = entries.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].version,
            VersionRef::Exact {
                minecraft: "1.20.1".into(),
                loaders: vec![],
            }
        );
    }
}
