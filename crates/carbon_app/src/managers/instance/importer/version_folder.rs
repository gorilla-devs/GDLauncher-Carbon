//! HMCL and PCL2 importer: instances are `.minecraft/versions/<id>/` folders, isolated or sharing the root.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::anyhow;
use tokio::sync::RwLock;
use tracing::{trace, warn};

use super::{
    GET_IMPORT_SCAN_STATUS, ImportScanStatus, ImportableInstance, ImporterState, InstanceImporter,
    InternalImportEntry, InvalidImportEntry,
    common::{
        DeclaredModLoader, ImportedSettings, ParsedVersionId, apply_imported_settings,
        copy_game_dir, dir_name, find_image_with_any_extension, first_existing_dir,
        group_for_import, json_bool, json_i64, json_positive_u32, json_string, json_trimmed_string,
        looks_like_minecraft_version, normalize_forge_version, parse_loader_type, read_json_value,
        resolve_standard_version, trimmed_non_empty,
    },
    technic::versions_from_version_json,
    vanilla::{is_launcher_managed, minecraft_dir_candidates},
};
use crate::{
    api::translation::Translation,
    domain::{
        instance::info::{GameVersion, ModLoaderType},
        vtask::VisualTaskId,
    },
    managers::{AppInner, instance::InstanceVersionSource},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionFolderFlavor {
    Hmcl,
    Pcl2,
}

impl VersionFolderFlavor {
    fn label(self) -> &'static str {
        match self {
            Self::Hmcl => "HMCL",
            Self::Pcl2 => "PCL2",
        }
    }
}

#[derive(Debug, Clone)]
struct Importable {
    filename: String,
    version_root: PathBuf,
    game_dir: PathBuf,
    name: String,
    icon_path: Option<PathBuf>,
    minecraft_version: String,
    loaders: Vec<DeclaredModLoader>,
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
pub struct VersionFolderImporter {
    flavor: VersionFolderFlavor,
    state: RwLock<ImporterState<Importable>>,
}

impl VersionFolderImporter {
    pub fn new(flavor: VersionFolderFlavor) -> Self {
        Self {
            flavor,
            state: RwLock::new(ImporterState::NoResults),
        }
    }

    /// PCL2 keeps its directory list in the registry, so the stock `.minecraft` is offered for it.
    pub async fn get_default_scan_path(
        flavor: VersionFolderFlavor,
    ) -> anyhow::Result<Option<PathBuf>> {
        let mut candidates = Vec::new();

        if flavor == VersionFolderFlavor::Hmcl {
            for home in hmcl_home_candidates() {
                candidates.extend(hmcl_game_directories(&home).await);
            }
        }

        candidates.extend(minecraft_dir_candidates());

        Ok(first_existing_dir(candidates))
    }

    async fn scan_version(
        &self,
        version_root: PathBuf,
        base_dir: &Path,
    ) -> anyhow::Result<Option<InternalImportEntry<Importable>>> {
        if !version_root.is_dir() {
            return Ok(None);
        }
        let id = dir_name(&version_root);
        let json_path = version_root.join(format!("{id}.json"));
        if !json_path.is_file() {
            return Ok(None);
        }

        let Some(json) = read_json_value(&json_path).await else {
            warn!("Cannot read version json at {json_path:?}");
            return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                name: id,
                reason: Translation::InstanceImportLegacyBadConfigFile,
            })));
        };

        let pcl_setup = parse_pcl_ini_file(&version_root.join("PCL").join("Setup.ini")).await;

        let mut versions = versions_from_hmcl_json(&json);
        if versions.minecraft.is_none() || versions.loaders.is_empty() {
            merge_versions(&mut versions, versions_from_pcl_setup(&pcl_setup));
        }
        let Some(minecraft_version) = versions.minecraft else {
            warn!(
                "{} version {id} at {version_root:?} declares no Minecraft version",
                self.flavor.label()
            );
            return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                name: id,
                reason: Translation::InstanceImportLegacyBadConfigFile,
            })));
        };

        let (game_dir, mut settings) = match self.flavor {
            VersionFolderFlavor::Hmcl => hmcl_run_dir_and_settings(&version_root, base_dir).await,
            VersionFolderFlavor::Pcl2 => (
                pcl_run_dir(&version_root, base_dir, &pcl_setup),
                pcl_settings(&pcl_setup),
            ),
        };

        // Either launcher may have been used on the same folder; take whichever left settings behind.
        if settings == ImportedSettings::default() {
            settings = match self.flavor {
                VersionFolderFlavor::Hmcl => pcl_settings(&pcl_setup),
                VersionFolderFlavor::Pcl2 => {
                    hmcl_run_dir_and_settings(&version_root, base_dir).await.1
                }
            };
        }

        let icon_path = find_image_with_any_extension(&version_root, "icon")
            .or_else(|| pcl_custom_logo(&version_root, &pcl_setup));

        Ok(Some(InternalImportEntry::Valid(Importable {
            filename: id.clone(),
            version_root,
            game_dir,
            name: id,
            icon_path,
            minecraft_version,
            loaders: versions.loaders,
            settings,
        })))
    }
}

#[async_trait::async_trait]
impl InstanceImporter for VersionFolderImporter {
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

        if is_version_folder(&scan_path) {
            let base_dir = scan_path
                .parent()
                .and_then(Path::parent)
                .map(Path::to_path_buf)
                .unwrap_or_else(|| scan_path.clone());
            if let Ok(Some(entry)) = self.scan_version(scan_path, &base_dir).await {
                self.state.write().await.set_single(entry).await;
                app.invalidate(GET_IMPORT_SCAN_STATUS, None);
            }
            return Ok(());
        }

        let (base_dir, versions_dir) = if scan_path.join("versions").is_dir() {
            (scan_path.clone(), scan_path.join("versions"))
        } else if dir_name(&scan_path) == "versions" {
            (
                scan_path
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| scan_path.clone()),
                scan_path.clone(),
            )
        } else {
            (scan_path.clone(), scan_path.clone())
        };

        let Ok(mut dir) = tokio::fs::read_dir(&versions_dir).await else {
            return Ok(());
        };

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if let Ok(Some(entry)) = self.scan_version(path, &base_dir).await {
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
        trace!(?index, ?name, "Beginning {} import", self.flavor.label());

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
        let instance_version_source =
            InstanceVersionSource::Version(GameVersion::Standard(standard_version));

        let initializer = |instance_path: PathBuf| {
            let instance = &instance;
            async move {
                let version_id = instance.filename.clone();
                copy_game_dir(&instance.game_dir, &instance_path, move |path| {
                    !is_launcher_managed(path) && !is_version_folder_file(path, &version_id)
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
                None,
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

fn is_version_folder_file(relative: &Path, version_id: &str) -> bool {
    let Some(first) = relative.components().next() else {
        return false;
    };
    let first = first.as_os_str().to_string_lossy();

    first == format!("{version_id}.json")
        || first == format!("{version_id}.jar")
        || first == ".hmcl"
        || first == "PCL"
        || first == "hmclversion.cfg"
        || first == "modpack.cfg"
        || first == "libraries"
        || first.starts_with("icon.")
}

fn is_version_folder(path: &Path) -> bool {
    path.is_dir() && path.join(format!("{}.json", dir_name(path))).is_file()
}

fn merge_versions(into: &mut ParsedVersionId, from: ParsedVersionId) {
    if into.minecraft.is_none() {
        into.minecraft = from.minecraft;
    }
    for loader in from.loaders {
        if !into.loaders.iter().any(|l| l.type_ == loader.type_) {
            into.loaders.push(loader);
        }
    }
}

/// HMCL's `patches` list is authoritative when present.
fn versions_from_hmcl_json(json: &serde_json::Value) -> ParsedVersionId {
    let mut from_patches = ParsedVersionId::default();

    if let Some(patches) = json.get("patches").and_then(|p| p.as_array()) {
        for patch in patches {
            if json_bool(patch, "hidden").unwrap_or(false) {
                continue;
            }
            let Some(id) = json_string(patch, "id") else {
                continue;
            };
            let Some(version) = json_trimmed_string(patch, "version") else {
                continue;
            };

            let id = id.trim().to_ascii_lowercase();
            if id == "game" {
                from_patches.minecraft = Some(version);
                continue;
            }
            if let Some(type_) = parse_loader_type(&id) {
                if !from_patches.loaders.iter().any(|l| l.type_ == type_) {
                    from_patches
                        .loaders
                        .push(DeclaredModLoader::new(type_, version));
                }
            }
        }
    }

    let mut result = from_patches;
    merge_versions(&mut result, versions_from_version_json(json));

    if let Some(mc) = result.minecraft.clone() {
        for loader in &mut result.loaders {
            if loader.type_ == ModLoaderType::Forge {
                loader.version = normalize_forge_version(&mc, &loader.version);
            }
        }
    }

    result
}

fn versions_from_pcl_setup(setup: &HashMap<String, String>) -> ParsedVersionId {
    let value = |key: &str| {
        setup
            .get(key)
            .map(|v| v.trim())
            .filter(|v| !v.is_empty() && *v != "未知版本" && *v != "Unknown")
            .map(str::to_string)
    };

    let mut parsed = ParsedVersionId {
        minecraft: value("VersionVanilla").filter(|v| looks_like_minecraft_version(v)),
        loaders: Vec::new(),
    };
    let mc = parsed.minecraft.clone().unwrap_or_default();

    for (key, type_) in [
        ("VersionForge", ModLoaderType::Forge),
        ("VersionNeoForge", ModLoaderType::Neoforge),
        ("VersionFabric", ModLoaderType::Fabric),
    ] {
        if let Some(version) = value(key) {
            let version = if type_ == ModLoaderType::Forge {
                normalize_forge_version(&mc, &version)
            } else {
                version
            };
            parsed.loaders.push(DeclaredModLoader::new(type_, version));
        }
    }

    parsed
}

fn parse_pcl_ini(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim_end_matches('\r');
            let (key, value) = line.split_once(':')?;
            let key = key.trim();
            (!key.is_empty()).then(|| (key.to_string(), value.trim().to_string()))
        })
        .collect()
}

async fn parse_pcl_ini_file(path: &Path) -> HashMap<String, String> {
    match tokio::fs::read_to_string(path).await {
        Ok(text) => parse_pcl_ini(&text),
        Err(_) => HashMap::new(),
    }
}

fn pcl_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

/// `VersionArgumentIndieV2`, the older `VersionArgumentIndie` (1 on, 2 off), or PCL2's own heuristic: a folder with mods or saves is isolated.
fn pcl_run_dir(version_root: &Path, base_dir: &Path, setup: &HashMap<String, String>) -> PathBuf {
    let isolated = setup
        .get("VersionArgumentIndieV2")
        .and_then(|v| pcl_bool(v))
        .or_else(
            || match setup.get("VersionArgumentIndie").map(|v| v.trim()) {
                Some("1") => Some(true),
                Some("2") => Some(false),
                _ => None,
            },
        )
        .unwrap_or_else(|| has_own_game_data(version_root));

    if isolated {
        version_root.to_path_buf()
    } else {
        base_dir.to_path_buf()
    }
}

fn has_own_game_data(version_root: &Path) -> bool {
    version_root.join("mods").is_dir() || version_root.join("saves").is_dir()
}

/// PCL2's `VersionRamCustom` slider → GB, as the launcher maps it.
fn pcl_ram_slider_to_mb(value: i64) -> u32 {
    let gb = if value <= 12 {
        value as f64 * 0.1 + 0.3
    } else if value <= 25 {
        (value - 12) as f64 * 0.5 + 1.5
    } else if value <= 33 {
        (value - 25) as f64 + 8.0
    } else {
        (value - 33) as f64 * 2.0 + 16.0
    };
    (gb * 1024.0).round().max(0.0) as u32
}

fn pcl_settings(setup: &HashMap<String, String>) -> ImportedSettings {
    let value = |key: &str| {
        setup
            .get(key)
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };

    let mut settings = ImportedSettings {
        notes: value("CustomInfo"),
        pre_launch_hook: value("VersionAdvanceRun"),
        ..Default::default()
    };

    // VersionRamType: 0 automatic, 1 custom slider, 2 (default) global.
    if value("VersionRamType").as_deref() == Some("1") {
        if let Some(slider) = value("VersionRamCustom").and_then(|v| v.parse::<i64>().ok()) {
            let mb = pcl_ram_slider_to_mb(slider);
            if mb > 0 {
                settings.memory_max_mb = Some(mb);
            }
        }
    }

    if let Some(args) = value("VersionAdvanceJvm") {
        settings.absorb_jvm_args(&args);
    }

    // A specific Java is stored as a serialized Java object; any path-like string in it is the executable or its folder.
    if let Some(java) = value("VersionArgumentJavaSelect") {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&java) {
            settings.java_path = trimmed_non_empty(
                ["PathJava", "PathJavaw", "Path", "PathFolder"]
                    .iter()
                    .find_map(|key| json_string(&json, key)),
            );
        }
    }

    settings
}

fn pcl_custom_logo(version_root: &Path, setup: &HashMap<String, String>) -> Option<PathBuf> {
    if !setup
        .get("LogoCustom")
        .and_then(|v| pcl_bool(v))
        .unwrap_or(false)
    {
        return None;
    }
    let logo = setup.get("Logo")?.trim();
    if logo.is_empty() {
        return None;
    }
    let path = PathBuf::from(logo.replace('\\', "/"));
    let path = if path.is_absolute() {
        path
    } else {
        version_root.join(path)
    };
    path.is_file().then_some(path)
}

async fn hmcl_run_dir_and_settings(
    version_root: &Path,
    base_dir: &Path,
) -> (PathBuf, ImportedSettings) {
    let is_modpack = version_root.join("modpack.cfg").is_file();

    if let Some(json) = read_json_value(
        &version_root
            .join(".hmcl")
            .join("config")
            .join("instance-game-settings.json"),
    )
    .await
    {
        let (run_dir, settings) = hmcl_current_settings(&json, version_root, base_dir);
        let run_dir = if is_modpack {
            version_root.to_path_buf()
        } else {
            run_dir
        };
        return (run_dir, settings);
    }

    if let Some(json) = read_json_value(&version_root.join("hmclversion.cfg")).await {
        let (run_dir, settings) = hmcl_legacy_settings(&json, version_root, base_dir);
        let run_dir = if is_modpack {
            version_root.to_path_buf()
        } else {
            run_dir
        };
        return (run_dir, settings);
    }

    let run_dir = if is_modpack || has_own_game_data(version_root) {
        version_root.to_path_buf()
    } else {
        base_dir.to_path_buf()
    };
    (run_dir, ImportedSettings::default())
}

/// Only properties listed in `overrideProperties` are the instance's own; a file without that list is taken at face value.
fn hmcl_current_settings(
    json: &serde_json::Value,
    version_root: &Path,
    base_dir: &Path,
) -> (PathBuf, ImportedSettings) {
    let overrides: Option<Vec<String>> = json
        .get("overrideProperties")
        .and_then(|o| o.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|v| v.as_str())
                .map(str::to_string)
                .collect()
        });
    let is_local = |key: &str| {
        overrides
            .as_ref()
            .is_none_or(|o| o.iter().any(|k| k == key))
    };

    let text = |key: &str| {
        is_local(key)
            .then(|| json_trimmed_string(json, key))
            .flatten()
    };
    let number = |key: &str| {
        is_local(key)
            .then(|| json_positive_u32(json, key))
            .flatten()
    };

    let run_dir = match text("runningDirectory") {
        Some(dir) => PathBuf::from(dir),
        None if is_local("runningDirectory") && overrides.is_some() => version_root.to_path_buf(),
        None => base_dir.to_path_buf(),
    };
    let run_dir = if run_dir.is_dir() {
        run_dir
    } else {
        base_dir.to_path_buf()
    };

    let mut settings = ImportedSettings {
        java_path: (json_string(json, "javaType")
            .map(|t| t.eq_ignore_ascii_case("custom"))
            .unwrap_or(true))
        .then(|| text("customJavaPath"))
        .flatten(),
        pre_launch_hook: text("preLaunchCommand"),
        post_exit_hook: text("postExitCommand"),
        wrapper_command: text("commandWrapper"),
        ..Default::default()
    };

    let auto_memory = is_local("autoMemory") && json_bool(json, "autoMemory").unwrap_or(false);
    if !auto_memory {
        settings.memory_max_mb = number("maxMemory");
        settings.memory_min_mb = number("minMemory");
    }

    if let Some(args) = text("jvmOptions") {
        settings.absorb_jvm_args(&args);
    }

    if let (Some(w), Some(h)) = (number("width"), number("height")) {
        settings.game_resolution = Some((w, h));
    }

    (run_dir, settings)
}

/// `usesGlobal` means every value is the global default rather than the instance's.
fn hmcl_legacy_settings(
    json: &serde_json::Value,
    version_root: &Path,
    base_dir: &Path,
) -> (PathBuf, ImportedSettings) {
    let text = |key: &str| json_trimmed_string(json, key);

    // gameDirType: 0 root, 1 version folder, 2 custom
    let dir_type = match json.get("gameDirType") {
        Some(serde_json::Value::Number(n)) => n.as_i64().unwrap_or(0),
        Some(serde_json::Value::String(s)) => match s.trim().to_ascii_uppercase().as_str() {
            "VERSION_FOLDER" => 1,
            "CUSTOM" => 2,
            _ => 0,
        },
        _ => {
            if has_own_game_data(version_root) {
                1
            } else {
                0
            }
        }
    };
    let run_dir = match dir_type {
        1 => version_root.to_path_buf(),
        2 => text("gameDir")
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
            .unwrap_or_else(|| version_root.to_path_buf()),
        _ => base_dir.to_path_buf(),
    };

    if json_bool(json, "usesGlobal").unwrap_or(false) {
        return (run_dir, ImportedSettings::default());
    }

    let positive = |key: &str| json_positive_u32(json, key);

    let mut settings = ImportedSettings {
        java_path: (text("java").is_some_and(|j| j.eq_ignore_ascii_case("custom")))
            .then(|| text("javaDir"))
            .flatten(),
        pre_launch_hook: text("precalledCommand"),
        post_exit_hook: text("postExitCommand"),
        wrapper_command: text("wrapper"),
        ..Default::default()
    };

    if !json_bool(json, "autoMemory").unwrap_or(false) {
        settings.memory_max_mb = positive("maxMemory");
        settings.memory_min_mb = positive("minMemory");
    }

    if let Some(args) = text("javaArgs") {
        settings.absorb_jvm_args(&args);
    }

    if let (Some(w), Some(h)) = (positive("width"), positive("height")) {
        settings.game_resolution = Some((w, h));
    }

    (run_dir, settings)
}

fn hmcl_home_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(dir) = std::env::var_os("HMCL_USER_HOME") {
        candidates.push(PathBuf::from(dir));
    }

    let Some(basedirs) = directories::BaseDirs::new() else {
        return candidates;
    };
    let home = basedirs.home_dir().to_path_buf();

    #[cfg(target_os = "windows")]
    candidates.push(basedirs.data_dir().join(".hmcl"));
    #[cfg(target_os = "macos")]
    candidates.push(home.join("Library/Application Support/hmcl"));

    candidates.push(basedirs.data_dir().join("hmcl"));
    candidates.push(home.join(".local/share/hmcl"));
    candidates.push(home.join(".hmcl"));

    candidates.into_iter().filter(|p| p.is_dir()).collect()
}

/// Only absolute paths can be resolved; relative ones are relative to the launcher's working directory.
async fn hmcl_game_directories(home: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    for file in [
        home.join("config").join("user-game-directories.json"),
        home.join("config").join("game-directories.json"),
    ] {
        let Some(json) = read_json_value(&file).await else {
            continue;
        };
        let Some(list) = json.get("directories").and_then(|d| d.as_array()) else {
            continue;
        };
        for entry in list {
            let path = match entry {
                serde_json::Value::String(s) => Some(s.clone()),
                other => json_string(other, "path"),
            };
            if let Some(path) = path.map(|p| PathBuf::from(p.trim())) {
                if path.is_absolute() && path.is_dir() && !dirs.contains(&path) {
                    dirs.push(path);
                }
            }
        }
    }

    dirs
}

#[cfg(test)]
mod test {
    use super::*;

    fn write_version(base: &Path, id: &str, json: serde_json::Value) -> PathBuf {
        let root = base.join("versions").join(id);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(format!("{id}.json")), json.to_string()).unwrap();
        root
    }

    #[test]
    fn hmcl_patches_take_precedence_over_libraries() {
        let json = serde_json::json!({
            "id": "1.20.1-forge",
            "inheritsFrom": "1.20.1",
            "patches": [
                { "id": "game", "version": "1.20.1" },
                { "id": "forge", "version": "1.20.1-47.2.0" },
                { "id": "optifine", "version": "HD_U_I6" },
                { "id": "hidden-thing", "version": "1", "hidden": true }
            ],
            "libraries": [{ "name": "net.minecraftforge:forge:1.20.1-47.2.0" }]
        });
        let v = versions_from_hmcl_json(&json);
        assert_eq!(v.minecraft.as_deref(), Some("1.20.1"));
        assert_eq!(
            v.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Forge, "47.2.0")]
        );

        let json = serde_json::json!({
            "id": "fabric-1.20.4",
            "inheritsFrom": "1.20.4",
            "libraries": [
                { "name": "net.fabricmc:fabric-loader:0.15.6" },
                { "name": "net.fabricmc:intermediary:1.20.4" }
            ]
        });
        let v = versions_from_hmcl_json(&json);
        assert_eq!(v.minecraft.as_deref(), Some("1.20.4"));
        assert_eq!(
            v.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Fabric, "0.15.6")]
        );

        let json = serde_json::json!({
            "id": "neo",
            "patches": [
                { "id": "game", "version": "1.20.4" },
                { "id": "neoforge", "version": "20.4.80-beta" }
            ]
        });
        let v = versions_from_hmcl_json(&json);
        assert_eq!(
            v.loaders,
            vec![DeclaredModLoader::new(
                ModLoaderType::Neoforge,
                "20.4.80-beta"
            )]
        );
    }

    #[test]
    fn pcl_ini_and_cached_versions_are_parsed() {
        let setup = parse_pcl_ini(
            "State:1\r\nVersionVanilla:1.12.2\r\nVersionForge:14.23.5.2860\r\nVersionFabric:\r\nCustomInfo:my notes\r\nVersionArgumentIndieV2:True\r\nVersionRamType:1\r\nVersionRamCustom:20\r\nVersionAdvanceJvm:-XX:+UseG1GC\r\nVersionAdvanceRun:echo hi\r\nbroken line without colon\r\n",
        );
        assert_eq!(
            setup.get("VersionVanilla").map(String::as_str),
            Some("1.12.2")
        );
        assert_eq!(setup.get("VersionFabric").map(String::as_str), Some(""));

        let v = versions_from_pcl_setup(&setup);
        assert_eq!(v.minecraft.as_deref(), Some("1.12.2"));
        assert_eq!(
            v.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Forge, "14.23.5.2860")]
        );

        let s = pcl_settings(&setup);
        assert_eq!(s.notes.as_deref(), Some("my notes"));
        // slider 20 → (20-12)*0.5+1.5 = 5.5 GB
        assert_eq!(s.memory(), Some((5632, 5632)));
        assert_eq!(s.extra_java_args.as_deref(), Some("-XX:+UseG1GC"));
        assert_eq!(s.pre_launch_hook.as_deref(), Some("echo hi"));

        let setup = parse_pcl_ini("VersionRamType:2\r\nVersionRamCustom:20\r\n");
        assert_eq!(pcl_settings(&setup).memory(), None);

        assert_eq!(pcl_ram_slider_to_mb(0), 307);
        assert_eq!(pcl_ram_slider_to_mb(12), 1536);
        assert_eq!(pcl_ram_slider_to_mb(33), 16384);
        assert_eq!(pcl_ram_slider_to_mb(35), 20480);
    }

    #[test]
    fn pcl_run_dir_follows_isolation_settings() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let root = write_version(base, "1.20.1", serde_json::json!({ "id": "1.20.1" }));

        assert_eq!(pcl_run_dir(&root, base, &HashMap::new()), base);

        std::fs::create_dir_all(root.join("mods")).unwrap();
        assert_eq!(pcl_run_dir(&root, base, &HashMap::new()), root);

        let off = parse_pcl_ini("VersionArgumentIndieV2:False\r\n");
        assert_eq!(pcl_run_dir(&root, base, &off), base);
        let legacy_on = parse_pcl_ini("VersionArgumentIndie:1\r\n");
        assert_eq!(pcl_run_dir(&root, base, &legacy_on), root);
        let legacy_off = parse_pcl_ini("VersionArgumentIndie:2\r\n");
        assert_eq!(pcl_run_dir(&root, base, &legacy_off), base);
    }

    #[tokio::test]
    async fn hmcl_current_and_legacy_settings_are_read() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let root = write_version(base, "Modded", serde_json::json!({ "id": "Modded" }));

        let cfg_dir = root.join(".hmcl").join("config");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        std::fs::write(
            cfg_dir.join("instance-game-settings.json"),
            serde_json::json!({
                "overrideProperties": ["runningDirectory", "maxMemory", "jvmOptions", "customJavaPath", "javaType", "preLaunchCommand"],
                "runningDirectory": "",
                "autoMemory": false,
                "maxMemory": 6144,
                "minMemory": 1024,
                "jvmOptions": "-XX:+UseZGC",
                "javaType": "CUSTOM",
                "customJavaPath": "/opt/jdk/bin/java",
                "preLaunchCommand": "echo pre",
                "commandWrapper": "inherited-not-ours",
                "width": 1920,
                "height": 1080
            })
            .to_string(),
        )
        .unwrap();
        let (run_dir, s) = hmcl_run_dir_and_settings(&root, base).await;
        assert_eq!(run_dir, root);
        assert_eq!(s.memory(), Some((6144, 6144)));
        assert_eq!(s.extra_java_args.as_deref(), Some("-XX:+UseZGC"));
        assert_eq!(s.java_path.as_deref(), Some("/opt/jdk/bin/java"));
        assert_eq!(s.pre_launch_hook.as_deref(), Some("echo pre"));
        assert_eq!(s.wrapper_command, None);
        assert_eq!(s.game_resolution, None);

        std::fs::write(
            cfg_dir.join("instance-game-settings.json"),
            serde_json::json!({ "overrideProperties": [], "maxMemory": 6144 }).to_string(),
        )
        .unwrap();
        let (run_dir, s) = hmcl_run_dir_and_settings(&root, base).await;
        assert_eq!(run_dir, base);
        assert_eq!(s, ImportedSettings::default());

        std::fs::write(root.join("modpack.cfg"), "{}").unwrap();
        let (run_dir, _) = hmcl_run_dir_and_settings(&root, base).await;
        assert_eq!(run_dir, root);
        std::fs::remove_file(root.join("modpack.cfg")).unwrap();
        std::fs::remove_dir_all(root.join(".hmcl")).unwrap();

        std::fs::write(
            root.join("hmclversion.cfg"),
            serde_json::json!({
                "usesGlobal": false,
                "gameDirType": 1,
                "java": "Custom",
                "javaDir": "C:\\\\jdk\\\\bin\\\\java.exe",
                "javaArgs": "-Xmx4G -XX:+UseG1GC",
                "maxMemory": 8192,
                "width": 1280,
                "height": 720,
                "wrapper": "gamemoderun",
                "precalledCommand": "echo legacy",
                "postExitCommand": "echo bye"
            })
            .to_string(),
        )
        .unwrap();
        let (run_dir, s) = hmcl_run_dir_and_settings(&root, base).await;
        assert_eq!(run_dir, root);
        assert_eq!(s.memory(), Some((4096, 4096)));
        assert_eq!(s.extra_java_args.as_deref(), Some("-XX:+UseG1GC"));
        assert_eq!(s.java_path.as_deref(), Some("C:\\\\jdk\\\\bin\\\\java.exe"));
        assert_eq!(s.game_resolution, Some((1280, 720)));
        assert_eq!(s.wrapper_command.as_deref(), Some("gamemoderun"));
        assert_eq!(s.pre_launch_hook.as_deref(), Some("echo legacy"));
        assert_eq!(s.post_exit_hook.as_deref(), Some("echo bye"));

        std::fs::write(
            root.join("hmclversion.cfg"),
            serde_json::json!({ "usesGlobal": true, "gameDirType": "ROOT_FOLDER", "maxMemory": 8192 }).to_string(),
        )
        .unwrap();
        let (run_dir, s) = hmcl_run_dir_and_settings(&root, base).await;
        assert_eq!(run_dir, base);
        assert_eq!(s, ImportedSettings::default());
    }

    #[tokio::test]
    async fn versions_are_scanned_from_a_minecraft_root() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let forge = write_version(
            base,
            "ATM9",
            serde_json::json!({
                "id": "ATM9",
                "patches": [
                    { "id": "game", "version": "1.20.1" },
                    { "id": "forge", "version": "47.2.0" }
                ]
            }),
        );
        std::fs::create_dir_all(forge.join("mods")).unwrap();
        std::fs::write(forge.join("icon.png"), b"png").unwrap();

        let vanilla = write_version(base, "1.20.4", serde_json::json!({ "id": "1.20.4" }));
        std::fs::create_dir_all(vanilla.join("PCL")).unwrap();
        std::fs::write(
            vanilla.join("PCL").join("Setup.ini"),
            "VersionArgumentIndieV2:False\r\nCustomInfo:plain\r\n",
        )
        .unwrap();

        std::fs::create_dir_all(base.join("versions").join("junk")).unwrap();

        let importer = VersionFolderImporter::new(VersionFolderFlavor::Hmcl);
        let entry = importer
            .scan_version(forge.clone(), base)
            .await
            .unwrap()
            .unwrap();
        let InternalImportEntry::Valid(i) = entry else {
            panic!("expected valid entry");
        };
        assert_eq!(i.name, "ATM9");
        assert_eq!(i.game_dir, forge);
        assert_eq!(i.minecraft_version, "1.20.1");
        assert_eq!(
            i.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Forge, "47.2.0")]
        );
        assert_eq!(i.icon_path, Some(forge.join("icon.png")));

        let importer = VersionFolderImporter::new(VersionFolderFlavor::Pcl2);
        let entry = importer
            .scan_version(vanilla.clone(), base)
            .await
            .unwrap()
            .unwrap();
        let InternalImportEntry::Valid(i) = entry else {
            panic!("expected valid entry");
        };
        assert_eq!(i.game_dir, base);
        assert_eq!(i.minecraft_version, "1.20.4");
        assert!(i.loaders.is_empty());
        assert_eq!(i.settings.notes.as_deref(), Some("plain"));

        assert!(
            importer
                .scan_version(base.join("versions").join("junk"), base)
                .await
                .unwrap()
                .is_none()
        );
        assert!(is_version_folder(&forge));
        assert!(!is_version_folder(base));
    }

    #[test]
    fn version_folder_files_are_excluded_from_copy() {
        assert!(is_version_folder_file(Path::new("ATM9.json"), "ATM9"));
        assert!(is_version_folder_file(Path::new("ATM9.jar"), "ATM9"));
        assert!(is_version_folder_file(
            Path::new(".hmcl/config/x.json"),
            "ATM9"
        ));
        assert!(is_version_folder_file(Path::new("PCL/Setup.ini"), "ATM9"));
        assert!(is_version_folder_file(Path::new("icon.png"), "ATM9"));
        assert!(!is_version_folder_file(Path::new("mods/a.jar"), "ATM9"));
        assert!(!is_version_folder_file(Path::new("other.json"), "ATM9"));
    }

    #[tokio::test]
    async fn hmcl_game_directories_are_listed() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("hmcl");
        std::fs::create_dir_all(home.join("config")).unwrap();
        let game = tmp.path().join("games").join(".minecraft");
        std::fs::create_dir_all(game.join("versions")).unwrap();
        std::fs::write(
            home.join("config").join("user-game-directories.json"),
            serde_json::json!({
                "$schema": "game-directories/1.0.0",
                "directories": [
                    { "id": "home", "name": "Home", "path": game.to_string_lossy() },
                    { "id": "rel", "path": ".minecraft" },
                    { "id": "gone", "path": "/nonexistent/dir" }
                ]
            })
            .to_string(),
        )
        .unwrap();
        assert_eq!(hmcl_game_directories(&home).await, vec![game]);
        assert!(
            hmcl_game_directories(&tmp.path().join("nope"))
                .await
                .is_empty()
        );
    }
}
