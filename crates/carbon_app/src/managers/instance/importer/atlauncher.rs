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
        json_bool, json_i64, json_string, normalize_forge_version, parse_loader_type,
        parse_version_id, resolve_standard_version,
    },
};
use crate::{
    api::translation::Translation,
    domain::{
        instance::info::{CurseforgeModpack, GameVersion, ModLoaderType, Modpack, ModrinthModpack},
        vtask::VisualTaskId,
    },
    managers::{AppInner, instance::InstanceVersionSource},
};

#[derive(Debug, Clone)]
struct Importable {
    filename: String,
    path: PathBuf,
    name: String,
    icon_path: Option<PathBuf>,
    minecraft_version: String,
    loaders: Vec<DeclaredModLoader>,
    modpack: Option<Modpack>,
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
pub struct AtLauncherImporter {
    state: RwLock<ImporterState<Importable>>,
}

impl AtLauncherImporter {
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
        let data = basedirs.data_dir().to_path_buf();

        let mut candidates = vec![
            data.join("ATLauncher"),
            data.join("atlauncher"),
            home.join("ATLauncher"),
            home.join("atlauncher"),
            home.join(".atlauncher"),
            home.join(".local/share/atlauncher"),
            home.join("Desktop/ATLauncher"),
            home.join("Downloads/ATLauncher"),
            home.join("Documents/ATLauncher"),
            home.join("Games/ATLauncher"),
            home.join("Applications/ATLauncher"),
        ];

        #[cfg(target_os = "windows")]
        {
            candidates.push(basedirs.data_local_dir().join("ATLauncher"));
            candidates.push(
                basedirs
                    .data_local_dir()
                    .join("Programs")
                    .join("ATLauncher"),
            );
            candidates.push(PathBuf::from("C:\\ATLauncher"));
            candidates.push(PathBuf::from("C:\\Games\\ATLauncher"));
        }

        #[cfg(not(target_os = "windows"))]
        {
            candidates.push(PathBuf::from("/opt/atlauncher"));
        }

        // Only a directory that actually has instances counts.
        let launcher_dir = first_existing_dir(candidates.into_iter())
            .filter(|dir| dir.join("instances").is_dir() || dir.join("configs").is_dir());

        Ok(launcher_dir.map(|dir| dir.join("instances")))
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

        let parsed = tokio::fs::read_to_string(&json_path)
            .await
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .and_then(|value| parse_instance_json(&value, &filename));

        let Some(parsed) = parsed else {
            warn!("Cannot read ATLauncher instance metadata at {json_path:?}");
            return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                name: filename,
                reason: Translation::InstanceImportLegacyBadConfigFile,
            })));
        };

        let icon_path = find_icon(&path, parsed.pack_name.as_deref());

        Ok(Some(InternalImportEntry::Valid(Importable {
            filename,
            path,
            name: parsed.name,
            icon_path,
            minecraft_version: parsed.minecraft_version,
            loaders: parsed.loaders,
            modpack: parsed.modpack,
            last_played: parsed.last_played,
            settings: parsed.settings,
        })))
    }
}

#[async_trait::async_trait]
impl InstanceImporter for AtLauncherImporter {
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

        let instances_dir = if scan_path.join("instances").is_dir() {
            scan_path.join("instances")
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
        trace!(?index, ?name, "Beginning ATLauncher import");

        let instance = self
            .state
            .read()
            .await
            .get(index)
            .await
            .cloned()
            .ok_or_else(|| anyhow!("invalid importable instance index {index}"))?;

        let icon = match &instance.icon_path {
            Some(icon_path) => app
                .instance_manager()
                .load_icon(icon_path.clone())
                .await
                .ok(),
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

fn is_launcher_file(relative: &Path) -> bool {
    let mut components = relative.components();
    let Some(first) = components.next() else {
        return false;
    };
    let first = first.as_os_str().to_string_lossy();

    first.starts_with("instance.json")
        || first == "instance.png"
        || first == "bin"
        || first == "natives"
        || first == ".atlauncher"
}

fn find_icon(instance_root: &Path, pack_name: Option<&str>) -> Option<PathBuf> {
    let custom = instance_root.join("instance.png");
    if custom.is_file() {
        return Some(custom);
    }

    // Pack images are cached by the launcher as configs/images/<safe pack name>.png
    let pack_name = pack_name?;
    let safe_name: String = pack_name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    if safe_name.is_empty() {
        return None;
    }

    let launcher_dir = instance_root.parent()?.parent()?;
    let image = launcher_dir
        .join("configs")
        .join("images")
        .join(format!("{safe_name}.png"));
    image.is_file().then_some(image)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedInstance {
    name: String,
    pack_name: Option<String>,
    minecraft_version: String,
    loaders: Vec<DeclaredModLoader>,
    modpack: Option<Modpack>,
    last_played: Option<DateTime<Utc>>,
    settings: ImportedSettings,
}

fn instance_settings(launcher: Option<&serde_json::Value>) -> ImportedSettings {
    let Some(launcher) = launcher else {
        return ImportedSettings::default();
    };

    let text = |key: &str| {
        json_string(launcher, key)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };

    let mut settings = ImportedSettings {
        notes: text("description"),
        memory_max_mb: json_i64(launcher, "maximumMemory")
            .filter(|m| *m > 0)
            .and_then(|m| u32::try_from(m).ok()),
        java_path: text("javaPath"),
        ..Default::default()
    };

    if let Some(args) = text("javaArguments") {
        settings.absorb_jvm_args(&args);
    }

    if json_bool(launcher, "enableCommands").unwrap_or(false) {
        settings.pre_launch_hook = text("preLaunchCommand");
        settings.post_exit_hook = text("postExitCommand");
        settings.wrapper_command = text("wrapperCommand");
    }

    settings
}

fn parse_instance_json(value: &serde_json::Value, fallback_name: &str) -> Option<ParsedInstance> {
    let launcher = value.get("launcher").filter(|l| l.is_object());
    let launcher_str = |key: &str| launcher.and_then(|l| json_string(l, key));
    let launcher_obj = |key: &str| launcher.and_then(|l| l.get(key)).filter(|v| v.is_object());

    let name = launcher_str("name")
        .or_else(|| json_string(value, "name"))
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| fallback_name.to_string());

    let pack_name = launcher_str("pack").filter(|p| !p.trim().is_empty());

    // `id` is the vanilla version id; older layouts used `minecraftVersion`.
    let raw_id = json_string(value, "id")
        .or_else(|| json_string(value, "minecraftVersion"))
        .or_else(|| launcher_str("minecraftVersion"))
        .or_else(|| launcher_str("minecraft"))
        .or_else(|| json_string(value, "inheritsFrom"))?;
    let parsed_id = parse_version_id(&raw_id);
    let minecraft_version = parsed_id.minecraft?;

    let loader_version = launcher_obj("loaderVersion")
        .or_else(|| value.get("loaderVersion").filter(|v| v.is_object()));

    let mut loaders = Vec::new();
    if let Some(loader_version) = loader_version {
        let type_ = json_string(loader_version, "type").and_then(|t| parse_loader_type(&t));
        let version = json_string(loader_version, "version")
            .or_else(|| json_string(loader_version, "rawVersion"))
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());

        if let (Some(type_), Some(version)) = (type_, version) {
            let version = if type_ == ModLoaderType::Forge {
                normalize_forge_version(&minecraft_version, &version)
            } else {
                version
            };
            loaders.push(DeclaredModLoader::new(type_, version));
        }
    }
    if loaders.is_empty() {
        loaders = parsed_id.loaders;
    }

    let modpack = {
        let cf_project = launcher_obj("curseForgeProject").and_then(|p| json_i64(p, "id"));
        let cf_file = launcher_obj("curseForgeFile").and_then(|f| json_i64(f, "id"));
        let mr_project = launcher_obj("modrinthProject").and_then(|p| json_string(p, "id"));
        let mr_version = launcher_obj("modrinthVersion").and_then(|v| json_string(v, "id"));

        match (cf_project, cf_file, mr_project, mr_version) {
            (Some(project), Some(file), _, _) if project > 0 && file > 0 => {
                match (u32::try_from(project), u32::try_from(file)) {
                    (Ok(project_id), Ok(file_id)) => Some(Modpack::Curseforge(CurseforgeModpack {
                        project_id,
                        file_id,
                    })),
                    _ => None,
                }
            }
            (_, _, Some(project_id), Some(version_id))
                if !project_id.is_empty() && !version_id.is_empty() =>
            {
                Some(Modpack::Modrinth(ModrinthModpack {
                    project_id,
                    version_id,
                }))
            }
            _ => None,
        }
    };

    let last_played = launcher
        .and_then(|l| l.get("lastPlayed"))
        .or_else(|| value.get("lastPlayed"))
        .and_then(parse_instant);

    Some(ParsedInstance {
        name,
        pack_name,
        minecraft_version,
        loaders,
        modpack,
        last_played,
        settings: instance_settings(launcher),
    })
}

/// Epoch milliseconds, or RFC 3339 strings from older Gson-formatted builds.
fn parse_instant(value: &serde_json::Value) -> Option<DateTime<Utc>> {
    match value {
        serde_json::Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_f64().map(|f| f as i64))
            .and_then(datetime_from_epoch_secs_or_millis),
        serde_json::Value::String(s) => {
            let s = s.trim();
            if let Ok(n) = s.parse::<i64>() {
                return datetime_from_epoch_secs_or_millis(n);
            }
            DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|d| d.with_timezone(&Utc))
        }
        _ => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn launcher_settings_are_read() {
        let launcher = serde_json::json!({
            "description": "My notes",
            "maximumMemory": 6144,
            "javaArguments": "-XX:+UseG1GC -Xms1G",
            "javaPath": "C:\\Java\\bin\\java.exe",
            "enableCommands": true,
            "preLaunchCommand": "echo pre",
            "postExitCommand": "echo post",
            "wrapperCommand": "gamemoderun"
        });
        let s = instance_settings(Some(&launcher));
        assert_eq!(s.notes.as_deref(), Some("My notes"));
        assert_eq!(s.memory(), Some((1024, 6144)));
        assert_eq!(s.extra_java_args.as_deref(), Some("-XX:+UseG1GC"));
        assert_eq!(s.java_path.as_deref(), Some("C:\\Java\\bin\\java.exe"));
        assert_eq!(s.pre_launch_hook.as_deref(), Some("echo pre"));
        assert_eq!(s.post_exit_hook.as_deref(), Some("echo post"));
        assert_eq!(s.wrapper_command.as_deref(), Some("gamemoderun"));

        // Commands are only honoured when the launcher had them enabled
        let launcher =
            serde_json::json!({ "preLaunchCommand": "echo pre", "enableCommands": false });
        assert_eq!(instance_settings(Some(&launcher)).pre_launch_hook, None);
        assert_eq!(instance_settings(None), ImportedSettings::default());
    }

    #[test]
    fn current_instance_json_is_parsed() {
        let value = serde_json::json!({
            "id": "1.20.1",
            "mainClass": "net.minecraft.client.main.Main",
            "libraries": [],
            "launcher": {
                "name": "ATM9",
                "pack": "All the Mods 9",
                "version": "0.2.44",
                "loaderVersion": {
                    "type": "Forge",
                    "version": "47.2.0",
                    "rawVersion": "1.20.1-47.2.0",
                    "recommended": false
                },
                "curseForgeProject": { "id": 715572, "name": "All the Mods 9" },
                "curseForgeFile": { "id": 5142600, "displayName": "ATM9-0.2.44.zip" },
                "lastPlayed": 1700000000000i64,
                "numPlays": 3
            }
        });

        let parsed = parse_instance_json(&value, "ATM9dir").unwrap();
        assert_eq!(parsed.name, "ATM9");
        assert_eq!(parsed.pack_name.as_deref(), Some("All the Mods 9"));
        assert_eq!(parsed.minecraft_version, "1.20.1");
        assert_eq!(
            parsed.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Forge, "47.2.0")]
        );
        assert_eq!(
            parsed.modpack,
            Some(Modpack::Curseforge(CurseforgeModpack {
                project_id: 715572,
                file_id: 5142600,
            }))
        );
        assert_eq!(
            parsed.last_played.map(|d| d.timestamp()),
            Some(1_700_000_000)
        );
    }

    #[test]
    fn modrinth_and_loader_variants_are_parsed() {
        let value = serde_json::json!({
            "id": "1.20.4",
            "launcher": {
                "name": "Fabulously Optimized",
                "loaderVersion": { "type": "Fabric", "version": "0.15.6" },
                "modrinthProject": { "id": "1KVo5zza", "slug": "fabulously-optimized" },
                "modrinthVersion": { "id": "abcd1234", "projectId": "1KVo5zza" },
                "lastPlayed": "2024-01-02T03:04:05Z"
            }
        });
        let parsed = parse_instance_json(&value, "dir").unwrap();
        assert_eq!(
            parsed.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Fabric, "0.15.6")]
        );
        assert_eq!(
            parsed.modpack,
            Some(Modpack::Modrinth(ModrinthModpack {
                project_id: "1KVo5zza".to_string(),
                version_id: "abcd1234".to_string(),
            }))
        );
        assert_eq!(
            parsed.last_played.map(|d| d.timestamp()),
            Some(1_704_164_645)
        );

        let value = serde_json::json!({
            "id": "1.20.1",
            "launcher": {
                "name": "Neo",
                "loaderVersion": { "type": "NeoForge", "version": "47.1.79", "rawVersion": "1.20.1-47.1.79" }
            }
        });
        let parsed = parse_instance_json(&value, "dir").unwrap();
        assert_eq!(
            parsed.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Neoforge, "47.1.79")]
        );

        let value = serde_json::json!({
            "id": "1.20.1",
            "launcher": {
                "name": "Server",
                "loaderVersion": { "type": "Paper", "version": "123" }
            }
        });
        let parsed = parse_instance_json(&value, "dir").unwrap();
        assert!(parsed.loaders.is_empty());
        assert_eq!(parsed.modpack, None);
    }

    #[test]
    fn legacy_instance_json_is_parsed() {
        let value = serde_json::json!({
            "name": "Old Pack",
            "pack": "OldPack",
            "minecraftVersion": "1.7.10",
            "version": "1.2.3",
            "loaderVersion": { "type": "Forge", "version": "10.13.4.1614-1.7.10" }
        });
        let parsed = parse_instance_json(&value, "dir").unwrap();
        assert_eq!(parsed.name, "Old Pack");
        assert_eq!(parsed.minecraft_version, "1.7.10");
        assert_eq!(
            parsed.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Forge, "10.13.4.1614")]
        );

        let value = serde_json::json!({ "launcher": { "name": "Nothing" } });
        assert!(parse_instance_json(&value, "dir").is_none());

        let value = serde_json::json!({ "id": "1.16.5" });
        let parsed = parse_instance_json(&value, "dir").unwrap();
        assert_eq!(parsed.name, "dir");
        assert!(parsed.loaders.is_empty());
    }

    #[test]
    fn launcher_files_are_excluded_from_copy() {
        assert!(is_launcher_file(Path::new("instance.json")));
        assert!(is_launcher_file(Path::new("instance.json.bak")));
        assert!(is_launcher_file(Path::new("instance.png")));
        assert!(is_launcher_file(Path::new("bin/modpack.jar")));
        assert!(!is_launcher_file(Path::new("mods/foo.jar")));
        assert!(!is_launcher_file(Path::new("config")));
        assert!(!is_launcher_file(Path::new("")));
    }

    #[tokio::test]
    async fn instances_are_scanned_from_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let launcher = tmp.path();
        let inst = launcher.join("instances").join("ATM9");
        std::fs::create_dir_all(inst.join("mods")).unwrap();
        std::fs::write(inst.join("instance.png"), b"png").unwrap();
        std::fs::write(
            inst.join("instance.json"),
            serde_json::json!({
                "id": "1.20.1",
                "launcher": {
                    "name": "ATM9",
                    "loaderVersion": { "type": "Forge", "version": "47.2.0" }
                }
            })
            .to_string(),
        )
        .unwrap();

        let broken = launcher.join("instances").join("Broken");
        std::fs::create_dir_all(&broken).unwrap();
        std::fs::write(broken.join("instance.json"), "{ not json").unwrap();

        let importer = AtLauncherImporter::new();
        let entry = importer.scan_instance(inst.clone()).await.unwrap().unwrap();
        let InternalImportEntry::Valid(importable) = entry else {
            panic!("expected valid entry");
        };
        assert_eq!(importable.name, "ATM9");
        assert_eq!(importable.icon_path, Some(inst.join("instance.png")));
        assert_eq!(importable.minecraft_version, "1.20.1");

        let entry = importer.scan_instance(broken).await.unwrap().unwrap();
        assert!(matches!(entry, InternalImportEntry::Invalid(_)));

        assert!(
            importer
                .scan_instance(launcher.join("instances"))
                .await
                .unwrap()
                .is_none()
        );
    }
}
