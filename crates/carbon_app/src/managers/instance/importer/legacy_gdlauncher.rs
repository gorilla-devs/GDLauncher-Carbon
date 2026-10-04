use super::{
    ImportScanStatus, ImportableInstance, ImporterState, InstanceImporter, InternalImportEntry,
    InvalidImportEntry,
    common::{ImportedSettings, apply_imported_settings, dir_name},
};
use crate::{
    api::keys::instance::*,
    api::translation::Translation,
    domain::{
        instance::info::{
            CurseforgeModpack, GameVersion, ModLoader, ModLoaderType, Modpack, StandardVersion,
        },
        vtask::VisualTaskId,
    },
    managers::{AppInner, instance::InstanceVersionSource},
};
use anyhow::anyhow;
use chrono::{DateTime, NaiveDateTime, Utc};
use std::{collections::HashSet, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use tracing::{trace, warn};

#[derive(Debug, Clone)]
struct Importable {
    filename: String,
    path: PathBuf,
    config: LegacyGDLauncherConfig,
    settings: ImportedSettings,
}

impl From<Importable> for ImportableInstance {
    fn from(value: Importable) -> Self {
        let warnings = value.settings.warnings();
        Self {
            filename: value.filename.clone(),
            instance_name: value.filename,
            warnings,
        }
    }
}

#[derive(Debug)]
pub struct LegacyGDLauncherImporter {
    state: RwLock<ImporterState<Importable>>,
}

impl LegacyGDLauncherImporter {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(ImporterState::NoResults),
        }
    }

    pub async fn get_default_scan_path() -> anyhow::Result<PathBuf> {
        let basedirs = directories::BaseDirs::new().ok_or(anyhow!("Cannot build basedirs"))?;

        // old gdl did not respect the xdg basedirs spec on linux
        #[cfg(target_os = "linux")]
        let p = basedirs.config_dir();
        #[cfg(not(target_os = "linux"))]
        let p = basedirs.data_dir();

        let mut p = p.join("gdlauncher_next");

        let override_path = p.join("override.data");
        if override_path.exists() {
            let override_path = tokio::fs::read_to_string(override_path).await;
            if let Ok(override_path) = override_path {
                let override_path = PathBuf::from(override_path);
                if override_path.is_dir() {
                    p = override_path;
                }
            }
        }

        Ok(p.join("instances"))
    }

    async fn scan_instance(
        &self,
        path: PathBuf,
    ) -> anyhow::Result<Option<InternalImportEntry<Importable>>> {
        let config_path = path.join("config.json");
        if !config_path.is_file() {
            return Ok(None);
        }

        // A drive root has no folder name; it is named by its path instead.
        let filename = dir_name(&path);
        // One unreadable config (corrupt text, permissions, an offline cloud
        // file) gets its own row instead of hiding the instances after it.
        let config = match tokio::fs::read_to_string(&config_path).await {
            Ok(config) => serde_json::from_str::<LegacyGDLauncherConfig>(&config),
            Err(e) => {
                warn!("Cannot read {config_path:?}: {e}");
                return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                    name: filename,
                    reason: Translation::InstanceImportConfigUnreadable,
                })));
            }
        };

        match config {
            Ok(config) => {
                let mut settings = config.settings();
                settings.skip_missing_java_path();
                Ok(Some(InternalImportEntry::Valid(Importable {
                    filename,
                    path,
                    config,
                    settings,
                })))
            }
            Err(_) => Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                name: filename,
                reason: Translation::InstanceImportLegacyBadConfigFile,
            }))),
        }
    }
}

#[async_trait::async_trait]
impl InstanceImporter for LegacyGDLauncherImporter {
    async fn scan(&self, app: &Arc<AppInner>, scan_path: PathBuf) -> anyhow::Result<()> {
        self.scan_path(scan_path, &|| app.invalidate(GET_IMPORT_SCAN_STATUS, None))
            .await
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
        trace!(?index, ?name, "Beginning legacy gdl import");

        let instance = self
            .state
            .read()
            .await
            .get(index)
            .await
            .cloned()
            .ok_or_else(|| anyhow!("invalid importable instance index"))?;

        let instance_version_source = 'version_source: {
            let modloader = match &*instance.config.loader.loader_type {
                "forge" => Some(ModLoaderType::Forge),
                "fabric" => Some(ModLoaderType::Fabric),
                _ => None,
            }
            .and_then(|loader_type| {
                let Some(ref loader_version) = instance.config.loader.loader_version else {
                    return None;
                };

                Some(ModLoader {
                    type_: loader_type,
                    version: loader_version.to_owned(),
                })
            })
            .map(|modloader| HashSet::from_iter(vec![modloader]))
            .unwrap_or_default();

            let standard_version = GameVersion::Standard(StandardVersion {
                release: instance.config.loader.mc_version.clone(),
                modloaders: modloader,
            });

            'cf: {
                if let Some(ref source) = instance.config.loader.source {
                    if source != "curseforge" {
                        break 'cf;
                    }

                    let Some(project_id) = instance.config.loader.project_id else {
                        trace!(
                            "Legacy gdl import specifies curseforge source but is missing project id, ignoring"
                        );
                        break 'cf;
                    };
                    let Some(file_id) = instance.config.loader.file_id else {
                        trace!(
                            "Legacy gdl import specifies curseforge source but is missing file id, ignoring"
                        );
                        break 'cf;
                    };

                    let curseforge_modpack = CurseforgeModpack {
                        project_id: project_id as u32,
                        file_id: file_id as u32,
                    };

                    break 'version_source InstanceVersionSource::ModpackWithKnownVersion(
                        standard_version,
                        Modpack::Curseforge(curseforge_modpack),
                        true,
                    );
                }
            }

            InstanceVersionSource::Version(standard_version)
        };

        let icon = match &instance.config.background {
            Some(background) => app
                .instance_manager()
                .load_icon(instance.path.join(background))
                .await
                .ok(),
            None => None,
        };

        let seconds_played = instance.config.time_played.map(|x| x * 60);
        let last_played = instance.config.last_played.and_then(|x| {
            let Some(naive_date_time) = NaiveDateTime::from_timestamp_millis(x as i64) else {
                return None;
            };

            Some(DateTime::<Utc>::from_utc(naive_date_time, Utc))
        });

        let initializer = |instance_path: PathBuf| {
            let instance = &instance;
            async move {
                let path = instance_path.join("instance");

                trace!("Copying files from legacy instance");
                // create copy-filter function in file utils for all importers
                carbon_rt_path::copy_dir_filter(&instance.path, &path, |path| {
                    match path.to_str() {
                        Some("config.json" | "manifest.json" | "installing.lock" | "natives") => {
                            false
                        }
                        Some(p)
                            if Some(p)
                                == instance.config.background.as_ref().map(|x| x.as_str()) =>
                        {
                            false
                        }
                        _ => true,
                    }
                })
                .await?;

                tokio::fs::create_dir_all(instance_path.join(".setup").join("modpack-complete"))
                    .await?;

                Ok(())
            }
        };

        let settings = &instance.settings;

        let id = app
            .instance_manager()
            .create_instance_ext(
                app.instance_manager().get_default_group().await?,
                name.unwrap_or_else(|| instance.filename.clone()),
                icon,
                seconds_played,
                last_played,
                instance_version_source,
                settings.notes.clone().unwrap_or_default(),
                initializer,
            )
            .await?;

        apply_imported_settings(app, id, settings).await;

        app.instance_manager()
            .prepare_game(id, None, None, true)
            .await
            .map(|r| r.1)
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyGDLauncherConfig {
    loader: _Loader,
    time_played: Option<u32>,
    last_played: Option<u64>,
    background: Option<String>,
    #[serde(default)]
    java_memory: Option<u32>,
    /// A string in practice; tolerated as an array too.
    #[serde(default)]
    java_args: Option<serde_json::Value>,
    #[serde(default)]
    custom_java_path: Option<String>,
    #[serde(default)]
    resolution: Option<LegacyResolution>,
    #[serde(default)]
    notes: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct LegacyResolution {
    width: Option<u32>,
    height: Option<u32>,
}

impl LegacyGDLauncherConfig {
    fn settings(&self) -> ImportedSettings {
        let mut settings = ImportedSettings {
            notes: self
                .notes
                .as_deref()
                .map(str::trim)
                .filter(|n| !n.is_empty())
                .map(str::to_string),
            memory_max_mb: self.java_memory.filter(|m| *m > 0),
            java_path: self
                .custom_java_path
                .as_deref()
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(str::to_string),
            game_resolution: self
                .resolution
                .as_ref()
                .and_then(|r| Some((r.width.filter(|w| *w > 0)?, r.height.filter(|h| *h > 0)?))),
            ..Default::default()
        };

        let args = match &self.java_args {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Array(items)) => items
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            _ => String::new(),
        };
        if !args.trim().is_empty() {
            settings.absorb_jvm_args(&args);
        }

        settings
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct _Loader {
    loader_type: String,
    loader_version: Option<String>,
    mc_version: String,
    #[serde(rename = "fileID")]
    file_id: Option<i32>,
    #[serde(rename = "projectID")]
    project_id: Option<i32>,
    source: Option<String>,
    source_name: Option<String>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn instance_settings_are_read_from_config_json() {
        let config: LegacyGDLauncherConfig = serde_json::from_value(serde_json::json!({
            "loader": { "loaderType": "forge", "loaderVersion": "47.2.0", "mcVersion": "1.20.1" },
            "timePlayed": 5,
            "javaMemory": 6144,
            "javaArgs": "-XX:+UseG1GC -Xms1G",
            "customJavaPath": "/opt/jdk/bin/java",
            "resolution": { "width": 1600, "height": 900 },
            "notes": "hello"
        }))
        .unwrap();
        let settings = config.settings();
        assert_eq!(settings.notes.as_deref(), Some("hello"));
        assert_eq!(settings.memory(), Some((1024, 6144)));
        assert_eq!(settings.extra_java_args.as_deref(), Some("-XX:+UseG1GC"));
        assert_eq!(settings.java_path.as_deref(), Some("/opt/jdk/bin/java"));
        assert_eq!(settings.game_resolution, Some((1600, 900)));

        let config: LegacyGDLauncherConfig = serde_json::from_value(serde_json::json!({
            "loader": { "loaderType": "vanilla", "mcVersion": "1.16.5" },
            "javaArgs": ["-XX:+UseZGC"]
        }))
        .unwrap();
        let settings = config.settings();
        assert_eq!(settings.extra_java_args.as_deref(), Some("-XX:+UseZGC"));
        assert_eq!(settings.memory(), None);
        assert_eq!(settings.notes, None);
    }
}

impl LegacyGDLauncherImporter {
    async fn scan_path(
        &self,
        scan_path: PathBuf,
        notify: &(dyn Fn() + Send + Sync),
    ) -> anyhow::Result<()> {
        let scan_path = if scan_path
            .file_name()
            .is_some_and(|name| name == "config.json")
            && scan_path.is_file()
        {
            scan_path
                .parent()
                .ok_or_else(|| anyhow!("Selected config has no parent"))?
                .to_path_buf()
        } else {
            scan_path
        };
        if scan_path.join("config.json").is_file() {
            if let Some(entry) = self.scan_instance(scan_path).await? {
                self.state.write().await.set_single(entry).await;
                notify();
            }
            return Ok(());
        }
        if scan_path.is_dir() {
            let mut dir = super::common::scan_directory(&scan_path).await?;

            while let Some(path) = dir.next_entry().await? {
                match path.metadata().await {
                    Ok(metadata) if metadata.is_dir() => {}
                    Ok(_) => continue,
                    Err(e) => {
                        warn!("Cannot inspect {:?}, skipping it: {e}", path.path());
                        continue;
                    }
                }
                if let Some(entry) = self.scan_instance(path.path()).await? {
                    self.state.write().await.push_multi(entry).await;
                    notify();
                }
            }
        } else if let Some(entry) = self.scan_instance(scan_path).await? {
            self.state.write().await.set_single(entry).await;
            notify();
        }

        Ok(()) // TODO: invalidate on iter
    }
}

#[cfg(test)]
mod task13_scan_tests {
    use super::*;
    #[tokio::test]
    async fn task13_single_root_and_config_selection() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("Synthetic");
        std::fs::create_dir(&root).unwrap();
        // Malformed recognized metadata remains a named Invalid row, not absence.
        let config = root.join("config.json");
        std::fs::write(&config, b"invalid synthetic metadata").unwrap();
        for path in [root.clone(), config] {
            let scanner = LegacyGDLauncherImporter::new();
            scanner.scan_path(path, &|| {}).await.unwrap();
            assert!(matches!(
                scanner.get_status().await,
                ImportScanStatus::SingleResult(super::super::ImportEntry::Invalid(_))
            ));
        }
        let scanner = LegacyGDLauncherImporter::new();
        scanner
            .scan_path(t.path().to_path_buf(), &|| {})
            .await
            .unwrap();
        assert!(matches!(
            scanner.get_status().await,
            ImportScanStatus::MultiResult(_)
        ));
    }
}

#[cfg(test)]
mod task13_valid_controls {
    use super::*;
    #[tokio::test]
    async fn task13_valid_single_config_collection_and_empty() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("Synthetic");
        std::fs::create_dir(&root).unwrap();
        let config = root.join("config.json");
        std::fs::write(
            &config,
            r#"{"loader":{"loaderType":"vanilla","mcVersion":"1.20.1"}}"#,
        )
        .unwrap();
        for path in [root, config] {
            let scanner = LegacyGDLauncherImporter::new();
            scanner.scan_path(path, &|| {}).await.unwrap();
            assert!(matches!(
                scanner.get_status().await,
                ImportScanStatus::SingleResult(super::super::ImportEntry::Valid(_))
            ));
        }
        let scanner = LegacyGDLauncherImporter::new();
        scanner
            .scan_path(t.path().to_path_buf(), &|| {})
            .await
            .unwrap();
        let ImportScanStatus::MultiResult(entries) = scanner.get_status().await else {
            panic!("missing collection")
        };
        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0], super::super::ImportEntry::Valid(_)));
        let empty = tempfile::tempdir().unwrap();
        let scanner = LegacyGDLauncherImporter::new();
        scanner
            .scan_path(empty.path().to_path_buf(), &|| {})
            .await
            .unwrap();
        assert!(matches!(
            scanner.get_status().await,
            ImportScanStatus::NoResults
        ));
    }
}

#[cfg(test)]
mod entry_failure_tests {
    use super::super::ImportEntry;
    use super::*;

    /// One config that cannot be read (here: not UTF-8) used to stop the scan
    /// and hide every instance after it; now it is its own row.
    #[tokio::test]
    async fn an_unreadable_config_gets_its_own_row_and_the_scan_goes_on() {
        let t = tempfile::tempdir().unwrap();
        for name in ["Alpha", "Broken", "Zulu"] {
            std::fs::create_dir(t.path().join(name)).unwrap();
            std::fs::write(
                t.path().join(name).join("config.json"),
                r#"{"loader":{"loaderType":"vanilla","mcVersion":"1.20.1"}}"#,
            )
            .unwrap();
        }
        std::fs::write(t.path().join("Broken/config.json"), [0xff, 0xfe, 0x00]).unwrap();
        let scanner = LegacyGDLauncherImporter::new();
        scanner
            .scan_path(t.path().to_path_buf(), &|| {})
            .await
            .unwrap();
        let ImportScanStatus::MultiResult(entries) = scanner.get_status().await else {
            panic!("missing collection")
        };
        let mut valid: Vec<_> = entries
            .iter()
            .filter_map(|e| match e {
                ImportEntry::Valid(v) => Some(v.filename.clone()),
                ImportEntry::Invalid(_) => None,
            })
            .collect();
        valid.sort();
        assert_eq!(valid, ["Alpha", "Zulu"]);
        assert!(entries.iter().any(|e| matches!(
            e,
            ImportEntry::Invalid(InvalidImportEntry {
                name,
                reason: Translation::InstanceImportConfigUnreadable,
            }) if name == "Broken"
        )));
    }

    /// A folder without a name of its own (a drive root such as `E:\`) used to
    /// panic the scanner and break every importer until GDLauncher restarted.
    #[tokio::test]
    async fn a_folder_without_a_name_does_not_panic() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir(t.path().join("sub")).unwrap();
        std::fs::write(
            t.path().join("config.json"),
            r#"{"loader":{"loaderType":"vanilla","mcVersion":"1.20.1"}}"#,
        )
        .unwrap();
        let unnamed = t.path().join("sub").join("..");
        assert!(unnamed.file_name().is_none());
        let entry = LegacyGDLauncherImporter::new()
            .scan_instance(unnamed.clone())
            .await
            .unwrap();
        let Some(InternalImportEntry::Valid(entry)) = entry else {
            panic!("expected an importable instance")
        };
        assert_eq!(entry.filename, unnamed.to_string_lossy());
    }
}

#[cfg(test)]
mod stale_java_tests {
    use super::*;

    #[tokio::test]
    async fn a_java_path_that_is_gone_is_left_out() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(
            t.path().join("config.json"),
            serde_json::json!({
                "loader": { "loaderType": "vanilla", "mcVersion": "1.20.1" },
                "customJavaPath": "/gdl-no-such-folder/bin/java"
            })
            .to_string(),
        )
        .unwrap();
        let Some(InternalImportEntry::Valid(entry)) = LegacyGDLauncherImporter::new()
            .scan_instance(t.path().to_path_buf())
            .await
            .unwrap()
        else {
            panic!("an instance with a stale Java path was refused")
        };
        assert_eq!(entry.settings.java_path, None);
        assert_eq!(
            ImportableInstance::from(entry).warnings,
            [Translation::InstanceImportJavaPathSkipped]
        );
    }
}
