use super::{
    GET_IMPORT_SCAN_STATUS, ImportScanStatus, ImportableInstance, ImporterState, InstanceImporter,
    InternalImportEntry, InvalidImportEntry,
    common::{ImportedSettings, apply_imported_settings, dedup_paths, read_json_value},
};
use crate::api::translation::Translation;
use crate::domain::instance::info::{CurseforgeModpack, GameVersion, Modpack};
use crate::managers::AppInner;
use crate::managers::instance::InstanceVersionSource;
use crate::managers::modplatforms::curseforge::convert_cf_version_to_standard_version;
use crate::{domain::vtask::VisualTaskId, managers::instance::anyhow};
use carbon_platforms::curseforge::manifest::{Manifest, Minecraft, ModLoaders};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, trace};

/// The app writes a relocated modding folder into its own `settings.json`, but it
/// is closed source and the key is undocumented, so every absolute path in that
/// file is tried and only one that really holds an `Instances` directory is
/// accepted. A wrong guess therefore costs nothing and falls back to the default.
async fn instances_dir_from_settings() -> Option<PathBuf> {
    let basedirs = directories::BaseDirs::new()?;
    let settings_files = dedup_paths(vec![
        basedirs.data_dir().join("CurseForge").join("settings.json"),
        basedirs
            .data_local_dir()
            .join("CurseForge")
            .join("settings.json"),
    ]);

    for file in settings_files {
        let Some(json) = read_json_value(&file).await else {
            continue;
        };
        if let Some(dir) = find_instances_dir(&json) {
            return Some(dir);
        }
    }

    None
}

/// `<modding folder>/Instances` is the layout the app uses, so both the folder
/// itself and its parent are worth recognising.
fn find_instances_dir(value: &serde_json::Value) -> Option<PathBuf> {
    match value {
        serde_json::Value::String(s) => {
            let path = PathBuf::from(s.trim());
            if !path.is_absolute() {
                return None;
            }
            if path.file_name().is_some_and(|n| n == "Instances") && path.is_dir() {
                return Some(path);
            }
            let nested = path.join("minecraft").join("Instances");
            if nested.is_dir() {
                return Some(nested);
            }
            let direct = path.join("Instances");
            direct.is_dir().then_some(direct)
        }
        serde_json::Value::Array(items) => items.iter().find_map(find_instances_dir),
        serde_json::Value::Object(map) => map.values().find_map(find_instances_dir),
        _ => None,
    }
}

#[derive(Debug, Clone)]
struct Importable {
    filename: String,
    path: PathBuf,
    manifest: Option<Manifest>,
    meta: CfMetadata,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfMetadata {
    name: String,
    #[serde(rename = "projectID")]
    project_id: u32,
    #[serde(rename = "fileID")]
    file_id: u32,
    installed_modpack: Option<InstalledModpack>,
    base_mod_loader: Option<BaseModLoader>,
    game_version: String,
    last_played: Option<String>,
    is_unlocked: bool,
    profile_image_path: Option<String>,
    /// Memory in MB, meaningful when `isMemoryOverride` is set.
    #[serde(default)]
    allocated_memory: Option<u32>,
    #[serde(default)]
    is_memory_override: Option<bool>,
    #[serde(default)]
    java_args_override: Option<String>,
}

impl CfMetadata {
    fn settings(&self) -> ImportedSettings {
        let mut settings = ImportedSettings::default();
        if self.is_memory_override == Some(true) {
            settings.memory_max_mb = self.allocated_memory.filter(|m| *m > 0);
        }
        if let Some(args) = self.java_args_override.as_deref() {
            settings.absorb_jvm_args(args);
        }
        settings
    }
}

#[derive(Debug, Clone, Deserialize)]
struct InstalledModpack {
    #[serde(rename = "thumbnailUrl")]
    thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct BaseModLoader {
    name: String,
}

impl From<Importable> for ImportableInstance {
    fn from(value: Importable) -> Self {
        Self {
            filename: value.filename,
            instance_name: value.meta.name,
        }
    }
}

#[derive(Debug)]
pub struct CurseforgeImporter {
    state: RwLock<ImporterState<Importable>>,
}

impl CurseforgeImporter {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(ImporterState::NoResults),
        }
    }

    pub async fn get_default_scan_path() -> anyhow::Result<PathBuf> {
        if let Some(relocated) = instances_dir_from_settings().await {
            return Ok(relocated);
        }

        let userdirs = directories::UserDirs::new().ok_or(anyhow!("Cannot determine user directories. Please manually select your CurseForge instances folder."))?;

        #[cfg(not(target_os = "windows"))]
        let p = userdirs
            .document_dir()
            .ok_or(anyhow!("Cannot find Documents folder. This is common on WSL or minimal Linux installations. Please manually select your CurseForge instances folder (usually ~/Documents/curseforge/minecraft/Instances)."))?;
        #[cfg(target_os = "windows")]
        let p = userdirs.home_dir();

        Ok(p.join("curseforge").join("minecraft").join("Instances"))
    }

    async fn scan_instance(
        &self,
        path: PathBuf,
    ) -> anyhow::Result<Option<InternalImportEntry<Importable>>> {
        let config: PathBuf = path.join("minecraftinstance.json");
        if !config.is_file() {
            return Ok(None);
        }

        let manifest_path: PathBuf = path.join("manifest.json");

        let config = tokio::fs::read_to_string(config).await?;
        let config = serde_json::from_str::<CfMetadata>(&config);

        let filename = path
            .file_name()
            .expect("filename cannot be empty")
            .to_string_lossy()
            .to_string();

        let manifest = tokio::fs::read_to_string(manifest_path)
            .await
            .ok()
            .map(|text| serde_json::from_str::<Manifest>(&text).ok())
            .flatten();

        let merged = config.map(|config| Importable {
            filename: filename.clone(),
            path,
            manifest,
            meta: config,
        });

        match merged {
            Ok(i) => Ok(Some(InternalImportEntry::Valid(i))),
            Err(_) => Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                name: filename,
                reason: Translation::InstanceImportLegacyBadConfigFile,
            }))),
        }
    }
}

#[async_trait::async_trait]
impl InstanceImporter for CurseforgeImporter {
    async fn scan(&self, app: &Arc<AppInner>, scan_path: PathBuf) -> anyhow::Result<()> {
        if scan_path.is_dir() {
            let Ok(mut dir) = tokio::fs::read_dir(&scan_path).await else {
                return Ok(());
            };

            while let Some(path) = dir.next_entry().await? {
                if path.metadata().await?.is_dir() {
                    if let Ok(Some(entry)) = self.scan_instance(path.path()).await {
                        self.state.write().await.push_multi(entry).await;
                        app.invalidate(GET_IMPORT_SCAN_STATUS, None);
                    }
                }
            }
        } else if let Ok(Some(entry)) = self.scan_instance(scan_path).await {
            self.state.write().await.set_single(entry).await;
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
        trace!(?index, ?name, "Beginning curseforge import");

        let instance = self
            .state
            .read()
            .await
            .get(index)
            .await
            .cloned()
            .ok_or_else(|| anyhow!("invalid importable instance index {index}"))?;

        let icon = match &instance.meta {
            CfMetadata {
                profile_image_path: Some(image),
                ..
            } => app
                .instance_manager()
                .load_icon(PathBuf::from(image.clone()))
                .await
                .ok(),
            CfMetadata {
                installed_modpack: Some(installed_modpack),
                ..
            } => {
                if let Some(thumbnail_url) = installed_modpack.thumbnail_url.as_ref() {
                    app.instance_manager()
                        .download_icon(thumbnail_url.to_string())
                        .await
                        .ok()
                } else {
                    None
                }
            }
            _ => None,
        };

        let initializer = |instance_path: PathBuf| {
            let instance = &instance;
            async move {
                let path = instance_path.join("instance");

                trace!("Copying files from legacy instance");
                // create copy-filter function in file utils for all importers
                carbon_rt_path::copy_dir_filter(&instance.path, &path, |path| {
                    match path.to_str() {
                        Some("minecraftinstance.json" | "manifest.json" | "modelist.html") => false,
                        Some(p) if p.starts_with("profileImage") => false,
                        _ => true,
                    }
                })
                .await?;

                tokio::fs::create_dir_all(instance_path.join(".setup").join("modpack-complete"))
                    .await?;

                Ok(())
            }
        };

        let dummy_string = daedalus::BRANDING
            .get_or_init(daedalus::Branding::default)
            .dummy_replace_string
            .clone();

        let standard_version = convert_cf_version_to_standard_version(
            app.clone(),
            Minecraft {
                version: instance.meta.game_version.clone(),
                mod_loaders: instance
                    .meta
                    .base_mod_loader
                    .as_ref()
                    .map(|loader| ModLoaders {
                        id: {
                            let mut name = &loader.name as &str;

                            if name.starts_with("fabric") || name.starts_with("quilt") {
                                if let Some((name2, _)) = name.rsplit_once('-') {
                                    name = name2;
                                }
                            }

                            name.to_string()
                        },
                        primary: true,
                    })
                    .into_iter()
                    .collect(),
            },
            dummy_string,
        )
        .await?;

        let version = GameVersion::Standard(standard_version);

        let instance_version_source = match &instance.manifest {
            Some(manifest) => InstanceVersionSource::ModpackWithKnownVersion(
                version,
                Modpack::Curseforge(CurseforgeModpack {
                    project_id: instance.meta.project_id,
                    file_id: instance.meta.file_id,
                }),
                instance.meta.is_unlocked,
            ),
            None => InstanceVersionSource::Version(version),
        };

        let id = app
            .instance_manager()
            .create_instance_ext(
                app.instance_manager().get_default_group().await?,
                name.unwrap_or_else(|| instance.meta.name.clone()),
                icon,
                None,
                None,
                instance_version_source,
                String::new(),
                initializer,
            )
            .await?;

        apply_imported_settings(app, id, &instance.meta.settings()).await;

        app.instance_manager()
            .prepare_game(id, None, None, false)
            .await
            .map(|r| r.1)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn instance_overrides_are_read_from_minecraftinstance_json() {
        let meta: CfMetadata = serde_json::from_value(serde_json::json!({
            "name": "ATM9",
            "projectID": 715572,
            "fileID": 5142600,
            "gameVersion": "1.20.1",
            "isUnlocked": false,
            "allocatedMemory": 8192,
            "isMemoryOverride": true,
            "javaArgsOverride": "-XX:+UseG1GC -Xms2G"
        }))
        .unwrap();
        let settings = meta.settings();
        assert_eq!(settings.memory(), Some((2048, 8192)));
        assert_eq!(settings.extra_java_args.as_deref(), Some("-XX:+UseG1GC"));

        // Without the override flag the memory value is the app's global, not the instance's.
        let meta: CfMetadata = serde_json::from_value(serde_json::json!({
            "name": "Old",
            "projectID": 1,
            "fileID": 2,
            "gameVersion": "1.12.2",
            "isUnlocked": true,
            "allocatedMemory": 4096,
            "isMemoryOverride": false,
            "javaArgsOverride": null
        }))
        .unwrap();
        assert_eq!(meta.settings(), ImportedSettings::default());

        let meta: CfMetadata = serde_json::from_value(serde_json::json!({
            "name": "Older",
            "projectID": 1,
            "fileID": 2,
            "gameVersion": "1.7.10",
            "isUnlocked": true
        }))
        .unwrap();
        assert_eq!(meta.settings(), ImportedSettings::default());
    }

    #[test]
    fn relocated_modding_folder_is_recognised_only_when_it_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let modding = tmp.path().join("D_drive").join("curseforge");
        std::fs::create_dir_all(modding.join("minecraft").join("Instances")).unwrap();

        // The app records the modding folder; instances sit under minecraft/Instances.
        let json =
            serde_json::json!({ "minecraft": { "moddingFolder": modding.to_string_lossy() } });
        assert_eq!(
            find_instances_dir(&json),
            Some(modding.join("minecraft").join("Instances"))
        );

        // The instances folder named outright is accepted too.
        let direct = serde_json::json!([modding
            .join("minecraft")
            .join("Instances")
            .to_string_lossy()]);
        assert_eq!(
            find_instances_dir(&direct),
            Some(modding.join("minecraft").join("Instances"))
        );

        // A path that does not exist is ignored, so a wrong guess costs nothing.
        let missing = serde_json::json!({ "a": "/no/such/place", "b": "relative/path" });
        assert_eq!(find_instances_dir(&missing), None);
    }
}
