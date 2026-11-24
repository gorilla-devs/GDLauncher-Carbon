use super::{
    BuildSource, CatalogCache, Managed, ManagedJavaBuild, ManagedJavaCatalog, Step,
    register_managed_java,
};
use crate::{
    domain::java::{JavaArch, JavaOs},
    managers::java::{build_version::JavaBuildVersion, java_checker::JavaChecker},
};
use anyhow::Context;
use carbon_net::{Checksum, DownloadOptions, Downloadable, Progress};
use carbon_repos::db_exec::Db;
use carbon_rt_path::{ManagedJavasPath, TempPath};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{sync::watch::Sender, task::spawn_blocking};
use tracing::{instrument, trace};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::runtime_lifecycle::{RuntimeLifecycle, ShutdownPurpose};
    struct Checker(&'static str, Arc<RuntimeLifecycle>);
    #[async_trait::async_trait]
    impl JavaChecker for Checker {
        async fn get_bin_info(
            &self,
            _: &Path,
            _: crate::domain::java::JavaComponentType,
        ) -> anyhow::Result<crate::domain::java::JavaComponent> {
            match self.0 {
                "freeze" => {
                    let owner = crate::managers::java::java_checker::admit_probe(&self.1)?;
                    owner.finish();
                    unreachable!()
                }
                "cancel" => Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "controlled reaped cancellation",
                )
                .into()),
                "unknown" => Err(crate::managers::runtime_lifecycle::ChildSettlementUnknown.into()),
                "bookkeeping" => {
                    Err(crate::managers::runtime_lifecycle::tests_support::bookkeeping_error())
                }
                _ => anyhow::bail!("controlled ordinary invalid Java"),
            }
        }
    }
    async fn cleanup_case(mode: &'static str) {
        let root = tempfile::tempdir().unwrap();
        let db_path = root.path().join("java.db");
        let mut conn = rusqlite::Connection::open(&db_path).unwrap();
        carbon_repos::get_migrations()
            .0
            .to_latest(&mut conn)
            .unwrap();
        drop(conn);
        let db = Db::open(&db_path, 2, true).unwrap();
        let install = root.path().join("existing-java");
        std::fs::create_dir(&install).unwrap();
        let binary = install.join("java");
        std::fs::write(&binary, b"existing healthy bytes").unwrap();
        let lifecycle = Arc::new(RuntimeLifecycle::default());
        lifecycle.reconciliation_finished(Ok(()));
        // Actual entered worker IO settles before a now-frozen probe admission.
        let work_path = install.join("extracted");
        let (entered, wait_entered) = tokio::sync::oneshot::channel();
        let (release, wait_release) = std::sync::mpsc::channel();
        let write = tokio::task::spawn_blocking(move || {
            entered.send(()).unwrap();
            wait_release.recv().unwrap();
            std::fs::write(work_path, b"settled extraction").unwrap();
        });
        wait_entered.await.unwrap();
        if mode == "freeze" {
            lifecycle
                .begin_shutdown("controlled", ShutdownPurpose::Quit)
                .unwrap();
        }
        release.send(()).unwrap();
        write.await.unwrap();
        assert!(
            validate_managed_install(&db, &Checker(mode, lifecycle), &binary, &install)
                .await
                .is_err()
        );
        if mode == "invalid" {
            assert!(
                !install.exists(),
                "ordinary invalid validation still cleans installation"
            );
        } else {
            assert_eq!(
                std::fs::read(&binary).unwrap(),
                b"existing healthy bytes",
                "inconclusive validation must preserve installation"
            );
        }
    }
    #[tokio::test]
    async fn managed_frozen_probe_preserves_existing_install() {
        cleanup_case("freeze").await;
    }
    #[tokio::test]
    async fn managed_cancelled_probe_preserves_existing_install() {
        cleanup_case("cancel").await;
    }
    #[tokio::test]
    async fn managed_unknown_and_invalid_cleanup_controls() {
        cleanup_case("unknown").await;
        cleanup_case("invalid").await;
    }
    #[tokio::test]
    async fn managed_probe_bookkeeping_failure_preserves_existing_install() {
        cleanup_case("bookkeeping").await;
    }
}

#[derive(Debug)]
pub struct AzulZulu {
    base_url: String,
    max_retries: u32,
    pub(super) catalog: CatalogCache,
}

impl Default for AzulZulu {
    fn default() -> Self {
        // Debug builds can point the catalog at another server, e.g. to
        // simulate Azul being down.
        #[cfg(debug_assertions)]
        if let Ok(url) = std::env::var("GDL_AZUL_BASE_URL") {
            if !url.is_empty() {
                return Self::with_base_url(url);
            }
        }

        Self {
            base_url: AZUL_BASE_URL.to_string(),
            max_retries: AZUL_MAX_RETRIES,
            catalog: CatalogCache::default(),
        }
    }
}

impl AzulZulu {
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

    /// For tests: a fetch that never ends, until the guard is dropped.
    #[cfg(test)]
    pub async fn hold_fetch_for_test(&self) -> impl Sized + '_ {
        self.catalog.hold_fetch_for_test().await
    }
}

/// Probes and records a freshly extracted install. A probe that could not
/// tell (shutdown, cancellation, lost bookkeeping) leaves the install in
/// place; any other failure removes it.
async fn validate_managed_install<G: JavaChecker + Send + Sync>(
    db: &Db,
    java_checker: &G,
    main_binary_path: &Path,
    install_root: &Path,
) -> anyhow::Result<String> {
    match register_managed_java(main_binary_path, java_checker, db).await {
        Ok(java_id) => Ok(java_id),
        Err(e) => {
            if crate::managers::java::java_checker::validation_inconclusive(&e) {
                return Err(e)
                    .context("Managed Java validation is inconclusive; installation retained");
            }
            // A partially or incorrectly extracted JRE would otherwise persist and
            // fail every launch — the size-based skip in the extraction loop never
            // re-writes an existing same-size file, so it can't self-heal. Remove
            // the install directory to force a clean re-download on the next try.
            if let Err(rm) = std::fs::remove_dir_all(install_root) {
                tracing::warn!(
                    "Could not clean up unusable managed JRE at {:?}: {}",
                    install_root,
                    rm
                );
            }

            Err(e)
        }
    }
}

#[async_trait::async_trait]
impl Managed for AzulZulu {
    async fn catalog(&self) -> anyhow::Result<ManagedJavaCatalog> {
        self.catalog
            .get_or_fetch(|| async {
                let builds = AzulAPI::fetch_builds(
                    &JavaOs::get_current_os()?,
                    &JavaArch::get_current_arch()?,
                    &self.base_url,
                    self.max_retries,
                )
                .await?;

                Ok(ManagedJavaCatalog::from_builds(builds))
            })
            .await
    }

    #[instrument(skip(self, java_checker, db, progress_report))]
    async fn setup<G: JavaChecker + Send + Sync>(
        &self,
        build: &ManagedJavaBuild,
        tmp_path: TempPath,
        base_managed_java_path: ManagedJavasPath,
        java_checker: &G,
        db: &Db,
        progress_report: Sender<Step>,
    ) -> anyhow::Result<String> {
        let progress_report = Arc::new(progress_report);

        let download_temp_path = tmp_path.to_path().join(&build.name);

        trace!("Download path: {:?}", download_temp_path);

        let BuildSource::AzulZip {
            download_url,
            sha256,
        } = &build.source
        else {
            anyhow::bail!("Not an Azul build: {}", build.id);
        };

        // Bounded the same way as every other Azul request in this module
        // (see `azul_client`): calling the bare default client directly here
        // had no timeout at all, so a server that accepts the connection and
        // stalls would hang the whole managed-JRE install indefinitely on a
        // probe whose result is merely a progress-bar size hint.
        let content_length = azul_client(AZUL_REQUEST_TIMEOUT, AZUL_MAX_RETRIES)?
            .get(download_url)
            .send()
            .await?
            .content_length();

        // The list carries the checksum. Azul's per-package detail endpoint is the
        // best-effort fallback when it didn't; a failure there must never block the
        // install, it just means the download proceeds unchecked.
        let sha256 = match sha256 {
            Some(sha256) => Some(sha256.clone()),
            None => fetch_package_sha256(&self.base_url, &build.id).await,
        };

        let mut downloadable = Downloadable::new(download_url, download_temp_path);
        if let Some(content_length) = content_length {
            downloadable = downloadable.with_size(content_length);
        }
        if let Some(sha256) = sha256 {
            downloadable = downloadable.with_checksum(Some(Checksum::Sha256(sha256)));
        }

        trace!("Downloadable: {:?}", downloadable);

        let (p_sender, mut p_recv) = tokio::sync::watch::channel(Progress::new());

        let progress_report_clone = progress_report.clone();
        let progress_proxy = tokio::spawn(async move {
            while p_recv.changed().await.is_ok() {
                let progress = p_recv.borrow();
                progress_report_clone.send(Step::Downloading(
                    progress.current_size,
                    progress.total_size,
                ))?;
            }

            Ok::<(), anyhow::Error>(())
        });

        let result = {
            let download_result = carbon_net::download_multiple(
                &[downloadable.clone()],
                DownloadOptions::builder()
                    .cancel_token(java_checker.cancellation().unwrap_or_default())
                    .concurrency(1)
                    .progress_sender(p_sender)
                    .build(),
            )
            .await;
            progress_proxy.await??;
            download_result?;
            anyhow::ensure!(
                !java_checker
                    .cancellation()
                    .is_some_and(|c| c.is_cancelled()),
                "Managed Java setup cancelled"
            );
            trace!("Download complete");

            let file_handle = std::fs::File::open(&downloadable.path).with_context(|| {
                format!("Could not open downloaded file: {:?}", &downloadable.path)
            })?;
            let mut archive = zip::ZipArchive::new(file_handle).with_context(|| {
                format!(
                    "Could not open downloaded file as zip: {:?}",
                    &downloadable.path
                )
            })?;

            let progress_report_clone = progress_report.clone();
            let version_name = build.name.clone();
            let (main_binary_path, install_root) = spawn_blocking(move || {
                let total_archive_files = archive.len() as u64;

                let root_dir = {
                    let root: PathBuf = archive
                        .by_index(0)?
                        .enclosed_name()
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "Invalid zip. Cannot get enclosed name for item 0 of zip"
                            )
                        })?
                        .to_owned();

                    single_root_component(&root)?
                };

                let is_single_root_dir = archive.file_names().all(|file_name| {
                    let path = Path::new(file_name);
                    let Some(os_str) = path.components().next() else {
                        return false;
                    };
                    os_str.as_os_str() == root_dir
                });

                let java_managed_path = if is_single_root_dir {
                    base_managed_java_path.to_path()
                } else {
                    let removed_extension = PathBuf::from(version_name).with_extension("");
                    base_managed_java_path.to_path().join(removed_extension)
                };

                // Directory to wipe if this install turns out unusable. Always a
                // per-install subdirectory (base/<root_dir> in the single-root case,
                // where java_managed_path is the managed-javas root itself), never
                // the managed-javas root, so cleanup can't take out other JREs.
                let install_root = if is_single_root_dir {
                    java_managed_path.join(&root_dir)
                } else {
                    java_managed_path.clone()
                };

                std::fs::create_dir_all(&java_managed_path).with_context(|| {
                    format!("Could not create directory: {:?}", &java_managed_path)
                })?;

                let mut main_binary_path = None;

                for i in 0..archive.len() {
                    let mut file = archive.by_index(i)?;
                    let outpath = match file.enclosed_name() {
                        Some(path) => Path::new(&java_managed_path).join(path),
                        None => continue,
                    };

                    // Skip symlink entries. The JRE zip ships convenience aliases
                    // (e.g. bin -> zulu-.../Contents/Home/bin) as symlinks whose body
                    // is just the target-path text; writing that as a regular file
                    // corrupts the tree. The real bin/java is a regular file, so this
                    // must run before main-binary detection to avoid selecting a link.
                    if file
                        .unix_mode()
                        .is_some_and(|mode| mode & 0o170000 == 0o120000)
                    {
                        continue;
                    }

                    if (*file.name()).ends_with("bin/java")
                        || (*file.name()).ends_with("bin/java.exe")
                    {
                        main_binary_path = Some(outpath.clone());
                    }

                    if (*file.name()).ends_with('/') {
                        std::fs::create_dir_all(&outpath)?;
                    } else {
                        if let Some(p) = outpath.parent() {
                            if !p.exists() {
                                std::fs::create_dir_all(p).map_err(|err| {
                                    anyhow::anyhow!("Can't create directory {:?} - {}", p, err)
                                })?;
                            }
                        }

                        if !outpath.exists() || file.size() != outpath.metadata()?.len() {
                            let mut outfile = std::fs::File::create(&outpath).map_err(|err| {
                                anyhow::anyhow!("Can't create file {:?} - {}", outpath, err)
                            })?;

                            std::io::copy(&mut file, &mut outfile).map_err(|err| {
                                anyhow::anyhow!(
                                    "Can't copy file {} -> {:?} - {}",
                                    file.name(),
                                    outpath,
                                    err
                                )
                            })?;
                        }
                    }

                    // Get and Set permissions
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;

                        if let Some(mode) = file.unix_mode() {
                            std::fs::set_permissions(
                                &outpath,
                                std::fs::Permissions::from_mode(mode),
                            )
                            .map_err(|err| {
                                anyhow::anyhow!(
                                    "Can't set file permission on {} - {}",
                                    file.name(),
                                    err
                                )
                            })?;
                        }
                    }

                    progress_report_clone.send(Step::Extracting(i as u64, total_archive_files))?;
                }

                let main_binary_path =
                    main_binary_path.ok_or_else(|| anyhow::anyhow!("No main binary found"))?;

                Ok::<_, anyhow::Error>((main_binary_path, install_root))
            })
            .await??;

            progress_report.send(Step::Done)?;

            Ok::<_, anyhow::Error>((main_binary_path, install_root))
        };

        let delete = std::fs::remove_file(&downloadable.path);

        if let Err(e) = delete {
            tracing::warn!("Could not delete downloaded file: {}", e);
        }

        let (main_binary_path, install_root) = result?;
        let main_binary_path = match dunce::canonicalize(&main_binary_path) {
            Ok(p) => p,
            Err(_) => main_binary_path,
        };

        validate_managed_install(db, java_checker, &main_binary_path, &install_root).await
    }
}

/// Extracts and validates the first path component of a zip's root entry name,
/// which decides both whether the archive has a single common root directory
/// (`is_single_root_dir` in `setup`) and, if so, what that directory's cleanup
/// path (`install_root`) will be.
///
/// `enclosed_name()` already guards against absolute paths and `..` components,
/// but not against a leading `./`: an archive whose entries are all
/// `./`-prefixed (corrupt or crafted) resolves its first component to a bare
/// `.` (`Component::CurDir`), not a real directory name. Joining the
/// managed-javas root with "." is a no-op, so trusting that as `install_root`
/// would make it alias the managed-javas root itself -- and the failure branch
/// in `setup` `remove_dir_all`s `install_root`, wiping every other installed
/// JRE along with the bad one. Only a genuine, single named directory
/// component (`Component::Normal`) is accepted.
fn single_root_component(root_entry_name: &Path) -> anyhow::Result<std::ffi::OsString> {
    let first_component = root_entry_name
        .components()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No root component"))?;

    let std::path::Component::Normal(name) = first_component else {
        return Err(anyhow::anyhow!(
            "Invalid zip. Root entry is not a plain directory name: {:?}",
            first_component
        ));
    };

    Ok(name.to_owned())
}

/// Best-effort fetch of a package's sha256 from Azul's per-package detail endpoint
/// (`{base_url}{package_uuid}`), for packages whose list entry carried no checksum.
///
/// Returns `None` on any failure -- request error, timeout, non-success status,
/// unexpected response shape, or a missing/null field -- so the install always
/// proceeds regardless: a missing checksum just means the download isn't
/// verified against one, and only a genuinely served hash gets checked.
async fn fetch_package_sha256(base_url: &str, package_uuid: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct PackageDetail {
        sha256_hash: Option<String>,
    }

    let url = format!("{base_url}{package_uuid}");

    let fetch = async {
        reqwest::get(&url)
            .await
            .ok()?
            .error_for_status()
            .ok()?
            .json::<PackageDetail>()
            .await
            .ok()?
            .sha256_hash
    };

    // A stall here must not hold up the whole install: this is an integrity-check
    // enhancement, not something the download can't proceed without.
    match tokio::time::timeout(std::time::Duration::from_secs(10), fetch).await {
        Ok(sha256) => sha256.map(|hash| hash.trim().to_lowercase()),
        Err(_) => {
            trace!(
                "Timed out fetching sha256 for Azul package {package_uuid}; installing without a checksum"
            );
            None
        }
    }
}

const AZUL_BASE_URL: &str = "https://api.azul.com/metadata/v1/zulu/packages/";

/// Ceiling on a single Azul request. Azul normally answers in well under a
/// second, so this is generous headroom rather than an expected duration — but
/// it must exist: see `azul_client` for why an unbounded attempt makes the
/// retry actively harmful.
pub(super) const AZUL_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Retries per request, on top of the initial attempt.
pub(super) const AZUL_MAX_RETRIES: u32 = 3;

/// A retrying HTTP client for the Azul metadata API.
///
/// Both halves are load-bearing and neither works alone. Retrying without a
/// per-attempt timeout does not bound failure, it multiplies it: a server that
/// accepts and never answers would stall every attempt in turn. A single
/// transient failure is common enough (roughly one request in thirty) that
/// without a retry it would regularly fail a whole catalog fetch.
pub(super) fn azul_client(
    timeout: Duration,
    max_retries: u32,
) -> anyhow::Result<reqwest_middleware::ClientWithMiddleware> {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(max_retries);

    Ok(
        ClientBuilder::new(reqwest::Client::builder().timeout(timeout).build()?)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build(),
    )
}
struct AzulAPI;

impl AzulAPI {
    async fn fetch_builds(
        os: &JavaOs,
        arch: &JavaArch,
        base_url: &str,
        max_retries: u32,
    ) -> anyhow::Result<Vec<ManagedJavaBuild>> {
        let mut results: Vec<AzulZuluPackage> = Vec::new();
        let mut page = 1;

        let client = azul_client(AZUL_REQUEST_TIMEOUT, max_retries)?;

        loop {
            let url = format!(
                "{base_url}?java_package_type=jre&javafx_bundled=false&release_status=ga&availability_types=CA&archive_type=zip&os={}&arch={}&include_fields=build_date,size,sha256_hash,support_term&page={page}&page_size=100",
                match os {
                    JavaOs::Windows => "windows",
                    JavaOs::Linux => "linux",
                    JavaOs::MacOs => "macos",
                },
                match arch {
                    JavaArch::X86_64 => "amd64",
                    JavaArch::X86_32 => "i686",
                    JavaArch::Arm32 => "aarch32",
                    JavaArch::Arm64 => "aarch64",
                }
            );

            let req = client.get(&url).send().await?.error_for_status()?;

            let pagination: Pagination = serde_json::from_str(
                req.headers()
                    .get("X-Pagination")
                    .ok_or_else(|| anyhow::anyhow!("No pagination header"))?
                    .to_str()?,
            )?;

            let mut packages = req.json::<Vec<AzulZuluPackage>>().await?;
            let page_was_empty = packages.is_empty();
            results.append(&mut packages);

            if page_was_empty || results.len() as u64 >= pagination.total {
                break;
            }

            page += 1;
        }

        results
            .into_iter()
            .map(ManagedJavaBuild::try_from)
            .collect()
    }
}

#[derive(Deserialize, Debug)]
struct Pagination {
    total: u64,
}

#[derive(Deserialize, Debug)]
struct AzulZuluPackage {
    package_uuid: String,
    name: String,
    java_version: Vec<u32>,
    download_url: String,
    build_date: Option<String>,
    size: Option<u64>,
    sha256_hash: Option<String>,
    support_term: Option<String>,
}

impl TryFrom<AzulZuluPackage> for ManagedJavaBuild {
    type Error = anyhow::Error;

    fn try_from(package: AzulZuluPackage) -> anyhow::Result<Self> {
        let major = package
            .java_version
            .first()
            .and_then(|major| u8::try_from(*major).ok())
            .ok_or_else(|| anyhow::anyhow!("No major version found for {}", package.name))?;

        Ok(Self {
            id: package.package_uuid,
            name: package.name,
            major,
            version: JavaBuildVersion::from_components(&package.java_version),
            source: BuildSource::AzulZip {
                download_url: package.download_url,
                sha256: package
                    .sha256_hash
                    .map(|hash| hash.trim().to_lowercase())
                    .filter(|hash| !hash.is_empty()),
            },
            size_bytes: package.size,
            release_date: package.build_date,
            lts: package.support_term.as_deref() == Some("lts"),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_get_available_versions() {
        let catalog = AzulZulu::default().catalog().await.unwrap();

        assert!(catalog.latest_for_major(17).is_some());
    }

    /// One package of an Azul list response.
    fn package(uuid: &str, version: &[u32], lts: bool) -> serde_json::Value {
        let joined = version
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(".");
        serde_json::json!({
            "package_uuid": uuid,
            "name": format!("zulu-jre{joined}.zip"),
            "java_version": version,
            "download_url": format!("https://example.invalid/{uuid}.zip"),
            "build_date": "2025-01-02T03:04:05Z",
            "size": 1234,
            "sha256_hash": format!("ABC{uuid}"),
            "support_term": if lts { "lts" } else { "mts" },
        })
    }

    fn pagination_header(total: usize) -> String {
        format!(r#"{{"total":{total},"total_pages":2}}"#)
    }

    fn catalog_query(page: &str) -> mockito::Matcher {
        mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded(
                "include_fields".into(),
                "build_date,size,sha256_hash,support_term".into(),
            ),
            mockito::Matcher::UrlEncoded(
                "os".into(),
                match JavaOs::get_current_os().unwrap() {
                    JavaOs::Windows => "windows",
                    JavaOs::Linux => "linux",
                    JavaOs::MacOs => "macos",
                }
                .into(),
            ),
            mockito::Matcher::UrlEncoded("page".into(), page.into()),
        ])
    }

    /// Serves three builds, deliberately out of order, over two pages.
    async fn mock_two_pages(server: &mut mockito::Server, expect: usize) -> Vec<mockito::Mock> {
        let page1 = server
            .mock("GET", "/")
            .match_query(catalog_query("1"))
            .with_status(200)
            .with_header("X-Pagination", &pagination_header(3))
            .with_body(
                serde_json::json!([
                    package("a17-19", &[17, 0, 19], true),
                    package("b21-13", &[21, 0, 13, 1], true),
                ])
                .to_string(),
            )
            .expect(expect)
            .create_async()
            .await;
        let page2 = server
            .mock("GET", "/")
            .match_query(catalog_query("2"))
            .with_status(200)
            .with_header("X-Pagination", &pagination_header(3))
            .with_body(serde_json::json!([package("c17-21", &[17, 0, 21, 1], false)]).to_string())
            .expect(expect)
            .create_async()
            .await;
        vec![page1, page2]
    }

    #[tokio::test]
    async fn catalog_groups_and_sorts_current_platform() {
        let mut server = mockito::Server::new_async().await;
        let mocks = mock_two_pages(&mut server, 1).await;

        let azul = AzulZulu::with_base_url(format!("{}/", server.url()));
        let catalog = azul.catalog().await.unwrap();

        assert_eq!(
            catalog.majors.iter().map(|m| m.major).collect::<Vec<_>>(),
            vec![21, 17]
        );
        let seventeen = &catalog.majors[1];
        assert!(seventeen.lts);
        assert_eq!(
            seventeen
                .builds
                .iter()
                .map(|b| b.version.to_string())
                .collect::<Vec<_>>(),
            vec!["17.0.21.1", "17.0.19"]
        );
        assert!(seventeen.builds[1].lts);
        assert!(!seventeen.builds[0].lts);

        let newest = catalog.latest_for_major(17).unwrap();
        assert_eq!(newest.id, "c17-21");
        assert_eq!(newest.release_date.as_deref(), Some("2025-01-02T03:04:05Z"));
        assert_eq!(newest.size_bytes, Some(1234));
        match &newest.source {
            BuildSource::AzulZip {
                download_url,
                sha256,
            } => {
                assert_eq!(download_url, "https://example.invalid/c17-21.zip");
                assert_eq!(sha256.as_deref(), Some("abcc17-21"));
            }
            other => panic!("not an Azul build: {other:?}"),
        }
        assert_eq!(catalog.find("b21-13").unwrap().major, 21);

        for mock in mocks {
            mock.assert_async().await;
        }
    }

    #[tokio::test]
    async fn catalog_is_cached_within_ttl() {
        let mut server = mockito::Server::new_async().await;
        let mocks = mock_two_pages(&mut server, 1).await;

        let azul = AzulZulu::with_base_url(format!("{}/", server.url()));
        azul.catalog().await.unwrap();
        azul.catalog().await.unwrap();

        for mock in mocks {
            mock.assert_async().await;
        }
    }

    #[tokio::test]
    async fn cached_catalog_never_fetches() {
        let mut server = mockito::Server::new_async().await;
        let mocks = mock_two_pages(&mut server, 1).await;
        let azul = AzulZulu::with_base_url(format!("{}/", server.url()));

        // Nothing fetched yet: nothing to read, and no request.
        assert!(azul.cached_catalog().is_none());

        let majors = |catalog: &ManagedJavaCatalog| {
            catalog.majors.iter().map(|m| m.major).collect::<Vec<_>>()
        };
        let fetched = azul.catalog().await.unwrap();
        assert_eq!(
            azul.cached_catalog().as_ref().map(majors),
            Some(majors(&fetched))
        );

        // While a fetch waits on the network, a peek reads the last catalog
        // rather than waiting or reading nothing.
        let fetching = azul.hold_fetch_for_test().await;
        assert_eq!(
            azul.cached_catalog().as_ref().map(majors),
            Some(majors(&fetched))
        );
        drop(fetching);
        assert!(azul.cached_catalog().is_some());
        // Only the one fetch above.
        for mock in mocks {
            mock.assert_async().await;
        }
    }

    #[tokio::test]
    async fn catalog_error_is_not_cached() {
        let mut server = mockito::Server::new_async().await;
        let failing = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(500)
            .expect(1)
            .create_async()
            .await;

        let azul = AzulZulu::with_base_url(format!("{}/", server.url()));
        assert!(azul.catalog().await.is_err());
        failing.assert_async().await;
        failing.remove_async().await;

        let mocks = mock_two_pages(&mut server, 1).await;
        assert!(azul.catalog().await.is_ok());
        for mock in mocks {
            mock.assert_async().await;
        }
    }

    /// One page of a well-formed Azul list response.
    fn azul_page_body() -> String {
        serde_json::json!([{
            "package_uuid": "11111111-2222-3333-4444-555555555555",
            "name": "zulu17.50.19-ca-jre17.0.11-linux_x64.zip",
            "java_version": [17, 0, 11],
            "openjdk_build_number": 9,
            "latest": true,
            "download_url": "https://example.invalid/zulu.zip",
            "product": "zulu",
            "distro_version": [17, 50, 19],
            "availability_type": "CA"
        }])
        .to_string()
    }

    /// A single transient failure must not fail the whole catalog fetch.
    ///
    /// Asserts the transient 500 is retried rather than propagated. Serves the
    /// failure first and the success second; mockito hands each request to the
    /// first matching mock that has not yet met its expected hit count, so the
    /// retry is what reaches the second mock.
    #[tokio::test]
    async fn fetch_builds_retries_a_transient_failure() {
        let mut server = mockito::Server::new_async().await;

        let failing = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(500)
            .expect(1)
            .create_async()
            .await;

        let succeeding = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("X-Pagination", r#"{"total":1,"total_pages":1}"#)
            .with_body(azul_page_body())
            .expect(1)
            .create_async()
            .await;

        let base_url = format!("{}/", server.url());
        let result = AzulAPI::fetch_builds(
            &JavaOs::Linux,
            &JavaArch::X86_64,
            &base_url,
            AZUL_MAX_RETRIES,
        )
        .await;

        let versions = result.expect("a single transient 500 must be retried, not propagated");
        assert_eq!(versions.len(), 1);

        failing.assert_async().await;
        succeeding.assert_async().await;
    }

    /// Regression test for a defect introduced by the retry above: retrying a
    /// request whose individual attempts are unbounded does not bound failure,
    /// it multiplies it. `reqwest::Client::new()` sets no timeout, so a server
    /// that accepts the connection and then never answers would hang each of
    /// the attempts in turn -- turning one indefinite stall into several, and
    /// blowing budgets far downstream (the e2e harness allows 11 minutes for an
    /// instance install).
    ///
    /// Drives a listener that accepts and never responds. The call must return
    /// an error promptly rather than hang; the assertion is the outer
    /// `tokio::time::timeout`, which fires only if the client is unbounded.
    /// Short timeout/retry values are passed so the test stays fast — the
    /// behaviour under test is that the bound exists and is honoured, not its
    /// production magnitude.
    #[tokio::test]
    async fn azul_client_bounds_each_attempt_against_a_stalled_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Accept connections and hold them open without ever writing a
        // response. Held in a Vec so the sockets are not dropped (a dropped
        // socket would close the connection and let the client fail fast for
        // the wrong reason).
        tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((stream, _)) = listener.accept().await {
                held.push(stream);
            }
        });

        let client = azul_client(Duration::from_millis(400), 1).unwrap();

        let call = client.get(format!("http://{addr}/")).send();
        let outcome = tokio::time::timeout(Duration::from_secs(20), call).await;

        let result = outcome.expect(
            "the request must be bounded by its own timeout and return, not hang — an unbounded \
             client turns each retry into another indefinite stall",
        );
        assert!(
            result.is_err(),
            "a server that never responds must surface as an error"
        );
    }

    /// Regression: `setup`'s Content-Length probe used to call the bare,
    /// unbounded `reqwest::get` directly -- unlike every other network call
    /// in this module, it had neither a timeout nor a retry policy, so a
    /// server that accepted the connection and stalled would hang the whole
    /// managed-JRE install indefinitely. `fetch_package_sha256` keeps its
    /// own bespoke `tokio::time::timeout` wrapper around a bare
    /// `reqwest::get` (unrelated, untouched here, and already bounded by
    /// that wrapper); this only asserts the probe inside `setup` itself no
    /// longer bypasses the module's bounded `azul_client`.
    #[test]
    fn setup_probes_content_length_through_the_bounded_azul_client() {
        let source = include_str!("azul_zulu.rs");
        let setup_start = source.find("async fn setup<G").expect("setup fn not found");
        let setup_end = setup_start
            + source[setup_start..]
                .find("\n}\n\n/// Extracts and validates")
                .expect("end of setup fn not found");
        let setup_body = &source[setup_start..setup_end];

        assert!(
            !setup_body.contains("reqwest::get"),
            "setup() must not call the bare, unbounded reqwest::get directly"
        );
        assert!(
            setup_body.contains("azul_client("),
            "setup() must bound its Content-Length probe through azul_client"
        );
    }

    #[test]
    fn single_root_component_accepts_a_plain_directory_name() {
        let root = single_root_component(Path::new("zulu-21.0.1-jre/bin/java")).unwrap();
        assert_eq!(root, "zulu-21.0.1-jre");
    }

    /// Regression test: an archive whose entries are all `./`-prefixed must be
    /// rejected, not accepted with `root_dir == "."` -- which would otherwise
    /// make `install_root` alias the managed-javas root and let the cleanup
    /// path in `setup` wipe every installed JRE.
    #[test]
    fn single_root_component_rejects_a_leading_dot_slash() {
        assert!(single_root_component(Path::new("./bin/java")).is_err());
    }

    #[test]
    fn single_root_component_rejects_a_bare_current_dir() {
        assert!(single_root_component(Path::new(".")).is_err());
    }

    #[test]
    fn single_root_component_rejects_a_bare_parent_dir() {
        assert!(single_root_component(Path::new("..")).is_err());
    }
}
