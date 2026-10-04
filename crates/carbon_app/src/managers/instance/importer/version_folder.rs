//! HMCL and PCL2 importer: instances are `.minecraft/versions/<id>/` folders, isolated or sharing the root.

use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::anyhow;
use tokio::sync::RwLock;
use tracing::{trace, warn};

use super::{
    GET_IMPORT_SCAN_STATUS, ImportScanStatus, ImportableInstance, ImporterState, InstanceImporter,
    InternalImportEntry, InvalidImportEntry,
    common::{
        DeclaredModLoader, ImportedSettings, ParsedVersionId, apply_imported_settings,
        copy_game_dir, dedup_paths, dir_name, find_image_with_any_extension, first_existing_dir,
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
    managers::{AppInner, instance::InstanceVersionSource, minecraft::modrinth::path_clean},
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
    /// Shown under the name instead of the version id when a scan spans
    /// several game folders, so versions with the same id can be told apart.
    location: Option<String>,
}

impl From<Importable> for ImportableInstance {
    fn from(value: Importable) -> Self {
        let warnings = value.settings.warnings();
        Self {
            filename: value.location.unwrap_or(value.filename),
            instance_name: value.name,
            warnings,
        }
    }
}

#[derive(Debug)]
pub struct VersionFolderImporter {
    flavor: VersionFolderFlavor,
    source_home: RwLock<Option<HmclLocalHome>>,
    show_locations: AtomicBool,
    state: RwLock<ImporterState<Importable>>,
}

impl VersionFolderImporter {
    pub fn new(flavor: VersionFolderFlavor) -> Self {
        Self {
            flavor,
            source_home: RwLock::new(None),
            show_locations: AtomicBool::new(false),
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
                if hmcl_home_has_game_directory(&home).await {
                    return Ok(Some(home));
                }
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

        let (mut game_dir, mut settings) = match self.flavor {
            VersionFolderFlavor::Hmcl => hmcl_run_dir_and_settings(&version_root, base_dir).await,
            VersionFolderFlavor::Pcl2 => (
                pcl_run_dir(&version_root, base_dir, &pcl_setup),
                pcl_settings(&pcl_setup),
            ),
        };

        if self.flavor == VersionFolderFlavor::Hmcl {
            let source_home = self.source_home.read().await.clone();
            match inherited_hmcl_directory(&version_root, base_dir, source_home.as_ref()).await {
                Ok(Some(path)) => game_dir = path,
                Ok(None) => {}
                Err(UnplaceableHmclRunDir(running_directory)) => {
                    return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                        name: format!("{id}: {running_directory}"),
                        reason: Translation::InstanceImportHmclRelativeRunDir,
                    })));
                }
            }
        }
        if !game_dir.is_dir() {
            return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                name: format!("{id}: {}", game_dir.display()),
                reason: Translation::InstanceImportSourceUnavailable,
            })));
        }
        for loader in &mut versions.loaders {
            if loader.type_ == ModLoaderType::Forge {
                loader.version = normalize_forge_version(&minecraft_version, &loader.version);
            }
        }

        // Either launcher may have been used on the same folder; take whichever left settings behind.
        if settings == ImportedSettings::default() {
            settings = match self.flavor {
                VersionFolderFlavor::Hmcl => pcl_settings(&pcl_setup),
                VersionFolderFlavor::Pcl2 => {
                    hmcl_run_dir_and_settings(&version_root, base_dir).await.1
                }
            };
        }

        settings.skip_missing_java_path();
        let icon_path = find_image_with_any_extension(&version_root, "icon")
            .or_else(|| pcl_custom_logo(&version_root, &pcl_setup));
        let location = self
            .show_locations
            .load(Ordering::Relaxed)
            .then(|| version_root.display().to_string());

        Ok(Some(InternalImportEntry::Valid(Importable {
            filename: id.clone(),
            version_root,
            game_dir,
            name: id,
            icon_path,
            minecraft_version,
            loaders: versions.loaders,
            settings,
            location,
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

        if self.flavor == VersionFolderFlavor::Hmcl {
            if let Some(home) = HmclLocalHome::selected(&scan_path) {
                return self.scan_hmcl_install(app, scan_path, home).await;
            }
        }

        self.scan_game_dir(app, scan_path).await
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
            async move { initialize_version_copy(instance, &instance_path).await }
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

impl VersionFolderImporter {
    /// Scans every game folder an HMCL install lists, the way HMCL shows them.
    ///
    /// A listed folder that is gone is skipped while another one exists, so
    /// one stale entry (an unplugged drive, a deleted folder) does not hide
    /// the rest. A folder that cannot be read gets its own row instead of
    /// stopping the scan.
    async fn scan_hmcl_install(
        &self,
        app: &Arc<AppInner>,
        selected: PathBuf,
        home: HmclLocalHome,
    ) -> anyhow::Result<()> {
        let (available, missing): (Vec<_>, Vec<_>) = home
            .game_directories()
            .await
            .into_iter()
            .partition(|dir| dir.is_dir());
        for dir in &missing {
            warn!(?dir, home = ?home.dir, "Skipping an HMCL game directory that does not exist");
        }
        *self.source_home.write().await = Some(home);

        if available.is_empty() {
            if missing.is_empty() {
                // Nothing HMCL lists can be located, so the selected folder is all there is.
                return self.scan_game_dir(app, selected).await;
            }
            // Name the folders HMCL expected, so the user knows what to reconnect or select.
            for dir in missing {
                self.push_invalid(
                    app,
                    dir.display().to_string(),
                    Translation::InstanceImportSourceUnavailable,
                )
                .await;
            }
            return Ok(());
        }

        self.show_locations
            .store(available.len() > 1, Ordering::Relaxed);
        for dir in available {
            if let Err(error) = self.scan_game_dir(app, dir.clone()).await {
                warn!(?dir, "Cannot scan an HMCL game directory: {error:#}");
                self.push_invalid(
                    app,
                    format!("{}: {error:#}", dir.display()),
                    Translation::InstanceImportSourceUnavailable,
                )
                .await;
            }
        }
        Ok(())
    }

    async fn push_invalid(&self, app: &Arc<AppInner>, name: String, reason: Translation) {
        self.state
            .write()
            .await
            .push_multi(InternalImportEntry::Invalid(InvalidImportEntry {
                name,
                reason,
            }))
            .await;
        app.invalidate(GET_IMPORT_SCAN_STATUS, None);
    }

    /// Scans a game directory, its `versions` folder or a single version folder.
    async fn scan_game_dir(&self, app: &Arc<AppInner>, scan_path: PathBuf) -> anyhow::Result<()> {
        if is_version_folder(&scan_path) {
            let base_dir = scan_path
                .parent()
                .and_then(Path::parent)
                .map(Path::to_path_buf)
                .unwrap_or_else(|| scan_path.clone());
            if let Some(entry) = self.scan_version(scan_path, &base_dir).await? {
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

        let mut dir = super::common::scan_directory(&versions_dir).await?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if let Some(entry) = self.scan_version(path, &base_dir).await? {
                self.state.write().await.push_multi(entry).await;
                app.invalidate(GET_IMPORT_SCAN_STATUS, None);
            }
        }

        Ok(())
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
        minecraft: value("VersionVanillaName").filter(|v| looks_like_minecraft_version(v)),
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
/// The running directory follows HMCL's own rule, see [`hmcl_runs_in_own_directory`].
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

    let run_dir = if hmcl_runs_in_own_directory(json) {
        json_trimmed_string(json, "runningDirectory")
            .map(PathBuf::from)
            .unwrap_or_else(|| version_root.to_path_buf())
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

/// Whether HMCL runs an instance in a directory of its own rather than the one
/// its preset gives. HMCL counts a running directory the file sets even when
/// `overrideProperties` does not list it, and a file without that list
/// inherits everything else.
fn hmcl_runs_in_own_directory(json: &serde_json::Value) -> bool {
    json.get("overrideProperties")
        .and_then(|list| list.as_array())
        .is_some_and(|list| {
            list.iter()
                .any(|key| key.as_str() == Some("runningDirectory"))
        })
        || json_trimmed_string(json, "runningDirectory").is_some()
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
            .unwrap_or_else(|| version_root.join("unresolved-custom-directory")),
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

    dedup_paths(candidates)
        .into_iter()
        .filter(|p| p.is_dir())
        .collect()
}

/// Only absolute paths can be resolved; relative ones are relative to the launcher's working directory.
async fn hmcl_game_directories(home: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    for file in [
        home.join("config").join("user-game-directories.json"),
        home.join("config").join("game-directories.json"),
    ] {
        for entry in read_hmcl_directory_list(&file).await.unwrap_or_default() {
            let path = PathBuf::from(entry.trim());
            if path.is_absolute() && path.is_dir() && !dirs.contains(&path) {
                dirs.push(path);
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
            "State:1\r\nVersionVanillaName:1.12.2\r\nVersionForge:14.23.5.2860\r\nVersionFabric:\r\nCustomInfo:my notes\r\nVersionArgumentIndieV2:True\r\nVersionRamType:1\r\nVersionRamCustom:20\r\nVersionAdvanceJvm:-XX:+UseG1GC\r\nVersionAdvanceRun:echo hi\r\nbroken line without colon\r\n",
        );
        assert_eq!(
            setup.get("VersionVanillaName").map(String::as_str),
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
    async fn a_java_path_that_is_gone_is_left_out() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let root = write_version(
            base,
            "Plain",
            serde_json::json!({ "id": "Plain", "inheritsFrom": "1.20.1" }),
        );
        let cfg_dir = root.join(".hmcl").join("config");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        std::fs::write(
            cfg_dir.join("instance-game-settings.json"),
            serde_json::json!({
                "overrideProperties": ["customJavaPath", "javaType"],
                "javaType": "CUSTOM",
                "customJavaPath": "/gdl-no-such-folder/bin/java"
            })
            .to_string(),
        )
        .unwrap();
        let Some(InternalImportEntry::Valid(entry)) =
            VersionFolderImporter::new(VersionFolderFlavor::Hmcl)
                .scan_version(root, base)
                .await
                .unwrap()
        else {
            panic!("a version with a stale Java path was refused")
        };
        assert_eq!(entry.settings.java_path, None);
        assert_eq!(
            ImportableInstance::from(entry).warnings,
            [Translation::InstanceImportJavaPathSkipped]
        );
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

#[cfg(test)]
mod task13_tests {
    use super::*;
    #[test]
    fn task13_pcl_exact_release_and_comparison_only() {
        for release in ["1.20.1", "24w14a"] {
            let setup = parse_pcl_ini(&format!(
                "VersionVanillaName:{release}\nVersionVanilla:20.0.1\n"
            ));
            assert_eq!(
                versions_from_pcl_setup(&setup).minecraft.as_deref(),
                Some(release)
            );
        }
        assert!(
            versions_from_pcl_setup(&parse_pcl_ini("VersionVanilla:20.0.1\n"))
                .minecraft
                .is_none()
        );
    }
    #[test]
    fn task13_custom_id_yields_to_authoritative_library() {
        let json = serde_json::json!({"id":"ATM9", "libraries":[{"name":"net.minecraftforge:forge:1.20.1-47.2.0"}]});
        let v = versions_from_hmcl_json(&json);
        assert_eq!(v.minecraft.as_deref(), Some("1.20.1"));
        assert_eq!(v.loaders[0].version, "47.2.0");
        assert_eq!(
            versions_from_hmcl_json(&serde_json::json!({"id":"1.20.4"}))
                .minecraft
                .as_deref(),
            Some("1.20.4")
        );
    }
}

#[cfg(test)]
mod task13_hmcl_tests {
    use super::*;
    #[tokio::test]
    async fn task13_inherited_running_directory_uses_associated_preset() {
        let t = tempfile::tempdir().unwrap();
        let base = t.path().join(".minecraft");
        let version = base.join("versions/Synthetic");
        let custom = t.path().join("custom");
        std::fs::create_dir_all(version.join(".hmcl/config")).unwrap();
        std::fs::create_dir(&custom).unwrap();
        // HMCL 3.16 layout: <dir>/.hmcl/config next to <dir>/.minecraft.
        std::fs::create_dir_all(t.path().join(".hmcl/config")).unwrap();
        std::fs::write(version.join("Synthetic.json"), r#"{"id":"1.20.1"}"#).unwrap();
        std::fs::write(version.join(".hmcl/config/instance-game-settings.json"), r#"{"parent":"game-settings-preset:123e4567-e89b-12d3-a456-426614174000","overrideProperties":[]}"#).unwrap();
        std::fs::write(t.path().join(".hmcl/config/game-settings.json"), serde_json::json!({"presets":[{"id":"game-settings-preset:123e4567-e89b-12d3-a456-426614174000","runningDirectory":custom}]}).to_string()).unwrap();
        let scanner = VersionFolderImporter::new(VersionFolderFlavor::Hmcl);
        let Some(InternalImportEntry::Valid(entry)) =
            scanner.scan_version(version, &base).await.unwrap()
        else {
            panic!("expected supported inheritance")
        };
        assert_eq!(entry.game_dir, custom);
    }
}

/// A running directory an HMCL preset gives relative to where HMCL runs, for
/// an install whose working directory cannot be told from its files.
#[derive(Debug, PartialEq, Eq)]
struct UnplaceableHmclRunDir(String);

/// The local home of an HMCL install (`HMCL_LOCAL_HOME`). It holds the presets
/// (`config/game-settings.json`), the launcher settings and the local game directory list.
///
/// HMCL puts it at `<working directory>/.hmcl` unless `hmcl.dir` or `HMCL_LOCAL_HOME`
/// moves it, so the default `.minecraft` game directory sits next to it, not inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct HmclLocalHome {
    dir: PathBuf,
    /// HMCL resolves relative paths against its working directory, which is only
    /// known for the default `.hmcl` layout.
    working_dir: Option<PathBuf>,
    /// `HMCL_USER_HOME` candidates. HMCL adds the directories in their
    /// `user-game-directories.json` to every install's list.
    user_homes: Vec<PathBuf>,
}

impl HmclLocalHome {
    fn at(dir: &Path) -> Option<Self> {
        Self::at_with(dir, hmcl_home_candidates())
    }

    fn at_with(dir: &Path, user_homes: Vec<PathBuf>) -> Option<Self> {
        if !dir.join("config").join("game-settings.json").is_file() {
            return None;
        }
        let working_dir = dir
            .parent()
            .filter(|_| dir.file_name().is_some_and(|name| name == ".hmcl"))
            .map(Path::to_path_buf);
        Some(Self {
            dir: dir.to_path_buf(),
            working_dir,
            user_homes,
        })
    }

    /// A selected folder stands for an HMCL install when it is the local home
    /// itself or the HMCL folder that holds `.hmcl`. A game folder is always
    /// scanned as one, even when HMCL was run from inside it.
    fn selected(path: &Path) -> Option<Self> {
        if is_game_directory(path) {
            return None;
        }
        Self::at(path).or_else(|| Self::at(&path.join(".hmcl")))
    }

    /// The install whose game directory list names `game_dir`: the `.hmcl` next
    /// to it, the one inside it when HMCL runs from the game folder, or the home
    /// `HMCL_LOCAL_HOME` points at.
    async fn of_game_dir(game_dir: &Path, local_home_env: Option<OsString>) -> Option<Self> {
        let beside = game_dir
            .parent()
            .and_then(|parent| Self::at(&parent.join(".hmcl")));
        let inside = Self::at(&game_dir.join(".hmcl"));
        // A moved home says nothing about where HMCL runs from.
        let moved = local_home_env
            .filter(|dir| !dir.is_empty())
            .and_then(|dir| Self::at(Path::new(&dir)))
            .map(|home| Self {
                working_dir: None,
                ..home
            });
        for home in [beside, inside, moved].into_iter().flatten() {
            if home.lists(game_dir).await {
                return Some(home);
            }
        }
        None
    }

    async fn lists(&self, game_dir: &Path) -> bool {
        let game_dir = path_clean(game_dir);
        self.game_directories()
            .await
            .iter()
            .any(|dir| path_clean(dir) == game_dir)
    }

    /// HMCL keeps absolute paths as written and resolves relative ones, the
    /// empty path included, against its working directory.
    fn resolve(&self, path: &str) -> Option<PathBuf> {
        let path = Path::new(path.trim());
        if path.has_root() {
            Some(path.to_path_buf())
        } else {
            self.working_dir
                .as_ref()
                .map(|dir| path_clean(dir.join(path)))
        }
    }

    /// The game directories HMCL lists for this install. Entries whose location
    /// cannot be worked out are left out.
    async fn game_directories(&self) -> Vec<PathBuf> {
        let config = self.dir.join("config");
        let local = read_hmcl_directory_list(&config.join("game-directories.json")).await;
        let mut entries = local.clone().unwrap_or_default();
        let user_lists = std::iter::once(config.join("user-game-directories.json"))
            .chain(
                self.user_homes
                    .iter()
                    .map(|home| home.join("config").join("user-game-directories.json")),
            )
            .collect();
        for list in dedup_paths(user_lists) {
            entries.extend(read_hmcl_directory_list(&list).await.unwrap_or_default());
        }
        // HMCL adds `.minecraft` when it creates the local list, or when no list names a directory.
        if local.is_none() || entries.is_empty() {
            entries.push(".minecraft".to_string());
        }
        dedup_paths(
            entries
                .iter()
                .filter_map(|entry| self.resolve(entry))
                .collect(),
        )
    }

    async fn has_game_directory(&self) -> bool {
        self.game_directories().await.iter().any(|dir| dir.is_dir())
    }
}

/// A folder the importer scans as a game folder rather than as an HMCL install.
fn is_game_directory(path: &Path) -> bool {
    path.join("versions").is_dir() || is_version_folder(path) || dir_name(path) == "versions"
}

/// The `path` of every entry in an HMCL game directory list, or `None` when the
/// list is missing or unreadable.
async fn read_hmcl_directory_list(path: &Path) -> Option<Vec<String>> {
    let json = read_hmcl_settings(path).await?;
    let Some(list) = json.get("directories").and_then(|list| list.as_array()) else {
        warn!(
            ?path,
            "Ignoring an HMCL game directory list without directories"
        );
        return None;
    };
    Some(
        list.iter()
            .filter_map(|entry| match entry {
                serde_json::Value::String(path) => Some(path.clone()),
                entry => json_string(entry, "path"),
            })
            .collect(),
    )
}

/// An HMCL settings file, or `None` when it is missing or cannot be read.
/// HMCL itself falls back to defaults for a file it cannot read, so the
/// importer does not refuse anything over one either.
async fn read_hmcl_settings(path: &Path) -> Option<serde_json::Value> {
    let text = match tokio::fs::read_to_string(path).await {
        Ok(text) => text,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                warn!(?path, %error, "Cannot read an HMCL settings file; using HMCL's defaults");
            }
            return None;
        }
    };
    match serde_json::from_str(&text) {
        Ok(json) => Some(json),
        Err(error) => {
            warn!(?path, %error, "Cannot parse an HMCL settings file; using HMCL's defaults");
            None
        }
    }
}

/// The default scan path offers an HMCL home only when one of its game folders
/// exists, so the automatic scan cannot open on an error.
async fn hmcl_home_has_game_directory(home: &Path) -> bool {
    match HmclLocalHome::at(home) {
        Some(home) => home.has_game_directory().await,
        None => false,
    }
}

/// The running directory a version inherits from its HMCL preset.
///
/// `Ok(None)` keeps the directory read from the version itself, as the importer
/// did before it read presets. With HMCL's default blank preset that is the game
/// directory HMCL uses too, and it is the safe choice whenever the presets
/// cannot be found or read: it never imports a folder the version does not use.
async fn inherited_hmcl_directory(
    version_root: &Path,
    base: &Path,
    source_home: Option<&HmclLocalHome>,
) -> Result<Option<PathBuf>, UnplaceableHmclRunDir> {
    if version_root.join("modpack.cfg").is_file() {
        return Ok(Some(version_root.to_path_buf()));
    }
    let config = version_root.join(".hmcl/config/instance-game-settings.json");
    let legacy_path = version_root.join("hmclversion.cfg");
    let local = if config.exists() {
        match read_hmcl_settings(&config).await {
            Some(local) => local,
            None => return Ok(None),
        }
    } else if legacy_path.exists() {
        // Modern instance settings supersede legacy settings only when they
        // exist. Migrated local intent must not be replaced by a default.
        match read_hmcl_settings(&legacy_path).await {
            Some(legacy) if json_bool(&legacy, "usesGlobal").unwrap_or(false) => {
                serde_json::json!({"overrideProperties":[]})
            }
            _ => return Ok(None),
        }
    } else if has_own_game_data(version_root) {
        // HMCL never configured it, yet it holds its own mods or worlds: another
        // launcher (PCL2) isolated it, and those files belong with it.
        return Ok(None);
    } else {
        serde_json::json!({"overrideProperties":[]})
    };
    if hmcl_runs_in_own_directory(&local) {
        return Ok(None);
    }
    let home = match source_home {
        Some(home) if home.lists(base).await => Some(home.clone()),
        _ => HmclLocalHome::of_game_dir(base, std::env::var_os("HMCL_LOCAL_HOME")).await,
    };
    let Some(home) = home else {
        return Ok(None);
    };
    let presets_path = home.dir.join("config").join("game-settings.json");
    let Some(presets) = read_hmcl_settings(&presets_path).await else {
        return Ok(None);
    };
    let Some(presets) = presets.get("presets").and_then(|v| v.as_array()) else {
        // HMCL starts over from a blank default preset.
        warn!(path = ?presets_path, "HMCL settings have no presets; using HMCL's defaults");
        return Ok(None);
    };
    let find = |id: String| {
        presets
            .iter()
            .find(|preset| json_string(preset, "id").as_ref() == Some(&id))
    };
    let default_id = read_json_value(&home.dir.join("config").join("launcher-settings.json"))
        .await
        .and_then(|settings| json_string(&settings, "defaultGameSettingsPreset"));
    // Like HMCL: a missing parent falls back to the default preset, then to the first one.
    let preset = json_string(&local, "parent")
        .and_then(find)
        .or_else(|| default_id.and_then(find))
        .or_else(|| presets.first());
    let Some(running_directory) =
        preset.and_then(|preset| json_trimmed_string(preset, "runningDirectory"))
    else {
        return Ok(None);
    };
    home.resolve(&running_directory)
        .map(Some)
        .ok_or(UnplaceableHmclRunDir(running_directory))
}

#[cfg(test)]
mod hmcl_home_tests {
    use super::super::ImportEntry;
    use super::*;

    fn write_json(path: &Path, json: serde_json::Value) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, json.to_string()).unwrap();
    }

    fn write_version(game_dir: &Path, id: &str) -> PathBuf {
        let root = game_dir.join("versions").join(id);
        write_json(
            &root.join(format!("{id}.json")),
            serde_json::json!({"id":"1.20.1"}),
        );
        root
    }

    async fn scan(root: &Path, base: &Path) -> InternalImportEntry<Importable> {
        VersionFolderImporter::new(VersionFolderFlavor::Hmcl)
            .scan_version(root.to_path_buf(), base)
            .await
            .unwrap()
            .unwrap()
    }

    fn game_dir(entry: InternalImportEntry<Importable>) -> PathBuf {
        match entry {
            InternalImportEntry::Valid(entry) => entry.game_dir,
            other => panic!("expected an importable version, got {other:?}"),
        }
    }

    fn invalid_reason(entry: InternalImportEntry<Importable>) -> Translation {
        match entry {
            InternalImportEntry::Invalid(entry) => entry.reason,
            other => panic!("expected an invalid row, got {other:?}"),
        }
    }

    /// Scans `path` the way the import modal does and returns its rows.
    async fn scan_rows(
        app: &Arc<AppInner>,
        path: &Path,
    ) -> (VersionFolderImporter, Vec<InternalImportEntry<Importable>>) {
        let scanner = VersionFolderImporter::new(VersionFolderFlavor::Hmcl);
        scanner.scan(app, path.to_path_buf()).await.unwrap();
        let rows = match scanner.state.read().await.clone() {
            ImporterState::NoResults => vec![],
            ImporterState::SingleResult(row) => vec![row],
            ImporterState::MultiResult(rows) => rows,
        };
        (scanner, rows)
    }

    fn valid(rows: &[InternalImportEntry<Importable>]) -> Vec<(String, PathBuf)> {
        rows.iter()
            .filter_map(|row| match row {
                InternalImportEntry::Valid(entry) => {
                    Some((entry.filename.clone(), entry.game_dir.clone()))
                }
                InternalImportEntry::Invalid(_) => None,
            })
            .collect()
    }

    fn invalid(rows: &[InternalImportEntry<Importable>]) -> Vec<(String, Translation)> {
        rows.iter()
            .filter_map(|row| match row {
                InternalImportEntry::Invalid(entry) => {
                    Some((entry.name.clone(), entry.reason.clone()))
                }
                InternalImportEntry::Valid(_) => None,
            })
            .collect()
    }

    #[tokio::test]
    async fn inherited_running_directory_is_read_from_the_hmcl_folder_beside_the_game_folder() {
        // HMCL 3.16 run from <t>: presets in <t>/.hmcl/config, games in <t>/.minecraft.
        let t = tempfile::tempdir().unwrap();
        let config = t.path().join(".hmcl/config");
        let base = t.path().join(".minecraft");
        let root = base.join("versions/Vanilla");
        let instance = root.join(".hmcl/config/instance-game-settings.json");
        write_json(
            &root.join("Vanilla.json"),
            serde_json::json!({"id":"1.20.1"}),
        );
        // Written when a shared (non-isolated) instance overrides any other setting.
        write_json(
            &instance,
            serde_json::json!({"overrideProperties":["maxMemory"],"maxMemory":4096}),
        );
        write_json(
            &config.join("game-directories.json"),
            serde_json::json!({"directories":[{"id":"d","path":".minecraft"}]}),
        );
        write_json(
            &config.join("launcher-settings.json"),
            serde_json::json!({"defaultGameSettingsPreset":"default"}),
        );
        let presets = |running_directory: serde_json::Value| {
            write_json(
                &config.join("game-settings.json"),
                serde_json::json!({"presets":[{"id":"default","runningDirectory":running_directory}]}),
            )
        };

        // HMCL's default blank preset runs the version in the shared game folder.
        presets(serde_json::json!(""));
        let InternalImportEntry::Valid(entry) = scan(&root, &base).await else {
            panic!("a version that inherits its running directory was refused")
        };
        assert_eq!(entry.game_dir, base);
        assert_eq!(entry.settings.memory_max_mb, Some(4096));

        let worlds = t.path().join("worlds");
        std::fs::create_dir(&worlds).unwrap();
        presets(serde_json::json!(worlds));
        assert_eq!(game_dir(scan(&root, &base).await), worlds);
        // Relative paths are relative to the folder HMCL runs from.
        presets(serde_json::json!("worlds"));
        assert_eq!(game_dir(scan(&root, &base).await), worlds);
        // Without a default, HMCL falls back to its first preset.
        std::fs::remove_file(config.join("launcher-settings.json")).unwrap();
        assert_eq!(game_dir(scan(&root, &base).await), worlds);

        // A file without overrideProperties overrides nothing, so the preset still decides.
        write_json(&instance, serde_json::json!({"maxMemory":4096}));
        assert_eq!(game_dir(scan(&root, &base).await), worlds);
        // A running directory the file sets is the instance's own, listed or not (as in HMCL).
        let own = t.path().join("own");
        std::fs::create_dir(&own).unwrap();
        for settings in [
            serde_json::json!({"runningDirectory":own}),
            serde_json::json!({"overrideProperties":["maxMemory"],"runningDirectory":own}),
        ] {
            write_json(&instance, settings);
            assert_eq!(game_dir(scan(&root, &base).await), own);
        }

        // A version HMCL never configured follows the preset, unless another
        // launcher isolated it and it holds its own mods or worlds.
        std::fs::remove_dir_all(root.join(".hmcl")).unwrap();
        assert_eq!(game_dir(scan(&root, &base).await), worlds);
        std::fs::create_dir(root.join("mods")).unwrap();
        assert_eq!(game_dir(scan(&root, &base).await), root);
        std::fs::remove_dir(root.join("mods")).unwrap();
        write_json(
            &instance,
            serde_json::json!({"overrideProperties":["maxMemory"],"maxMemory":4096}),
        );

        // A preset folder that is gone is a missing source, not a reason to import other data.
        presets(serde_json::json!(t.path().join("unplugged")));
        assert!(matches!(
            invalid_reason(scan(&root, &base).await),
            Translation::InstanceImportSourceUnavailable
        ));
        // Presets HMCL cannot read either are replaced by its blank default, so
        // the version runs in the shared game folder there and here.
        for broken in ["", "{", r#"{"$schema":"game-settings/9.0.0"}"#] {
            std::fs::write(config.join("game-settings.json"), broken).unwrap();
            assert_eq!(game_dir(scan(&root, &base).await), base, "{broken:?}");
        }

        // No HMCL folder next to the game folder: the game folder itself, as before.
        std::fs::remove_dir_all(t.path().join(".hmcl")).unwrap();
        assert_eq!(game_dir(scan(&root, &base).await), base);

        // A game folder with another name, listed nowhere, behaves the same.
        let other = t.path().join("games");
        let other_root = other.join("versions/Vanilla");
        std::fs::create_dir_all(other.join("versions")).unwrap();
        std::fs::rename(&root, &other_root).unwrap();
        assert_eq!(game_dir(scan(&other_root, &other).await), other);
        // An HMCL folder beside it that does not list it supplies no presets.
        presets(serde_json::json!(worlds));
        write_json(
            &config.join("game-directories.json"),
            serde_json::json!({"directories":[{"path":".minecraft"}]}),
        );
        assert_eq!(game_dir(scan(&other_root, &other).await), other);

        // Instance settings HMCL cannot parse do not refuse the version either:
        // its own folder layout decides, as it did before presets were read.
        let other_instance = other_root.join(".hmcl/config/instance-game-settings.json");
        write_json(
            &config.join("game-directories.json"),
            serde_json::json!({"directories":[{"path":other}]}),
        );
        write_json(
            &other_instance,
            serde_json::json!({"overrideProperties":[]}),
        );
        assert_eq!(game_dir(scan(&other_root, &other).await), worlds);
        std::fs::write(&other_instance, "{").unwrap();
        assert_eq!(game_dir(scan(&other_root, &other).await), other);
        std::fs::create_dir(other_root.join("saves")).unwrap();
        assert_eq!(game_dir(scan(&other_root, &other).await), other_root);
        std::fs::remove_dir(other_root.join("saves")).unwrap();
        // The same goes for a legacy hmclversion.cfg.
        std::fs::remove_dir_all(other_root.join(".hmcl")).unwrap();
        std::fs::write(other_root.join("hmclversion.cfg"), "{").unwrap();
        assert_eq!(game_dir(scan(&other_root, &other).await), other);
    }

    #[tokio::test]
    async fn selected_hmcl_folder_lists_game_folders_relative_to_where_hmcl_runs() {
        let t = tempfile::tempdir().unwrap();
        let hmcl = t.path().join("HMCL");
        let local = hmcl.join(".hmcl");
        let shared = hmcl.join(".minecraft");
        let external = t.path().join("external");
        let unplugged = t.path().join("usb/.minecraft");
        let official = t.path().join("official");
        let user = t.path().join("user-home");
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::create_dir_all(&external).unwrap();
        std::fs::create_dir_all(&official).unwrap();
        write_json(
            &local.join("config/game-settings.json"),
            serde_json::json!({"presets":[]}),
        );
        write_json(
            &local.join("config/game-directories.json"),
            serde_json::json!({"directories":[
                {"path":".minecraft"},
                {"path":external},
                {"path":unplugged},
                {"path":"../other/.minecraft"},
                {"path":""}
            ]}),
        );
        write_json(
            &user.join("config/user-game-directories.json"),
            serde_json::json!({"directories":[{"path":official}]}),
        );

        // Both the .hmcl folder and the HMCL folder holding it select the install.
        for selected in [&local, &hmcl] {
            let home = HmclLocalHome::selected(selected).unwrap();
            assert_eq!(home.dir, local);
            assert_eq!(home.working_dir.as_deref(), Some(hmcl.as_path()));
        }
        assert!(HmclLocalHome::selected(&shared).is_none());

        let home = HmclLocalHome::at_with(&local, vec![user.clone()]).unwrap();
        assert_eq!(
            home.game_directories().await,
            vec![
                shared.clone(),
                external.clone(),
                unplugged.clone(),
                t.path().join("other/.minecraft"),
                // An empty path is the folder HMCL runs from.
                hmcl.clone(),
                official.clone(),
            ]
        );
        assert!(home.lists(&shared).await);
        assert!(home.lists(&hmcl).await);

        // Without a local list HMCL uses the .minecraft beside its .hmcl folder.
        std::fs::remove_file(local.join("config/game-directories.json")).unwrap();
        let home = HmclLocalHome::at_with(&local, vec![]).unwrap();
        assert_eq!(home.game_directories().await, vec![shared.clone()]);
        assert!(home.has_game_directory().await);
        std::fs::remove_dir(&shared).unwrap();
        assert!(!home.has_game_directory().await);

        // A game folder HMCL was run from inside is still a game folder.
        write_version(&hmcl, "1.20.1");
        assert!(HmclLocalHome::selected(&hmcl).is_none());
        assert!(HmclLocalHome::selected(&hmcl.join("versions")).is_none());
        assert!(HmclLocalHome::selected(&hmcl.join("versions/1.20.1")).is_none());
        assert!(HmclLocalHome::selected(&local).is_some());

        // A moved home says nothing about where HMCL runs, so relative entries cannot be placed.
        let moved = t.path().join("moved");
        write_json(
            &moved.join("config/game-settings.json"),
            serde_json::json!({"presets":[]}),
        );
        let moved = HmclLocalHome::at_with(&moved, vec![]).unwrap();
        assert_eq!(moved.working_dir, None);
        assert!(moved.game_directories().await.is_empty());
    }

    #[tokio::test]
    async fn moved_hmcl_home_supplies_presets_only_for_the_folders_it_lists() {
        let t = tempfile::tempdir().unwrap();
        let moved = t.path().join("moved");
        let base = t.path().join("games");
        let root = base.join("versions/Vanilla");
        write_json(
            &root.join("Vanilla.json"),
            serde_json::json!({"id":"1.20.1"}),
        );
        write_json(
            &root.join(".hmcl/config/instance-game-settings.json"),
            serde_json::json!({"overrideProperties":[]}),
        );
        write_json(
            &moved.join("config/game-settings.json"),
            serde_json::json!({"presets":[{"id":"p","runningDirectory":"worlds"}]}),
        );
        write_json(
            &moved.join("config/game-directories.json"),
            serde_json::json!({"directories":[{"path":base}]}),
        );

        let env = Some(moved.clone().into_os_string());
        let home = HmclLocalHome::of_game_dir(&base, env.clone())
            .await
            .unwrap();
        assert_eq!(home.dir, moved);
        assert_eq!(home.working_dir, None);
        assert!(HmclLocalHome::of_game_dir(&base, None).await.is_none());
        assert!(
            HmclLocalHome::of_game_dir(&t.path().join("elsewhere"), env)
                .await
                .is_none()
        );
        // A relative inherited running directory cannot be placed without HMCL's working folder.
        assert_eq!(
            inherited_hmcl_directory(&root, &base, Some(&home)).await,
            Err(UnplaceableHmclRunDir("worlds".to_string()))
        );
        let scanner = VersionFolderImporter::new(VersionFolderFlavor::Hmcl);
        *scanner.source_home.write().await = Some(home);
        let row = scanner.scan_version(root, &base).await.unwrap().unwrap();
        assert!(matches!(
            invalid_reason(row),
            Translation::InstanceImportHmclRelativeRunDir
        ));
    }

    #[tokio::test]
    async fn scanning_an_hmcl_install_lists_the_versions_of_every_game_folder_it_names() {
        let env = crate::setup_managers_for_test().await;
        let app = &env.app;
        let t = tempfile::tempdir().unwrap();
        let hmcl = t.path().join("HMCL");
        let local = hmcl.join(".hmcl");
        let shared = hmcl.join(".minecraft");
        let root = write_version(&shared, "Vanilla");
        // A shared instance that overrides another setting inherits its running directory.
        write_json(
            &root.join(".hmcl/config/instance-game-settings.json"),
            serde_json::json!({"overrideProperties":["maxMemory"],"maxMemory":4096}),
        );
        write_json(
            &local.join("config/game-settings.json"),
            serde_json::json!({"presets":[{"id":"default","runningDirectory":""}]}),
        );
        write_json(
            &local.join("config/game-directories.json"),
            serde_json::json!({"directories":[
                {"path":".minecraft"},
                {"path":t.path().join("unplugged/.minecraft")}
            ]}),
        );

        // The .hmcl folder and the HMCL folder both list the shared folder's
        // versions; the stale entry is skipped rather than failing the scan.
        for selected in [&local, &hmcl] {
            let (scanner, rows) = scan_rows(app, selected).await;
            assert_eq!(
                valid(&rows),
                vec![("Vanilla".to_string(), shared.clone())],
                "{selected:?}"
            );
            assert!(invalid(&rows).is_empty(), "{rows:?}");
            assert_eq!(
                scanner.source_home.read().await.as_ref().map(|h| &h.dir),
                Some(&local)
            );
        }

        // A second listed folder with the same version id: each row shows its own folder.
        let external = t.path().join("external");
        let external_root = write_version(&external, "Vanilla");
        write_json(
            &local.join("config/game-directories.json"),
            serde_json::json!({"directories":[{"path":".minecraft"},{"path":external}]}),
        );
        let (scanner, rows) = scan_rows(app, &local).await;
        assert_eq!(
            valid(&rows),
            vec![
                ("Vanilla".to_string(), shared.clone()),
                ("Vanilla".to_string(), external.clone())
            ]
        );
        let ImportScanStatus::MultiResult(listed) = scanner.get_status().await else {
            panic!("expected one row per version")
        };
        let shown: Vec<_> = listed
            .into_iter()
            .map(|row| match row {
                ImportEntry::Valid(row) => (row.instance_name, row.filename),
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(
            shown,
            vec![
                ("Vanilla".to_string(), root.display().to_string()),
                ("Vanilla".to_string(), external_root.display().to_string())
            ]
        );
        // With a single folder the row shows the version id, as for any game folder.
        let (scanner, _) = scan_rows(app, &shared).await;
        let ImportScanStatus::MultiResult(listed) = scanner.get_status().await else {
            panic!("expected one row per version")
        };
        assert!(matches!(
            &listed[..],
            [ImportEntry::Valid(ImportableInstance { filename, .. })] if filename == "Vanilla"
        ));

        // When no listed folder exists, each one is named with a translated way out.
        std::fs::remove_dir_all(&external).unwrap();
        std::fs::rename(&shared, t.path().join("moved-away")).unwrap();
        let (_, rows) = scan_rows(app, &hmcl).await;
        assert!(valid(&rows).is_empty());
        assert_eq!(
            invalid(&rows),
            vec![
                (
                    shared.display().to_string(),
                    Translation::InstanceImportSourceUnavailable
                ),
                (
                    external.display().to_string(),
                    Translation::InstanceImportSourceUnavailable
                )
            ]
        );
    }

    #[tokio::test]
    async fn a_game_folder_hmcl_ran_from_is_scanned_as_a_game_folder() {
        let env = crate::setup_managers_for_test().await;
        let app = &env.app;
        let t = tempfile::tempdir().unwrap();
        // HMCL.exe placed inside an existing .minecraft and run once.
        let game = t.path().join("Game/.minecraft");
        let root = write_version(&game, "1.20.1");
        let home = game.join(".hmcl/config");
        write_json(
            &home.join("game-settings.json"),
            serde_json::json!({"presets":[{"id":"default","runningDirectory":""}]}),
        );
        write_json(
            &home.join("game-directories.json"),
            serde_json::json!({"directories":[{"path":".minecraft"}]}),
        );
        write_json(
            &root.join(".hmcl/config/instance-game-settings.json"),
            serde_json::json!({"overrideProperties":[]}),
        );

        for nested in [false, true] {
            if nested {
                std::fs::create_dir_all(game.join(".minecraft")).unwrap();
            }
            let (_, rows) = scan_rows(app, &game).await;
            assert_eq!(valid(&rows), vec![("1.20.1".to_string(), game.clone())]);
            assert!(invalid(&rows).is_empty(), "{rows:?}");
        }

        // When that HMCL lists the folder it runs from (an empty path), its presets apply.
        let worlds = t.path().join("worlds");
        std::fs::create_dir(&worlds).unwrap();
        write_json(
            &home.join("game-directories.json"),
            serde_json::json!({"directories":[{"path":".minecraft"},{"path":""}]}),
        );
        write_json(
            &home.join("game-settings.json"),
            serde_json::json!({"presets":[{"id":"default","runningDirectory":worlds}]}),
        );
        let (_, rows) = scan_rows(app, &game).await;
        assert_eq!(valid(&rows), vec![("1.20.1".to_string(), worlds)]);
    }

    #[tokio::test]
    async fn a_moved_hmcl_home_supplies_its_presets_to_the_folders_it_lists() {
        let env = crate::setup_managers_for_test().await;
        let app = &env.app;
        let t = tempfile::tempdir().unwrap();
        // HMCL_LOCAL_HOME pointed at <t>/cfg: only absolute entries can be placed.
        let moved = t.path().join("cfg");
        let games = t.path().join("games");
        let worlds = t.path().join("worlds");
        std::fs::create_dir(&worlds).unwrap();
        let root = write_version(&games, "Vanilla");
        write_json(
            &root.join(".hmcl/config/instance-game-settings.json"),
            serde_json::json!({"overrideProperties":[]}),
        );
        write_json(
            &moved.join("config/game-settings.json"),
            serde_json::json!({"presets":[{"id":"p","runningDirectory":worlds}]}),
        );
        write_json(
            &moved.join("config/game-directories.json"),
            serde_json::json!({"directories":[{"path":".minecraft"},{"path":games}]}),
        );

        let (_, rows) = scan_rows(app, &moved).await;
        assert_eq!(valid(&rows), vec![("Vanilla".to_string(), worlds)]);
        assert!(invalid(&rows).is_empty(), "{rows:?}");
        // Selected on its own, the game folder cannot be tied to that home.
        let (_, rows) = scan_rows(app, &games).await;
        assert_eq!(valid(&rows), vec![("Vanilla".to_string(), games.clone())]);

        // A home whose entries are all relative has nothing to place: the
        // selected folder is scanned like any other and holds no versions.
        write_json(
            &moved.join("config/game-directories.json"),
            serde_json::json!({"directories":[{"path":".minecraft"}]}),
        );
        let (_, rows) = scan_rows(app, &moved).await;
        assert!(rows.is_empty(), "{rows:?}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn an_unreadable_hmcl_game_folder_gets_its_own_row_and_does_not_stop_the_scan() {
        use std::os::unix::fs::PermissionsExt;

        let env = crate::setup_managers_for_test().await;
        let app = &env.app;
        let t = tempfile::tempdir().unwrap();
        let hmcl = t.path().join("HMCL");
        let locked = t.path().join("locked");
        write_version(&hmcl.join(".minecraft"), "Vanilla");
        write_version(&locked, "Other");
        write_json(
            &hmcl.join(".hmcl/config/game-settings.json"),
            serde_json::json!({"presets":[]}),
        );
        write_json(
            &hmcl.join(".hmcl/config/game-directories.json"),
            serde_json::json!({"directories":[{"path":locked},{"path":".minecraft"}]}),
        );
        let versions = locked.join("versions");
        std::fs::set_permissions(&versions, std::fs::Permissions::from_mode(0o000)).unwrap();
        if std::fs::read_dir(&versions).is_ok() {
            // Running with privileges that ignore permissions.
            std::fs::set_permissions(&versions, std::fs::Permissions::from_mode(0o755)).unwrap();
            return;
        }

        let (_, rows) = scan_rows(app, &hmcl).await;
        std::fs::set_permissions(&versions, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            valid(&rows),
            vec![("Vanilla".to_string(), hmcl.join(".minecraft"))]
        );
        let invalid = invalid(&rows);
        assert_eq!(invalid.len(), 1, "{invalid:?}");
        assert!(
            invalid[0].0.starts_with(&locked.display().to_string()),
            "{invalid:?}"
        );
        assert_eq!(invalid[0].1, Translation::InstanceImportSourceUnavailable);
    }
}

#[cfg(test)]
mod task13_hmcl_controls {
    use super::*;
    #[tokio::test]
    async fn task13_hmcl_external_default_and_unavailable_context() {
        let t = tempfile::tempdir().unwrap();
        let home = t.path().join("home");
        let base = t.path().join("external-game");
        let root = base.join("versions/Synthetic");
        let custom = t.path().join("custom");
        std::fs::create_dir_all(home.join("config")).unwrap();
        std::fs::create_dir_all(root.join(".hmcl/config")).unwrap();
        std::fs::create_dir(&custom).unwrap();
        std::fs::write(root.join("Synthetic.json"), r#"{"id":"1.20.1"}"#).unwrap();
        std::fs::write(
            home.join("config/game-directories.json"),
            serde_json::json!({"directories":[{"path":base}]}).to_string(),
        )
        .unwrap();
        std::fs::write(
            home.join("config/launcher-settings.json"),
            r#"{"defaultGameSettingsPreset":"default-id"}"#,
        )
        .unwrap();
        std::fs::write(home.join("config/game-settings.json"), serde_json::json!({"presets":[{"id":"unrelated","runningDirectory":t.path().join("wrong")},{"id":"default-id","runningDirectory":custom}]}).to_string()).unwrap();
        std::fs::write(
            root.join(".hmcl/config/instance-game-settings.json"),
            r#"{"overrideProperties":[]}"#,
        )
        .unwrap();
        let scanner = VersionFolderImporter::new(VersionFolderFlavor::Hmcl);
        // Without its HMCL home the game directory itself is used, as HMCL does with a blank preset.
        let Some(InternalImportEntry::Valid(entry)) =
            scanner.scan_version(root.clone(), &base).await.unwrap()
        else {
            panic!("a version without its HMCL home was refused")
        };
        assert_eq!(entry.game_dir, base);
        *scanner.source_home.write().await = Some(HmclLocalHome::at_with(&home, vec![]).unwrap());
        let Some(InternalImportEntry::Valid(entry)) =
            scanner.scan_version(root.clone(), &base).await.unwrap()
        else {
            panic!("associated default was not resolved")
        };
        assert_eq!(entry.game_dir, custom);
        std::fs::remove_dir(&custom).unwrap();
        assert!(matches!(
            scanner.scan_version(root.clone(), &base).await.unwrap(),
            Some(InternalImportEntry::Invalid(_))
        ));
        std::fs::write(root.join("modpack.cfg"), "{}").unwrap();
        let Some(InternalImportEntry::Valid(entry)) =
            scanner.scan_version(root.clone(), &base).await.unwrap()
        else {
            panic!("modpack isolation was lost")
        };
        assert_eq!(entry.game_dir, root);
    }
}

async fn initialize_version_copy(instance: &Importable, destination: &Path) -> anyhow::Result<()> {
    super::common::require_source_directory(&instance.version_root).await?;
    let version_id = instance.filename.clone();
    copy_game_dir(&instance.game_dir, destination, move |path| {
        !is_launcher_managed(path) && !is_version_folder_file(path, &version_id)
    })
    .await
}

#[cfg(test)]
mod task13_round01_tests {
    use super::*;
    fn version(base: &Path, id: &str, json: serde_json::Value) -> PathBuf {
        let root = base.join("versions").join(id);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(format!("{id}.json")), json.to_string()).unwrap();
        root
    }
    #[tokio::test]
    async fn task13_round01_renamed_digit_id_uses_library_or_exact_cache() {
        let t = tempfile::tempdir().unwrap();
        for libraries in [
            serde_json::json!([{"name":"net.minecraftforge:forge:1.20.1-47.2.0"}]),
            serde_json::json!([]),
        ] {
            let root = version(
                t.path(),
                "1.20.1-ATM9",
                serde_json::json!({"id":"1.20.1-ATM9","libraries":libraries}),
            );
            std::fs::create_dir_all(root.join("PCL")).unwrap();
            std::fs::write(
                root.join("PCL/Setup.ini"),
                "VersionVanillaName:1.20.1
VersionVanilla:20.0.1
",
            )
            .unwrap();
            let Some(InternalImportEntry::Valid(entry)) =
                VersionFolderImporter::new(VersionFolderFlavor::Pcl2)
                    .scan_version(root, t.path())
                    .await
                    .unwrap()
            else {
                panic!("missing proven release")
            };
            assert_eq!(entry.minecraft_version, "1.20.1");
        }
    }
    #[tokio::test]
    async fn task13_round01_legacy_local_directory_survives_modern_default() {
        let t = tempfile::tempdir().unwrap();
        let base = t.path().join(".minecraft");
        let root = version(&base, "Legacy", serde_json::json!({"id":"1.20.1"}));
        let custom = t.path().join("local-worlds");
        let unrelated = t.path().join("default-worlds");
        std::fs::create_dir(&custom).unwrap();
        std::fs::create_dir(&unrelated).unwrap();
        std::fs::create_dir_all(t.path().join(".hmcl/config")).unwrap();
        std::fs::write(
            t.path().join(".hmcl/config/game-settings.json"),
            serde_json::json!({"presets":[{"id":"default","runningDirectory":unrelated}]})
                .to_string(),
        )
        .unwrap();
        std::fs::write(
            t.path().join(".hmcl/config/launcher-settings.json"),
            r#"{"defaultGameSettingsPreset":"default"}"#,
        )
        .unwrap();
        for (kind, expected) in [(1, root.clone()), (2, custom.clone())] {
            std::fs::write(
                root.join("hmclversion.cfg"),
                serde_json::json!({"usesGlobal":false,"gameDirType":kind,"gameDir":custom})
                    .to_string(),
            )
            .unwrap();
            let Some(InternalImportEntry::Valid(entry)) =
                VersionFolderImporter::new(VersionFolderFlavor::Hmcl)
                    .scan_version(root.clone(), &base)
                    .await
                    .unwrap()
            else {
                panic!("legacy local intent rejected")
            };
            assert_eq!(entry.game_dir, expected);
        }
    }
    #[tokio::test]
    async fn task13_round01_removed_selected_root_with_present_payload_fails() {
        let t = tempfile::tempdir().unwrap();
        let root = version(t.path(), "Selected", serde_json::json!({"id":"1.20.1"}));
        let Some(InternalImportEntry::Valid(entry)) =
            VersionFolderImporter::new(VersionFolderFlavor::Pcl2)
                .scan_version(root.clone(), t.path())
                .await
                .unwrap()
        else {
            panic!("missing cached row")
        };
        let destination = tempfile::tempdir().unwrap();
        std::fs::rename(&root, t.path().join("moved-selected")).unwrap();
        let error = initialize_version_copy(&entry, destination.path())
            .await
            .expect_err("a removed selected root must require reconnection");
        assert!(error.to_string().contains(root.to_str().unwrap()));
        assert!(!destination.path().join(".setup/modpack-complete").exists());
    }
}

#[cfg(test)]
mod task13_round01_controls {
    use super::*;
    #[tokio::test]
    async fn task13_round01_composed_schema_and_present_root_controls() {
        let t = tempfile::tempdir().unwrap();
        let base = t.path().join(".minecraft");
        let root = base.join("versions/Synthetic");
        let custom = t.path().join("custom");
        let defaults = t.path().join("default");
        std::fs::create_dir_all(root.join(".hmcl/config")).unwrap();
        std::fs::create_dir(&custom).unwrap();
        std::fs::create_dir(&defaults).unwrap();
        std::fs::create_dir_all(t.path().join(".hmcl/config")).unwrap();
        std::fs::write(root.join("Synthetic.json"), r#"{"id":"1.20.1"}"#).unwrap();
        std::fs::write(
            root.join("hmclversion.cfg"),
            r#"{"usesGlobal":false,"gameDirType":1}"#,
        )
        .unwrap();
        std::fs::write(
            t.path().join(".hmcl/config/game-settings.json"),
            serde_json::json!({"presets":[{"id":"default","runningDirectory":defaults}]})
                .to_string(),
        )
        .unwrap();
        std::fs::write(
            t.path().join(".hmcl/config/launcher-settings.json"),
            r#"{"defaultGameSettingsPreset":"default"}"#,
        )
        .unwrap();
        let scanner = VersionFolderImporter::new(VersionFolderFlavor::Hmcl);
        let modern = root.join(".hmcl/config/instance-game-settings.json");
        for (settings, expected) in [
            (
                serde_json::json!({"overrideProperties":["runningDirectory"],"runningDirectory":custom}),
                custom.clone(),
            ),
            (
                serde_json::json!({"overrideProperties":[]}),
                defaults.clone(),
            ),
        ] {
            std::fs::write(&modern, settings.to_string()).unwrap();
            let Some(InternalImportEntry::Valid(entry)) =
                scanner.scan_version(root.clone(), &base).await.unwrap()
            else {
                panic!("modern settings not resolved")
            };
            assert_eq!(entry.game_dir, expected);
        }
        std::fs::write(root.join("modpack.cfg"), "{}").unwrap();
        let Some(InternalImportEntry::Valid(entry)) =
            scanner.scan_version(root.clone(), &base).await.unwrap()
        else {
            panic!("modpack isolation rejected")
        };
        assert_eq!(entry.game_dir, root);
        std::fs::remove_file(root.join("modpack.cfg")).unwrap();
        std::fs::remove_file(modern).unwrap();
        std::fs::write(
            root.join("hmclversion.cfg"),
            r#"{"usesGlobal":true,"gameDirType":0}"#,
        )
        .unwrap();
        let Some(InternalImportEntry::Valid(entry)) =
            scanner.scan_version(root.clone(), &base).await.unwrap()
        else {
            panic!("legacy inheritance rejected")
        };
        assert_eq!(entry.game_dir, defaults);
        // Missing modern default cannot invalidate an explicit legacy local root.
        std::fs::remove_file(t.path().join(".hmcl/config/launcher-settings.json")).unwrap();
        std::fs::write(
            root.join("hmclversion.cfg"),
            r#"{"usesGlobal":false,"gameDirType":1}"#,
        )
        .unwrap();
        let Some(InternalImportEntry::Valid(entry)) =
            scanner.scan_version(root.clone(), &base).await.unwrap()
        else {
            panic!("legacy local source became inherited")
        };
        assert_eq!(entry.game_dir, root);
        // Present selected root and separate common payload remain importable.
        let Some(InternalImportEntry::Valid(entry)) =
            VersionFolderImporter::new(VersionFolderFlavor::Pcl2)
                .scan_version(root, &base)
                .await
                .unwrap()
        else {
            panic!("common payload rejected")
        };
        assert_eq!(entry.game_dir, base);
        std::fs::write(base.join("options.txt"), b"synthetic options").unwrap();
        let destination = tempfile::tempdir().unwrap();
        initialize_version_copy(&entry, destination.path())
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(destination.path().join("instance/options.txt")).unwrap(),
            b"synthetic options"
        );
        assert!(destination.path().join(".setup/modpack-complete").exists());
    }
}

#[cfg(test)]
mod task13_round02_tests {
    use super::*;
    #[tokio::test]
    async fn task13_round02_published_release_families_scan_as_valid() {
        let t = tempfile::tempdir().unwrap();
        let mut rejected = Vec::new();
        for id in [
            "26.4-snapshot-2",
            "26.3-pre-1",
            "26.3-rc-1",
            "25w14craftmine",
            "24w14potato",
            "23w13a_or_b",
            "20w14infinite",
            "1.RV-Pre1",
            "b1.3b",
            "a1.2.2a",
            "a1.2.2b",
            "c0.0.13a",
            "c0.0.13a_03",
            "c0.0.11a",
        ] {
            let root = t.path().join("versions").join(id);
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(
                root.join(format!("{id}.json")),
                serde_json::json!({"id":id,"libraries":[{"name":"org.lwjgl:lwjgl:3.3.3"}]})
                    .to_string(),
            )
            .unwrap();
            match VersionFolderImporter::new(VersionFolderFlavor::Hmcl)
                .scan_version(root, t.path())
                .await
                .unwrap()
            {
                Some(InternalImportEntry::Valid(entry)) if entry.minecraft_version == id => {}
                other => rejected.push((id, format!("{other:?}"))),
            }
        }
        assert!(
            rejected.is_empty(),
            "{} published representative IDs failed composed scan: {rejected:?}",
            rejected.len()
        );
    }
}
