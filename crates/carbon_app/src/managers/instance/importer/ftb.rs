use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tracing::{trace, warn};

use super::{
    GET_IMPORT_SCAN_STATUS, ImportScanStatus, ImportableInstance, ImporterState, InstanceImporter,
    InternalImportEntry, InvalidImportEntry,
    common::{
        DeclaredModLoader, ImportedSettings, apply_imported_settings, copy_game_dir,
        datetime_from_epoch_secs_or_millis, dir_name, first_existing_dir, group_for_import,
        icon_from_data_url, json_i64, json_string, looks_like_minecraft_version, parse_version_id,
        read_json_value, resolve_standard_version, seconds_played_from_i64,
    },
};
use crate::{
    api::translation::Translation,
    domain::{
        instance::info::{CurseforgeModpack, GameVersion, Modpack},
        vtask::VisualTaskId,
    },
    managers::{AppInner, instance::InstanceVersionSource},
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum IconSource {
    TaggedFile(PathBuf),
    File(PathBuf),
    DataUrl(String),
}

#[derive(Debug, Clone)]
struct Importable {
    filename: String,
    path: PathBuf,
    name: String,
    icon: Option<IconSource>,
    minecraft_version: String,
    loaders: Vec<DeclaredModLoader>,
    modpack: Option<Modpack>,
    seconds_played: Option<u32>,
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
pub struct FtbImporter {
    state: RwLock<ImporterState<Importable>>,
}

impl FtbImporter {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(ImporterState::NoResults),
        }
    }

    pub async fn get_default_scan_path() -> anyhow::Result<Option<PathBuf>> {
        let Some(basedirs) = directories::BaseDirs::new() else {
            return Ok(None);
        };
        let home = basedirs.home_dir().to_path_buf();

        let mut candidates = Vec::new();
        #[cfg(target_os = "windows")]
        {
            candidates.push(basedirs.data_local_dir().join(".ftba"));
        }
        #[cfg(target_os = "macos")]
        {
            candidates.push(home.join("Library/Application Support/.ftba"));
        }
        candidates.push(home.join(".ftba"));
        candidates.push(basedirs.data_local_dir().join(".ftba"));
        candidates.push(basedirs.data_dir().join(".ftba"));

        let Some(root) = first_existing_dir(candidates) else {
            return Ok(None);
        };

        Ok(Some(instances_dir_for_root(&root).await))
    }

    async fn scan_instance(
        &self,
        path: PathBuf,
    ) -> anyhow::Result<Option<InternalImportEntry<Importable>>> {
        let json_path = path.join("instance.json");
        if !json_path.is_file() {
            return Ok(None);
        }

        let filename = dir_name(&path);

        let Some(json) = read_json_value(&json_path).await else {
            warn!("Cannot read FTB instance metadata at {json_path:?}");
            return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                name: filename,
                reason: Translation::InstanceImportLegacyBadConfigFile,
            })));
        };

        // ATLauncher also names its file instance.json; FTB's has a uuid / mcVersion.
        if json.get("uuid").is_none() && json.get("mcVersion").is_none() {
            return Ok(None);
        }

        let Some(mut parsed) = parse_instance_json(&json, &filename) else {
            warn!("FTB instance at {path:?} has no usable Minecraft version");
            return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                name: filename,
                reason: Translation::InstanceImportLegacyBadConfigFile,
            })));
        };

        let icon = find_icon(&path, parsed.art.as_deref());

        if let Some(instances_dir) = path.parent() {
            parsed.settings.group = category_name(instances_dir, &json).await;
        }

        Ok(Some(InternalImportEntry::Valid(Importable {
            filename,
            path,
            name: parsed.name,
            icon,
            minecraft_version: parsed.minecraft_version,
            loaders: parsed.loaders,
            modpack: parsed.modpack,
            seconds_played: parsed.seconds_played,
            last_played: parsed.last_played,
            settings: parsed.settings,
        })))
    }
}

/// The app's own "Default" category maps to GDL's default group.
async fn category_name(instances_dir: &Path, json: &serde_json::Value) -> Option<String> {
    let mut name = None;

    if let Some(category_id) = json_string(json, "categoryId").filter(|id| !id.trim().is_empty()) {
        if let Some(categories) = read_json_value(&instances_dir.join("categories.json")).await {
            name = categories.as_array().and_then(|list| {
                list.iter().find_map(|category| {
                    (json_string(category, "uuid")?.eq_ignore_ascii_case(category_id.trim()))
                        .then(|| json_string(category, "name"))
                        .flatten()
                })
            });
        }
    }

    name.or_else(|| json_string(json, "category"))
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty() && !n.eq_ignore_ascii_case("default"))
}

#[async_trait::async_trait]
impl InstanceImporter for FtbImporter {
    async fn scan(&self, app: &Arc<AppInner>, scan_path: PathBuf) -> anyhow::Result<()> {
        let scan_path = if scan_path.is_file() {
            match scan_path.parent() {
                Some(parent) => parent.to_path_buf(),
                None => return Ok(()),
            }
        } else {
            scan_path
        };

        if !scan_path.is_dir() {
            return Ok(());
        }

        if scan_path.join("instance.json").is_file() {
            if let Ok(Some(entry)) = self.scan_instance(scan_path).await {
                self.state.write().await.set_single(entry).await;
                app.invalidate(GET_IMPORT_SCAN_STATUS, None);
            }
            return Ok(());
        }

        let instances_dir = if is_app_root(&scan_path) {
            instances_dir_for_root(&scan_path).await
        } else {
            scan_path
        };

        let Ok(mut dir) = tokio::fs::read_dir(&instances_dir).await else {
            return Ok(());
        };

        while let Some(entry) = dir.next_entry().await? {
            let entry_path = entry.path();
            if !entry_path.is_dir() {
                continue;
            }

            if let Ok(Some(entry)) = self.scan_instance(entry_path).await {
                self.state.write().await.push_multi(entry).await;
                app.invalidate(GET_IMPORT_SCAN_STATUS, None);
            }
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
        trace!(?index, ?name, "Beginning FTB import");

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
            Some(IconSource::TaggedFile(path)) => read_tagged_artwork(path).await,
            Some(IconSource::DataUrl(url)) => icon_from_data_url(url),
            None => None,
        };

        let standard_version =
            resolve_standard_version(app, &instance.minecraft_version, &instance.loaders).await;
        let version = GameVersion::Standard(standard_version);

        let instance_version_source = match &instance.modpack {
            Some(modpack) => {
                InstanceVersionSource::ModpackWithKnownVersion(version, modpack.clone(), true)
            }
            None => InstanceVersionSource::Version(version),
        };

        let initializer = |instance_path: PathBuf| {
            let instance = &instance;
            async move {
                copy_game_dir(&instance.path, &instance_path, |path| {
                    !is_launcher_file(path)
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
                instance.seconds_played,
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

fn is_launcher_file(relative: &Path) -> bool {
    let Some(first) = relative.components().next() else {
        return false;
    };
    let first = first.as_os_str().to_string_lossy();

    first.starts_with("instance.json")
        || first == ".ftbapp"
        || first == ".ftba"
        || first == "version.json"
        || first == "version_mods.json"
        || first == "modifications.json"
        || first == "folder.jpg"
        || first == "art.png"
        || first == "art.jpg"
        || first == "natives"
}

fn is_app_root(path: &Path) -> bool {
    path.join("instances").is_dir()
        || path.join("storage").join("settings.json").is_file()
        || path.join("bin").join("settings.json").is_file()
}

async fn instances_dir_for_root(root: &Path) -> PathBuf {
    for settings in [
        root.join("storage").join("settings.json"),
        root.join("bin").join("settings.json"),
        root.join("settings.json"),
    ] {
        let Some(json) = read_json_value(&settings).await else {
            continue;
        };
        if let Some(location) = json_string(&json, "instanceLocation")
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
        {
            let location = PathBuf::from(location);
            let resolved = if location.is_absolute() {
                location
            } else {
                root.join(location)
            };
            if resolved.is_dir() {
                return resolved;
            }
        }
    }

    root.join("instances")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedInstance {
    name: String,
    minecraft_version: String,
    loaders: Vec<DeclaredModLoader>,
    modpack: Option<Modpack>,
    seconds_played: Option<u32>,
    last_played: Option<DateTime<Utc>>,
    art: Option<String>,
    settings: ImportedSettings,
}

/// Width/height are always written from the app's defaults, so they aren't per-instance.
fn instance_settings(json: &serde_json::Value) -> ImportedSettings {
    let mut settings = ImportedSettings {
        memory_max_mb: json_i64(json, "memory")
            .filter(|m| *m > 0)
            .and_then(|m| u32::try_from(m).ok()),
        java_path: json_string(json, "jrePath")
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty()),
        ..Default::default()
    };

    if let Some(args) = json_string(json, "jvmArgs") {
        settings.absorb_jvm_args(&args);
    }

    settings
}

fn parse_instance_json(json: &serde_json::Value, fallback_name: &str) -> Option<ParsedInstance> {
    let name = json_string(json, "name")
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| fallback_name.to_string());

    let mod_loader = json_string(json, "modLoader")
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty());
    let parsed_loader = mod_loader
        .as_deref()
        .map(parse_version_id)
        .unwrap_or_default();

    let minecraft_version = json_string(json, "mcVersion")
        .map(|v| v.trim().to_string())
        .filter(|v| looks_like_minecraft_version(v))
        .or(parsed_loader.minecraft)?;

    let modpack = match json_i64(json, "packType").unwrap_or(0) {
        1 => match (json_i64(json, "id"), json_i64(json, "versionId")) {
            (Some(project), Some(file)) if project > 0 && file > 0 => {
                match (u32::try_from(project), u32::try_from(file)) {
                    (Ok(project_id), Ok(file_id)) => Some(Modpack::Curseforge(CurseforgeModpack {
                        project_id,
                        file_id,
                    })),
                    _ => None,
                }
            }
            _ => None,
        },
        _ => None,
    };

    let seconds_played = json_i64(json, "totalPlayTime")
        .map(|ms| ms / 1000)
        .and_then(seconds_played_from_i64);
    let last_played = json_i64(json, "lastPlayed").and_then(datetime_from_epoch_secs_or_millis);

    Some(ParsedInstance {
        name,
        minecraft_version,
        loaders: parsed_loader.loaders,
        modpack,
        seconds_played,
        last_played,
        art: json_string(json, "art").filter(|a| a.starts_with("data:")),
        settings: instance_settings(json),
    })
}

fn find_icon(instance_root: &Path, art: Option<&str>) -> Option<IconSource> {
    let tagged = instance_root.join(".ftbapp").join("logo");
    if tagged.is_file() {
        return Some(IconSource::TaggedFile(tagged));
    }

    if let Some(art) = art {
        return Some(IconSource::DataUrl(art.to_string()));
    }

    ["art.png", "art.jpg", "folder.jpg", "icon.png"]
        .iter()
        .map(|name| instance_root.join(name))
        .find(|p| p.is_file())
        .map(IconSource::File)
}

/// Type byte: 0 png, 1 jpg, 2 gif, 3 webp; then the raw image.
async fn read_tagged_artwork(path: &Path) -> Option<(String, Vec<u8>)> {
    let data = tokio::fs::read(path).await.ok()?;
    decode_tagged_artwork(&data)
}

fn decode_tagged_artwork(data: &[u8]) -> Option<(String, Vec<u8>)> {
    let (tag, image) = data.split_first()?;
    if image.is_empty() {
        return None;
    }
    let ext = match tag {
        0 => "png",
        1 => "jpg",
        2 => "gif",
        3 => "webp",
        _ => return None,
    };
    Some((format!("icon.{ext}"), image.to_vec()))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::domain::instance::info::ModLoaderType;

    #[tokio::test]
    async fn settings_and_categories_are_read() {
        let json = serde_json::json!({
            "memory": 8192,
            "jvmArgs": "-XX:+UseG1GC",
            "jrePath": "/opt/jdk/bin/java",
            "categoryId": "AAAA-1",
            "width": 1280,
            "height": 720
        });
        let s = instance_settings(&json);
        assert_eq!(s.memory(), Some((8192, 8192)));
        assert_eq!(s.extra_java_args.as_deref(), Some("-XX:+UseG1GC"));
        assert_eq!(s.java_path.as_deref(), Some("/opt/jdk/bin/java"));
        assert_eq!(s.game_resolution, None);
        assert_eq!(
            instance_settings(&serde_json::json!({ "memory": 0, "jvmArgs": "" })),
            ImportedSettings::default()
        );

        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("categories.json"),
            r#"[{"uuid":"aaaa-1","name":"Skyblock"},{"uuid":"bbbb","name":"Default"}]"#,
        )
        .unwrap();
        assert_eq!(
            category_name(tmp.path(), &json).await.as_deref(),
            Some("Skyblock")
        );
        assert_eq!(
            category_name(tmp.path(), &serde_json::json!({ "categoryId": "bbbb" })).await,
            None
        );
        assert_eq!(
            category_name(
                tmp.path(),
                &serde_json::json!({ "category": "Kitchen sink" })
            )
            .await
            .as_deref(),
            Some("Kitchen sink")
        );
        assert_eq!(
            category_name(tmp.path(), &serde_json::json!({ "categoryId": "zzzz" })).await,
            None
        );
        assert_eq!(
            category_name(tmp.path(), &serde_json::json!({ "category": "Default" })).await,
            None
        );
    }

    #[test]
    fn instance_json_is_parsed() {
        let json = serde_json::json!({
            "uuid": "8c1e6f6e-0000-0000-0000-000000000000",
            "id": 91,
            "versionId": 2322,
            "name": "FTB Skies",
            "version": "1.7.0",
            "mcVersion": "1.19.2",
            "modLoader": "1.19.2-forge-43.2.14",
            "packType": 0,
            "totalPlayTime": 3_600_000,
            "lastPlayed": 1_700_000_000,
            "art": "data:image/png;base64,AAAA"
        });
        let parsed = parse_instance_json(&json, "dir").unwrap();
        assert_eq!(parsed.name, "FTB Skies");
        assert_eq!(parsed.minecraft_version, "1.19.2");
        assert_eq!(
            parsed.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Forge, "43.2.14")]
        );
        assert_eq!(parsed.modpack, None);
        assert_eq!(parsed.seconds_played, Some(3600));
        assert_eq!(
            parsed.last_played.map(|d| d.timestamp()),
            Some(1_700_000_000)
        );
        assert_eq!(parsed.art.as_deref(), Some("data:image/png;base64,AAAA"));
    }

    #[test]
    fn curseforge_packs_and_loader_variants_are_parsed() {
        let json = serde_json::json!({
            "uuid": "x",
            "id": 715572,
            "versionId": 5142600,
            "packType": 1,
            "name": "ATM9",
            "mcVersion": "1.20.1",
            "modLoader": "neoforge-47.1.79",
            "totalPlayTime": 0,
            "lastPlayed": 0
        });
        let parsed = parse_instance_json(&json, "dir").unwrap();
        assert_eq!(
            parsed.modpack,
            Some(Modpack::Curseforge(CurseforgeModpack {
                project_id: 715572,
                file_id: 5142600,
            }))
        );
        assert_eq!(
            parsed.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Neoforge, "47.1.79")]
        );
        assert_eq!(parsed.seconds_played, None);
        assert_eq!(parsed.last_played, None);

        let json = serde_json::json!({ "uuid": "x", "mcVersion": "1.20.4", "modLoader": "1.20.4" });
        let parsed = parse_instance_json(&json, "dir").unwrap();
        assert!(parsed.loaders.is_empty());
        assert_eq!(parsed.name, "dir");

        let json = serde_json::json!({ "uuid": "x", "modLoader": "fabric-loader-0.15.0-1.20.1" });
        let parsed = parse_instance_json(&json, "dir").unwrap();
        assert_eq!(parsed.minecraft_version, "1.20.1");
        assert_eq!(
            parsed.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Fabric, "0.15.0")]
        );

        assert!(parse_instance_json(&serde_json::json!({ "uuid": "x" }), "dir").is_none());
    }

    #[test]
    fn tagged_artwork_is_decoded() {
        assert_eq!(
            decode_tagged_artwork(&[0, 1, 2, 3]),
            Some(("icon.png".to_string(), vec![1, 2, 3]))
        );
        assert_eq!(
            decode_tagged_artwork(&[3, 9]),
            Some(("icon.webp".to_string(), vec![9]))
        );
        assert_eq!(decode_tagged_artwork(&[7, 1]), None);
        assert_eq!(decode_tagged_artwork(&[0]), None);
        assert_eq!(decode_tagged_artwork(&[]), None);
    }

    #[test]
    fn icons_are_found_in_priority_order() {
        let tmp = tempfile::tempdir().unwrap();
        let inst = tmp.path();

        assert_eq!(find_icon(inst, None), None);

        std::fs::write(inst.join("folder.jpg"), b"x").unwrap();
        assert_eq!(
            find_icon(inst, None),
            Some(IconSource::File(inst.join("folder.jpg")))
        );

        assert_eq!(
            find_icon(inst, Some("data:image/png;base64,AAAA")),
            Some(IconSource::DataUrl(
                "data:image/png;base64,AAAA".to_string()
            ))
        );

        std::fs::create_dir_all(inst.join(".ftbapp")).unwrap();
        std::fs::write(inst.join(".ftbapp").join("logo"), [0u8, 1, 2]).unwrap();
        assert_eq!(
            find_icon(inst, Some("data:image/png;base64,AAAA")),
            Some(IconSource::TaggedFile(inst.join(".ftbapp").join("logo")))
        );
    }

    #[tokio::test]
    async fn instances_dir_honours_settings() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("instances")).unwrap();
        assert_eq!(instances_dir_for_root(root).await, root.join("instances"));

        let custom = root.join("custom-instances");
        std::fs::create_dir_all(&custom).unwrap();
        std::fs::create_dir_all(root.join("storage")).unwrap();
        std::fs::write(
            root.join("storage").join("settings.json"),
            serde_json::json!({ "spec": "1.0.0", "instanceLocation": custom.to_string_lossy() })
                .to_string(),
        )
        .unwrap();
        assert_eq!(instances_dir_for_root(root).await, custom);
        assert!(is_app_root(root));
    }

    #[tokio::test]
    async fn instances_are_scanned_from_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let inst = tmp.path().join("instances").join("uuid-1");
        std::fs::create_dir_all(inst.join("mods")).unwrap();
        std::fs::write(
            inst.join("instance.json"),
            serde_json::json!({
                "uuid": "uuid-1",
                "name": "Skies",
                "mcVersion": "1.19.2",
                "modLoader": "1.19.2-forge-43.2.14"
            })
            .to_string(),
        )
        .unwrap();

        let other = tmp.path().join("instances").join("atl");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(
            other.join("instance.json"),
            serde_json::json!({ "id": "1.20.1", "launcher": { "name": "x" } }).to_string(),
        )
        .unwrap();

        let importer = FtbImporter::new();
        let entry = importer.scan_instance(inst.clone()).await.unwrap().unwrap();
        let InternalImportEntry::Valid(importable) = entry else {
            panic!("expected valid entry");
        };
        assert_eq!(importable.name, "Skies");
        assert_eq!(importable.filename, "uuid-1");

        assert!(importer.scan_instance(other).await.unwrap().is_none());
    }

    #[test]
    fn launcher_files_are_excluded_from_copy() {
        assert!(is_launcher_file(Path::new("instance.json")));
        assert!(is_launcher_file(Path::new("instance.json.bak")));
        assert!(is_launcher_file(Path::new(".ftbapp/logo")));
        assert!(is_launcher_file(Path::new("version.json")));
        assert!(!is_launcher_file(Path::new("mods/foo.jar")));
        assert!(!is_launcher_file(Path::new("saves/world/level.dat")));
    }
}
