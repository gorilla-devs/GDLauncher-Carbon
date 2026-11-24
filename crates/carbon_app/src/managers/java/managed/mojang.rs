//! Mojang's Java runtimes, the ones the official launcher installs, as a
//! second managed distribution.

use super::{
    BuildSource, CatalogCache, Managed, ManagedJavaBuild, ManagedJavaCatalog, Step,
    azul_zulu::{AZUL_MAX_RETRIES, AZUL_REQUEST_TIMEOUT, azul_client},
    register_managed_java,
};
use crate::{
    domain::java::{JavaArch, JavaOs, JavaVendor},
    managers::java::{build_version::JavaBuildVersion, java_checker::JavaChecker},
};
use anyhow::Context;
use carbon_net::{Checksum, DownloadOptions, Downloadable};
use carbon_repos::db_exec::Db;
use carbon_rt_path::{ManagedJavasPath, TempPath};
use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::{
    collections::{BTreeMap, HashMap},
    path::{Component, Path, PathBuf},
};
use tokio::sync::watch::Sender;
use tracing::instrument;

const MOJANG_BASE_URL: &str = "https://piston-meta.mojang.com/";
const MOJANG_INDEX_PATH: &str =
    "v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

/// Components in the order that decides which one a version is listed under
/// when several carry it.
const COMPONENT_PRIORITY: [&str; 7] = [
    "java-runtime-epsilon",
    "java-runtime-delta",
    "java-runtime-gamma",
    "java-runtime-beta",
    "java-runtime-gamma-snapshot",
    "java-runtime-alpha",
    "jre-legacy",
];

/// Not a Java: the Windows Minecraft launcher executable.
const SKIPPED_COMPONENT: &str = "minecraft-java-exe";

const LTS_MAJORS: [u8; 5] = [8, 11, 17, 21, 25];

#[derive(Debug)]
pub struct MojangRuntime {
    base_url: String,
    max_retries: u32,
    pub(super) catalog: CatalogCache,
}

impl Default for MojangRuntime {
    fn default() -> Self {
        Self {
            base_url: MOJANG_BASE_URL.to_string(),
            max_retries: AZUL_MAX_RETRIES,
            catalog: CatalogCache::default(),
        }
    }
}

impl MojangRuntime {
    /// For tests. Does not retry, so a failing mock answers at once.
    pub fn with_base_url(url: String) -> Self {
        Self {
            base_url: url,
            max_retries: 0,
            catalog: CatalogCache::default(),
        }
    }

    /// The last catalog fetched, however old, without fetching one or
    /// waiting, even while a fetch runs.
    pub fn cached_catalog(&self) -> Option<ManagedJavaCatalog> {
        self.catalog.last()
    }

    /// Whether the catalog was never fetched or is due a refresh.
    pub fn catalog_is_stale(&self) -> bool {
        self.catalog.is_stale()
    }

    /// For tests: serves `catalog` until the cache expires, with no request.
    #[cfg(test)]
    pub async fn cache_catalog_for_test(&self, catalog: ManagedJavaCatalog) {
        self.catalog.set_for_test(catalog);
    }
}

/// The key Mojang's index uses for a platform. `None` when Mojang has no
/// runtimes for it.
pub fn mojang_platform_key(os: &JavaOs, arch: &JavaArch) -> Option<&'static str> {
    match (os, arch) {
        (JavaOs::Linux, JavaArch::X86_64) => Some("linux"),
        (JavaOs::Linux, JavaArch::X86_32) => Some("linux-i386"),
        (JavaOs::MacOs, JavaArch::X86_64) => Some("mac-os"),
        (JavaOs::MacOs, JavaArch::Arm64) => Some("mac-os-arm64"),
        (JavaOs::Windows, JavaArch::X86_64) => Some("windows-x64"),
        (JavaOs::Windows, JavaArch::X86_32) => Some("windows-x86"),
        (JavaOs::Windows, JavaArch::Arm64) => Some("windows-arm64"),
        _ => None,
    }
}

/// The folder directly under the managed javas folder that holds `path`.
pub fn managed_install_folder(managed_root: &Path, path: &str) -> Option<String> {
    Path::new(path)
        .strip_prefix(managed_root)
        .ok()?
        .components()
        .next()?
        .as_os_str()
        .to_str()
        .map(str::to_string)
}

/// The distribution of a managed Java, from its install folder under the
/// managed javas folder: `mojang-…` is Mojang and `zulu…` is Azul.
pub fn managed_vendor_for_path(managed_root: &Path, path: &str) -> Option<JavaVendor> {
    let folder = managed_install_folder(managed_root, path)?;

    if folder.starts_with("mojang-") {
        Some(JavaVendor::Mojang)
    } else if folder.starts_with("zulu") {
        Some(JavaVendor::Azul)
    } else {
        None
    }
}

/// The folder a Mojang build installs into. `None` for other builds.
pub fn install_folder_for(build: &ManagedJavaBuild) -> Option<String> {
    match &build.source {
        BuildSource::MojangManifest { component, .. } => {
            Some(install_folder_name(component, &build.version))
        }
        _ => None,
    }
}

fn main_binary_relative_path(os: &JavaOs) -> &'static str {
    match os {
        JavaOs::Windows => "bin/java.exe",
        JavaOs::Linux => "bin/java",
        JavaOs::MacOs => "jre.bundle/Contents/Home/bin/java",
    }
}

fn install_folder_name(component: &str, version: &JavaBuildVersion) -> String {
    format!("mojang-{component}-{version}")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct IndexEntry {
    manifest: IndexManifest,
    version: IndexVersion,
}

#[derive(Debug, Deserialize)]
struct IndexManifest {
    sha1: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct IndexVersion {
    name: String,
    released: Option<String>,
}

/// Platform key, then component, then its listed entries.
type MojangIndex = HashMap<String, HashMap<String, Vec<IndexEntry>>>;

fn builds_from_index(index: &MojangIndex, platform_key: &str) -> Vec<ManagedJavaBuild> {
    let Some(components) = index.get(platform_key) else {
        return Vec::new();
    };

    let mut names = COMPONENT_PRIORITY
        .iter()
        .copied()
        .filter(|name| components.contains_key(*name))
        .collect::<Vec<_>>();
    let mut others = components
        .keys()
        .map(String::as_str)
        .filter(|name| *name != SKIPPED_COMPONENT && !COMPONENT_PRIORITY.contains(name))
        .collect::<Vec<_>>();
    others.sort_unstable();
    names.extend(others);

    let mut builds: Vec<ManagedJavaBuild> = Vec::new();

    for name in names {
        for entry in &components[name] {
            let Some(version) = JavaBuildVersion::parse(&entry.version.name) else {
                continue;
            };
            let Ok(major) = u8::try_from(version.major()) else {
                continue;
            };

            if builds.iter().any(|b| b.version == version) {
                continue;
            }

            builds.push(ManagedJavaBuild {
                id: entry.manifest.sha1.clone(),
                name: name.to_string(),
                major,
                version,
                source: BuildSource::MojangManifest {
                    component: name.to_string(),
                    manifest_url: entry.manifest.url.clone(),
                    manifest_sha1: entry.manifest.sha1.clone(),
                },
                size_bytes: None,
                release_date: entry.version.released.clone(),
                lts: LTS_MAJORS.contains(&major),
            });
        }
    }

    builds
}

#[derive(Debug, Deserialize)]
pub struct MojangFileManifest {
    files: BTreeMap<String, MojangFileEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum MojangFileEntry {
    Directory,
    File {
        downloads: MojangFileDownloads,
        #[serde(default)]
        executable: bool,
    },
    Link {
        target: String,
    },
}

#[derive(Debug, Deserialize)]
struct MojangFileDownloads {
    raw: MojangRawDownload,
}

#[derive(Debug, Deserialize)]
struct MojangRawDownload {
    sha1: String,
    size: u64,
    url: String,
}

/// `root` joined with a manifest path, refusing anything that could leave it.
fn join_inside(root: &Path, relative: &str) -> anyhow::Result<PathBuf> {
    let relative = Path::new(relative);

    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        anyhow::bail!("Unsafe path in runtime manifest: {relative:?}");
    }

    Ok(root.join(relative))
}

/// The manifest path a link points at, worked out without touching the disk.
/// Absolute targets and targets that climb out of the install folder are
/// refused.
fn resolve_link_target(link: &str, target: &str) -> anyhow::Result<String> {
    let mut parts = link.split('/').collect::<Vec<_>>();
    parts.pop();

    for component in Path::new(target).components() {
        match component {
            Component::Normal(name) => parts.push(
                name.to_str()
                    .ok_or_else(|| anyhow::anyhow!("Unsafe link target for {link}"))?,
            ),
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.pop().is_none() {
                    anyhow::bail!("Link target of {link} leaves the install folder: {target}");
                }
            }
            _ => anyhow::bail!("Link target of {link} is not relative: {target}"),
        }
    }

    Ok(parts.join("/"))
}

/// Lays a runtime out under `root`: directories, then every file (checked
/// against its sha1 and size), then executable bits, then links.
async fn install_files(
    manifest: &MojangFileManifest,
    root: &Path,
    progress: &Sender<Step>,
) -> anyhow::Result<()> {
    // Checked before anything is written. A link may only point at a file
    // the manifest installs, so no link chains and no way out of `root`.
    for (relative, entry) in &manifest.files {
        if let MojangFileEntry::Link { target } = entry {
            let resolved = resolve_link_target(relative, target)?;

            if !matches!(
                manifest.files.get(&resolved),
                Some(MojangFileEntry::File { .. })
            ) {
                anyhow::bail!("Link {relative} points at {target}, which is not a file");
            }
        }
    }

    let mut downloads = Vec::new();
    let mut executables = Vec::new();
    let mut links = Vec::new();

    for (relative, entry) in &manifest.files {
        let path = join_inside(root, relative)?;

        match entry {
            MojangFileEntry::Directory => {
                std::fs::create_dir_all(&path)
                    .with_context(|| format!("Could not create directory: {path:?}"))?;
            }
            MojangFileEntry::File {
                downloads: file,
                executable,
            } => {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("Could not create directory: {parent:?}"))?;
                }

                downloads.push(
                    Downloadable::new(&file.raw.url, &path)
                        .with_checksum(Some(Checksum::Sha1(file.raw.sha1.clone())))
                        .with_size(file.raw.size),
                );
                if *executable {
                    executables.push(path);
                }
            }
            MojangFileEntry::Link { target } => links.push((path, target.as_str())),
        }
    }

    let (p_sender, mut p_recv) = tokio::sync::watch::channel(carbon_net::Progress::new());
    let progress_proxy = {
        let progress = progress.clone();

        tokio::spawn(async move {
            while p_recv.changed().await.is_ok() {
                let p = p_recv.borrow().clone();
                let _ = progress.send(Step::Downloading(p.current_size, p.total_size));
            }
        })
    };

    let downloaded = carbon_net::download_multiple(
        &downloads,
        DownloadOptions::builder()
            .concurrency(8)
            .progress_sender(p_sender)
            .build(),
    )
    .await;
    let _ = progress_proxy.await;
    downloaded.context("Could not download the Java runtime files")?;

    #[cfg(unix)]
    for path in &executables {
        use std::os::unix::fs::PermissionsExt;

        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .with_context(|| format!("Could not make {path:?} executable"))?;
    }
    #[cfg(not(unix))]
    let _ = executables;

    let total_links = links.len() as u64;

    for (done, (path, target)) in links.into_iter().enumerate() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if path.symlink_metadata().is_ok() {
            std::fs::remove_file(&path)?;
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink(target, &path)
            .with_context(|| format!("Could not link {path:?} to {target}"))?;

        #[cfg(not(unix))]
        {
            let resolved = path.parent().unwrap_or(root).join(target);
            std::fs::copy(&resolved, &path)
                .with_context(|| format!("Could not copy {resolved:?} to {path:?}"))?;
        }

        let _ = progress.send(Step::Extracting(done as u64 + 1, total_links));
    }

    Ok(())
}

#[async_trait::async_trait]
impl Managed for MojangRuntime {
    async fn catalog(&self) -> anyhow::Result<ManagedJavaCatalog> {
        self.catalog
            .get_or_fetch(|| async {
                let Some(key) =
                    mojang_platform_key(&JavaOs::get_current_os()?, &JavaArch::get_current_arch()?)
                else {
                    return Ok(ManagedJavaCatalog::default());
                };

                let index = azul_client(AZUL_REQUEST_TIMEOUT, self.max_retries)?
                    .get(format!(
                        "{}/{MOJANG_INDEX_PATH}",
                        self.base_url.trim_end_matches('/')
                    ))
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<MojangIndex>()
                    .await
                    .context("Could not read Mojang's Java runtime index")?;

                Ok(ManagedJavaCatalog::from_builds(builds_from_index(
                    &index, key,
                )))
            })
            .await
    }

    #[instrument(skip(self, java_checker, db, progress_report))]
    async fn setup<G: JavaChecker + Send + Sync>(
        &self,
        build: &ManagedJavaBuild,
        _tmp_path: TempPath,
        base_managed_java_path: ManagedJavasPath,
        java_checker: &G,
        db: &Db,
        progress_report: Sender<Step>,
    ) -> anyhow::Result<String> {
        let BuildSource::MojangManifest {
            component,
            manifest_url,
            manifest_sha1,
        } = &build.source
        else {
            anyhow::bail!("Not a Mojang build: {}", build.id);
        };

        let install_root = base_managed_java_path
            .to_path()
            .join(install_folder_name(component, &build.version));

        // A registered Java here would be overwritten, and deleted should the
        // install fail. Checked before the cleanup below can touch it.
        if carbon_repos::repos::java::get_all_java(db)
            .await?
            .iter()
            .any(|row| Path::new(&row.path).starts_with(&install_root))
        {
            anyhow::bail!("This Java is already installed");
        }

        let result = async {
            let manifest_bytes = azul_client(AZUL_REQUEST_TIMEOUT, self.max_retries)?
                .get(manifest_url)
                .send()
                .await?
                .error_for_status()?
                .bytes()
                .await?;

            let actual = hex::encode(Sha1::digest(&manifest_bytes));
            if !actual.eq_ignore_ascii_case(manifest_sha1) {
                anyhow::bail!("The Java runtime manifest does not match its checksum");
            }

            let manifest: MojangFileManifest = serde_json::from_slice(&manifest_bytes)
                .context("Could not read the Java runtime manifest")?;

            std::fs::create_dir_all(&install_root)?;
            install_files(&manifest, &install_root, &progress_report).await?;

            let bin = install_root.join(main_binary_relative_path(&JavaOs::get_current_os()?));
            let bin = dunce::canonicalize(&bin).unwrap_or(bin);

            let java_id = register_managed_java(&bin, java_checker, db).await?;
            let _ = progress_report.send(Step::Done);

            Ok::<_, anyhow::Error>(java_id)
        }
        .await;

        if result.is_err()
            && let Err(e) = std::fs::remove_dir_all(&install_root)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!("Could not clean up unusable managed JRE at {install_root:?}: {e}");
        }

        result
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::managers::java::java_checker::MockJavaChecker;
    use serde_json::json;

    fn sha1_hex(bytes: &[u8]) -> String {
        hex::encode(Sha1::digest(bytes))
    }

    #[test]
    fn mojang_platform_key_maps_supported_platforms() {
        use JavaArch::*;
        use JavaOs::*;

        for (os, arch, key) in [
            (Linux, X86_64, "linux"),
            (Linux, X86_32, "linux-i386"),
            (MacOs, X86_64, "mac-os"),
            (MacOs, Arm64, "mac-os-arm64"),
            (Windows, X86_64, "windows-x64"),
            (Windows, X86_32, "windows-x86"),
            (Windows, Arm64, "windows-arm64"),
        ] {
            assert_eq!(
                mojang_platform_key(&os, &arch),
                Some(key),
                "{os:?} {arch:?}"
            );
        }

        assert_eq!(mojang_platform_key(&Linux, &Arm64), None);
    }

    fn entry(sha1: &str, version: &str) -> serde_json::Value {
        json!([{
            "availability": {"group": 1, "progress": 100},
            "manifest": {
                "sha1": sha1,
                "size": 1,
                "url": format!("https://example.invalid/{sha1}/manifest.json"),
            },
            "version": {"name": version, "released": "2025-05-19T08:11:57+00:00"},
        }])
    }

    #[tokio::test]
    async fn mojang_catalog_dedupes_and_sorts() {
        let Some(key) = mojang_platform_key(
            &JavaOs::get_current_os().unwrap(),
            &JavaArch::get_current_arch().unwrap(),
        ) else {
            return;
        };

        let mut server = mockito::Server::new_async().await;
        let index = server
            .mock("GET", format!("/{MOJANG_INDEX_PATH}").as_str())
            .with_body(
                json!({
                    "gamecore": {"jre-legacy": []},
                    key: {
                        "java-runtime-epsilon": entry("e25", "25.0.1"),
                        "java-runtime-delta": entry("d21", "21.0.7"),
                        "java-runtime-gamma": entry("g17", "17.0.15"),
                        "java-runtime-beta": entry("b17", "17.0.15"),
                        "java-runtime-gamma-snapshot": entry("s17", "17.0.15"),
                        "jre-legacy": entry("l8", "8u202"),
                        "minecraft-java-exe": entry("x", "1.0.0"),
                    },
                })
                .to_string(),
            )
            .create_async()
            .await;

        let catalog = MojangRuntime::with_base_url(format!("{}/", server.url()))
            .catalog()
            .await
            .unwrap();
        index.assert_async().await;

        let majors = catalog.majors.iter().map(|m| m.major).collect::<Vec<_>>();
        assert_eq!(majors, [25, 21, 17, 8]);
        assert!(catalog.majors.iter().all(|m| m.lts));

        let seventeen = &catalog.majors[2];
        assert_eq!(seventeen.builds.len(), 1);
        assert_eq!(seventeen.builds[0].name, "java-runtime-gamma");
        assert_eq!(seventeen.builds[0].id, "g17");

        let eight = &catalog.majors[3].builds[0];
        assert_eq!(eight.version, JavaBuildVersion::parse("8.0.202").unwrap());
        assert_eq!(eight.id, "l8");
    }

    fn manifest_for(files: serde_json::Value) -> MojangFileManifest {
        serde_json::from_value(json!({ "files": files })).unwrap()
    }

    fn file_entry(
        server_url: &str,
        name: &str,
        body: &[u8],
        executable: bool,
    ) -> serde_json::Value {
        json!({
            "type": "file",
            "executable": executable,
            "downloads": {"raw": {
                "sha1": sha1_hex(body),
                "size": body.len(),
                "url": format!("{server_url}/objects/{name}"),
            }},
        })
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn mojang_install_lays_out_files_links_and_exec_bits() {
        use std::os::unix::fs::PermissionsExt;

        let mut server = mockito::Server::new_async().await;
        let mut mocks = Vec::new();
        for (name, body) in [("java", "#!/bin/sh\n"), ("license", "MIT")] {
            mocks.push(
                server
                    .mock("GET", format!("/objects/{name}").as_str())
                    .with_body(body)
                    .create_async()
                    .await,
            );
        }

        let url = server.url();
        let manifest = manifest_for(json!({
            "bin": {"type": "directory"},
            "bin/java": file_entry(&url, "java", b"#!/bin/sh\n", true),
            "legal/a/LICENSE": file_entry(&url, "license", b"MIT", false),
            "legal/b/LICENSE": {"type": "link", "target": "../a/LICENSE"},
        }));

        let root = tempfile::tempdir().unwrap();
        let (progress, _recv) = tokio::sync::watch::channel(Step::Idle);
        install_files(&manifest, root.path(), &progress)
            .await
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(root.path().join("legal/a/LICENSE")).unwrap(),
            "MIT"
        );
        assert_eq!(
            std::fs::read_to_string(root.path().join("legal/b/LICENSE")).unwrap(),
            "MIT"
        );
        let mode = std::fs::metadata(root.path().join("bin/java"))
            .unwrap()
            .permissions()
            .mode();
        assert_ne!(mode & 0o111, 0);
        assert_eq!(
            std::fs::read_link(root.path().join("legal/b/LICENSE")).unwrap(),
            Path::new("../a/LICENSE")
        );
        let legal_mode = std::fs::metadata(root.path().join("legal/a/LICENSE"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(legal_mode & 0o111, 0);
    }

    #[tokio::test]
    async fn mojang_install_rejects_sha1_mismatch() {
        let mut server = mockito::Server::new_async().await;
        let _object = server
            .mock("GET", "/objects/java")
            .with_body("wrong bytes")
            .create_async()
            .await;

        let manifest = manifest_for(json!({
            "bin/java": file_entry(&server.url(), "java", b"expected bytes", true),
        }));

        let root = tempfile::tempdir().unwrap();
        let (progress, _recv) = tokio::sync::watch::channel(Step::Idle);

        assert!(
            install_files(&manifest, root.path(), &progress)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn mojang_install_rejects_paths_that_escape_the_folder() {
        let manifest = manifest_for(json!({"../evil": {"type": "directory"}}));
        let root = tempfile::tempdir().unwrap();
        let (progress, _recv) = tokio::sync::watch::channel(Step::Idle);

        assert!(
            install_files(&manifest, root.path(), &progress)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn mojang_setup_cleans_up_on_failure() {
        let env = crate::setup_managers_for_test().await;
        let runtime_path = env.app.settings_manager().runtime_path.clone();

        let mut server = mockito::Server::new_async().await;
        let _object = server
            .mock("GET", "/objects/java")
            .with_body("wrong bytes")
            .create_async()
            .await;

        let manifest = json!({"files": {
            "bin/java": file_entry(&server.url(), "java", b"expected bytes", true),
        }})
        .to_string();
        let _manifest = server
            .mock("GET", "/manifest.json")
            .with_body(&manifest)
            .create_async()
            .await;

        let build = ManagedJavaBuild {
            id: sha1_hex(manifest.as_bytes()),
            name: "java-runtime-gamma".into(),
            major: 17,
            version: JavaBuildVersion::parse("17.0.15").unwrap(),
            source: BuildSource::MojangManifest {
                component: "java-runtime-gamma".into(),
                manifest_url: format!("{}/manifest.json", server.url()),
                manifest_sha1: sha1_hex(manifest.as_bytes()),
            },
            size_bytes: None,
            release_date: None,
            lts: true,
        };

        let managed = runtime_path.get_managed_javas();
        let (progress, _recv) = tokio::sync::watch::channel(Step::Idle);
        let result = MojangRuntime::with_base_url(server.url())
            .setup(
                &build,
                runtime_path.get_temp(),
                runtime_path.get_managed_javas(),
                &MockJavaChecker,
                &env.app.db,
                progress,
            )
            .await;

        assert!(result.is_err());
        assert!(
            !managed
                .to_path()
                .join("mojang-java-runtime-gamma-17.0.15")
                .exists()
        );
    }

    #[tokio::test]
    async fn mojang_setup_refuses_a_folder_that_holds_a_registered_java() {
        let env = crate::setup_managers_for_test().await;
        let runtime_path = env.app.settings_manager().runtime_path.clone();
        let folder = runtime_path
            .get_managed_javas()
            .to_path()
            .join("mojang-java-runtime-alpha-16.0.1.9.1");
        std::fs::create_dir_all(folder.join("bin")).unwrap();
        std::fs::write(folder.join("bin/java"), b"").unwrap();
        carbon_repos::repos::java::insert_java(
            &env.app.db,
            carbon_repos::repos::java::JavaRow {
                id: "alpha".into(),
                path: folder.join("bin/java").to_string_lossy().into_owned(),
                major: 16,
                full_version: "16.0.1".into(),
                r#type: "managed".into(),
                os: "linux".into(),
                arch: "amd64".into(),
                vendor: "microsoft".into(),
                is_valid: true,
            },
        )
        .await
        .unwrap();

        let build = ManagedJavaBuild {
            id: "x".into(),
            name: "java-runtime-alpha".into(),
            major: 16,
            version: JavaBuildVersion::parse("16.0.1.9.1").unwrap(),
            source: BuildSource::MojangManifest {
                component: "java-runtime-alpha".into(),
                manifest_url: "http://127.0.0.1:1/m.json".into(),
                manifest_sha1: String::new(),
            },
            size_bytes: None,
            release_date: None,
            lts: false,
        };
        let (progress, _recv) = tokio::sync::watch::channel(Step::Idle);
        let err = MojangRuntime::with_base_url(String::new())
            .setup(
                &build,
                runtime_path.get_temp(),
                runtime_path.get_managed_javas(),
                &MockJavaChecker,
                &env.app.db,
                progress,
            )
            .await
            .unwrap_err();

        assert!(err.to_string().contains("already installed"), "{err:#}");
        assert!(folder.join("bin/java").exists(), "the install was deleted");
    }

    #[tokio::test]
    async fn mojang_setup_rejects_a_manifest_with_the_wrong_sha1() {
        let env = crate::setup_managers_for_test().await;
        let runtime_path = env.app.settings_manager().runtime_path.clone();

        let mut server = mockito::Server::new_async().await;
        let _manifest = server
            .mock("GET", "/manifest.json")
            .with_body(r#"{"files":{}}"#)
            .create_async()
            .await;

        let build = ManagedJavaBuild {
            id: "x".into(),
            name: "java-runtime-gamma".into(),
            major: 17,
            version: JavaBuildVersion::parse("17.0.15").unwrap(),
            source: BuildSource::MojangManifest {
                component: "java-runtime-gamma".into(),
                manifest_url: format!("{}/manifest.json", server.url()),
                manifest_sha1: sha1_hex(b"something else"),
            },
            size_bytes: None,
            release_date: None,
            lts: true,
        };

        let (progress, _recv) = tokio::sync::watch::channel(Step::Idle);
        let err = MojangRuntime::with_base_url(server.url())
            .setup(
                &build,
                runtime_path.get_temp(),
                runtime_path.get_managed_javas(),
                &MockJavaChecker,
                &env.app.db,
                progress,
            )
            .await
            .unwrap_err();

        assert!(err.to_string().contains("checksum"));
    }

    #[test]
    fn managed_vendor_for_path_detects_distribution() {
        let root = Path::new("/x/managed_javas");

        assert_eq!(
            managed_vendor_for_path(
                root,
                "/x/managed_javas/zulu17.70.201-ca-jre17.0.21.1-linux_x64/bin/java"
            ),
            Some(JavaVendor::Azul)
        );
        assert_eq!(
            managed_vendor_for_path(
                root,
                "/x/managed_javas/mojang-java-runtime-gamma-17.0.15/bin/java"
            ),
            Some(JavaVendor::Mojang)
        );
        assert_eq!(
            managed_vendor_for_path(root, "/usr/lib/jvm/java-17/bin/java"),
            None
        );
    }

    #[test]
    fn distribution_comes_from_the_folder_under_managed_javas() {
        let root = Path::new("/home/zuluaga/data/managed_javas");

        assert_eq!(
            managed_vendor_for_path(
                root,
                "/home/zuluaga/data/managed_javas/mojang-java-runtime-gamma-17.0.15/bin/java"
            ),
            Some(JavaVendor::Mojang)
        );
        // Outside the managed folder, whatever the parents are called.
        assert_eq!(
            managed_vendor_for_path(root, "/home/mojang-x/zulu17/bin/java"),
            None
        );
        assert_eq!(
            managed_vendor_for_path(
                Path::new("/d/managed_javas"),
                "/mojang-d/managed_javas/zulu17-x/bin/java"
            ),
            None
        );
    }

    #[tokio::test]
    async fn mojang_install_rejects_link_targets_that_escape() {
        let (progress, _recv) = tokio::sync::watch::channel(Step::Idle);

        for target in ["../../../etc/passwd", "/etc/passwd", "../../outside"] {
            let manifest = manifest_for(json!({
                "a/b": {"type": "link", "target": target},
            }));
            let root = tempfile::tempdir().unwrap();

            assert!(
                install_files(&manifest, root.path(), &progress)
                    .await
                    .is_err(),
                "{target}"
            );
            assert!(root.path().read_dir().unwrap().next().is_none());
        }
    }

    #[tokio::test]
    async fn mojang_install_rejects_links_to_non_files() {
        let (progress, _recv) = tokio::sync::watch::channel(Step::Idle);
        let manifest = manifest_for(json!({
            "d": {"type": "directory"},
            "x": {"type": "link", "target": "d"},
            "y": {"type": "link", "target": "x"},
        }));
        let root = tempfile::tempdir().unwrap();

        assert!(
            install_files(&manifest, root.path(), &progress)
                .await
                .is_err()
        );
    }

    #[test]
    fn install_folder_for_names_mojang_builds_only() {
        let mut build = ManagedJavaBuild {
            id: "x".into(),
            name: "java-runtime-alpha".into(),
            major: 16,
            version: JavaBuildVersion::parse("16.0.1.9.1").unwrap(),
            source: BuildSource::MojangManifest {
                component: "java-runtime-alpha".into(),
                manifest_url: String::new(),
                manifest_sha1: String::new(),
            },
            size_bytes: None,
            release_date: None,
            lts: false,
        };
        assert_eq!(
            install_folder_for(&build).as_deref(),
            Some("mojang-java-runtime-alpha-16.0.1.9.1")
        );

        build.source = BuildSource::AzulZip {
            download_url: String::new(),
            sha256: None,
        };
        assert_eq!(install_folder_for(&build), None);
    }

    #[test]
    fn install_folder_names_only_use_safe_characters() {
        let version = JavaBuildVersion::parse("8u202").unwrap();

        assert_eq!(
            install_folder_name("jre-legacy", &version),
            "mojang-jre-legacy-8.0.202"
        );
        assert_eq!(
            install_folder_name("../we ird/x", &version),
            "mojang-.._we_ird_x-8.0.202"
        );
    }

    /// Downloads about 100 MB.
    #[tokio::test]
    #[ignore]
    async fn installs_real_mojang_runtime() {
        use crate::managers::java::java_checker::RealJavaChecker;

        let env = crate::setup_managers_for_test().await;
        let runtime_path = env.app.settings_manager().runtime_path.clone();

        let runtime = MojangRuntime::default();
        let catalog = runtime.catalog().await.unwrap();
        let build = catalog
            .builds()
            .find(|b| b.name == "java-runtime-delta")
            .expect("java-runtime-delta is listed for this platform")
            .clone();

        let (progress, _recv) = tokio::sync::watch::channel(Step::Idle);
        let id = runtime
            .setup(
                &build,
                runtime_path.get_temp(),
                runtime_path.get_managed_javas(),
                &RealJavaChecker::new(env.app.runtime_lifecycle.clone()),
                &env.app.db,
                progress,
            )
            .await
            .unwrap();

        let row = carbon_repos::repos::java::get_java_by_id(&env.app.db, &id)
            .await
            .unwrap()
            .unwrap();
        println!("Installed at {}", row.path);

        assert_eq!(row.major, 21);
        assert!(Path::new(&row.path).exists());
    }
}
