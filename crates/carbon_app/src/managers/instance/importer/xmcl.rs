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
    InternalImportEntry, InvalidImportEntry,
    common::{
        DeclaredModLoader, ImportedSettings, apply_imported_settings, copy_game_dir,
        datetime_from_epoch_secs_or_millis, dedup_paths, dir_name, find_image_with_any_extension,
        first_existing_dir, group_for_import, icon_from_data_url, json_i64, json_positive_u32,
        json_string, json_trimmed_string, normalize_forge_version, parse_version_id,
        read_json_value, resolve_standard_version, seconds_played_from_i64,
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

const MEDIA_URL_PREFIX: &str = "http://launcher/media?path=";

#[derive(Debug, Clone, PartialEq, Eq)]
enum IconSource {
    File(PathBuf),
    Url(String),
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
        let warnings = value.settings.warnings();
        Self {
            filename: value.filename,
            instance_name: value.name,
            warnings,
        }
    }
}

#[derive(Debug)]
pub struct XmclImporter {
    state: RwLock<ImporterState<Importable>>,
}

impl XmclImporter {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(ImporterState::NoResults),
        }
    }

    pub async fn get_default_scan_path() -> anyhow::Result<Option<PathBuf>> {
        default_scan_path_from_candidates(app_data_candidates()).await
    }

    async fn scan_instance(
        &self,
        path: PathBuf,
        group: Option<String>,
    ) -> anyhow::Result<Option<InternalImportEntry<Importable>>> {
        let json_path = path.join("instance.json");
        if !json_path.is_file() {
            return Ok(None);
        }

        let filename = dir_name(&path);

        let Some(json) = read_json_value(&json_path).await else {
            warn!("Cannot read XMCL instance metadata at {json_path:?}");
            return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                name: filename,
                reason: Translation::InstanceImportLegacyBadConfigFile,
            })));
        };

        if !is_xmcl_instance_json(&json) {
            return Ok(None);
        }

        let Some(mut parsed) = parse_instance_json(&json, &filename) else {
            warn!("XMCL instance at {path:?} has no Minecraft version");
            return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                name: filename,
                reason: Translation::InstanceImportLegacyBadConfigFile,
            })));
        };

        parsed.settings.skip_missing_java_path();
        let icon = resolve_icon(&path, parsed.icon.as_deref());
        parsed.settings.group = group;

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

#[async_trait::async_trait]
impl InstanceImporter for XmclImporter {
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
            let mut group = None;
            if let Some(instances_dir) = scan_path.parent() {
                if let Some(app_data) = instances_dir.parent() {
                    group = instance_groups(app_data, instances_dir)
                        .await
                        .remove(&scan_path);
                }
            }
            if let Some(entry) = self.scan_instance(scan_path, group).await? {
                self.state.write().await.set_single(entry).await;
                app.invalidate(GET_IMPORT_SCAN_STATUS, None);
            }
            return Ok(());
        }

        let (instance_dirs, mut groups) = collect_instances(&scan_path).await?;

        for path in instance_dirs {
            let group = groups.remove(&path);
            if let Some(entry) = self.scan_instance(path, group).await? {
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
        trace!(?index, ?name, "Beginning XMCL import");

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
            Some(IconSource::Url(url)) => app.instance_manager().try_download_icon(url).await,
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

async fn default_scan_path_from_candidates(
    candidates: Vec<PathBuf>,
) -> anyhow::Result<Option<PathBuf>> {
    let Some(app_data) = first_existing_dir(candidates) else {
        return Ok(None);
    };
    Ok(Some(app_data))
}

async fn collect_instances(
    scan_path: &Path,
) -> anyhow::Result<(Vec<PathBuf>, HashMap<PathBuf, String>)> {
    let mut instance_dirs: Vec<PathBuf> = Vec::new();

    let (instances_dir, app_data) = if is_app_data_dir(&scan_path) {
        let root = game_root_for_app_data(&scan_path).await;
        (root.join("instances"), Some(scan_path.to_path_buf()))
    } else if scan_path.join("instances").is_dir() {
        (scan_path.join("instances"), Some(scan_path.to_path_buf()))
    } else {
        (
            scan_path.to_path_buf(),
            scan_path.parent().map(Path::to_path_buf),
        )
    };

    let directory = match super::common::scan_directory(&instances_dir).await {
        Ok(dir) => Some(dir),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && app_data.is_some() => None,
        Err(e) => return Err(e.into()),
    };
    if let Some(mut dir) = directory {
        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.is_dir() && !dir_name(&path).starts_with('.') {
                instance_dirs.push(path);
            }
        }
    }

    let mut groups = HashMap::new();
    if let Some(app_data) = app_data {
        for external in external_instances(&app_data, &instances_dir).await {
            if !instance_dirs.contains(&external) {
                instance_dirs.push(external);
            }
        }
        groups = instance_groups(&app_data, &instances_dir).await;
    }

    Ok((instance_dirs, groups))
}

/// Entries are a bare path (ungrouped) or `{"name", "instances": [paths]}`; paths may be relative to the instances directory.
async fn instance_groups(app_data: &Path, instances_dir: &Path) -> HashMap<PathBuf, String> {
    let mut groups = HashMap::new();

    let mut candidates = vec![app_data.join("instances.json")];
    if let Some(root) = instances_dir.parent() {
        candidates.push(root.join("instances.json"));
    }

    for file in candidates {
        let Some(json) = read_json_value(&file).await else {
            continue;
        };
        let Some(list) = json.get("groups").and_then(|g| g.as_array()) else {
            continue;
        };

        for entry in list {
            let Some(name) = json_trimmed_string(entry, "name") else {
                continue;
            };
            let Some(members) = entry.get("instances").and_then(|i| i.as_array()) else {
                continue;
            };
            for member in members.iter().filter_map(|m| m.as_str()) {
                let path = PathBuf::from(member.trim());
                let path = if path.is_absolute() {
                    path
                } else {
                    instances_dir.join(path)
                };
                groups.insert(path, name.clone());
            }
        }

        break;
    }

    groups
}

fn is_launcher_file(relative: &Path) -> bool {
    let Some(first) = relative.components().next() else {
        return false;
    };
    let first = first.as_os_str().to_string_lossy();
    first == "instance.json"
        || first == "natives"
        || first == "logs"
        || first == "crash-reports"
        || first == ".xmcl"
}

/// ATLauncher and FTB also use `instance.json`; XMCL's has a `runtime` block (or a bare `version` in old ones).
fn is_xmcl_instance_json(json: &serde_json::Value) -> bool {
    json.get("runtime").is_some_and(|r| r.is_object())
        || (json.get("version").is_some()
            && json.get("launcher").is_none()
            && json.get("uuid").is_none()
            && json.get("mcVersion").is_none())
}

fn is_app_data_dir(path: &Path) -> bool {
    path.join("root").is_file() || path.join("instances.json").is_file()
}

/// The `root` file of the app data directory, or the app data directory itself.
async fn game_root_for_app_data(app_data: &Path) -> PathBuf {
    if let Ok(text) = tokio::fs::read_to_string(app_data.join("root")).await {
        let root = PathBuf::from(text.trim());
        if root.is_absolute() && root.is_dir() {
            return root;
        }
    }
    app_data.to_path_buf()
}

async fn external_instances(app_data: &Path, instances_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![app_data.join("instances.json")];
    if let Some(root) = instances_dir.parent() {
        candidates.push(root.join("instances.json"));
    }

    for file in candidates {
        let Some(json) = read_json_value(&file).await else {
            continue;
        };
        let Some(list) = json.get("instances").and_then(|i| i.as_array()) else {
            continue;
        };
        return list
            .iter()
            .filter_map(|v| v.as_str())
            .map(|p| {
                let p = PathBuf::from(p.trim());
                if p.is_absolute() {
                    p
                } else {
                    instances_dir.join(p)
                }
            })
            .filter(|p| p.is_dir())
            .collect();
    }

    Vec::new()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedInstance {
    name: String,
    minecraft_version: String,
    loaders: Vec<DeclaredModLoader>,
    modpack: Option<Modpack>,
    icon: Option<String>,
    seconds_played: Option<u32>,
    last_played: Option<DateTime<Utc>>,
    settings: ImportedSettings,
}

fn instance_settings(json: &serde_json::Value) -> ImportedSettings {
    let mut settings = ImportedSettings {
        notes: json_trimmed_string(json, "description"),
        memory_min_mb: json_positive_u32(json, "minMemory"),
        memory_max_mb: json_positive_u32(json, "maxMemory"),
        java_path: json_trimmed_string(json, "java"),
        pre_launch_hook: json_trimmed_string(json, "preExecuteCommand"),
        wrapper_command: json_trimmed_string(json, "prependCommand"),
        game_resolution: json.get("resolution").and_then(|r| {
            Some((
                json_positive_u32(r, "width")?,
                json_positive_u32(r, "height")?,
            ))
        }),
        ..Default::default()
    };

    match json.get("vmOptions") {
        Some(serde_json::Value::Array(items)) => {
            settings.absorb_jvm_argv(items.iter().filter_map(|v| v.as_str()))
        }
        Some(serde_json::Value::String(command)) => settings.absorb_jvm_args(command),
        _ => {}
    }

    settings
}

fn parse_instance_json(json: &serde_json::Value, fallback_name: &str) -> Option<ParsedInstance> {
    let name = json_trimmed_string(json, "name").unwrap_or_else(|| fallback_name.to_string());

    let runtime = json.get("runtime").filter(|r| r.is_object());
    let runtime_str = |key: &str| runtime.and_then(|r| json_trimmed_string(r, key));

    // `version` is the resolved id XMCL launched; a fallback for instances written before `runtime`.
    let from_version_id = json_string(json, "version")
        .map(|v| parse_version_id(&v))
        .unwrap_or_default();

    let minecraft_version = runtime_str("minecraft").or(from_version_id.minecraft)?;

    let mut loaders = Vec::new();
    if let Some(forge) = runtime_str("forge") {
        loaders.push(DeclaredModLoader::new(
            ModLoaderType::Forge,
            normalize_forge_version(&minecraft_version, &forge),
        ));
    }
    if let Some(neoforge) = runtime_str("neoForged").or_else(|| runtime_str("neoforge")) {
        loaders.push(DeclaredModLoader::new(ModLoaderType::Neoforge, neoforge));
    }
    if let Some(fabric) = runtime_str("fabricLoader").or_else(|| runtime_str("fabric")) {
        loaders.push(DeclaredModLoader::new(ModLoaderType::Fabric, fabric));
    }
    if let Some(quilt) = runtime_str("quiltLoader").or_else(|| runtime_str("quilt")) {
        loaders.push(DeclaredModLoader::new(ModLoaderType::Quilt, quilt));
    }
    if loaders.is_empty() {
        loaders = from_version_id.loaders;
    }

    let modpack = json
        .get("upstream")
        .filter(|u| u.is_object())
        .and_then(|upstream| {
            let kind = json_string(upstream, "type").unwrap_or_default();
            match kind.as_str() {
                "curseforge-modpack" => {
                    let project = json_i64(upstream, "modId")?;
                    let file = json_i64(upstream, "fileId")?;
                    Some(Modpack::Curseforge(CurseforgeModpack {
                        project_id: u32::try_from(project).ok()?,
                        file_id: u32::try_from(file).ok()?,
                    }))
                }
                "modrinth-modpack" => {
                    let project_id = json_string(upstream, "projectId")?;
                    let version_id = json_string(upstream, "versionId")?;
                    (!project_id.is_empty() && !version_id.is_empty()).then_some(Modpack::Modrinth(
                        ModrinthModpack {
                            project_id,
                            version_id,
                        },
                    ))
                }
                _ => None,
            }
        });

    let seconds_played = json_i64(json, "playtime")
        .map(|ms| ms / 1000)
        .and_then(seconds_played_from_i64);
    let last_played = json_i64(json, "lastPlayedDate")
        .or_else(|| json_i64(json, "lastAccessDate"))
        .and_then(datetime_from_epoch_secs_or_millis);

    Some(ParsedInstance {
        name,
        minecraft_version,
        loaders,
        modpack,
        icon: json_trimmed_string(json, "icon"),
        seconds_played,
        last_played,
        settings: instance_settings(json),
    })
}

fn resolve_icon(instance_dir: &Path, icon: Option<&str>) -> Option<IconSource> {
    if let Some(icon) = icon {
        if let Some(raw) = icon.strip_prefix(MEDIA_URL_PREFIX) {
            let path = PathBuf::from(percent_decode(raw));
            if path.is_file() {
                return Some(IconSource::File(path));
            }
        } else if icon.starts_with("data:image/") {
            return Some(IconSource::DataUrl(icon.to_string()));
        } else if icon.starts_with("http://") || icon.starts_with("https://") {
            return Some(IconSource::Url(icon.to_string()));
        } else {
            let path = PathBuf::from(icon);
            let path = if path.is_absolute() {
                path
            } else {
                instance_dir.join(path)
            };
            if path.is_file() {
                return Some(IconSource::File(path));
            }
        }
    }

    find_image_with_any_extension(instance_dir, "icon").map(IconSource::File)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(hex) = s.get(i + 1..i + 3) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

fn app_data_candidates() -> Vec<PathBuf> {
    let Some(basedirs) = directories::BaseDirs::new() else {
        return Vec::new();
    };
    let home = basedirs.home_dir().to_path_buf();

    // XMCL keeps its data in Electron's `appData`: `~/.config/xmcl` on Linux,
    // `%APPDATA%\xmcl` on Windows, `~/Library/Application Support/xmcl` on macOS.
    let mut candidates = vec![
        basedirs.config_dir().join("xmcl"),
        basedirs.data_dir().join("xmcl"),
        home.join(".xmcl"),
        home.join(".var/app/app.xmcl.voxelum/config/xmcl"),
    ];

    // `Packages` only exists on Windows; elsewhere this is one failed read.
    candidates.extend(appx_candidates(&basedirs.data_local_dir().join("Packages")));

    dedup_paths(candidates)
}

/// A Microsoft Store build is sandboxed into a package folder whose name carries
/// a publisher hash that changes between releases, so it has to be matched by
/// prefix rather than named outright.
fn appx_candidates(packages_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(packages_dir) else {
        return Vec::new();
    };

    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("XMCL"))
        })
        .map(|path| path.join("LocalCache").join("Roaming").join("xmcl"))
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn microsoft_store_packages_are_found_by_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let packages = tmp.path().join("Packages");
        std::fs::create_dir_all(packages.join("XMCL_68mcaawk44tpj")).unwrap();
        std::fs::create_dir_all(packages.join("SomethingElse_abc")).unwrap();

        assert_eq!(
            appx_candidates(&packages),
            vec![
                packages
                    .join("XMCL_68mcaawk44tpj")
                    .join("LocalCache")
                    .join("Roaming")
                    .join("xmcl")
            ]
        );
        assert!(appx_candidates(&tmp.path().join("nope")).is_empty());
    }

    #[test]
    fn xmcl_app_data_candidates_are_unique() {
        super::super::common::assert_paths_unique(&app_data_candidates());
    }

    #[test]
    fn launch_settings_are_read() {
        let json = serde_json::json!({
            "runtime": { "minecraft": "1.20.1" },
            "description": "notes",
            "minMemory": 1024,
            "maxMemory": 8192,
            "vmOptions": ["-XX:+UseG1GC", "-Dfoo=bar"],
            "java": "/usr/lib/jvm/java-17/bin/java",
            "prependCommand": "gamemoderun",
            "preExecuteCommand": "echo hi",
            "resolution": { "width": 1600, "height": 900 }
        });
        let s = instance_settings(&json);
        assert_eq!(s.notes.as_deref(), Some("notes"));
        assert_eq!(s.memory(), Some((1024, 8192)));
        assert_eq!(s.extra_java_args.as_deref(), Some("-XX:+UseG1GC -Dfoo=bar"));
        assert_eq!(
            s.java_path.as_deref(),
            Some("/usr/lib/jvm/java-17/bin/java")
        );
        assert_eq!(s.wrapper_command.as_deref(), Some("gamemoderun"));
        assert_eq!(s.pre_launch_hook.as_deref(), Some("echo hi"));
        assert_eq!(s.game_resolution, Some((1600, 900)));
        assert_eq!(
            instance_settings(&serde_json::json!({})),
            ImportedSettings::default()
        );
    }

    #[tokio::test]
    async fn groups_come_from_instances_json() {
        let tmp = tempfile::tempdir().unwrap();
        let app_data = tmp.path().join("xmcl");
        std::fs::create_dir_all(&app_data).unwrap();
        let instances = tmp.path().join("instances");
        std::fs::write(
            app_data.join("instances.json"),
            serde_json::json!({
                "instances": ["A", "B"],
                "groups": [
                    "A",
                    { "name": "Modded", "id": "g1", "color": "", "instances": ["B", "/abs/C"] }
                ]
            })
            .to_string(),
        )
        .unwrap();
        let groups = instance_groups(&app_data, &instances).await;
        assert_eq!(
            groups.get(&instances.join("B")).map(String::as_str),
            Some("Modded")
        );
        assert_eq!(
            groups.get(Path::new("/abs/C")).map(String::as_str),
            Some("Modded")
        );
        assert!(!groups.contains_key(&instances.join("A")));
        assert!(
            instance_groups(&tmp.path().join("nope"), &instances)
                .await
                .is_empty()
        );
    }

    #[test]
    fn instance_json_is_parsed() {
        let json = serde_json::json!({
            "name": "ATM9",
            "author": "me",
            "description": "All the mods",
            "version": "1.20.1-forge-47.2.0",
            "runtime": {
                "minecraft": "1.20.1",
                "forge": "47.2.0",
                "neoForged": "",
                "fabricLoader": "",
                "quiltLoader": "",
                "optifine": "",
                "labyMod": ""
            },
            "upstream": { "type": "curseforge-modpack", "modId": 715572, "fileId": 5142600 },
            "icon": "icon.png",
            "playtime": 3_600_000,
            "lastPlayedDate": 1_700_000_000_000i64,
            "creationDate": 1_690_000_000_000i64
        });
        let parsed = parse_instance_json(&json, "dir").unwrap();
        assert_eq!(parsed.name, "ATM9");
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
        assert_eq!(parsed.icon.as_deref(), Some("icon.png"));
        assert_eq!(parsed.seconds_played, Some(3600));
        assert_eq!(
            parsed.last_played.map(|d| d.timestamp()),
            Some(1_700_000_000)
        );
        assert_eq!(parsed.settings.notes.as_deref(), Some("All the mods"));
        assert!(is_xmcl_instance_json(&json));
    }

    #[test]
    fn loader_variants_and_fallbacks_are_parsed() {
        let json = serde_json::json!({
            "name": "Fabric",
            "runtime": { "minecraft": "1.20.4", "fabricLoader": "0.15.6" },
            "upstream": { "type": "modrinth-modpack", "projectId": "1KVo5zza", "versionId": "abc" }
        });
        let parsed = parse_instance_json(&json, "dir").unwrap();
        assert_eq!(
            parsed.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Fabric, "0.15.6")]
        );
        assert_eq!(
            parsed.modpack,
            Some(Modpack::Modrinth(ModrinthModpack {
                project_id: "1KVo5zza".into(),
                version_id: "abc".into(),
            }))
        );

        let json = serde_json::json!({ "name": "", "version": "fabric-loader-0.14.9-1.19.2" });
        let parsed = parse_instance_json(&json, "dir").unwrap();
        assert_eq!(parsed.name, "dir");
        assert_eq!(parsed.minecraft_version, "1.19.2");
        assert_eq!(
            parsed.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Fabric, "0.14.9")]
        );
        assert!(is_xmcl_instance_json(&json));

        let json = serde_json::json!({
            "runtime": { "minecraft": "1.21" },
            "upstream": { "type": "curseforge-modpack", "modId": "1", "fileId": "2" }
        });
        let parsed = parse_instance_json(&json, "dir").unwrap();
        assert!(parsed.loaders.is_empty());
        assert_eq!(
            parsed.modpack,
            Some(Modpack::Curseforge(CurseforgeModpack {
                project_id: 1,
                file_id: 2,
            }))
        );

        assert!(parse_instance_json(&serde_json::json!({ "runtime": {} }), "dir").is_none());

        assert!(!is_xmcl_instance_json(
            &serde_json::json!({ "id": "1.20.1", "launcher": { "name": "x" } })
        ));
        assert!(!is_xmcl_instance_json(
            &serde_json::json!({ "uuid": "x", "mcVersion": "1.20.1", "version": "1.0" })
        ));
    }

    #[test]
    fn icons_are_resolved_from_every_form() {
        let tmp = tempfile::tempdir().unwrap();
        let inst = tmp.path();
        std::fs::write(inst.join("icon.png"), b"x").unwrap();
        let elsewhere = tmp.path().join("my icon.png");
        std::fs::write(&elsewhere, b"x").unwrap();

        assert_eq!(
            resolve_icon(inst, Some("icon.png")),
            Some(IconSource::File(inst.join("icon.png")))
        );
        assert_eq!(
            resolve_icon(inst, None),
            Some(IconSource::File(inst.join("icon.png")))
        );
        assert_eq!(
            resolve_icon(inst, Some(&elsewhere.to_string_lossy())),
            Some(IconSource::File(elsewhere.clone()))
        );
        let encoded = elsewhere.to_string_lossy().replace(' ', "%20");
        assert_eq!(
            resolve_icon(inst, Some(&format!("{MEDIA_URL_PREFIX}{encoded}"))),
            Some(IconSource::File(elsewhere.clone()))
        );
        assert_eq!(
            resolve_icon(inst, Some("https://example.com/i.png")),
            Some(IconSource::Url("https://example.com/i.png".into()))
        );
        assert_eq!(
            resolve_icon(inst, Some("data:image/png;base64,AAAA")),
            Some(IconSource::DataUrl("data:image/png;base64,AAAA".into()))
        );

        std::fs::remove_file(inst.join("icon.png")).unwrap();
        assert_eq!(resolve_icon(inst, Some("missing.png")), None);
        assert_eq!(percent_decode("a%20b%zz"), "a b%zz");
    }

    #[tokio::test]
    async fn instances_are_found_from_app_data_root_and_externals() {
        let tmp = tempfile::tempdir().unwrap();
        let app_data = tmp.path().join("xmcl");
        let root = tmp.path().join("gamedata");
        let instances = root.join("instances");
        let managed = instances.join("Managed");
        std::fs::create_dir_all(managed.join("mods")).unwrap();
        std::fs::write(
            managed.join("instance.json"),
            serde_json::json!({ "name": "Managed", "runtime": { "minecraft": "1.20.1" } })
                .to_string(),
        )
        .unwrap();
        let external = tmp.path().join("external");
        std::fs::create_dir_all(&external).unwrap();
        std::fs::write(
            external.join("instance.json"),
            serde_json::json!({ "name": "External", "runtime": { "minecraft": "1.19.2" } })
                .to_string(),
        )
        .unwrap();
        std::fs::create_dir_all(&app_data).unwrap();
        std::fs::write(
            app_data.join("root"),
            format!("{}\n", root.to_string_lossy()),
        )
        .unwrap();
        std::fs::write(
            app_data.join("instances.json"),
            serde_json::json!({ "instances": ["Managed", external.to_string_lossy()], "selectedInstance": "Managed" })
                .to_string(),
        )
        .unwrap();

        assert!(is_app_data_dir(&app_data));
        assert_eq!(game_root_for_app_data(&app_data).await, root);
        assert_eq!(
            external_instances(&app_data, &instances).await,
            vec![managed.clone(), external.clone()]
        );

        let importer = XmclImporter::new();
        let entry = importer
            .scan_instance(managed, Some("Packs".into()))
            .await
            .unwrap()
            .unwrap();
        let InternalImportEntry::Valid(importable) = entry else {
            panic!("expected valid entry");
        };
        assert_eq!(importable.name, "Managed");
        assert_eq!(importable.minecraft_version, "1.20.1");
        assert_eq!(importable.settings.group.as_deref(), Some("Packs"));

        let other = tmp.path().join("atl");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(
            other.join("instance.json"),
            serde_json::json!({ "id": "1.20.1", "launcher": { "name": "x" } }).to_string(),
        )
        .unwrap();
        assert!(importer.scan_instance(other, None).await.unwrap().is_none());
    }
}

#[cfg(test)]
mod task14_tests {
    use super::*;
    #[test]
    fn task14_r2_xmcl_minimum_without_managed_maximum_survives() {
        for args in [["-Xmx65536M", "-Xms512M"], ["-Xms512M", "-Xmx65536M"]] {
            let settings = instance_settings(&serde_json::json!({"vmOptions":args}));
            assert_eq!(
                super::super::common::task14_persisted_config(&settings).memory,
                None
            );
            assert_eq!(super::super::common::task14_persisted_args(&settings), args);
        }
    }

    #[test]
    fn task14_r1_xmcl_memory_tokens_survive_array_route() {
        let args = [
            "-Xmx2GB",
            "-Xms512MB",
            "-Xmx65536",
            "-Xms2097153",
            "-Xmx2g",
            "-Xms512m",
        ];
        let settings = instance_settings(&serde_json::json!({"vmOptions":args}));
        assert_eq!(super::super::common::task14_persisted_args(&settings), args);
        assert_eq!(
            super::super::common::task14_persisted_config(&settings).memory,
            None
        );
    }

    #[test]
    fn task14_argv_xmcl_array_persists_tokens() {
        let args = [
            "-Dlabel=My Pack",
            r"-javaagent:C:\Program Files\Local Agent\agent.jar",
        ];
        let settings = instance_settings(&serde_json::json!({"vmOptions":args}));
        assert_eq!(super::super::common::task14_persisted_args(&settings), args);
    }
    #[tokio::test]
    async fn task14_default_relocation_keeps_external_and_groups() {
        let tmp = tempfile::tempdir().unwrap();
        let appdata = tmp.path().join("appdata");
        let game = tmp.path().join("gamedata");
        let managed = game.join("instances/Managed");
        let external = tmp.path().join("external/Pack");
        std::fs::create_dir_all(&appdata).unwrap();
        for path in [&managed, &external] {
            std::fs::create_dir_all(path).unwrap();
            std::fs::write(path.join("instance.json"), serde_json::json!({"runtime":{"minecraft":if path == &managed { "1.20.1" } else { "1.19.2" }}}).to_string()).unwrap();
        }
        std::fs::write(appdata.join("root"), game.to_str().unwrap()).unwrap();
        std::fs::write(appdata.join("instances.json"), serde_json::json!({"instances":[external, "Managed"], "groups":[{"name":"Packs", "instances":["Managed"]}]}).to_string()).unwrap();
        let expected = collect_instances(&appdata).await.unwrap();
        assert_eq!(expected.0.len(), 2);
        assert_eq!(expected.1.get(&managed).map(String::as_str), Some("Packs"));
        let selected = default_scan_path_from_candidates(vec![appdata.clone()])
            .await
            .unwrap()
            .unwrap();
        let actual = collect_instances(&selected).await.unwrap();
        assert_eq!(actual, expected);
        let importer = XmclImporter::new();
        for path in actual.0 {
            let InternalImportEntry::Valid(entry) = importer
                .scan_instance(path.clone(), actual.1.get(&path).cloned())
                .await
                .unwrap()
                .unwrap()
            else {
                panic!("valid metadata required")
            };
            assert_eq!(
                entry.minecraft_version,
                if path == managed { "1.20.1" } else { "1.19.2" }
            );
            if path == managed {
                assert_eq!(entry.settings.group.as_deref(), Some("Packs"));
            }
        }
    }
}

#[cfg(test)]
mod task14_discovery_controls {
    use super::*;
    #[tokio::test]
    async fn task14_default_missing_invalid_and_legacy_layouts() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(
            default_scan_path_from_candidates(vec![tmp.path().join("missing")])
                .await
                .unwrap()
                .is_none()
        );
        let appdata = tmp.path().join("app");
        let managed = appdata.join("instances/Managed");
        std::fs::create_dir_all(&managed).unwrap();
        std::fs::write(
            managed.join("instance.json"),
            r#"{"runtime":{"minecraft":"1.20.1"}}"#,
        )
        .unwrap();
        let absent = tmp.path().join("absent");
        for root_contents in [
            None,
            Some("not-an-absolute-root"),
            Some(absent.to_str().unwrap()),
        ] {
            if let Some(contents) = root_contents {
                std::fs::write(appdata.join("root"), contents).unwrap();
            }
            let selected = default_scan_path_from_candidates(vec![
                tmp.path().join("missing"),
                appdata.clone(),
            ])
            .await
            .unwrap()
            .unwrap();
            assert_eq!(selected, appdata);
            let (dirs, groups) = collect_instances(&selected).await.unwrap();
            assert_eq!(dirs, vec![managed.clone()]);
            assert!(groups.is_empty());
        }
        let game = tmp.path().join("legacygame");
        let legacy = game.join("instances/Legacy");
        let external = tmp.path().join("external");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::create_dir_all(&external).unwrap();
        std::fs::write(appdata.join("root"), game.to_str().unwrap()).unwrap();
        std::fs::write(game.join("instances.json"),serde_json::json!({"instances":["Legacy",external,external],"groups":[{"name":"Legacy packs","instances":["Legacy"]}]}).to_string()).unwrap();
        let selected = default_scan_path_from_candidates(vec![appdata.clone()])
            .await
            .unwrap()
            .unwrap();
        let (dirs, groups) = collect_instances(&selected).await.unwrap();
        assert_eq!(dirs, vec![legacy.clone(), external]);
        assert_eq!(
            groups.get(&legacy).map(String::as_str),
            Some("Legacy packs")
        );
        assert_eq!(collect_instances(&appdata).await.unwrap(), (dirs, groups));
    }
    #[test]
    fn task14_xmcl_legacy_command_string_keeps_grammar() {
        let settings = instance_settings(
            &serde_json::json!({"vmOptions":r#"-Xmx2G -Dfoo=bar "-Dlabel=My Pack""#}),
        );
        assert_eq!(settings.memory_max_mb, Some(2048));
        assert_eq!(
            super::super::common::task14_persisted_args(&settings),
            ["-Dfoo=bar", "-Dlabel=My Pack"]
        );
    }
}

#[cfg(test)]
mod stale_java_tests {
    use super::*;

    #[tokio::test]
    async fn a_java_path_that_is_gone_is_left_out() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(
            t.path().join("instance.json"),
            serde_json::json!({
                "name": "Pack",
                "runtime": { "minecraft": "1.20.1" },
                "java": "/gdl-no-such-folder/bin/java"
            })
            .to_string(),
        )
        .unwrap();
        let Some(InternalImportEntry::Valid(entry)) = XmclImporter::new()
            .scan_instance(t.path().to_path_buf(), None)
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
