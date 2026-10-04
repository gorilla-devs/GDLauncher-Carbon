use std::{
    io::Read,
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
        copy_game_dir, dedup_paths, dir_name, first_existing_dir, json_i64, json_positive_u32,
        json_string, json_trimmed_string, loaders_from_library_names, looks_like_minecraft_version,
        parse_version_id, read_json_value, resolve_standard_version, trimmed_non_empty,
    },
};
use crate::{
    api::translation::Translation,
    domain::{instance::info::GameVersion, vtask::VisualTaskId},
    managers::{AppInner, instance::InstanceVersionSource},
};

const TECHNIC_API: &str = "https://api.technicpack.net/";
/// The launcher build number the API gates on; any recent number works.
const TECHNIC_LAUNCHER_BUILD: &str = "999";

#[derive(Debug, Clone)]
struct Importable {
    filename: String,
    path: PathBuf,
    name: String,
    icon_path: Option<PathBuf>,
    icon_url: Option<String>,
    minecraft_version: String,
    loaders: Vec<DeclaredModLoader>,
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
pub struct TechnicImporter {
    state: RwLock<ImporterState<Importable>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocatedPack {
    slug: String,
    path: PathBuf,
    launcher_root: Option<PathBuf>,
}

impl TechnicImporter {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(ImporterState::NoResults),
        }
    }

    pub async fn get_default_scan_path() -> anyhow::Result<Option<PathBuf>> {
        Ok(first_existing_dir(launcher_dir_candidates()))
    }

    async fn locate_pack_info(pack: &LocatedPack) -> (ParsedVersionId, Option<PathBuf>) {
        let versions = detect_versions(&pack.path.join("bin")).await;

        let icon_path = pack.launcher_root.as_ref().and_then(|root| {
            let icon = root
                .join("assets")
                .join("packs")
                .join(&pack.slug)
                .join("icon.png");
            icon.is_file().then_some(icon)
        });

        (versions, icon_path)
    }

    async fn scan_pack(
        &self,
        app: &Arc<AppInner>,
        pack: LocatedPack,
    ) -> Option<InternalImportEntry<Importable>> {
        if !pack.path.is_dir() {
            return None;
        }

        let filename = dir_name(&pack.path);
        let (mut versions, icon_path) = Self::locate_pack_info(&pack).await;

        // Best-effort: local packs aren't on the platform at all.
        let platform = fetch_platform_pack_info(app, &pack.slug).await;

        if versions.minecraft.is_none() {
            if let Some(mc) = platform.as_ref().and_then(|p| p.minecraft.clone()) {
                versions.minecraft = Some(mc);
            }
        }
        if versions.loaders.is_empty() {
            if let Some(forge) = platform.as_ref().and_then(|p| p.forge.clone()) {
                if let Some(mc) = versions.minecraft.as_deref() {
                    versions.loaders.push(DeclaredModLoader::new(
                        crate::domain::instance::info::ModLoaderType::Forge,
                        super::common::normalize_forge_version(mc, &forge),
                    ));
                }
            }
        }

        let Some(minecraft_version) = versions.minecraft else {
            warn!(
                "Cannot determine the Minecraft version of Technic pack {} at {:?}",
                pack.slug, pack.path
            );
            return Some(InternalImportEntry::Invalid(InvalidImportEntry {
                name: filename,
                reason: Translation::InstanceImportLegacyBadConfigFile,
            }));
        };

        let name = platform
            .as_ref()
            .and_then(|p| p.display_name.clone())
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| pack.slug.clone());

        // The memory the pack was launched with, in MB.
        let settings = ImportedSettings {
            memory_max_mb: read_json_value(&pack.path.join("bin").join("runData"))
                .await
                .and_then(|run_data| json_positive_u32(&run_data, "memory")),
            ..Default::default()
        };

        Some(InternalImportEntry::Valid(Importable {
            filename,
            path: pack.path,
            name,
            icon_path,
            icon_url: platform.and_then(|p| p.icon_url),
            minecraft_version,
            loaders: versions.loaders,
            settings,
        }))
    }
}

#[async_trait::async_trait]
impl InstanceImporter for TechnicImporter {
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

        let (single, packs) = locate_packs(&scan_path).await?;

        if single {
            if let Some(pack) = packs.into_iter().next() {
                if let Some(entry) = self.scan_pack(app, pack).await {
                    self.state.write().await.set_single(entry).await;
                    app.invalidate(GET_IMPORT_SCAN_STATUS, None);
                }
            }
            return Ok(());
        }

        for pack in packs {
            if let Some(entry) = self.scan_pack(app, pack).await {
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
        trace!(?index, ?name, "Beginning Technic import");

        let instance = self
            .state
            .read()
            .await
            .get(index)
            .await
            .cloned()
            .ok_or_else(|| anyhow!("invalid importable instance index {index}"))?;

        let mut icon = match &instance.icon_path {
            Some(icon_path) => app
                .instance_manager()
                .load_icon(icon_path.clone())
                .await
                .ok(),
            None => None,
        };
        if icon.is_none() {
            if let Some(url) = &instance.icon_url {
                icon = app.instance_manager().try_download_icon(url).await;
            }
        }

        let standard_version =
            resolve_standard_version(app, &instance.minecraft_version, &instance.loaders).await;
        let instance_version_source =
            InstanceVersionSource::Version(GameVersion::Standard(standard_version));

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

        let id = app
            .instance_manager()
            .create_instance_ext(
                app.instance_manager().get_default_group().await?,
                name.unwrap_or_else(|| instance.name.clone()),
                icon,
                None,
                None,
                instance_version_source,
                String::new(),
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
    first == "bin" || first == "natives" || first == "cache"
}

async fn locate_packs(scan_path: &Path) -> anyhow::Result<(bool, Vec<LocatedPack>)> {
    if is_pack_dir(scan_path) {
        let launcher_root = scan_path
            .parent()
            .filter(|p| dir_name(p) == "modpacks")
            .and_then(|p| p.parent())
            .map(Path::to_path_buf);
        return Ok((
            true,
            vec![LocatedPack {
                slug: dir_name(scan_path),
                path: scan_path.to_path_buf(),
                launcher_root,
            }],
        ));
    }

    let (launcher_root, modpacks_dir) =
        if scan_path.join("installedPacks").is_file() || scan_path.join("modpacks").is_dir() {
            (Some(scan_path.to_path_buf()), scan_path.join("modpacks"))
        } else if dir_name(scan_path) == "modpacks" {
            (
                scan_path.parent().map(Path::to_path_buf),
                scan_path.to_path_buf(),
            )
        } else {
            (None, scan_path.to_path_buf())
        };

    let mut packs: Vec<LocatedPack> = Vec::new();

    if let Some(root) = &launcher_root {
        if let Some(store) = read_json_value(&root.join("installedPacks")).await {
            for (slug, path) in installed_packs(&store, root, &modpacks_dir) {
                if path.is_dir() && !packs.iter().any(|p| p.path == path) {
                    packs.push(LocatedPack {
                        slug,
                        path,
                        launcher_root: Some(root.clone()),
                    });
                }
            }
        }
    }

    // Packs installedPacks doesn't mention (or no installedPacks at all).
    let dir = match super::common::scan_directory(&modpacks_dir).await {
        Ok(dir) => Some(dir),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && launcher_root.is_some() => None,
        Err(e) => return Err(e.into()),
    };
    if let Some(mut dir) = dir {
        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if !path.is_dir() || packs.iter().any(|p| p.path == path) || !is_pack_dir(&path) {
                continue;
            }
            packs.push(LocatedPack {
                slug: dir_name(&path),
                path,
                launcher_root: launcher_root.clone(),
            });
        }
    }

    Ok((false, packs))
}

fn is_pack_dir(path: &Path) -> bool {
    path.is_dir()
        && (path.join("bin").is_dir()
            || (path.join("mods").is_dir() && path.join("config").is_dir()))
        && !path.join("installedPacks").is_file()
        && !path.join("modpacks").is_dir()
}

/// Resolves the launcher's `%MODPACKS%\` and `launcher\` placeholders.
fn installed_packs(
    store: &serde_json::Value,
    launcher_root: &Path,
    modpacks_dir: &Path,
) -> Vec<(String, PathBuf)> {
    let Some(map) = store
        .get("installedPacks")
        .and_then(|v| v.as_object())
        .or_else(|| store.as_object())
    else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for (key, pack) in map {
        if !pack.is_object() {
            continue;
        }
        let slug = json_string(pack, "name")
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| key.clone());
        if slug == "addpack" {
            continue;
        }

        let path = match json_string(pack, "directory") {
            Some(dir) if !dir.trim().is_empty() => {
                resolve_pack_directory(&dir, launcher_root, modpacks_dir)
            }
            _ => modpacks_dir.join(&slug),
        };

        out.push((slug, path));
    }

    out
}

fn resolve_pack_directory(raw: &str, launcher_root: &Path, modpacks_dir: &Path) -> PathBuf {
    let raw = raw.trim();
    let lower = raw.to_ascii_lowercase();

    let (base, rest) = if let Some(rest) = lower
        .strip_prefix("%modpacks%\\")
        .or_else(|| lower.strip_prefix("%modpacks%/"))
    {
        (Some(modpacks_dir), &raw[raw.len() - rest.len()..])
    } else if let Some(rest) = lower
        .strip_prefix("launcher\\")
        .or_else(|| lower.strip_prefix("launcher/"))
    {
        (Some(launcher_root), &raw[raw.len() - rest.len()..])
    } else {
        (None, raw)
    };

    match base {
        Some(base) => {
            let mut path = base.to_path_buf();
            for component in rest.split(['\\', '/']).filter(|c| !c.is_empty()) {
                path.push(component);
            }
            path
        }
        None => PathBuf::from(raw),
    }
}

async fn detect_versions(bin: &Path) -> ParsedVersionId {
    let mut result = ParsedVersionId::default();

    if let Some(json) = read_json_value(&bin.join("version.json")).await {
        merge_versions(&mut result, versions_from_version_json(&json));
    }

    if result.minecraft.is_none() || result.loaders.is_empty() {
        let jar = bin.join("modpack.jar");
        if jar.is_file() {
            let from_jar = tokio::task::spawn_blocking(move || versions_from_modpack_jar(&jar))
                .await
                .unwrap_or_default();
            merge_versions(&mut result, from_jar);
        }
    }

    result
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

// A folder label is only a fallback when it matches a release ID grammar.
// In particular, a digit prefix does not make "1.20.1-MyPack" authoritative.
pub(super) fn is_release_id(value: &str) -> bool {
    static RELEASE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(concat!(
            r"^(?:",
            r"[0-9]+\.[0-9]+(?:\.[0-9]+)?(?:-(?:snapshot-[0-9]+|(?:pre|rc)-?[0-9]+)| Pre-[Rr]elease [0-9]+| Release Candidate [0-9]+)?",
            // Published exceptional snapshots have named suffix families; do
            // not accept arbitrary appended pack names as release authority.
            r"|[0-9]{2}w[0-9]{2}(?:[a-z](?:_or_[a-z])?|craftmine|potato|infinite|oneblockatatime)",
            r"|3D Shareware v1\.34",
            r"|1\.RV-Pre[0-9]+",
            r"|[abc][0-9]+(?:\.[0-9]+)+[a-z]?(?:_[0-9]+[a-z]?)?(?:-pre[0-9]+)?",
            r"|rd-[0-9]+|inf-[0-9]+(?:-[0-9]+)?",
            r")$"
        )).expect("release ID grammar")
    });
    RELEASE.is_match(value)
}

pub(super) fn versions_from_version_json(json: &serde_json::Value) -> ParsedVersionId {
    let mut result = ParsedVersionId::default();

    let library_names: Vec<&str> = json
        .get("libraries")
        .and_then(|l| l.as_array())
        .map(|libs| {
            libs.iter()
                .filter_map(|lib| lib.get("name").and_then(|n| n.as_str()))
                .collect()
        })
        .unwrap_or_default();
    let from_libraries = loaders_from_library_names(library_names.iter().copied());

    let from_id = json_string(json, "id")
        .map(|id| parse_version_id(&id))
        .unwrap_or_default();

    result.minecraft = json_string(json, "inheritsFrom")
        .filter(|v| is_release_id(v))
        .or(from_id.minecraft.clone().filter(|v| is_release_id(v)))
        .or(from_libraries.minecraft.clone());

    // Keep the id's loader when the libraries don't mention one (very old Forge jsons).
    result.loaders = from_libraries.loaders;
    for loader in from_id.loaders {
        if !result.loaders.iter().any(|l| l.type_ == loader.type_) {
            result.loaders.push(loader);
        }
    }

    result
}

/// `install_profile.json` nests the version json under `versionInfo` (old) or `version` (new).
fn versions_from_modpack_jar(jar: &Path) -> ParsedVersionId {
    let mut result = ParsedVersionId::default();

    let Ok(file) = std::fs::File::open(jar) else {
        return result;
    };
    let Ok(mut zip) = zip::ZipArchive::new(file) else {
        return result;
    };

    let mut read_json = |name: &str| -> Option<serde_json::Value> {
        let mut entry = zip.by_name(name).ok()?;
        let mut data = Vec::new();
        entry.read_to_end(&mut data).ok()?;
        serde_json::from_slice(&data).ok()
    };

    if let Some(json) = read_json("version.json") {
        merge_versions(&mut result, versions_from_version_json(&json));
    }

    if let Some(profile) = read_json("install_profile.json") {
        if let Some(version_info) = profile.get("versionInfo").filter(|v| v.is_object()) {
            merge_versions(&mut result, versions_from_version_json(version_info));
        }

        let mut from_profile = ParsedVersionId::default();
        if let Some(mc) =
            json_string(&profile, "minecraft").filter(|v| looks_like_minecraft_version(v))
        {
            from_profile.minecraft = Some(mc);
        }
        if let Some(version) = json_string(&profile, "version") {
            let parsed = parse_version_id(&version);
            if from_profile.minecraft.is_none() {
                from_profile.minecraft = parsed.minecraft;
            }
            from_profile.loaders = parsed.loaders;
        }
        merge_versions(&mut result, from_profile);
    }

    result
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct PlatformPackInfo {
    display_name: Option<String>,
    minecraft: Option<String>,
    forge: Option<String>,
    icon_url: Option<String>,
}

fn launcher_dir_candidates() -> Vec<PathBuf> {
    let Some(basedirs) = directories::BaseDirs::new() else {
        return Vec::new();
    };
    let home = basedirs.home_dir().to_path_buf();

    let mut candidates = Vec::new();
    #[cfg(target_os = "windows")]
    candidates.push(basedirs.data_dir().join(".technic"));
    #[cfg(target_os = "macos")]
    candidates.push(home.join("Library/Application Support/technic"));

    candidates.push(home.join(".technic"));
    candidates.push(home.join("technic"));
    candidates.push(basedirs.data_dir().join(".technic"));
    candidates.push(basedirs.data_dir().join("technic"));

    dedup_paths(candidates)
}

fn parse_platform_pack_info(json: &serde_json::Value) -> PlatformPackInfo {
    PlatformPackInfo {
        display_name: trimmed_non_empty(
            json_string(json, "displayName").or_else(|| json_string(json, "name")),
        ),
        minecraft: json_trimmed_string(json, "minecraft")
            .filter(|v| looks_like_minecraft_version(v)),
        forge: json_trimmed_string(json, "forge").filter(|v| v != "null"),
        icon_url: json
            .get("icon")
            .and_then(|icon| match icon {
                serde_json::Value::Object(_) => json_string(icon, "url"),
                serde_json::Value::String(url) => Some(url.clone()),
                _ => None,
            })
            .filter(|url| url.starts_with("http")),
    }
}

async fn fetch_platform_pack_info(app: &Arc<AppInner>, slug: &str) -> Option<PlatformPackInfo> {
    let slug: String = slug
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if slug.is_empty() {
        return None;
    }

    let url = format!("{TECHNIC_API}modpack/{slug}?build={TECHNIC_LAUNCHER_BUILD}");
    let response = app
        .reqwest_client
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .and_then(|r| r.error_for_status().map_err(Into::into));

    let response = match response {
        Ok(response) => response,
        Err(e) => {
            trace!("Technic platform lookup for {slug} failed: {e}");
            return None;
        }
    };

    let json = response.json::<serde_json::Value>().await.ok()?;
    if json.get("error").is_some() {
        return None;
    }
    Some(parse_platform_pack_info(&json))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn technic_launcher_dir_candidates_are_unique() {
        super::super::common::assert_paths_unique(&launcher_dir_candidates());
    }
    use crate::domain::instance::info::ModLoaderType;

    #[test]
    fn installed_packs_placeholders_are_resolved() {
        let root = Path::new("/home/user/.technic");
        let modpacks = root.join("modpacks");

        let store = serde_json::json!({
            "installedPacks": {
                "tekkit": { "name": "tekkit", "build": "1.2.3", "directory": "%MODPACKS%\\tekkit" },
                "custom": { "name": "custom", "build": "recommended", "directory": "launcher\\custompacks\\mine" },
                "elsewhere": { "name": "elsewhere", "build": "1", "directory": "/mnt/games/elsewhere" },
                "nodir": { "name": "nodir", "build": "1" },
                "addpack": { "name": "addpack" }
            },
            "byIndex": ["tekkit", "custom"],
            "selected": "tekkit"
        });

        let mut packs = installed_packs(&store, root, &modpacks);
        packs.sort();
        assert_eq!(
            packs,
            vec![
                ("custom".to_string(), root.join("custompacks").join("mine")),
                (
                    "elsewhere".to_string(),
                    PathBuf::from("/mnt/games/elsewhere")
                ),
                ("nodir".to_string(), modpacks.join("nodir")),
                ("tekkit".to_string(), modpacks.join("tekkit")),
            ]
        );

        assert_eq!(
            resolve_pack_directory("%modpacks%/foo/bar", root, &modpacks),
            modpacks.join("foo").join("bar")
        );
    }

    #[test]
    fn version_json_forms_are_understood() {
        let json = serde_json::json!({
            "id": "1.16.5-forge-36.2.34",
            "inheritsFrom": "1.16.5",
            "libraries": [
                { "name": "net.minecraftforge:forge:1.16.5-36.2.34" },
                { "name": "org.ow2.asm:asm:9.1" }
            ]
        });
        let v = versions_from_version_json(&json);
        assert_eq!(v.minecraft.as_deref(), Some("1.16.5"));
        assert_eq!(
            v.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Forge, "36.2.34")]
        );

        let json = serde_json::json!({
            "id": "1.7.10-Forge10.13.4.1614-1.7.10",
            "libraries": [
                { "name": "net.minecraftforge:forge:1.7.10-10.13.4.1614-1.7.10" }
            ]
        });
        let v = versions_from_version_json(&json);
        assert_eq!(v.minecraft.as_deref(), Some("1.7.10"));
        assert_eq!(
            v.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Forge, "10.13.4.1614")]
        );

        let json = serde_json::json!({
            "id": "neoforge-20.4.80-beta",
            "inheritsFrom": "1.20.4",
            "libraries": [{ "name": "net.neoforged:neoforge:20.4.80-beta" }]
        });
        let v = versions_from_version_json(&json);
        assert_eq!(v.minecraft.as_deref(), Some("1.20.4"));
        assert_eq!(
            v.loaders,
            vec![DeclaredModLoader::new(
                ModLoaderType::Neoforge,
                "20.4.80-beta"
            )]
        );

        let json = serde_json::json!({
            "id": "fabric-loader-0.14.9-1.19.2",
            "inheritsFrom": "1.19.2",
            "libraries": [
                { "name": "net.fabricmc:fabric-loader:0.14.9" },
                { "name": "net.fabricmc:intermediary:1.19.2" }
            ]
        });
        let v = versions_from_version_json(&json);
        assert_eq!(v.minecraft.as_deref(), Some("1.19.2"));
        assert_eq!(
            v.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Fabric, "0.14.9")]
        );

        let json = serde_json::json!({ "id": "1.12.2", "libraries": [] });
        let v = versions_from_version_json(&json);
        assert_eq!(v.minecraft.as_deref(), Some("1.12.2"));
        assert!(v.loaders.is_empty());
    }

    #[test]
    fn modpack_jar_profiles_are_read() {
        let tmp = tempfile::tempdir().unwrap();
        let jar = tmp.path().join("modpack.jar");
        {
            let file = std::fs::File::create(&jar).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            use std::io::Write;
            zip.start_file("install_profile.json", options).unwrap();
            zip.write_all(
                serde_json::json!({
                    "minecraft": "1.12.2",
                    "versionInfo": {
                        "id": "1.12.2-forge1.12.2-14.23.5.2860",
                        "libraries": [{ "name": "net.minecraftforge:forge:1.12.2-14.23.5.2860" }]
                    }
                })
                .to_string()
                .as_bytes(),
            )
            .unwrap();
            zip.finish().unwrap();
        }

        let v = versions_from_modpack_jar(&jar);
        assert_eq!(v.minecraft.as_deref(), Some("1.12.2"));
        assert_eq!(
            v.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Forge, "14.23.5.2860")]
        );

        let jar = tmp.path().join("modern.jar");
        {
            let file = std::fs::File::create(&jar).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            use std::io::Write;
            zip.start_file("install_profile.json", options).unwrap();
            zip.write_all(
                serde_json::json!({ "minecraft": "1.20.1", "version": "1.20.1-forge-47.2.0" })
                    .to_string()
                    .as_bytes(),
            )
            .unwrap();
            zip.finish().unwrap();
        }
        let v = versions_from_modpack_jar(&jar);
        assert_eq!(v.minecraft.as_deref(), Some("1.20.1"));
        assert_eq!(
            v.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Forge, "47.2.0")]
        );

        assert_eq!(
            versions_from_modpack_jar(&tmp.path().join("missing.jar")),
            ParsedVersionId::default()
        );
    }

    #[test]
    fn platform_pack_info_is_parsed() {
        let json = serde_json::json!({
            "id": 1,
            "name": "tekkit",
            "displayName": "Tekkit",
            "minecraft": "1.12.2",
            "forge": null,
            "icon": { "url": "https://example.com/icon.png", "md5": "x" }
        });
        assert_eq!(
            parse_platform_pack_info(&json),
            PlatformPackInfo {
                display_name: Some("Tekkit".to_string()),
                minecraft: Some("1.12.2".to_string()),
                forge: None,
                icon_url: Some("https://example.com/icon.png".to_string()),
            }
        );
    }

    #[tokio::test]
    async fn packs_are_located_from_launcher_root() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let modpacks = root.join("modpacks");

        let tekkit = modpacks.join("tekkit");
        std::fs::create_dir_all(tekkit.join("bin")).unwrap();
        std::fs::write(
            tekkit.join("bin").join("version.json"),
            serde_json::json!({
                "id": "1.12.2-forge1.12.2-14.23.5.2860",
                "libraries": [{ "name": "net.minecraftforge:forge:1.12.2-14.23.5.2860" }]
            })
            .to_string(),
        )
        .unwrap();

        let orphan = modpacks.join("orphan");
        std::fs::create_dir_all(orphan.join("bin")).unwrap();

        std::fs::create_dir_all(modpacks.join("junk")).unwrap();

        std::fs::write(
            root.join("installedPacks"),
            serde_json::json!({
                "installedPacks": {
                    "tekkit": { "name": "tekkit", "build": "1.0", "directory": "%MODPACKS%\\tekkit" },
                    "gone": { "name": "gone", "build": "1.0", "directory": "%MODPACKS%\\gone" }
                }
            })
            .to_string(),
        )
        .unwrap();

        let (single, mut packs) = locate_packs(root).await.unwrap();
        assert!(!single);
        packs.sort_by(|a, b| a.slug.cmp(&b.slug));
        assert_eq!(
            packs,
            vec![
                LocatedPack {
                    slug: "orphan".to_string(),
                    path: orphan.clone(),
                    launcher_root: Some(root.to_path_buf()),
                },
                LocatedPack {
                    slug: "tekkit".to_string(),
                    path: tekkit.clone(),
                    launcher_root: Some(root.to_path_buf()),
                },
            ]
        );

        let (single, packs) = locate_packs(&tekkit).await.unwrap();
        assert!(single);
        assert_eq!(packs[0].launcher_root, Some(root.to_path_buf()));

        let (single, packs) = locate_packs(&modpacks).await.unwrap();
        assert!(!single);
        assert_eq!(packs.len(), 2);

        let versions = detect_versions(&tekkit.join("bin")).await;
        assert_eq!(versions.minecraft.as_deref(), Some("1.12.2"));
        assert_eq!(
            versions.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Forge, "14.23.5.2860")]
        );

        let versions = detect_versions(&orphan.join("bin")).await;
        assert_eq!(versions, ParsedVersionId::default());
    }

    #[test]
    fn launcher_files_are_excluded_from_copy() {
        assert!(is_launcher_file(Path::new("bin/modpack.jar")));
        assert!(is_launcher_file(Path::new("bin")));
        assert!(!is_launcher_file(Path::new("mods/foo.jar")));
        assert!(!is_launcher_file(Path::new("config/foo.cfg")));
    }
}

#[cfg(test)]
mod task13_release_controls {
    use super::*;
    #[test]
    fn task13_ordinary_and_legacy_release_ids_remain_supported() {
        for id in [
            "1.20.1",
            "24w14a",
            "b1.7.3",
            "a1.2.6",
            "c0.30_01c",
            "rd-132211",
        ] {
            assert_eq!(
                versions_from_version_json(&serde_json::json!({"id":id}))
                    .minecraft
                    .as_deref(),
                Some(id)
            );
        }
        assert!(
            versions_from_version_json(&serde_json::json!({"id":"ATM9"}))
                .minecraft
                .is_none()
        );
    }
}

#[cfg(test)]
mod task13_round01_controls {
    use super::*;
    #[test]
    fn task13_round01_release_provenance_and_supported_ids() {
        for id in [
            "1.20.1",
            "24w14a",
            "1.21-pre1",
            "1.21-rc1",
            "1.14 Pre-Release 1",
            "b1.7.3",
            "b1.9-pre1",
            "a1.2.6",
            "c0.30_01c",
            "rd-132211",
            "inf-20100618",
        ] {
            assert_eq!(
                versions_from_version_json(&serde_json::json!({"id":id}))
                    .minecraft
                    .as_deref(),
                Some(id),
                "{id}"
            );
        }
        for id in ["1.20.1-forge-47.2.0", "fabric-loader-0.15.0-1.20.1"] {
            assert_eq!(
                versions_from_version_json(&serde_json::json!({"id":id}))
                    .minecraft
                    .as_deref(),
                Some("1.20.1")
            );
        }
        let conflict = serde_json::json!({"id":"1.20.4","libraries":[{"name":"net.minecraftforge:forge:1.20.1-47.2.0"}]});
        assert_eq!(
            versions_from_version_json(&conflict).minecraft.as_deref(),
            Some("1.20.4")
        );
        let inherited = serde_json::json!({"id":"1.20.1-Custom","inheritsFrom":"1.19.4","libraries":[{"name":"net.minecraftforge:forge:1.20.1-47.2.0"}]});
        assert_eq!(
            versions_from_version_json(&inherited).minecraft.as_deref(),
            Some("1.19.4")
        );
        assert!(
            versions_from_version_json(&serde_json::json!({"id":"1.20.1-Custom"}))
                .minecraft
                .is_none()
        );
    }
}

#[cfg(test)]
mod task13_round02_tests {
    use super::*;
    #[test]
    fn task13_round02_preserves_all_previously_supported_official_ids() {
        let ids: Vec<_> = include_str!("fixtures/task13_official_release_ids.txt")
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();
        assert_eq!(ids.len(), 903, "keep the attributed cached corpus complete");
        let mut rejected = Vec::new();
        for id in ids {
            let direct = versions_from_version_json(&serde_json::json!({"id":id})).minecraft;
            let inherited = versions_from_version_json(
                &serde_json::json!({"id":"Synthetic Custom", "inheritsFrom":id}),
            )
            .minecraft;
            if direct.as_deref() != Some(id) || inherited.as_deref() != Some(id) {
                rejected.push((id, direct, inherited));
            }
        }
        assert!(
            rejected.is_empty(),
            "{} cached official IDs failed direct/inherited production parsing: {rejected:?}",
            rejected.len()
        );
    }
}

#[cfg(test)]
mod task13_round02_custom_controls {
    use super::*;
    #[test]
    fn task13_round02_release_shaped_pack_labels_remain_unproven() {
        for id in [
            "1.20.1-ATM9",
            "26.4-snapshot-MyPack",
            "26.4-snapshot-2-ATM",
            "25w14mypack",
            "b1.3b-custom",
            "1.RV-Pre1-custom",
        ] {
            assert!(
                versions_from_version_json(&serde_json::json!({"id":id}))
                    .minecraft
                    .is_none(),
                "custom label became authority: {id}"
            );
            let library = versions_from_version_json(
                &serde_json::json!({"id":id,"libraries":[{"name":"net.minecraftforge:forge:1.20.1-47.2.0"}]}),
            );
            assert_eq!(library.minecraft.as_deref(), Some("1.20.1"));
        }
    }
}
