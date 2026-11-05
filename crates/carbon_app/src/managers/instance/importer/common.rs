use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use tracing::{trace, warn};

use crate::{
    api::translation::Translation,
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
    require_source_directory(from).await?;
    let to = instance_path.join("instance");
    carbon_rt_path::copy_dir_filter(from, &to, filter).await?;
    tokio::fs::create_dir_all(instance_path.join(".setup/modpack-complete")).await?;
    Ok(())
}

/// Validate a required selected root separately when it differs from its payload.
pub(super) async fn require_source_directory(from: &Path) -> anyhow::Result<()> {
    let metadata = tokio::fs::metadata(from).await.map_err(|e| {
        anyhow::anyhow!(
            "Import source {} is unavailable; reconnect it and rescan: {e}",
            from.display()
        )
    })?;
    anyhow::ensure!(
        metadata.is_dir(),
        "Import source {} is not a directory; rescan",
        from.display()
    );
    Ok(())
}

/// Only callers that observed an optional/default payload absent during scanning may use this.
pub async fn copy_optional_game_dir<F>(
    from: &Path,
    instance_path: &Path,
    filter: F,
) -> anyhow::Result<()>
where
    F: for<'a> Fn(&'a Path) -> bool,
{
    match tokio::fs::metadata(from).await {
        Ok(meta) if meta.is_dir() => return copy_game_dir(from, instance_path, filter).await,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Ok(_) => anyhow::bail!(
            "Optional import payload {} is not a directory",
            from.display()
        ),
        Err(error) => return Err(error.into()),
    }

    tokio::fs::create_dir_all(instance_path.join(".setup").join("modpack-complete")).await?;
    Ok(())
}

pub fn first_existing_dir(candidates: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|p| p.is_dir())
}

/// Keeps the first occurrence of each path and drops the rest.
///
/// Launcher candidate lists name the same directory more than once: `data_dir`
/// and `config_dir` are the same directory on macOS and Windows, and `data_dir`
/// is `~/.local/share` on Linux, which several lists also spell out by hand. The
/// repeats are only visible at runtime, so they cannot be removed at the call site.
pub fn dedup_paths(candidates: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    candidates
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .collect()
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

/// Trims, and treats an all-whitespace value as absent.
pub fn trimmed_non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Rejects zero, negatives, and anything too wide for a `u32`.
pub fn positive_u32(value: Option<i64>) -> Option<u32> {
    value.filter(|v| *v > 0).and_then(|v| u32::try_from(v).ok())
}

pub fn json_trimmed_string(value: &serde_json::Value, key: &str) -> Option<String> {
    trimmed_non_empty(json_string(value, key))
}

pub fn json_positive_u32(value: &serde_json::Value, key: &str) -> Option<u32> {
    positive_u32(json_i64(value, key))
}

/// Fails with the offending path when a candidate list repeats an entry.
///
/// Which paths coincide depends on the platform, so this only proves the list is
/// clean on the host it runs on; CI covers Linux, macOS and Windows separately.
#[cfg(test)]
pub fn assert_paths_unique(candidates: &[PathBuf]) {
    let mut seen = HashSet::new();
    for path in candidates {
        assert!(
            seen.insert(path),
            "duplicate candidate {path:?} in {candidates:?}"
        );
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

/// A source setting left out of an import because GDLauncher cannot use it.
/// The instance is still imported; the row tells the user what was left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkippedSetting {
    /// The source's Java path no longer exists, so GDLauncher picks Java itself.
    MissingJavaPath,
    /// Enabled commands use launcher variables GDLauncher cannot fill in.
    LauncherVariableCommands,
}

impl SkippedSetting {
    pub fn translation(self) -> Translation {
        match self {
            Self::MissingJavaPath => Translation::InstanceImportJavaPathSkipped,
            Self::LauncherVariableCommands => Translation::InstanceImportCommandsSkipped,
        }
    }
}

/// Launch runs the path as written, so a bare command (Prism's default `java`)
/// is found the way launch would find it: on `PATH`. Windows also starts a path
/// written without `.exe`.
fn java_command_is_found(path: &str) -> bool {
    if path.contains(['/', '\\']) {
        return Path::new(path).is_file()
            || (cfg!(windows) && Path::new(&format!("{path}.exe")).is_file());
    }
    let names = [path.to_string(), format!("{path}.exe")];
    std::env::var_os("PATH").is_some_and(|dirs| {
        std::env::split_paths(&dirs).any(|dir| names.iter().any(|name| dir.join(name).is_file()))
    })
}

fn java_binary_in_folder(folder: &Path) -> Option<PathBuf> {
    let binary = if cfg!(windows) { "java.exe" } else { "java" };
    [folder.join(binary), folder.join("bin").join(binary)]
        .into_iter()
        .find(|path| path.is_file())
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
    /// Source settings dropped at scan time, shown on the import row.
    pub skipped: Vec<SkippedSetting>,
}

impl ImportedSettings {
    pub fn skip(&mut self, setting: SkippedSetting) {
        if !self.skipped.contains(&setting) {
            self.skipped.push(setting);
        }
    }

    pub fn warnings(&self) -> Vec<Translation> {
        self.skipped.iter().map(|s| s.translation()).collect()
    }

    /// A Java override whose binary is gone would stop every launch, so the
    /// instance is imported without it and GDLauncher picks Java instead. A
    /// folder holding Java points at its binary, as launch needs the binary.
    pub fn skip_missing_java_path(&mut self) {
        let Some(path) = self.java_path.as_deref() else {
            return;
        };
        if java_command_is_found(path) {
            return;
        }
        if let Some(binary) = java_binary_in_folder(Path::new(path)) {
            self.java_path = Some(binary.to_string_lossy().into_owned());
            return;
        }
        warn!("Source Java `{path}` no longer exists; importing without a Java override");
        self.java_path = None;
        self.skip(SkippedSetting::MissingJavaPath);
    }

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

    /// Source arrays already contain exact tokens. Encode them for the launch
    /// tokenizer without trimming or splitting their literal contents.
    /// Blank elements are dropped, as XMCL and the Modrinth App do when they
    /// launch: Java would take one as the main class and refuse to start.
    pub(crate) fn absorb_jvm_argv<'a>(&mut self, args: impl IntoIterator<Item = &'a str>) {
        let args: Vec<_> = args
            .into_iter()
            .filter(|arg| !arg.trim().is_empty())
            .collect();
        let is_heap_arg = |arg: &str| arg.starts_with("-Xmx") || arg.starts_with("-Xms");
        let mut proposed = Self {
            memory_min_mb: self.memory_min_mb,
            memory_max_mb: self.memory_max_mb,
            ..Default::default()
        };
        // Managed flags precede extra argv at launch. Moving any heap token
        // ahead of a retained one would change source precedence, even across calls.
        let mut absorb_heap = !self.extra_java_args.as_deref().is_some_and(|existing| {
            crate::managers::minecraft::minecraft::parse_extra_java_args(existing)
                .iter()
                .any(|arg| is_heap_arg(arg))
        });
        for arg in &args {
            let target = if let Some(value) = arg.strip_prefix("-Xmx") {
                Some((&mut proposed.memory_max_mb, value))
            } else {
                arg.strip_prefix("-Xms")
                    .map(|value| (&mut proposed.memory_min_mb, value))
            };
            if let Some((field, value)) = target {
                if let Some(mb) = parse_jvm_memory_mb(value) {
                    *field = Some(mb);
                } else {
                    absorb_heap = false;
                }
            }
        }
        // GDL always passes its own -Xmx first, so a literal -Xms without any
        // maximum could exceed it and stop Java from starting. The source's
        // JVM raised its default maximum to fit instead. Keep such a minimum
        // only as the managed value, which pairs with a maximum if one comes.
        let lone_minimum = absorb_heap && proposed.memory_max_mb.is_none_or(|max| max == 0);
        // Check the actual published pair after last-wins planning: memory()
        // otherwise omits a lone minimum or clamps values to the maximum/u16.
        absorb_heap &= lone_minimum
            || proposed.memory().is_some_and(|(min, max)| {
                proposed.memory_max_mb == Some(u32::from(max))
                    && proposed
                        .memory_min_mb
                        .map_or(true, |value| value == u32::from(min))
            });
        if absorb_heap {
            self.memory_min_mb = proposed.memory_min_mb;
            self.memory_max_mb = proposed.memory_max_mb;
        }

        let mut rest = Vec::new();
        for arg in args {
            if absorb_heap && is_heap_arg(arg) {
                continue;
            }
            let encoded = if arg.is_empty()
                || arg
                    .chars()
                    .any(|c| c.is_whitespace() || c == '"' || c == '\\')
            {
                format!("\"{}\"", arg.replace('\\', "\\\\").replace('"', "\\\""))
            } else {
                arg.to_string()
            };
            rest.push(encoded);
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

/// Convert an exact JVM heap-size value to the existing managed MiB field.
/// Bare decimal values are bytes. Other JVM forms are left literal unless the
/// whole value is recognized and representable without truncation or clamping.
fn parse_jvm_memory_mb(value: &str) -> Option<u32> {
    const MIB: u64 = 1024 * 1024;
    let number_end = value
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(value.len());
    let (digits, suffix) = value.split_at(number_end);
    let amount = digits.parse::<u64>().ok()?;
    let scale = match suffix {
        "" => 1,
        "K" | "k" => 1024,
        "M" | "m" => MIB,
        "G" | "g" => MIB * 1024,
        "T" | "t" => MIB * MIB,
        _ => return None,
    };
    let bytes = amount.checked_mul(scale)?;
    if bytes == 0 || bytes % MIB != 0 {
        return None;
    }
    u16::try_from(bytes / MIB).ok().map(u32::from)
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
mod java_path_tests {
    use super::*;

    fn skipped(java_path: &str) -> ImportedSettings {
        let mut settings = ImportedSettings {
            java_path: Some(java_path.to_string()),
            ..Default::default()
        };
        settings.skip_missing_java_path();
        settings
    }

    #[test]
    fn a_java_path_that_is_gone_is_dropped_and_reported() {
        let t = tempfile::tempdir().unwrap();
        let settings = skipped(&t.path().join("jre").join("javaw.exe").display().to_string());
        assert_eq!(settings.java_path, None);
        assert_eq!(settings.skipped, [SkippedSetting::MissingJavaPath]);
    }

    #[test]
    fn a_java_binary_that_is_there_is_kept() {
        let t = tempfile::tempdir().unwrap();
        let binary = t.path().join("my-java");
        std::fs::write(&binary, b"").unwrap();
        let settings = skipped(binary.to_str().unwrap());
        assert_eq!(settings.java_path.as_deref(), binary.to_str());
        assert!(settings.skipped.is_empty());
    }

    /// Windows starts `C:\...\bin\javaw` as `javaw.exe`.
    #[cfg(windows)]
    #[test]
    fn a_windows_path_without_exe_is_kept() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join("javaw.exe"), b"").unwrap();
        let declared = t.path().join("javaw").display().to_string();
        let settings = skipped(&declared);
        assert_eq!(settings.java_path.as_deref(), Some(declared.as_str()));
        assert!(settings.skipped.is_empty());
    }

    /// Launch looks a bare command up on PATH, so that is what decides.
    #[test]
    fn a_bare_command_is_looked_up_on_path() {
        let on_path = if cfg!(windows) { "cmd" } else { "sh" };
        let settings = skipped(on_path);
        assert_eq!(settings.java_path.as_deref(), Some(on_path));
        assert!(settings.skipped.is_empty());

        let settings = skipped("gdl-no-such-java-command");
        assert_eq!(settings.java_path, None);
        assert_eq!(settings.skipped, [SkippedSetting::MissingJavaPath]);
    }

    /// PCL can store the folder Java is in, and launch needs the binary.
    #[test]
    fn a_folder_holding_java_becomes_its_binary() {
        let t = tempfile::tempdir().unwrap();
        let binary_name = if cfg!(windows) { "java.exe" } else { "java" };
        let bin = t.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(bin.join(binary_name), b"").unwrap();
        for folder in [t.path(), bin.as_path()] {
            let settings = skipped(folder.to_str().unwrap());
            assert_eq!(
                settings.java_path.as_deref(),
                bin.join(binary_name).to_str()
            );
            assert!(settings.skipped.is_empty());
        }

        let empty = tempfile::tempdir().unwrap();
        assert_eq!(skipped(empty.path().to_str().unwrap()).java_path, None);
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

    #[test]
    fn blank_strings_are_treated_as_absent() {
        let value = serde_json::json!({
            "name": "  Pack  ",
            "blank": "   ",
            "empty": "",
            "number": 12,
        });

        assert_eq!(
            json_trimmed_string(&value, "name"),
            Some("Pack".to_string())
        );
        assert_eq!(json_trimmed_string(&value, "blank"), None);
        assert_eq!(json_trimmed_string(&value, "empty"), None);
        assert_eq!(json_trimmed_string(&value, "missing"), None);
        // `json_string` coerces numbers, so the trimmed form does too.
        assert_eq!(
            json_trimmed_string(&value, "number"),
            Some("12".to_string())
        );
        assert_eq!(trimmed_non_empty(None), None);
    }

    #[test]
    fn repeated_candidate_paths_collapse_to_the_first() {
        let candidates = vec![
            PathBuf::from("/a"),
            PathBuf::from("/b"),
            PathBuf::from("/a"),
            PathBuf::from("/c"),
            PathBuf::from("/b"),
        ];

        assert_eq!(
            dedup_paths(candidates),
            vec![
                PathBuf::from("/a"),
                PathBuf::from("/b"),
                PathBuf::from("/c")
            ]
        );
        assert!(dedup_paths(Vec::new()).is_empty());
    }

    #[test]
    fn non_positive_and_oversized_numbers_are_rejected() {
        let value = serde_json::json!({
            "ok": 4096,
            "zero": 0,
            "negative": -1,
            "huge": 5_000_000_000i64,
            "stringy": "2048",
        });

        assert_eq!(json_positive_u32(&value, "ok"), Some(4096));
        assert_eq!(json_positive_u32(&value, "zero"), None);
        assert_eq!(json_positive_u32(&value, "negative"), None);
        assert_eq!(json_positive_u32(&value, "huge"), None);
        assert_eq!(json_positive_u32(&value, "missing"), None);
        // `json_i64` parses numeric strings, so the positive form does too.
        assert_eq!(json_positive_u32(&value, "stringy"), Some(2048));
        assert_eq!(positive_u32(None), None);
    }
}

#[cfg(test)]
mod task06_tests {
    use super::*;
    #[cfg(unix)]
    #[tokio::test]
    async fn task06_copy_marker_follows_included_success() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let world = source.join("saves/world");
        std::fs::create_dir_all(&world).unwrap();
        std::fs::write(world.join("level.dat"), b"world").unwrap();
        struct Restore(PathBuf);
        impl Drop for Restore {
            fn drop(&mut self) {
                std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o700)).unwrap();
            }
        }
        let restore = Restore(world.clone());
        std::fs::set_permissions(&world, std::fs::Permissions::from_mode(0)).unwrap();
        assert!(std::fs::read_dir(&world).is_err());
        let destination = root.path().join("failed");
        assert!(
            copy_game_dir(&source, &destination, |_| true)
                .await
                .is_err()
        );
        assert!(
            !destination.join(".setup/modpack-complete").exists(),
            "failed copy cannot have a completion marker"
        );
        drop(restore);
        let destination = root.path().join("success");
        copy_game_dir(&source, &destination, |_| true)
            .await
            .unwrap();
        assert!(destination.join(".setup/modpack-complete").is_dir());
        assert_eq!(
            std::fs::read(destination.join("instance/saves/world/level.dat")).unwrap(),
            b"world"
        );
        let destination = root.path().join("empty");
        copy_optional_game_dir(&root.path().join("missing"), &destination, |_| true)
            .await
            .unwrap();
        assert!(destination.join(".setup/modpack-complete").is_dir());
    }
}

#[cfg(test)]
mod task13_tests {
    use super::*;
    #[tokio::test]
    async fn task13_moved_required_source_fails_without_completion() {
        let t = tempfile::tempdir().unwrap();
        let source = t.path().join("source");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("world"), b"synthetic world").unwrap();
        std::fs::rename(&source, t.path().join("moved")).unwrap();
        let dest = t.path().join("destination");
        assert!(copy_game_dir(&source, &dest, |_| true).await.is_err());
        assert!(!dest.join(".setup/modpack-complete").exists());
        assert_eq!(
            std::fs::read(t.path().join("moved/world")).unwrap(),
            b"synthetic world"
        );
    }
}

pub(super) struct ScanDirectory {
    inner: tokio::fs::ReadDir,
    #[cfg(test)]
    path: PathBuf,
    #[cfg(test)]
    delivered: usize,
}
impl ScanDirectory {
    pub async fn next_entry(&mut self) -> std::io::Result<Option<tokio::fs::DirEntry>> {
        #[cfg(test)]
        if SCAN_FAILURES
            .lock()
            .unwrap()
            .get(&self.path)
            .is_some_and(|limit| self.delivered >= *limit)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "synthetic iteration failure",
            ));
        }
        let result = self.inner.next_entry().await?;
        #[cfg(test)]
        {
            if result.is_some() {
                self.delivered += 1;
            }
        }
        Ok(result)
    }
}
pub(super) async fn scan_directory(path: &Path) -> std::io::Result<ScanDirectory> {
    #[cfg(test)]
    if SCAN_FAILURES.lock().unwrap().get(path) == Some(&0) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "synthetic scan IO failure",
        ));
    }
    Ok(ScanDirectory {
        inner: tokio::fs::read_dir(path).await?,
        #[cfg(test)]
        path: path.to_path_buf(),
        #[cfg(test)]
        delivered: 0,
    })
}
#[cfg(test)]
static SCAN_FAILURES: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<PathBuf, usize>>,
> = std::sync::LazyLock::new(Default::default);
#[cfg(test)]
pub(super) fn fail_scan_directory(path: PathBuf) {
    SCAN_FAILURES.lock().unwrap().insert(path, 0);
}

#[cfg(test)]
pub(super) fn fail_scan_after(path: PathBuf, count: usize) {
    SCAN_FAILURES.lock().unwrap().insert(path, count);
}
#[cfg(test)]
pub(super) fn clear_scan_failure(path: &Path) {
    SCAN_FAILURES.lock().unwrap().remove(path);
}
#[cfg(test)]
mod task13_copy_controls {
    use super::*;
    #[tokio::test]
    async fn task13_present_empty_and_optional_missing_are_distinct() {
        let t = tempfile::tempdir().unwrap();
        let source = t.path().join("source");
        std::fs::create_dir(&source).unwrap();
        let empty = t.path().join("empty");
        copy_game_dir(&source, &empty, |_| true).await.unwrap();
        assert!(empty.join(".setup/modpack-complete").exists());
        std::fs::create_dir_all(source.join("saves/World")).unwrap();
        std::fs::write(source.join("saves/World/marker"), b"synthetic world").unwrap();
        let copied = t.path().join("copied");
        copy_game_dir(&source, &copied, |_| true).await.unwrap();
        assert_eq!(
            std::fs::read(copied.join("instance/saves/World/marker")).unwrap(),
            b"synthetic world"
        );
        let optional = t.path().join("optional");
        copy_optional_game_dir(&t.path().join("absent"), &optional, |_| true)
            .await
            .unwrap();
        assert!(optional.join(".setup/modpack-complete").exists());
    }
}

/// Round-trip the actual persisted schema before using the launch tokenizer.
#[cfg(test)]
pub(super) fn task14_persisted_config(
    settings: &ImportedSettings,
) -> crate::domain::instance::info::GameConfig {
    use crate::domain::instance::info::{GameConfig, Instance, InstanceIcon};
    use crate::managers::instance::schema::{make_instance_config, parse_instance_config};
    let info = Instance {
        name: "argv fixture".into(),
        icon: InstanceIcon::Default,
        date_created: chrono::Utc::now(),
        date_updated: chrono::Utc::now(),
        last_played: None,
        seconds_played: 0,
        modpack: None,
        game_configuration: GameConfig {
            version: None,
            global_java_args: false,
            extra_java_args: settings.extra_java_args.clone(),
            java_override: None,
            memory: settings.memory(),
            game_resolution: None,
        },
        pre_launch_hook: None,
        post_exit_hook: None,
        wrapper_command: None,
        mod_sources: None,
        notes: String::new(),
    };
    parse_instance_config(&make_instance_config(info).unwrap())
        .unwrap()
        .game_configuration
}

#[cfg(test)]
pub(super) fn task14_persisted_args(settings: &ImportedSettings) -> Vec<String> {
    let restored = task14_persisted_config(settings);
    crate::managers::minecraft::minecraft::parse_extra_java_args(
        restored.extra_java_args.as_deref().unwrap_or(""),
    )
}

#[cfg(test)]
mod task14_argv_controls {
    use super::*;
    #[test]
    fn task14_exact_argv_grammar_memory_and_append() {
        let expected = [
            "-Dfoo=bar",
            "-XX:+UseG1GC",
            "-Da=b=c",
            "-Dlabel=My  Pack",
            " leading",
            "trailing ",
            "\t\n",
            "",
            "   ",
            r#"quote"and\slash"#,
            r"C:\path\",
            r"\\server\share\file",
            "-Xmxgarbage",
            "-Xmx0G",
            "-Xmx 2G",
            "-Xmx2G ",
            "-Xms1G suffix",
            "'literal'",
        ];
        let mut settings = ImportedSettings {
            memory_max_mb: Some(8192),
            ..Default::default()
        };
        settings.absorb_jvm_argv(expected);
        // Blank elements are dropped: Java would read one as the main class.
        let kept: Vec<_> = expected
            .iter()
            .copied()
            .filter(|arg| !arg.trim().is_empty())
            .collect();
        assert_eq!(kept.len(), expected.len() - 3);
        assert_eq!(task14_persisted_args(&settings), kept);
        settings.absorb_jvm_argv(["-Xmx2G", "-Xms512M", "-Xmx3G"]);
        assert_eq!(settings.memory(), Some((8192, 8192)));
        let mut appended = kept.clone();
        appended.extend(["-Xmx2G", "-Xms512M", "-Xmx3G"]);
        assert_eq!(task14_persisted_args(&settings), appended);
        settings.absorb_jvm_argv(["", "next", " "]);
        appended.push("next");
        assert_eq!(task14_persisted_args(&settings), appended);
        let before = settings.clone();
        settings.absorb_jvm_argv([]);
        assert_eq!(settings, before);
    }
}

#[cfg(test)]
mod task14_r1_memory_tests {
    use super::*;

    fn assert_literal(tokens: &[&str]) {
        let mut settings = ImportedSettings::default();
        settings.absorb_jvm_argv(tokens.iter().copied());
        assert_eq!(task14_persisted_args(&settings), tokens);
        assert_eq!(settings.memory_max_mb, None);
        assert_eq!(settings.memory_min_mb, None);
        assert_eq!(task14_persisted_config(&settings).memory, None);
    }

    #[test]
    fn task14_r1_multi_letter_memory_units_remain_literal() {
        assert_literal(&["-Xmx2GB", "-Xms512MB", "-Xmx4096kb", "-Xms1TB"]);
    }

    #[test]
    fn task14_r1_sub_mib_bare_numbers_remain_literal() {
        assert_literal(&["-Xmx65536", "-Xms1024", "-Xmx1048575"]);
    }

    #[test]
    fn task14_r1_nonintegral_mib_memory_remains_literal() {
        assert_literal(&["-Xmx2097153", "-Xms1025K", "-Xmx67107841k"]);
    }

    #[test]
    fn task14_r1_out_of_representation_memory_remains_literal() {
        assert_literal(&[
            "-Xmx65536M",
            "-Xms64G",
            "-Xmx68719476736",
            "-Xmx18446744073709551615M",
            "-Xmx18446744073709551616",
            "-Xmx999999999999999999999999999G",
            "-Xmx1T",
        ]);
    }

    #[test]
    fn task14_r1_exact_memory_values_round_trip() {
        for (value, mb) in [
            ("1048576", 1),
            ("2097152", 2),
            ("2G", 2048),
            ("2g", 2048),
            ("512M", 512),
            ("512m", 512),
            ("2048K", 2),
            ("2048k", 2),
            ("65535M", 65535),
            ("67107840K", 65535),
            ("68718428160", 65535),
            ("63G", 64512),
        ] {
            let mut settings = ImportedSettings::default();
            let args = [format!("-Xmx{value}"), format!("-Xms{value}")];
            settings.absorb_jvm_argv(args.iter().map(String::as_str));
            assert_eq!(settings.memory_max_mb, Some(mb), "{value}");
            assert_eq!(settings.memory_min_mb, Some(mb), "{value}");
            assert_eq!(
                task14_persisted_config(&settings).memory,
                Some((mb as u16, mb as u16))
            );
            assert!(task14_persisted_args(&settings).is_empty());
        }
    }

    #[test]
    fn task14_r1_unrecognized_memory_forms_and_order_are_preserved() {
        assert_literal(&[
            "-Xmx0",
            "-Xms0M",
            "-Xmx+2G",
            "-Xms-2G",
            "-Xmx0x200000",
            "-Xmx1.5G",
            "-Xmx2 G",
            "-Xmx2G ",
            "-Xmx",
            "-XmsM",
            "-Xmx２G",
        ]);
        let mut settings = ImportedSettings::default();
        settings.absorb_jvm_argv(["-Xmx2G", "-Xms512M", "-Xmx2GB", "-Xms1025K"]);
        assert_eq!(task14_persisted_config(&settings).memory, None);
        assert_eq!(
            task14_persisted_args(&settings),
            ["-Xmx2G", "-Xms512M", "-Xmx2GB", "-Xms1025K"]
        );
        settings.absorb_jvm_argv(["-Xmx1G", "-Xms256m"]);
        assert_eq!(task14_persisted_config(&settings).memory, None);
        assert_eq!(
            task14_persisted_args(&settings),
            [
                "-Xmx2G",
                "-Xms512M",
                "-Xmx2GB",
                "-Xms1025K",
                "-Xmx1G",
                "-Xms256m"
            ]
        );
    }

    #[test]
    fn task14_r1_legacy_human_memory_policy_is_unchanged() {
        assert_eq!(parse_memory_size("2GB"), Some(2048));
        assert_eq!(parse_memory_size("512MB"), Some(512));
        assert_eq!(parse_memory_size("65536"), Some(65536));
        assert_eq!(parse_memory_size("2097153"), Some(2));
        let mut settings = ImportedSettings::default();
        settings.absorb_jvm_args(r#"-Xmx2GB -Xms512MB "-Dlabel=My Pack""#);
        assert_eq!(task14_persisted_config(&settings).memory, Some((512, 2048)));
        assert_eq!(task14_persisted_args(&settings), ["-Dlabel=My Pack"]);
    }
}

#[cfg(test)]
mod task14_r2_memory_tests {
    use super::*;

    #[test]
    fn task14_r2_retained_and_valid_maxima_preserve_source_precedence() {
        for args in [
            ["-Xmx65536M", "-Xmx2G", "-Xms512M"],
            ["-Xms512M", "-Xmx2G", "-Xmx65536M"],
        ] {
            for maximum in [None, Some(8192)] {
                let mut settings = ImportedSettings {
                    memory_max_mb: maximum,
                    ..Default::default()
                };
                let before = settings.memory();
                settings.absorb_jvm_argv(args);
                assert_eq!(task14_persisted_args(&settings), args);
                assert_eq!(task14_persisted_config(&settings).memory, before);
                assert_eq!(settings.memory_min_mb, None);
                assert_eq!(settings.memory_max_mb, maximum);
            }
        }
    }

    #[test]
    fn task14_r2_repeated_arrays_preserve_retained_heap_precedence() {
        let mut settings = ImportedSettings::default();
        settings.absorb_jvm_argv(["-Xmx65536M"]);
        settings.absorb_jvm_argv(["-Xmx2G", "-Xms256M", "-Dlabel=My Pack"]);
        assert_eq!(
            task14_persisted_args(&settings),
            ["-Xmx65536M", "-Xmx2G", "-Xms256M", "-Dlabel=My Pack"]
        );
        assert_eq!(task14_persisted_config(&settings).memory, None);
        // A lone minimum is managed, so a later pair still decides by last-wins.
        let mut settings = ImportedSettings::default();
        settings.absorb_jvm_argv(["-Xms512M"]);
        settings.absorb_jvm_argv(["-Xmx2G", "-Xms256M", "-Dlabel=My Pack"]);
        assert_eq!(task14_persisted_args(&settings), ["-Dlabel=My Pack"]);
        assert_eq!(task14_persisted_config(&settings).memory, Some((256, 2048)));
    }

    #[test]
    fn task14_r2_heap_free_prior_args_allow_managed_absorption() {
        let args = ["-Dlabel=-Xmx65536M", "-Dtext=prefix -Xms512M", ""];
        let mut settings = ImportedSettings::default();
        settings.absorb_jvm_argv(args);
        settings.absorb_jvm_argv(["-Xms512M", "-Xmx2G"]);
        settings.absorb_jvm_argv(["-Xmx3G"]);
        assert_eq!(task14_persisted_args(&settings), &args[..2]);
        assert_eq!(task14_persisted_config(&settings).memory, Some((512, 3072)));
    }

    #[test]
    fn task14_r2_unpublishable_existing_maxima_do_not_consume_minimum() {
        // A zero maximum is no maximum; see lone_minimum_without_a_maximum_is_not_passed_literally.
        let mut settings = ImportedSettings {
            memory_max_mb: Some(65536),
            ..Default::default()
        };
        let before = settings.memory();
        settings.absorb_jvm_argv(["-Xms512M"]);
        assert_eq!(task14_persisted_args(&settings), ["-Xms512M"]);
        assert_eq!(settings.memory_min_mb, None);
        assert_eq!(settings.memory_max_mb, Some(65536));
        assert_eq!(task14_persisted_config(&settings).memory, before);
    }

    #[test]
    fn task14_r2_retained_maximum_keeps_minimum_in_both_orders() {
        for maximum in ["-Xmx65536M", "-Xmx2GB"] {
            for args in [[maximum, "-Xms512M"], ["-Xms512M", maximum]] {
                let mut settings = ImportedSettings::default();
                settings.absorb_jvm_argv(args);
                assert_eq!(task14_persisted_config(&settings).memory, None);
                assert_eq!(task14_persisted_args(&settings), args);
            }
        }
    }

    /// GDL passes its own -Xmx before extra arguments; a literal -Xms above it
    /// stops Java ("Initial heap size set to a larger value than the maximum").
    #[test]
    fn lone_minimum_without_a_maximum_is_not_passed_literally() {
        for maximum in [None, Some(0)] {
            let mut settings = ImportedSettings {
                memory_max_mb: maximum,
                ..Default::default()
            };
            settings.absorb_jvm_argv(["-Xms4G", "-XX:+UseG1GC"]);
            assert_eq!(task14_persisted_args(&settings), ["-XX:+UseG1GC"]);
            assert_eq!(task14_persisted_config(&settings).memory, None);
            assert_eq!(settings.memory_min_mb, Some(4096));
        }
        let mut settings = ImportedSettings::default();
        settings.absorb_jvm_argv(["-Xms512M"]);
        assert!(task14_persisted_args(&settings).is_empty());
        assert!(!settings.has_launch_overrides());
        // A maximum that arrives later pairs with the kept minimum.
        settings.memory_max_mb = Some(2048);
        assert_eq!(settings.memory(), Some((512, 2048)));
    }

    #[test]
    fn task14_r2_clamped_combination_keeps_literal_order() {
        for args in [["-Xms4G", "-Xmx2G"], ["-Xmx2G", "-Xms4G"]] {
            let mut settings = ImportedSettings::default();
            settings.absorb_jvm_argv(args);
            assert_eq!(task14_persisted_args(&settings), args);
            assert_eq!(task14_persisted_config(&settings).memory, None);
        }
    }

    #[test]
    fn task14_r2_preexisting_minimum_is_not_clamped_by_consumption() {
        let mut settings = ImportedSettings {
            memory_min_mb: Some(4096),
            memory_max_mb: Some(8192),
            ..Default::default()
        };
        settings.absorb_jvm_argv(["-Xmx2G"]);
        assert_eq!(task14_persisted_args(&settings), ["-Xmx2G"]);
        assert_eq!(
            task14_persisted_config(&settings).memory,
            Some((4096, 8192))
        );
    }

    #[test]
    fn task14_r2_later_and_existing_valid_maxima_publish_exactly() {
        for args in [["-Xms512M", "-Xmx2G"], ["-Xmx2G", "-Xms512M"]] {
            let mut settings = ImportedSettings::default();
            settings.absorb_jvm_argv(args);
            assert_eq!(task14_persisted_config(&settings).memory, Some((512, 2048)));
            assert!(task14_persisted_args(&settings).is_empty());
        }
        let mut settings = ImportedSettings {
            memory_max_mb: Some(4096),
            ..Default::default()
        };
        settings.absorb_jvm_argv(["-Xms512M"]);
        assert_eq!(task14_persisted_config(&settings).memory, Some((512, 4096)));
        assert!(task14_persisted_args(&settings).is_empty());
    }

    #[test]
    fn task14_r2_last_valid_values_decide_combination() {
        let mut settings = ImportedSettings::default();
        settings.absorb_jvm_argv(["-Xms4G", "-Xmx2G", "-Xms512M", "-Xmx3G"]);
        assert_eq!(task14_persisted_config(&settings).memory, Some((512, 3072)));
        assert!(task14_persisted_args(&settings).is_empty());
        let mut settings = ImportedSettings::default();
        let args = ["-Xms512M", "-Xmx2G", "-Dlabel=My Pack", "-Xms4G"];
        settings.absorb_jvm_argv(args);
        assert_eq!(task14_persisted_args(&settings), args);
        assert_eq!(task14_persisted_config(&settings).memory, None);
    }
}
