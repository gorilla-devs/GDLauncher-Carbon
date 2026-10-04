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
        DeclaredModLoader, ImportedSettings, SkippedSetting, apply_imported_settings,
        copy_game_dir, datetime_from_epoch_millis, dedup_paths, dir_name,
        find_image_with_any_extension, first_existing_dir, group_for_import,
        normalize_forge_version, parse_loader_type, read_json_value, resolve_standard_version,
        seconds_played_from_i64,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmcFlavor {
    MultiMC,
    Prism,
}

impl MmcFlavor {
    fn label(self) -> &'static str {
        match self {
            Self::MultiMC => "MultiMC",
            Self::Prism => "Prism Launcher",
        }
    }
}

#[derive(Debug, Clone)]
struct Importable {
    filename: String,
    path: PathBuf,
    game_dir: PathBuf,
    game_dir_required: bool,
    name: String,
    icon_path: Option<PathBuf>,
    minecraft_version: String,
    loaders: Vec<DeclaredModLoader>,
    managed: Option<Modpack>,
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
pub struct MmcImporter {
    flavor: MmcFlavor,
    state: RwLock<ImporterState<Importable>>,
}

impl MmcImporter {
    pub fn new(flavor: MmcFlavor) -> Self {
        Self {
            flavor,
            state: RwLock::new(ImporterState::NoResults),
        }
    }

    /// Honours a custom `InstanceDir` from the launcher's config.
    pub async fn get_default_scan_path(flavor: MmcFlavor) -> anyhow::Result<Option<PathBuf>> {
        let Some(data_dir) = first_existing_dir(data_dir_candidates(flavor)) else {
            return Ok(None);
        };

        Ok(Some(instances_dir_for_data_dir(&data_dir).await))
    }

    async fn scan_instance(
        &self,
        path: PathBuf,
    ) -> anyhow::Result<Option<InternalImportEntry<Importable>>> {
        let cfg_path = path.join("instance.cfg");
        if !cfg_path.is_file() {
            return Ok(None);
        }

        let filename = dir_name(&path);

        let cfg_text = match tokio::fs::read_to_string(&cfg_path).await {
            Ok(text) => text,
            Err(e) => {
                warn!("Cannot read {cfg_path:?}: {e}");
                return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                    name: filename,
                    reason: Translation::InstanceImportConfigUnreadable,
                })));
            }
        };

        let pack_text = tokio::fs::read_to_string(path.join("mmc-pack.json"))
            .await
            .ok();

        let Some(mut meta) = parse_mmc_meta(&cfg_text, pack_text.as_deref()) else {
            trace!(
                "{} instance at {path:?} declares no Minecraft version, skipping",
                self.flavor.label()
            );
            return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                name: filename,
                reason: Translation::InstanceImportLegacyBadConfigFile,
            })));
        };

        meta.settings.skip_missing_java_path();
        let icon_path = find_icon(&path, meta.icon_key.as_deref()).await;

        if let Some(instances_dir) = path.parent() {
            meta.settings.group = group_of_instance(instances_dir, &filename).await;
        }

        let game_dir_required = match game_dir(&path).try_exists() {
            Ok(exists) => exists,
            Err(e) => {
                warn!("Cannot check the game folder of {path:?}: {e}");
                return Ok(Some(InternalImportEntry::Invalid(InvalidImportEntry {
                    name: filename,
                    reason: Translation::InstanceImportSourceUnavailable,
                })));
            }
        };

        Ok(Some(InternalImportEntry::Valid(Importable {
            filename: filename.clone(),
            game_dir_required,
            game_dir: game_dir(&path),
            path,
            name: meta.name.unwrap_or(filename),
            icon_path,
            minecraft_version: meta.minecraft_version,
            loaders: meta.loaders,
            managed: meta.managed,
            seconds_played: meta.seconds_played,
            last_played: meta.last_played,
            settings: meta.settings,
        })))
    }
}

async fn group_of_instance(instances_dir: &Path, instance_dir_name: &str) -> Option<String> {
    let json = read_json_value(&instances_dir.join("instgroups.json")).await?;
    let groups = json.get("groups")?.as_object()?;

    groups.iter().find_map(|(name, group)| {
        let members = group.get("instances")?.as_array()?;
        let is_member = members
            .iter()
            .filter_map(|m| m.as_str())
            .any(|m| m == instance_dir_name);
        (is_member && !name.trim().is_empty()).then(|| name.trim().to_string())
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MmcMeta {
    pub name: Option<String>,
    pub icon_key: Option<String>,
    pub minecraft_version: String,
    pub loaders: Vec<DeclaredModLoader>,
    pub managed: Option<Modpack>,
    pub seconds_played: Option<u32>,
    pub last_played: Option<DateTime<Utc>>,
    pub settings: ImportedSettings,
}

pub(super) fn parse_mmc_meta(cfg_text: &str, pack_text: Option<&str>) -> Option<MmcMeta> {
    let cfg = parse_instance_cfg(cfg_text);
    let pack = pack_text.map(parse_mmc_pack).unwrap_or_default();
    let versions = resolve_versions(&cfg, &pack)?;

    let cfg_value = |key: &str| {
        cfg.get(key)
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };

    Some(MmcMeta {
        name: cfg_value("name"),
        icon_key: cfg_value("iconKey"),
        minecraft_version: versions.minecraft,
        loaders: versions.loaders,
        managed: managed_pack(&cfg),
        seconds_played: cfg_value("totalTimePlayed")
            .and_then(|v| v.parse::<i64>().ok())
            .and_then(seconds_played_from_i64),
        last_played: cfg_value("lastLaunchTime")
            .and_then(|v| v.parse::<i64>().ok())
            .and_then(datetime_from_epoch_millis),
        settings: instance_settings(&cfg),
    })
}

/// Each group of keys only counts when its `Override*` switch is on.
fn instance_settings(cfg: &HashMap<String, String>) -> ImportedSettings {
    let value = |key: &str| {
        cfg.get(key)
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };
    let enabled = |key: &str| cfg.get(key).is_some_and(|v| parse_bool(v));
    let number = |key: &str| {
        value(key)
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|n| *n > 0)
    };

    let mut settings = ImportedSettings {
        notes: value("notes"),
        ..Default::default()
    };

    if enabled("OverrideMemory") {
        settings.memory_min_mb = number("MinMemAlloc");
        settings.memory_max_mb = number("MaxMemAlloc");
    }
    if enabled("OverrideJavaArgs") {
        if let Some(args) = value("JvmArgs") {
            settings.absorb_jvm_args(&args);
        }
    }
    if enabled("OverrideJavaLocation") {
        settings.java_path = value("JavaPath");
    }
    if enabled("OverrideCommands") || enabled("OverrideLaunchCmd") {
        settings.pre_launch_hook = value("PreLaunchCommand");
        settings.post_exit_hook = value("PostExitCommand");
        settings.wrapper_command = value("WrapperCommand");
        skip_launcher_variable_commands(&mut settings);
    }
    if enabled("OverrideWindow") {
        if let (Some(w), Some(h)) = (number("MinecraftWinWidth"), number("MinecraftWinHeight")) {
            settings.game_resolution = Some((w, h));
        }
    }

    settings
}

#[async_trait::async_trait]
impl InstanceImporter for MmcImporter {
    async fn scan(&self, app: &Arc<AppInner>, scan_path: PathBuf) -> anyhow::Result<()> {
        // Pointing at instance.cfg itself is the same as pointing at its folder.
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

        if scan_path.join("instance.cfg").is_file() {
            if let Some(entry) = self.scan_instance(scan_path).await? {
                self.state.write().await.set_single(entry).await;
                app.invalidate(GET_IMPORT_SCAN_STATUS, None);
            }
            return Ok(());
        }

        // The launcher's data directory is accepted as well as its instances directory.
        let data_dir = is_launcher_data_dir(&scan_path).then(|| scan_path.clone());
        let instances_dir = match &data_dir {
            Some(data_dir) => instances_dir_for_data_dir(data_dir).await,
            None => scan_path,
        };

        let mut dir = match super::common::scan_directory(&instances_dir).await {
            Ok(dir) => dir,
            // A data folder without instances has nothing to import, unless
            // its settings point at a folder that is gone (an unplugged drive).
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let Some(data_dir) = data_dir else {
                    return Err(e.into());
                };
                return match configured_instances_dir(&data_dir).await {
                    Some(configured)
                        if configured != data_dir.join("instances") && !configured.is_dir() =>
                    {
                        Err(anyhow!(e).context(format!(
                            "{}'s instances folder {} is not available",
                            self.flavor.label(),
                            configured.display()
                        )))
                    }
                    _ => Ok(()),
                };
            }
            Err(e) => return Err(e.into()),
        };

        while let Some(entry) = dir.next_entry().await? {
            let entry_path = entry.path();
            if !entry_path.is_dir() {
                continue;
            }

            if let Some(entry) = self.scan_instance(entry_path).await? {
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
        let version = GameVersion::Standard(standard_version);

        let instance_version_source = match &instance.managed {
            Some(modpack) => {
                InstanceVersionSource::ModpackWithKnownVersion(version, modpack.clone(), true)
            }
            None => InstanceVersionSource::Version(version),
        };

        let initializer = |instance_path: PathBuf| {
            let instance = &instance;
            async move {
                anyhow::ensure!(
                    tokio::fs::metadata(&instance.path).await?.is_dir(),
                    "Import source {} is unavailable; rescan",
                    instance.path.display()
                );
                if instance.game_dir_required {
                    copy_game_dir(&instance.game_dir, &instance_path, |_| true).await?;
                } else {
                    super::common::copy_optional_game_dir(
                        &instance.game_dir,
                        &instance_path,
                        |_| true,
                    )
                    .await?;
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

pub(super) fn game_dir(instance_root: &Path) -> PathBuf {
    let dot = instance_root.join(".minecraft");
    let plain = instance_root.join("minecraft");
    if plain.is_dir() && !dot.is_dir() {
        plain
    } else {
        dot
    }
}

async fn instances_dir_for_data_dir(data_dir: &Path) -> PathBuf {
    if let Some(cfg) = read_launcher_cfg(data_dir).await {
        if let Some(inst_dir) = cfg
            .get("InstanceDir")
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
        {
            let inst_dir = PathBuf::from(inst_dir);
            let resolved = if inst_dir.is_absolute() {
                inst_dir
            } else {
                data_dir.join(inst_dir)
            };
            if resolved.is_dir() {
                return resolved;
            }
        }
    }

    data_dir.join("instances")
}

const LAUNCHER_CFG_NAMES: [&str; 3] = ["prismlauncher.cfg", "polymc.cfg", "multimc.cfg"];

async fn read_launcher_cfg(data_dir: &Path) -> Option<HashMap<String, String>> {
    for name in LAUNCHER_CFG_NAMES {
        let path = data_dir.join(name);
        if let Ok(text) = tokio::fs::read_to_string(&path).await {
            return Some(parse_instance_cfg(&text));
        }
    }
    None
}

fn is_launcher_data_dir(path: &Path) -> bool {
    LAUNCHER_CFG_NAMES
        .iter()
        .any(|name| path.join(name).is_file())
        || (path.join("instances").is_dir() && !path.join("instance.cfg").is_file())
}

/// Built-in icons (`default`, `bee`, ...) have no file and yield `None`.
async fn find_icon(instance_root: &Path, icon_key: Option<&str>) -> Option<PathBuf> {
    let key = icon_key?.trim();
    if key.is_empty() || key == "default" {
        return None;
    }

    let instances_dir = instance_root.parent()?;
    let data_dir = instances_dir.parent()?;

    let mut icon_dirs = vec![data_dir.join("icons"), instances_dir.join("icons")];

    if let Some(cfg) = read_launcher_cfg(data_dir).await {
        if let Some(dir) = cfg
            .get("IconsDir")
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
        {
            let dir = PathBuf::from(dir);
            icon_dirs.insert(
                0,
                if dir.is_absolute() {
                    dir
                } else {
                    data_dir.join(dir)
                },
            );
        }
    }

    icon_dirs
        .iter()
        .find_map(|dir| find_image_with_any_extension(dir, key))
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ParsedPack {
    minecraft: Option<String>,
    loaders: Vec<DeclaredModLoader>,
}

/// A broken file yields an empty pack so the `instance.cfg` fallback still applies.
fn parse_mmc_pack(text: &str) -> ParsedPack {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return ParsedPack::default();
    };

    let mut pack = ParsedPack::default();

    let Some(components) = value.get("components").and_then(|c| c.as_array()) else {
        return pack;
    };

    for component in components {
        let Some(uid) = component.get("uid").and_then(|u| u.as_str()) else {
            continue;
        };

        let version = ["version", "cachedVersion"]
            .iter()
            .filter_map(|key| component.get(key).and_then(|v| v.as_str()))
            .map(str::trim)
            .find(|v| !v.is_empty());

        let Some(version) = version else {
            continue;
        };

        let uid_lower = uid.to_ascii_lowercase();
        if uid_lower == "net.minecraft" {
            pack.minecraft = Some(version.to_string());
            continue;
        }

        let loader_type = match uid_lower.as_str() {
            "net.minecraftforge" => Some(ModLoaderType::Forge),
            "net.neoforged" => Some(ModLoaderType::Neoforge),
            "net.fabricmc.fabric-loader" => Some(ModLoaderType::Fabric),
            "org.quiltmc.quilt-loader" => Some(ModLoaderType::Quilt),
            other if other.contains("intermediary") || other.contains("lwjgl") => None,
            other => parse_loader_type(other),
        };

        if let Some(type_) = loader_type {
            if !pack.loaders.iter().any(|l| l.type_ == type_) {
                pack.loaders.push(DeclaredModLoader::new(type_, version));
            }
        }
    }

    pack
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedVersions {
    minecraft: String,
    loaders: Vec<DeclaredModLoader>,
}

fn resolve_versions(cfg: &HashMap<String, String>, pack: &ParsedPack) -> Option<ResolvedVersions> {
    let cfg_value = |key: &str| {
        cfg.get(key)
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };

    let minecraft = pack
        .minecraft
        .clone()
        .or_else(|| cfg_value("IntendedVersion"))
        .or_else(|| cfg_value("MinecraftVersion"))?;

    let mut loaders = pack.loaders.clone();

    if loaders.is_empty() {
        if let Some(forge) = cfg_value("ForgeVersion") {
            loaders.push(DeclaredModLoader::new(
                ModLoaderType::Forge,
                normalize_forge_version(&minecraft, &forge),
            ));
        }
    }

    for loader in &mut loaders {
        if loader.type_ == ModLoaderType::Forge {
            loader.version = normalize_forge_version(&minecraft, &loader.version);
        }
    }

    Some(ResolvedVersions { minecraft, loaders })
}

/// Only "flame" (CurseForge) and Modrinth packs can be linked back to their platform.
fn managed_pack(cfg: &HashMap<String, String>) -> Option<Modpack> {
    let managed = cfg
        .get("ManagedPack")
        .map(|v| parse_bool(v))
        .unwrap_or(false);
    if !managed {
        return None;
    }

    let pack_type = cfg.get("ManagedPackType")?.trim().to_ascii_lowercase();
    let id = cfg.get("ManagedPackID")?.trim();
    let version_id = cfg.get("ManagedPackVersionID")?.trim();
    if id.is_empty() || version_id.is_empty() {
        return None;
    }

    match pack_type.as_str() {
        "flame" | "curseforge" => Some(Modpack::Curseforge(CurseforgeModpack {
            project_id: id.parse().ok()?,
            file_id: version_id.parse().ok()?,
        })),
        "modrinth" => Some(Modpack::Modrinth(ModrinthModpack {
            project_id: id.to_string(),
            version_id: version_id.to_string(),
        })),
        _ => None,
    }
}

fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "on"
    )
}

/// Keys outside `[General]` are prefixed with `Section/`.
pub(crate) fn parse_instance_cfg(text: &str) -> HashMap<String, String> {
    // Prism decides between the two formats by the presence of ConfigVersion.
    let old_format = !text.lines().any(|l| {
        let l = l.trim_start();
        l.starts_with('[') || l.starts_with("ConfigVersion")
    });

    let mut map = HashMap::new();
    let mut section: Option<String> = None;

    for raw_line in text.lines() {
        let mut line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if line.starts_with('[') {
            let name = line.trim_start_matches('[').trim_end_matches(']').trim();
            section = if name.eq_ignore_ascii_case("General") || name.is_empty() {
                None
            } else {
                Some(name.to_string())
            };
            continue;
        }

        if old_format {
            if let Some(idx) = find_unescaped_hash(line) {
                line = line[..idx].trim();
            }
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let key = unescape_key(key.trim());
        if key.is_empty() {
            continue;
        }

        let value = unquote(&unescape_value(value.trim()));

        let full_key = match &section {
            Some(section) => format!("{section}/{key}"),
            None => key,
        };
        map.insert(full_key, value);
    }

    map
}

fn find_unescaped_hash(line: &str) -> Option<usize> {
    let mut prev_backslash = false;
    for (idx, c) in line.char_indices() {
        if c == '#' && !prev_backslash {
            return Some(idx);
        }
        prev_backslash = c == '\\' && !prev_backslash;
    }
    None
}

/// QSettings percent-encodes unusual characters in keys.
fn unescape_key(key: &str) -> String {
    let bytes = key.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() && key.is_char_boundary(i + 3) {
            if let Ok(byte) = u8::from_str_radix(&key[i + 1..i + 3], 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| key.to_string())
}

fn unescape_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }

        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('0') => out.push('\0'),
            Some('x') => {
                let mut hex = String::new();
                while hex.len() < 4 {
                    match chars.peek() {
                        Some(h) if h.is_ascii_hexdigit() => {
                            hex.push(*h);
                            chars.next();
                        }
                        _ => break,
                    }
                }
                match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    Some(decoded) if !hex.is_empty() => out.push(decoded),
                    _ => {
                        out.push_str("\\x");
                        out.push_str(&hex);
                    }
                }
            }
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }

    out
}

fn unquote(value: &str) -> String {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

fn data_dir_candidates(flavor: MmcFlavor) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    let Some(basedirs) = directories::BaseDirs::new() else {
        return candidates;
    };
    let home = basedirs.home_dir().to_path_buf();
    let data = basedirs.data_dir().to_path_buf();

    match flavor {
        MmcFlavor::Prism => {
            if let Some(dir) = std::env::var_os("PRISMLAUNCHER_DATA_DIR") {
                candidates.push(PathBuf::from(dir));
            }
            candidates.push(data.join("PrismLauncher"));
            candidates
                .push(home.join(".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher"));
            candidates.push(home.join("snap/prismlauncher/common"));
            candidates.push(data.join("prismlauncher"));
            // PolyMC's layout is identical and may still be in use.
            candidates.push(data.join("PolyMC"));
            candidates.push(home.join(".var/app/org.polymc.PolyMC/data/PolyMC"));
        }
        MmcFlavor::MultiMC => {
            if let Some(dir) = std::env::var_os("MULTIMC_DATA_DIR") {
                candidates.push(PathBuf::from(dir));
            }
            candidates.push(data.join("multimc"));
            candidates.push(data.join("MultiMC"));
            candidates.push(home.join("MultiMC"));
            candidates.push(home.join("multimc"));
            candidates.push(home.join(".multimc"));
            candidates.push(home.join(".local/share/multimc"));
            candidates.push(home.join("Desktop/MultiMC"));
            candidates.push(home.join("Downloads/MultiMC"));
            candidates.push(home.join("Documents/MultiMC"));
            candidates.push(home.join("Applications/MultiMC.app/Data"));
            candidates.push(PathBuf::from("/Applications/MultiMC.app/Data"));
            candidates.push(home.join("Games/MultiMC"));
            #[cfg(target_os = "windows")]
            {
                candidates.push(PathBuf::from("C:\\MultiMC"));
                candidates.push(PathBuf::from("C:\\Games\\MultiMC"));
                candidates.push(PathBuf::from("D:\\MultiMC"));
                candidates.push(basedirs.data_local_dir().join("MultiMC"));
                candidates.push(basedirs.data_local_dir().join("Programs").join("MultiMC"));
            }
        }
    }

    dedup_paths(candidates)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn data_dir_candidates_are_unique_per_flavor() {
        for flavor in [MmcFlavor::Prism, MmcFlavor::MultiMC] {
            super::super::common::assert_paths_unique(&data_dir_candidates(flavor));
        }
    }

    #[test]
    fn instance_overrides_are_read_only_when_enabled() {
        let cfg = parse_instance_cfg(
            "[General]\nConfigVersion=1.2\nname=X\nnotes=hello\nOverrideMemory=true\nMinMemAlloc=1024\nMaxMemAlloc=8192\nOverrideJavaArgs=true\nJvmArgs=-XX:+UseG1GC\nOverrideJavaLocation=true\nJavaPath=/usr/bin/java\nOverrideCommands=true\nPreLaunchCommand=echo pre\nPostExitCommand=echo post\nWrapperCommand=gamemoderun\nOverrideWindow=true\nMinecraftWinWidth=1920\nMinecraftWinHeight=1080\n",
        );
        let s = instance_settings(&cfg);
        assert_eq!(s.notes.as_deref(), Some("hello"));
        assert_eq!(s.memory(), Some((1024, 8192)));
        assert_eq!(s.extra_java_args.as_deref(), Some("-XX:+UseG1GC"));
        assert_eq!(s.java_path.as_deref(), Some("/usr/bin/java"));
        assert_eq!(s.pre_launch_hook.as_deref(), Some("echo pre"));
        assert_eq!(s.post_exit_hook.as_deref(), Some("echo post"));
        assert_eq!(s.wrapper_command.as_deref(), Some("gamemoderun"));
        assert_eq!(s.game_resolution, Some((1920, 1080)));

        let cfg = parse_instance_cfg(
            "name=X\nMaxMemAlloc=8192\nJvmArgs=-XX:+UseG1GC\nJavaPath=/usr/bin/java\nPreLaunchCommand=echo\nMinecraftWinWidth=1920\nMinecraftWinHeight=1080\n",
        );
        assert_eq!(instance_settings(&cfg), ImportedSettings::default());
    }

    #[tokio::test]
    async fn groups_come_from_instgroups_json() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("instgroups.json"),
            r#"{"formatVersion":"1","groups":{"Modded":{"hidden":false,"instances":["ATM9","Skies"]},"":{"instances":["Loose"]}}}"#,
        )
        .unwrap();
        assert_eq!(
            group_of_instance(tmp.path(), "ATM9").await.as_deref(),
            Some("Modded")
        );
        assert_eq!(group_of_instance(tmp.path(), "Loose").await, None);
        assert_eq!(group_of_instance(tmp.path(), "Nope").await, None);
        assert_eq!(
            group_of_instance(&tmp.path().join("missing"), "ATM9").await,
            None
        );
    }

    #[test]
    fn old_multimc_cfg_is_parsed() {
        let text = "InstanceType=OneSix\nname=My \\#1 Pack # a comment\niconKey=bee\nlastLaunchTime=1700000000000\ntotalTimePlayed=3600\nnotes=line1\\nline2\n";
        let cfg = parse_instance_cfg(text);
        assert_eq!(cfg.get("InstanceType").unwrap(), "OneSix");
        assert_eq!(cfg.get("name").unwrap(), "My #1 Pack");
        assert_eq!(cfg.get("iconKey").unwrap(), "bee");
        assert_eq!(cfg.get("lastLaunchTime").unwrap(), "1700000000000");
        assert_eq!(cfg.get("notes").unwrap(), "line1\nline2");
    }

    #[test]
    fn qsettings_cfg_is_parsed() {
        let text = "[General]\nConfigVersion=1.2\nInstanceType=OneSix\nname=\"Hello, World; #1\"\niconKey=default\nManagedPack=true\nManagedPackType=flame\nManagedPackID=12345\nManagedPackVersionID=67890\nnotes=caf\\xe9\n\n[Other]\nkey=value\n";
        let cfg = parse_instance_cfg(text);
        assert_eq!(cfg.get("name").unwrap(), "Hello, World; #1");
        assert_eq!(cfg.get("ManagedPack").unwrap(), "true");
        assert_eq!(cfg.get("notes").unwrap(), "café");
        assert_eq!(cfg.get("Other/key").unwrap(), "value");
        assert!(cfg.get("Other").is_none());
    }

    #[test]
    fn managed_packs_link_to_platforms() {
        let mut cfg = HashMap::new();
        cfg.insert("ManagedPack".to_string(), "true".to_string());
        cfg.insert("ManagedPackType".to_string(), "flame".to_string());
        cfg.insert("ManagedPackID".to_string(), "12345".to_string());
        cfg.insert("ManagedPackVersionID".to_string(), "67890".to_string());
        assert_eq!(
            managed_pack(&cfg),
            Some(Modpack::Curseforge(CurseforgeModpack {
                project_id: 12345,
                file_id: 67890,
            }))
        );

        cfg.insert("ManagedPackType".to_string(), "modrinth".to_string());
        cfg.insert("ManagedPackID".to_string(), "AABBCCDD".to_string());
        cfg.insert("ManagedPackVersionID".to_string(), "EEFFGGHH".to_string());
        assert_eq!(
            managed_pack(&cfg),
            Some(Modpack::Modrinth(ModrinthModpack {
                project_id: "AABBCCDD".to_string(),
                version_id: "EEFFGGHH".to_string(),
            }))
        );

        cfg.insert("ManagedPackType".to_string(), "technic".to_string());
        assert_eq!(managed_pack(&cfg), None);

        cfg.insert("ManagedPack".to_string(), "false".to_string());
        cfg.insert("ManagedPackType".to_string(), "modrinth".to_string());
        assert_eq!(managed_pack(&cfg), None);
    }

    #[test]
    fn mmc_pack_components_are_parsed() {
        let text = r#"{
            "formatVersion": 1,
            "components": [
                {"uid": "org.lwjgl3", "version": "3.3.1", "dependencyOnly": true},
                {"uid": "net.minecraft", "version": "1.20.1", "important": true},
                {"uid": "net.fabricmc.intermediary", "version": "1.20.1", "dependencyOnly": true},
                {"uid": "net.fabricmc.fabric-loader", "version": "0.15.0"}
            ]
        }"#;
        let pack = parse_mmc_pack(text);
        assert_eq!(pack.minecraft.as_deref(), Some("1.20.1"));
        assert_eq!(
            pack.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Fabric, "0.15.0")]
        );

        let text = r#"{"components": [
            {"uid": "net.minecraft", "cachedVersion": "1.20.1"},
            {"uid": "net.neoforged", "version": "47.1.79"},
            {"uid": "net.minecraftforge", "version": "1.20.1-47.2.0"}
        ]}"#;
        let pack = parse_mmc_pack(text);
        assert_eq!(pack.minecraft.as_deref(), Some("1.20.1"));
        assert_eq!(pack.loaders.len(), 2);

        assert_eq!(parse_mmc_pack("not json"), ParsedPack::default());
    }

    #[test]
    fn versions_fall_back_to_legacy_cfg_keys() {
        let mut cfg = HashMap::new();
        cfg.insert("IntendedVersion".to_string(), "1.7.10".to_string());
        cfg.insert(
            "ForgeVersion".to_string(),
            "10.13.4.1614-1.7.10".to_string(),
        );

        let resolved = resolve_versions(&cfg, &ParsedPack::default()).unwrap();
        assert_eq!(resolved.minecraft, "1.7.10");
        assert_eq!(
            resolved.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Forge, "10.13.4.1614")]
        );

        let pack = ParsedPack {
            minecraft: Some("1.20.1".to_string()),
            loaders: vec![DeclaredModLoader::new(
                ModLoaderType::Forge,
                "1.20.1-47.2.0",
            )],
        };
        let resolved = resolve_versions(&cfg, &pack).unwrap();
        assert_eq!(resolved.minecraft, "1.20.1");
        assert_eq!(
            resolved.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Forge, "47.2.0")]
        );

        assert!(resolve_versions(&HashMap::new(), &ParsedPack::default()).is_none());
    }

    #[tokio::test]
    async fn instances_are_scanned_from_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path();
        std::fs::write(
            data.join("prismlauncher.cfg"),
            "[General]\nConfigVersion=1.2\nInstanceDir=instances\n",
        )
        .unwrap();
        std::fs::create_dir_all(data.join("icons")).unwrap();
        std::fs::write(data.join("icons").join("mypack.png"), b"png").unwrap();

        let inst = data.join("instances").join("My Pack");
        std::fs::create_dir_all(inst.join(".minecraft").join("mods")).unwrap();
        std::fs::write(
            inst.join("instance.cfg"),
            "[General]\nConfigVersion=1.2\nInstanceType=OneSix\nname=My Pack\niconKey=mypack\ntotalTimePlayed=120\nlastLaunchTime=1700000000000\n",
        )
        .unwrap();
        std::fs::write(
            inst.join("mmc-pack.json"),
            r#"{"components":[{"uid":"net.minecraft","version":"1.20.1"},{"uid":"net.minecraftforge","version":"47.2.0"}]}"#,
        )
        .unwrap();

        let broken = data.join("instances").join("Broken");
        std::fs::create_dir_all(&broken).unwrap();
        std::fs::write(broken.join("instance.cfg"), "name=Broken\n").unwrap();

        std::fs::create_dir_all(data.join("instances").join("_LAUNCHER_TEMP")).unwrap();

        let importer = MmcImporter::new(MmcFlavor::Prism);

        let entry = importer.scan_instance(inst.clone()).await.unwrap().unwrap();
        let InternalImportEntry::Valid(importable) = entry else {
            panic!("expected a valid entry");
        };
        assert_eq!(importable.name, "My Pack");
        assert_eq!(importable.filename, "My Pack");
        assert_eq!(importable.minecraft_version, "1.20.1");
        assert_eq!(
            importable.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Forge, "47.2.0")]
        );
        assert_eq!(importable.game_dir, inst.join(".minecraft"));
        assert_eq!(
            importable.icon_path,
            Some(data.join("icons").join("mypack.png"))
        );
        assert_eq!(importable.seconds_played, Some(120));
        assert_eq!(
            importable.last_played.map(|d| d.timestamp()),
            Some(1_700_000_000)
        );
        assert_eq!(importable.managed, None);

        let entry = importer.scan_instance(broken).await.unwrap().unwrap();
        assert!(matches!(entry, InternalImportEntry::Invalid(_)));

        let entry = importer
            .scan_instance(data.join("instances").join("_LAUNCHER_TEMP"))
            .await
            .unwrap();
        assert!(entry.is_none());

        assert!(is_launcher_data_dir(data));
        assert_eq!(
            instances_dir_for_data_dir(data).await,
            data.join("instances")
        );
    }

    #[test]
    fn game_dir_prefers_dot_minecraft() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(game_dir(tmp.path()), tmp.path().join(".minecraft"));

        std::fs::create_dir_all(tmp.path().join("minecraft")).unwrap();
        assert_eq!(game_dir(tmp.path()), tmp.path().join("minecraft"));

        std::fs::create_dir_all(tmp.path().join(".minecraft")).unwrap();
        assert_eq!(game_dir(tmp.path()), tmp.path().join(".minecraft"));
    }
}

#[cfg(test)]
mod task13_tests {
    use super::*;
    async fn scan_valid(importer: &MmcImporter, path: &Path) -> Importable {
        match importer.scan_instance(path.to_path_buf()).await.unwrap() {
            Some(InternalImportEntry::Valid(entry)) => entry,
            other => panic!("expected an importable instance, got {other:?}"),
        }
    }

    /// The packwiz setup Prism documents: the instance and its files import,
    /// only the command GDLauncher cannot run is left out, and the row says so.
    #[tokio::test]
    async fn task13_enabled_placeholder_commands_are_skipped_not_refused() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(
            t.path().join("mmc-pack.json"),
            r#"{"components":[{"uid":"net.minecraft","version":"1.20.1"}]}"#,
        )
        .unwrap();
        std::fs::write(
            t.path().join("instance.cfg"),
            "name=Synthetic\nOverrideCommands=true\nPreLaunchCommand=\"$INST_JAVA\" -jar packwiz-installer-bootstrap.jar https://example.invalid/pack.toml\nPostExitCommand=echo done\nWrapperCommand=%INST_DIR%\\\\wrap.cmd\n",
        )
        .unwrap();
        let importer = MmcImporter::new(MmcFlavor::Prism);
        assert!(matches!(
            importer
                .scan_instance(t.path().to_path_buf())
                .await
                .unwrap(),
            Some(InternalImportEntry::Invalid(_))
        ));
        std::fs::write(
            t.path().join("instance.cfg"),
            "name=Synthetic\nOverrideCommands=false\nPreLaunchCommand=echo $INST_DIR\n",
        )
        .unwrap();
        let entry = scan_valid(&importer, t.path()).await;
        assert_eq!(entry.settings.pre_launch_hook, None);
        assert!(ImportableInstance::from(entry).warnings.is_empty());
        for command in ["echo ordinary", "printf %s hello", "echo 100%"] {
            assert!(!has_launcher_placeholder(command));
        }
        for command in ["echo $INST_DIR", "echo ${INST_DIR}", "echo %INST_DIR%"] {
            assert!(has_launcher_placeholder(command));
            assert!(needs_launcher_expansion(command));
        }
        // A shell GDLauncher starts expands the environment by itself, but it
        // has no `INST_*` to read, and nothing else expands a bare command.
        for command in [
            r#"sh -c "cp $HOME/a b""#,
            r#"/bin/bash -c 'echo ${HOME}'"#,
            r"C:\Windows\System32\cmd.exe /c echo %APPDATA%",
            r#""powershell.exe" -Command "echo $env:APPDATA""#,
            r#""C:\Program Files\Git\bin\bash.exe" -c "echo $HOME""#,
            r#""C:\Program Files\PowerShell\7\pwsh.exe" -Command "echo $env:APPDATA""#,
            r#"'/opt/my tools/zsh' -c 'echo $HOME'"#,
            r#"/opt/my\ tools/zsh -c 'echo $HOME'"#,
        ] {
            assert!(!needs_launcher_expansion(command), "{command}");
        }
        for command in [
            "$HOME/sync.sh",
            "%APPDATA%\\sync.cmd",
            r#"sh -c "$INST_JAVA -jar a.jar""#,
            r#"cmd /c %INST_DIR%\sync.cmd"#,
            r#""$HOME/bin/bash" -c "echo hello""#,
            r#""C:\Program Files\Tool\tool.exe" %APPDATA%"#,
            r#"sh -c "echo $HOME"#,
        ] {
            assert!(needs_launcher_expansion(command), "{command}");
        }
    }
}

/// Prism and MultiMC fill `$INST_JAVA`, `%INST_DIR%` and the environment into
/// these commands before running them; GDLauncher runs them as written, so such
/// a command would fail at every launch. Each one is left out, the rest of the
/// instance still imports, and the row says so (packwiz setups are the usual case).
/// The notes keep what was dropped, as the pack's URL may only be in there.
fn skip_launcher_variable_commands(settings: &mut ImportedSettings) {
    let mut dropped = Vec::new();
    for (name, command) in [
        ("PreLaunchCommand", &mut settings.pre_launch_hook),
        ("PostExitCommand", &mut settings.post_exit_hook),
        ("WrapperCommand", &mut settings.wrapper_command),
    ] {
        if command.as_deref().is_some_and(needs_launcher_expansion) {
            warn!("Not importing {name}: it uses launcher variables");
            dropped.extend(command.take().map(|command| format!("{name}={command}")));
        }
    }
    if dropped.is_empty() {
        return;
    }
    let dropped = dropped.join("\n");
    settings.notes = Some(match settings.notes.take() {
        Some(notes) => format!("{notes}\n\n{dropped}"),
        None => dropped,
    });
    settings.skip(SkippedSetting::LauncherVariableCommands);
}

/// `INST_*` only exists inside the source launcher. Other variables are the
/// environment, which a shell the command starts expands by itself.
fn needs_launcher_expansion(command: &str) -> bool {
    let uses_instance_variable = ["$INST_", "${INST_", "%INST_"]
        .iter()
        .any(|prefix| command.contains(prefix));
    uses_instance_variable || (has_launcher_placeholder(command) && !starts_a_shell(command))
}

/// Reads the program the way the hook runner (`split_hook_command`) splits the
/// line: POSIX style on macOS and Linux, and with backslashes kept on Windows,
/// where most of these commands come from. Either way quotes group words
/// (`"C:\Program Files\Git\bin\bash.exe"`, `'/opt/my tools/zsh'`). A program
/// that needs expanding itself cannot start at all.
fn starts_a_shell(command: &str) -> bool {
    [
        shlex::split(command),
        shlex::split(&command.replace('\\', "\\\\")),
    ]
    .into_iter()
    .filter_map(|words| words?.into_iter().next())
    .any(|program| is_shell_program(&program))
}

fn is_shell_program(program: &str) -> bool {
    if has_launcher_placeholder(program) {
        return false;
    }
    let name = program
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let name = name.strip_suffix(".exe").unwrap_or(&name);
    [
        "sh",
        "bash",
        "dash",
        "zsh",
        "fish",
        "cmd",
        "powershell",
        "pwsh",
    ]
    .contains(&name)
}

fn has_launcher_placeholder(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.iter().enumerate().any(|(i, b)| {
        if *b == b'$' {
            return bytes
                .get(i + 1)
                .is_some_and(|next| next.is_ascii_alphabetic() || *next == b'_' || *next == b'{');
        }
        if *b == b'%' {
            let rest = &value[i + 1..];
            return rest.split_once('%').is_some_and(|(name, _)| {
                !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            });
        }
        false
    })
}
