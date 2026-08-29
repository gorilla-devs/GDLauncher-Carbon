use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use tracing::{trace, warn};

use crate::{
    domain::instance::{
        GroupId, InstanceId, InstanceSettingsUpdate,
        info::{GameResolution, JavaOverride, ModLoader, ModLoaderType, StandardVersion},
    },
    managers::{AppInner, modplatforms::curseforge::convert_cf_version_to_standard_version},
};
use carbon_platforms::curseforge::manifest::{Minecraft, ModLoaders};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeclaredModLoader {
    pub type_: ModLoaderType,
    pub version: String,
}

impl DeclaredModLoader {
    pub fn new(type_: ModLoaderType, version: impl Into<String>) -> Self {
        Self {
            type_,
            version: version.into(),
        }
    }
}

/// Unknown or server-only loaders yield `None` so the instance imports as vanilla.
pub fn parse_loader_type(name: &str) -> Option<ModLoaderType> {
    let normalized: String = name
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();

    match normalized.as_str() {
        "neoforge" | "neoforged" | "netneoforged" => Some(ModLoaderType::Neoforge),
        "forge" | "minecraftforge" | "netminecraftforge" | "forgeloader" => {
            Some(ModLoaderType::Forge)
        }
        // Legacy Fabric still runs the regular fabric-loader.
        "fabric" | "fabricloader" | "netfabricmcfabricloader" | "legacyfabric" => {
            Some(ModLoaderType::Fabric)
        }
        "quilt" | "quiltloader" | "orgquiltmcquiltloader" => Some(ModLoaderType::Quilt),
        _ => None,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedVersionId {
    pub minecraft: Option<String>,
    pub loaders: Vec<DeclaredModLoader>,
}

/// `1.20.1-forge-47.2.0`, `1.12.2-forge1.12.2-14.23.5.2860`, `neoforge-20.4.80-beta`, `fabric-loader-0.15.0-1.20.1`; unknown ids count as vanilla.
pub fn parse_version_id(id: &str) -> ParsedVersionId {
    let id = id.trim();
    if id.is_empty() {
        return ParsedVersionId::default();
    }

    let lower = id.to_ascii_lowercase();

    // fabric-loader-<loader>-<mc> / quilt-loader-<loader>-<mc>
    for (prefix, type_) in [
        ("fabric-loader-", ModLoaderType::Fabric),
        ("quilt-loader-", ModLoaderType::Quilt),
    ] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let rest = &id[id.len() - rest.len()..];
            return match rest.split_once('-') {
                Some((loader, mc)) if !loader.is_empty() && !mc.is_empty() => ParsedVersionId {
                    minecraft: Some(mc.to_string()),
                    loaders: vec![DeclaredModLoader::new(type_, loader)],
                },
                _ => ParsedVersionId {
                    minecraft: None,
                    loaders: vec![DeclaredModLoader::new(type_, rest)],
                },
            };
        }
    }

    // neoforge-<version>
    if let Some(rest) = lower.strip_prefix("neoforge-") {
        let version = &id[id.len() - rest.len()..];
        return ParsedVersionId {
            minecraft: None,
            loaders: vec![DeclaredModLoader::new(ModLoaderType::Neoforge, version)],
        };
    }

    // forge-<version> / fabric-<version> / quilt-<version> (FTB-style targets)
    for (prefix, type_) in [
        ("forge-", ModLoaderType::Forge),
        ("fabric-", ModLoaderType::Fabric),
        ("quilt-", ModLoaderType::Quilt),
    ] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let version = &id[id.len() - rest.len()..];
            if !version.is_empty() {
                return ParsedVersionId {
                    minecraft: None,
                    loaders: vec![DeclaredModLoader::new(type_, version)],
                };
            }
        }
    }

    // <mc>-neoforge-<version>
    if let Some(idx) = lower.find("-neoforge-") {
        let mc = &id[..idx];
        let version = &id[idx + "-neoforge-".len()..];
        if !mc.is_empty() && !version.is_empty() {
            return ParsedVersionId {
                minecraft: Some(mc.to_string()),
                loaders: vec![DeclaredModLoader::new(ModLoaderType::Neoforge, version)],
            };
        }
    }

    // <mc>-forge-<version> and <mc>-forge<mc>-<version>[-<mc>]
    if let Some(idx) = lower.find("-forge") {
        let mc = &id[..idx];
        let mut rest = &id[idx + "-forge".len()..];
        if let Some(stripped) = rest.strip_prefix('-') {
            rest = stripped;
        } else if let Some(stripped) = rest.strip_prefix(mc) {
            rest = stripped.strip_prefix('-').unwrap_or(stripped);
        }

        if !mc.is_empty() && !rest.is_empty() {
            return ParsedVersionId {
                minecraft: Some(mc.to_string()),
                loaders: vec![DeclaredModLoader::new(
                    ModLoaderType::Forge,
                    normalize_forge_version(mc, rest),
                )],
            };
        }
    }

    ParsedVersionId {
        minecraft: Some(id.to_string()),
        loaders: Vec::new(),
    }
}

/// GDL's metadata lookup matches by substring, so strip the `<mc>-` prefix / `-<mc>` suffix Forge versions sometimes carry.
pub fn normalize_forge_version(mc_version: &str, forge_version: &str) -> String {
    let mut version = forge_version.trim();
    if !mc_version.is_empty() {
        if let Some(stripped) = version.strip_prefix(&format!("{mc_version}-")) {
            version = stripped;
        }
        if let Some(stripped) = version.strip_suffix(&format!("-{mc_version}")) {
            version = stripped;
        }
    }
    version.to_string()
}

pub fn loaders_from_library_names<'a>(names: impl IntoIterator<Item = &'a str>) -> ParsedVersionId {
    let mut parsed = ParsedVersionId::default();

    for name in names {
        let mut parts = name.split(':');
        let (Some(group), Some(artifact), Some(version)) =
            (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };

        match (group, artifact) {
            ("net.minecraftforge", "forge" | "fmlloader") => {
                let (mc, forge) = split_forge_library_version(version);
                if parsed.minecraft.is_none() {
                    parsed.minecraft = mc;
                }
                if !parsed
                    .loaders
                    .iter()
                    .any(|l| l.type_ == ModLoaderType::Forge)
                {
                    parsed
                        .loaders
                        .push(DeclaredModLoader::new(ModLoaderType::Forge, forge));
                }
            }
            ("net.neoforged", "neoforge") => {
                if !parsed
                    .loaders
                    .iter()
                    .any(|l| l.type_ == ModLoaderType::Neoforge)
                {
                    parsed
                        .loaders
                        .push(DeclaredModLoader::new(ModLoaderType::Neoforge, version));
                }
            }
            ("net.fabricmc", "fabric-loader") => {
                if !parsed
                    .loaders
                    .iter()
                    .any(|l| l.type_ == ModLoaderType::Fabric)
                {
                    parsed
                        .loaders
                        .push(DeclaredModLoader::new(ModLoaderType::Fabric, version));
                }
            }
            ("org.quiltmc", "quilt-loader") => {
                if !parsed
                    .loaders
                    .iter()
                    .any(|l| l.type_ == ModLoaderType::Quilt)
                {
                    parsed
                        .loaders
                        .push(DeclaredModLoader::new(ModLoaderType::Quilt, version));
                }
            }
            ("net.fabricmc" | "net.legacyfabric", "intermediary") => {
                if parsed.minecraft.is_none() {
                    parsed.minecraft = Some(version.to_string());
                }
            }
            _ => {}
        }
    }

    parsed
}

/// `net.minecraftforge:forge` versions come as `1.20.1-47.2.0`, `1.12.2-14.23.5.2860` or `1.7.10-10.13.4.1614-1.7.10`.
fn split_forge_library_version(version: &str) -> (Option<String>, String) {
    match version.split_once('-') {
        Some((mc, rest)) if looks_like_minecraft_version(mc) => {
            (Some(mc.to_string()), normalize_forge_version(mc, rest))
        }
        _ => (None, version.to_string()),
    }
}

pub fn looks_like_minecraft_version(s: &str) -> bool {
    let s = s.trim();
    !s.is_empty()
        && s.chars().next().is_some_and(|c| c.is_ascii_digit())
        && s.len() <= 16
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
}

/// Falls back to the declared versions when GDL's metadata can't resolve them, rather than refusing the import.
pub async fn resolve_standard_version(
    app: &Arc<AppInner>,
    minecraft_version: &str,
    loaders: &[DeclaredModLoader],
) -> StandardVersion {
    let raw = StandardVersion {
        release: minecraft_version.to_string(),
        modloaders: loaders
            .iter()
            .map(|l| ModLoader {
                type_: l.type_,
                version: l.version.clone(),
            })
            .collect::<HashSet<_>>(),
    };

    if loaders.is_empty() {
        return raw;
    }

    let dummy_string = daedalus::BRANDING
        .get_or_init(daedalus::Branding::default)
        .dummy_replace_string
        .clone();

    let cf_version = Minecraft {
        version: minecraft_version.to_string(),
        mod_loaders: loaders
            .iter()
            .map(|l| ModLoaders {
                id: format!("{}-{}", l.type_.to_string(), l.version),
                primary: true,
            })
            .collect(),
    };

    match convert_cf_version_to_standard_version(app.clone(), cf_version, dummy_string).await {
        Ok(resolved) => resolved,
        Err(e) => {
            warn!(
                "Could not resolve modloaders {loaders:?} for Minecraft {minecraft_version} against GDL metadata, importing them as declared: {e}"
            );
            raw
        }
    }
}

pub fn datetime_from_epoch_millis(millis: i64) -> Option<DateTime<Utc>> {
    if millis <= 0 {
        return None;
    }
    DateTime::<Utc>::from_timestamp_millis(millis)
}

pub fn datetime_from_epoch_secs(secs: i64) -> Option<DateTime<Utc>> {
    if secs <= 0 {
        return None;
    }
    DateTime::<Utc>::from_timestamp(secs, 0)
}

// Past the year 5000 as seconds → it was milliseconds.
pub fn datetime_from_epoch_secs_or_millis(value: i64) -> Option<DateTime<Utc>> {
    const YEAR_5000_SECS: i64 = 95_617_584_000;
    if value > YEAR_5000_SECS {
        datetime_from_epoch_millis(value)
    } else {
        datetime_from_epoch_secs(value)
    }
}

pub fn seconds_played_from_i64(secs: i64) -> Option<u32> {
    if secs <= 0 {
        return None;
    }
    Some(u32::try_from(secs).unwrap_or(u32::MAX))
}

pub fn dir_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

/// Marks the modpack complete so `prepare_game` only installs Minecraft and the loader.
pub async fn copy_game_dir<F>(from: &Path, instance_path: &Path, filter: F) -> anyhow::Result<()>
where
    F: for<'a> Fn(&'a Path) -> bool,
{
    tokio::fs::create_dir_all(instance_path.join(".setup").join("modpack-complete")).await?;

    if !from.is_dir() {
        trace!("Game directory {from:?} does not exist, importing an empty instance");
        return Ok(());
    }

    let to = instance_path.join("instance");
    trace!("Copying game files from {from:?} to {to:?}");
    carbon_rt_path::copy_dir_filter(from, &to, filter).await?;

    Ok(())
}

pub fn first_existing_dir(candidates: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|p| p.is_dir())
}

pub fn find_image_with_any_extension(dir: &Path, stem: &str) -> Option<PathBuf> {
    const EXTENSIONS: [&str; 8] = ["png", "jpg", "jpeg", "gif", "webp", "ico", "svg", "bmp"];

    let stem = stem.trim();
    if stem.is_empty() {
        return None;
    }

    // The key may already carry the extension.
    let direct = dir.join(stem);
    if direct.is_file() {
        return Some(direct);
    }

    EXTENSIONS
        .iter()
        .map(|ext| dir.join(format!("{stem}.{ext}")))
        .find(|p| p.is_file())
}

pub async fn read_json_value(path: &Path) -> Option<serde_json::Value> {
    let text = tokio::fs::read_to_string(path).await.ok()?;
    serde_json::from_str(&text).ok()
}

/// Accepts numbers too; launchers flip between the two across versions.
pub fn json_string(value: &serde_json::Value, key: &str) -> Option<String> {
    match value.get(key)? {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

pub fn json_i64(value: &serde_json::Value, key: &str) -> Option<i64> {
    match value.get(key)? {
        serde_json::Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        serde_json::Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

pub fn json_bool(value: &serde_json::Value, key: &str) -> Option<bool> {
    match value.get(key)? {
        serde_json::Value::Bool(b) => Some(*b),
        serde_json::Value::Number(n) => n.as_i64().map(|n| n != 0),
        serde_json::Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

pub fn icon_from_data_url(data_url: &str) -> Option<(String, Vec<u8>)> {
    use base64::Engine;

    let rest = data_url.strip_prefix("data:")?;
    let (meta, payload) = rest.split_once(',')?;
    let mime = meta.split(';').next()?;
    let subtype = mime.strip_prefix("image/")?;
    let ext = match subtype {
        "jpeg" => "jpg",
        "svg+xml" => "svg",
        other => other,
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload.trim())
        .ok()?;
    if bytes.is_empty() {
        return None;
    }
    Some((format!("icon.{ext}"), bytes))
}

/// Memory in MB; resolution is `(width, height)`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportedSettings {
    pub notes: Option<String>,
    pub group: Option<String>,
    pub memory_min_mb: Option<u32>,
    pub memory_max_mb: Option<u32>,
    pub extra_java_args: Option<String>,
    pub java_path: Option<String>,
    pub pre_launch_hook: Option<String>,
    pub post_exit_hook: Option<String>,
    pub wrapper_command: Option<String>,
    pub game_resolution: Option<(u32, u32)>,
}

impl ImportedSettings {
    /// `-Xmx`/`-Xms` become the memory settings (GDL passes its own); the rest is kept.
    pub fn absorb_jvm_args(&mut self, args: &str) {
        let mut rest: Vec<&str> = Vec::new();

        for arg in args.split_whitespace() {
            if let Some(size) = arg.strip_prefix("-Xmx") {
                if let Some(mb) = parse_memory_size(size) {
                    self.memory_max_mb = Some(mb);
                    continue;
                }
            }
            if let Some(size) = arg.strip_prefix("-Xms") {
                if let Some(mb) = parse_memory_size(size) {
                    self.memory_min_mb = Some(mb);
                    continue;
                }
            }
            rest.push(arg);
        }

        if !rest.is_empty() {
            let joined = rest.join(" ");
            self.extra_java_args = Some(match self.extra_java_args.take() {
                Some(existing) if !existing.trim().is_empty() => format!("{existing} {joined}"),
                _ => joined,
            });
        }
    }

    /// Without a minimum the maximum is used for both, like most launchers.
    pub fn memory(&self) -> Option<(u16, u16)> {
        let max = self.memory_max_mb.filter(|m| *m > 0)?;
        let min = self
            .memory_min_mb
            .filter(|m| *m > 0)
            .unwrap_or(max)
            .min(max);
        Some((clamp_u16(min), clamp_u16(max)))
    }

    fn has_launch_overrides(&self) -> bool {
        self.memory().is_some()
            || self.extra_java_args.is_some()
            || self.java_path.is_some()
            || self.pre_launch_hook.is_some()
            || self.post_exit_hook.is_some()
            || self.wrapper_command.is_some()
            || self.game_resolution.is_some()
    }
}

fn clamp_u16(value: u32) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

pub fn parse_memory_size(value: &str) -> Option<u32> {
    let value = value.trim();
    let (digits, unit) = match value.char_indices().find(|(_, c)| !c.is_ascii_digit()) {
        Some((idx, _)) => value.split_at(idx),
        None => (value, ""),
    };
    let amount: u64 = digits.parse().ok()?;
    let mb = match unit.trim().to_ascii_lowercase().as_str() {
        "g" | "gb" => amount.checked_mul(1024)?,
        "m" | "mb" | "" if !unit.is_empty() || amount < 1_048_576 => {
            if unit.is_empty() && amount > 65_536 {
                // A bare number that large is bytes, not MB.
                amount / (1024 * 1024)
            } else {
                amount
            }
        }
        "" => amount / (1024 * 1024),
        "k" | "kb" => amount / 1024,
        "t" | "tb" => amount.checked_mul(1024 * 1024)?,
        _ => return None,
    };
    (mb > 0).then(|| u32::try_from(mb).unwrap_or(u32::MAX))
}

pub async fn group_for_import(app: &Arc<AppInner>, group: Option<&str>) -> anyhow::Result<GroupId> {
    if let Some(name) = group.map(str::trim).filter(|g| !g.is_empty()) {
        match app.instance_manager().create_group(name.to_string()).await {
            Ok(id) => return Ok(id),
            Err(e) => {
                warn!("Cannot create group `{name}` for import, using the default group: {e}")
            }
        }
    }

    app.instance_manager().get_default_group().await
}

/// Best effort: the instance already exists, so a failure only costs the settings.
pub async fn apply_imported_settings(
    app: &Arc<AppInner>,
    instance_id: InstanceId,
    settings: &ImportedSettings,
) {
    if !settings.has_launch_overrides() {
        return;
    }

    let update = InstanceSettingsUpdate {
        instance_id,
        name: None,
        use_loaded_icon: None,
        notes: None,
        version: None,
        modloader: None,
        java_override: settings
            .java_path
            .clone()
            .map(|path| Some(JavaOverride::Path(Some(path)))),
        global_java_args: None,
        extra_java_args: settings.extra_java_args.clone().map(Some),
        memory: settings.memory().map(Some),
        pre_launch_hook: settings.pre_launch_hook.clone().map(Some),
        post_exit_hook: settings.post_exit_hook.clone().map(Some),
        wrapper_command: settings.wrapper_command.clone().map(Some),
        game_resolution: settings
            .game_resolution
            .map(|(w, h)| Some(GameResolution::Custom(clamp_u16(w), clamp_u16(h)))),
        mod_sources: None,
        modpack_locked: None,
    };

    if let Err(e) = app.instance_manager().update_instance(update).await {
        warn!("Cannot apply imported launch settings to instance {instance_id:?}: {e}");
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn loader_names_are_parsed_leniently() {
        assert_eq!(parse_loader_type("Forge"), Some(ModLoaderType::Forge));
        assert_eq!(
            parse_loader_type("net.minecraftforge"),
            Some(ModLoaderType::Forge)
        );
        assert_eq!(parse_loader_type("NeoForge"), Some(ModLoaderType::Neoforge));
        assert_eq!(
            parse_loader_type("neo-forge"),
            Some(ModLoaderType::Neoforge)
        );
        assert_eq!(
            parse_loader_type("fabric-loader"),
            Some(ModLoaderType::Fabric)
        );
        assert_eq!(
            parse_loader_type("LegacyFabric"),
            Some(ModLoaderType::Fabric)
        );
        assert_eq!(parse_loader_type("quilt"), Some(ModLoaderType::Quilt));
        assert_eq!(parse_loader_type("Paper"), None);
        assert_eq!(parse_loader_type(""), None);
    }

    #[test]
    fn version_ids_are_parsed() {
        assert_eq!(
            parse_version_id("1.20.1"),
            ParsedVersionId {
                minecraft: Some("1.20.1".into()),
                loaders: vec![],
            }
        );
        assert_eq!(
            parse_version_id("1.20.1-forge-47.2.0"),
            ParsedVersionId {
                minecraft: Some("1.20.1".into()),
                loaders: vec![DeclaredModLoader::new(ModLoaderType::Forge, "47.2.0")],
            }
        );
        assert_eq!(
            parse_version_id("1.12.2-forge1.12.2-14.23.5.2860"),
            ParsedVersionId {
                minecraft: Some("1.12.2".into()),
                loaders: vec![DeclaredModLoader::new(ModLoaderType::Forge, "14.23.5.2860")],
            }
        );
        assert_eq!(
            parse_version_id("1.7.10-Forge10.13.4.1614-1.7.10"),
            ParsedVersionId {
                minecraft: Some("1.7.10".into()),
                loaders: vec![DeclaredModLoader::new(ModLoaderType::Forge, "10.13.4.1614")],
            }
        );
        assert_eq!(
            parse_version_id("neoforge-20.4.80-beta"),
            ParsedVersionId {
                minecraft: None,
                loaders: vec![DeclaredModLoader::new(
                    ModLoaderType::Neoforge,
                    "20.4.80-beta"
                )],
            }
        );
        assert_eq!(
            parse_version_id("1.20.1-neoforge-47.1.79"),
            ParsedVersionId {
                minecraft: Some("1.20.1".into()),
                loaders: vec![DeclaredModLoader::new(ModLoaderType::Neoforge, "47.1.79")],
            }
        );
        assert_eq!(
            parse_version_id("fabric-loader-0.15.0-1.20.1"),
            ParsedVersionId {
                minecraft: Some("1.20.1".into()),
                loaders: vec![DeclaredModLoader::new(ModLoaderType::Fabric, "0.15.0")],
            }
        );
        assert_eq!(
            parse_version_id("quilt-loader-0.21.0-1.20.1"),
            ParsedVersionId {
                minecraft: Some("1.20.1".into()),
                loaders: vec![DeclaredModLoader::new(ModLoaderType::Quilt, "0.21.0")],
            }
        );
        assert_eq!(
            parse_version_id("forge-47.2.0"),
            ParsedVersionId {
                minecraft: None,
                loaders: vec![DeclaredModLoader::new(ModLoaderType::Forge, "47.2.0")],
            }
        );
        assert_eq!(parse_version_id("  "), ParsedVersionId::default());
    }

    #[test]
    fn library_names_reveal_loaders() {
        let parsed = loaders_from_library_names([
            "com.google.guava:guava:21.0",
            "net.minecraftforge:forge:1.16.5-36.2.34",
        ]);
        assert_eq!(parsed.minecraft.as_deref(), Some("1.16.5"));
        assert_eq!(
            parsed.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Forge, "36.2.34")]
        );

        let parsed = loaders_from_library_names([
            "net.fabricmc:fabric-loader:0.14.9",
            "net.fabricmc:intermediary:1.19.2",
        ]);
        assert_eq!(parsed.minecraft.as_deref(), Some("1.19.2"));
        assert_eq!(
            parsed.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Fabric, "0.14.9")]
        );

        let parsed = loaders_from_library_names(["net.neoforged:neoforge:20.4.80-beta"]);
        assert_eq!(parsed.minecraft, None);
        assert_eq!(
            parsed.loaders,
            vec![DeclaredModLoader::new(
                ModLoaderType::Neoforge,
                "20.4.80-beta"
            )]
        );

        let parsed =
            loaders_from_library_names(["net.minecraftforge:forge:1.7.10-10.13.4.1614-1.7.10"]);
        assert_eq!(parsed.minecraft.as_deref(), Some("1.7.10"));
        assert_eq!(
            parsed.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Forge, "10.13.4.1614")]
        );
    }

    #[test]
    fn forge_versions_are_normalized() {
        assert_eq!(normalize_forge_version("1.20.1", "47.2.0"), "47.2.0");
        assert_eq!(normalize_forge_version("1.20.1", "1.20.1-47.2.0"), "47.2.0");
        assert_eq!(
            normalize_forge_version("1.7.10", "1.7.10-10.13.4.1614-1.7.10"),
            "10.13.4.1614"
        );
        assert_eq!(normalize_forge_version("", "47.2.0"), "47.2.0");
    }

    #[test]
    fn timestamps_convert() {
        assert_eq!(datetime_from_epoch_millis(0), None);
        assert_eq!(datetime_from_epoch_secs(-5), None);
        assert_eq!(
            datetime_from_epoch_millis(1_700_000_000_000).map(|d| d.timestamp()),
            Some(1_700_000_000)
        );
        assert_eq!(
            datetime_from_epoch_secs_or_millis(1_700_000_000).map(|d| d.timestamp()),
            Some(1_700_000_000)
        );
        assert_eq!(
            datetime_from_epoch_secs_or_millis(1_700_000_000_000).map(|d| d.timestamp()),
            Some(1_700_000_000)
        );
        assert_eq!(seconds_played_from_i64(0), None);
        assert_eq!(seconds_played_from_i64(90), Some(90));
        assert_eq!(seconds_played_from_i64(i64::MAX), Some(u32::MAX));
    }

    #[test]
    fn data_url_icons_decode() {
        // 1x1 transparent PNG
        let png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";
        let (name, bytes) = icon_from_data_url(&format!("data:image/png;base64,{png}")).unwrap();
        assert_eq!(name, "icon.png");
        assert_eq!(&bytes[1..4], b"PNG");

        let (name, _) = icon_from_data_url(&format!("data:image/jpeg;base64,{png}")).unwrap();
        assert_eq!(name, "icon.jpg");

        assert_eq!(icon_from_data_url("data:text/plain;base64,aGk="), None);
        assert_eq!(icon_from_data_url("https://example.com/icon.png"), None);
    }

    #[test]
    fn images_are_found_with_any_extension() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("nested")).unwrap();
        std::fs::write(tmp.path().join("bee.jpg"), b"x").unwrap();
        std::fs::write(tmp.path().join("nested").join("cat.png"), b"x").unwrap();

        assert_eq!(
            find_image_with_any_extension(tmp.path(), "bee"),
            Some(tmp.path().join("bee.jpg"))
        );
        assert_eq!(
            find_image_with_any_extension(tmp.path(), "bee.jpg"),
            Some(tmp.path().join("bee.jpg"))
        );
        assert_eq!(
            find_image_with_any_extension(tmp.path(), "nested/cat"),
            Some(tmp.path().join("nested/cat.png"))
        );
        assert_eq!(find_image_with_any_extension(tmp.path(), "default"), None);
        assert_eq!(find_image_with_any_extension(tmp.path(), ""), None);
    }

    #[test]
    fn memory_sizes_are_parsed() {
        assert_eq!(parse_memory_size("4G"), Some(4096));
        assert_eq!(parse_memory_size("4g"), Some(4096));
        assert_eq!(parse_memory_size("4096M"), Some(4096));
        assert_eq!(parse_memory_size("4096m"), Some(4096));
        assert_eq!(parse_memory_size("2097152k"), Some(2048));
        assert_eq!(parse_memory_size("4294967296"), Some(4096));
        assert_eq!(parse_memory_size("4096"), Some(4096));
        assert_eq!(parse_memory_size("0"), None);
        assert_eq!(parse_memory_size("lots"), None);
        assert_eq!(parse_memory_size(""), None);
    }

    #[test]
    fn jvm_args_are_absorbed_into_settings() {
        let mut settings = ImportedSettings::default();
        settings.absorb_jvm_args("-Xmx8G -Xms2048M -XX:+UseG1GC -Dfoo=bar");
        assert_eq!(settings.memory_max_mb, Some(8192));
        assert_eq!(settings.memory_min_mb, Some(2048));
        assert_eq!(
            settings.extra_java_args.as_deref(),
            Some("-XX:+UseG1GC -Dfoo=bar")
        );
        assert_eq!(settings.memory(), Some((2048, 8192)));

        settings.absorb_jvm_args("-XX:+UseZGC");
        assert_eq!(
            settings.extra_java_args.as_deref(),
            Some("-XX:+UseG1GC -Dfoo=bar -XX:+UseZGC")
        );

        let mut settings = ImportedSettings::default();
        settings.absorb_jvm_args("   ");
        assert_eq!(settings, ImportedSettings::default());
        assert!(!settings.has_launch_overrides());

        let settings = ImportedSettings {
            memory_max_mb: Some(6144),
            ..Default::default()
        };
        assert_eq!(settings.memory(), Some((6144, 6144)));
        let settings = ImportedSettings {
            memory_min_mb: Some(9000),
            memory_max_mb: Some(6144),
            ..Default::default()
        };
        assert_eq!(settings.memory(), Some((6144, 6144)));
        let settings = ImportedSettings {
            memory_min_mb: Some(1024),
            ..Default::default()
        };
        assert_eq!(settings.memory(), None);
        let settings = ImportedSettings {
            memory_max_mb: Some(100_000),
            ..Default::default()
        };
        assert_eq!(settings.memory(), Some((u16::MAX, u16::MAX)));
        assert!(settings.has_launch_overrides());
    }

    #[test]
    fn json_helpers_are_lenient() {
        let value: serde_json::Value = serde_json::json!({
            "num": 42,
            "numstr": "43",
            "flag": "true",
            "flagnum": 0,
        });
        assert_eq!(json_string(&value, "num").as_deref(), Some("42"));
        assert_eq!(json_i64(&value, "numstr"), Some(43));
        assert_eq!(json_bool(&value, "flag"), Some(true));
        assert_eq!(json_bool(&value, "flagnum"), Some(false));
        assert_eq!(json_i64(&value, "missing"), None);
    }
}
