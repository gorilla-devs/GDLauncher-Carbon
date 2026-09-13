//! Minecraft Launcher, Lunar, Badlion and Feather importer; all share the `.minecraft` directory.

use std::{
    collections::HashMap,
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
        copy_game_dir, first_existing_dir, group_for_import, icon_from_data_url, json_i64,
        json_positive_u32, json_string, json_trimmed_string, looks_like_minecraft_version,
        parse_version_id, read_json_value, resolve_standard_version,
    },
    technic::versions_from_version_json,
};
use crate::{
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
}

impl From<Importable> for ImportableInstance {
    fn from(value: Importable) -> Self {
        Self {
            filename: value.filename,
            instance_name: value.name,
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

    async fn scan_entries(&self, game_dir: &Path) -> Vec<Importable> {
        let installed = read_installed_versions(game_dir).await;

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
            });
        }

        entries
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

        let entries = self.scan_entries(&game_dir).await;

        if entries.len() == 1 {
            let entry = entries.into_iter().next().unwrap();
            self.state
                .write()
                .await
                .set_single(InternalImportEntry::Valid(entry))
                .await;
            app.invalidate(GET_IMPORT_SCAN_STATUS, None);
            return Ok(());
        }

        for entry in entries {
            self.state
                .write()
                .await
                .push_multi(InternalImportEntry::Valid(entry))
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
                copy_game_dir(&instance.game_dir, &instance_path, |path| {
                    !is_launcher_managed(path)
                })
                .await?;
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

async fn read_installed_versions(game_dir: &Path) -> HashMap<String, ParsedVersionId> {
    let mut installed = HashMap::new();

    let Ok(mut dir) = tokio::fs::read_dir(game_dir.join("versions")).await else {
        return installed;
    };

    while let Ok(Some(entry)) = dir.next_entry().await {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();

        let mut parsed = match read_json_value(&path.join(format!("{id}.json"))).await {
            Some(json) => versions_from_version_json(&json),
            None => parse_version_id(&id),
        };

        // A json that only names itself is a vanilla version.
        if parsed.minecraft.is_none()
            && looks_like_minecraft_version(&id)
            && parsed.loaders.is_empty()
        {
            parsed.minecraft = Some(id.clone());
        }

        installed.insert(id, parsed);
    }

    installed
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

    let parsed = installed
        .get(version_id.trim())
        .cloned()
        .unwrap_or_else(|| parse_version_id(version_id));

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
                .filter(|d| d.is_dir())
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

    candidates
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

    candidates.into_iter().filter(|p| p.is_dir()).collect()
}

/// None of the clients documents where a custom game directory is stored; look through their small json files for one.
async fn game_dir_hint_from_settings(client_dir: &Path) -> Option<PathBuf> {
    const MAX_FILE_SIZE: u64 = 1024 * 1024;

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
                if depth < 2 {
                    dirs.push((path, depth + 1));
                }
            } else if meta.len() <= MAX_FILE_SIZE && path.extension().is_some_and(|e| e == "json") {
                json_files.push(path);
            }
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
        let mut entries = importer.scan_entries(game_dir).await;
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
        let entries = importer.scan_entries(game_dir).await;
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
        let entries = importer.scan_entries(empty.path()).await;
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

        let installed = read_installed_versions(game_dir).await;

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
        let entries = importer.scan_entries(game_dir).await;
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
}
