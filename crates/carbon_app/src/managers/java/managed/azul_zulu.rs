use super::{Managed, ManagedJavaArchMap, ManagedJavaOsMap, ManagedJavaVersion, Step};
use crate::{
    domain::java::{JavaArch, JavaOs, JavaVersion},
    managers::java::{java_checker::JavaChecker, scan_and_sync::upsert_java_component_to_db},
};
use anyhow::Context;
use carbon_net::{Checksum, DownloadOptions, Downloadable, Progress};
use carbon_repos::db_exec::Db;
use carbon_rt_path::{ManagedJavasPath, TempPath};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use strum::IntoEnumIterator;
use tokio::{
    sync::{RwLock, watch::Sender},
    task::spawn_blocking,
};
use tracing::{instrument, trace};

#[derive(Debug, Default)]
pub struct AzulZulu {
    versions: Arc<RwLock<ManagedJavaOsMap>>,
}

#[async_trait::async_trait]
impl Managed for AzulZulu {
    #[instrument(skip(self, java_checker, db, progress_report))]
    async fn setup<G: JavaChecker + Send + Sync>(
        &self,
        version: &ManagedJavaVersion,
        tmp_path: TempPath,
        base_managed_java_path: ManagedJavasPath,
        java_checker: &G,
        db: &Db,
        progress_report: Sender<Step>,
    ) -> anyhow::Result<String> {
        let progress_report = Arc::new(progress_report);

        let download_temp_path = tmp_path.to_path().join(&version.name);

        trace!("Download path: {:?}", download_temp_path);

        let download_url = &version.download_url;

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

        // Best-effort: the list endpoint behind `fetch_all_versions` doesn't carry a
        // checksum at all, but Azul's per-package detail endpoint does. A failure here
        // (network blip, timeout, unexpected response shape) must never block the
        // install -- it just means the download proceeds unchecked, exactly as before
        // this existed.
        let sha256 = fetch_package_sha256(&version.id).await;

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
            carbon_net::download_multiple(
                &[downloadable.clone()],
                DownloadOptions::builder()
                    .concurrency(1)
                    .progress_sender(p_sender)
                    .build(),
            )
            .await?;

            trace!("Download complete");

            progress_proxy.await??;

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
            let version_name = version.name.clone();
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

        let java_component = match java_checker
            .get_bin_info(
                &main_binary_path,
                crate::domain::java::JavaComponentType::Managed,
            )
            .await
        {
            Ok(component) => component,
            Err(e) => {
                // A partially or incorrectly extracted JRE would otherwise persist and
                // fail every launch — the size-based skip in the extraction loop never
                // re-writes an existing same-size file, so it can't self-heal. Remove
                // the install directory to force a clean re-download on the next try.
                if let Err(rm) = std::fs::remove_dir_all(&install_root) {
                    tracing::warn!(
                        "Could not clean up unusable managed JRE at {:?}: {}",
                        install_root,
                        rm
                    );
                }

                return Err(e).with_context(|| {
                    format!(
                        "Could not get bin info for main binary: {:?}",
                        &main_binary_path
                    )
                });
            }
        };

        let java_id = upsert_java_component_to_db(db, java_component).await?;

        Ok(java_id)
    }

    async fn fetch_all_versions(&self) -> anyhow::Result<ManagedJavaOsMap> {
        let mut versions = self.versions.write().await;
        if versions.is_empty() {
            let results = AzulAPI::get_all_versions().await?;
            *versions = results;
        }

        Ok(versions.clone())
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
/// (`{AZUL_BASE_URL}{package_uuid}`). The list endpoint behind `get_all_by_os_arch`
/// doesn't carry a checksum at all, so this is a second call keyed on the package
/// UUID already known from that list.
///
/// Returns `None` on any failure -- request error, timeout, non-success status,
/// unexpected response shape, or a missing/null field -- so the install always
/// proceeds regardless: a missing checksum just means the download isn't
/// verified against one, and only a genuinely served hash gets checked.
async fn fetch_package_sha256(package_uuid: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct PackageDetail {
        sha256_hash: Option<String>,
    }

    let url = format!("{AZUL_BASE_URL}{package_uuid}");

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
const AZUL_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Retries per request, on top of the initial attempt.
const AZUL_MAX_RETRIES: u32 = 3;

/// A retrying HTTP client for the Azul metadata API.
///
/// `get_all_versions` spawns one request chain per `JavaOs` x `JavaArch`
/// (3 x 4 = 12) and joins them with `task.await??`, which is fail-fast: a
/// single transient failure among the twelve aborts the entire JRE version
/// resolution and reaches the user as a failed instance install. With no retry
/// at all, even a 3% per-request failure rate makes that `1 - 0.97^12` — about
/// a 31% chance of failing outright, matching the roughly one-in-three rate
/// observed live.
///
/// Both halves are load-bearing and neither works alone. Retrying without a
/// per-attempt timeout does not bound failure, it multiplies it: a server that
/// accepts and never answers would stall every attempt in turn. The fail-fast
/// join in `get_all_versions` is deliberately left as it is — a genuinely
/// unavailable Azul should fail loudly rather than silently yield a partial
/// version map.
fn azul_client(
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
    async fn get_all_versions() -> anyhow::Result<ManagedJavaOsMap> {
        let results = HashMap::new();

        let rwlock_results = Arc::new(RwLock::new(results));
        let mut tasks = Vec::new();

        for os in JavaOs::iter() {
            for arch in JavaArch::iter() {
                let os = os.clone();
                let arch = arch.clone();
                let arced_rwlock_results = rwlock_results.clone();
                tasks.push(tokio::spawn(async move {
                    let versions = Self::get_all_by_os_arch(&os, &arch, AZUL_BASE_URL).await?;

                    let mut results = arced_rwlock_results.write().await;
                    let os = results
                        .entry(os.clone())
                        .or_insert_with(|| ManagedJavaArchMap(HashMap::new()));

                    let arch = os.entry(arch).or_insert_with(Vec::new);

                    for version in versions {
                        arch.push(ManagedJavaVersion {
                            name: version.name.clone(),
                            download_url: version.download_url.clone(),
                            id: version.package_uuid.clone(),
                            java_version: JavaVersion {
                                major: version.java_version.first().cloned().ok_or(
                                    anyhow::anyhow!("No major version found for {}", version.name),
                                )?,
                                minor: version.java_version.get(1).cloned().unwrap_or(0),
                                patch: version
                                    .java_version
                                    .get(2)
                                    .cloned()
                                    .map(|v| v.to_string())
                                    .unwrap_or("0".to_string()),
                                build_metadata: None,
                                prerelease: None,
                                update_number: None,
                            },
                        });
                    }

                    Ok::<(), anyhow::Error>(())
                }));
            }
        }

        for task in tasks {
            task.await??;
        }

        // Get the hashmap out of the rwlock
        Ok(ManagedJavaOsMap(std::mem::take(
            &mut *rwlock_results.write_owned().await,
        )))
    }

    async fn get_all_by_os_arch(
        os: &JavaOs,
        arch: &JavaArch,
        base_url: &str,
    ) -> anyhow::Result<Vec<AzulZuluVersion>> {
        let mut results: Vec<AzulZuluVersion> = Vec::new();
        let mut page = 1;

        let client = azul_client(AZUL_REQUEST_TIMEOUT, AZUL_MAX_RETRIES)?;

        loop {
            let url = format!(
                "{base_url}?java_package_type=jre&javafx_bundled=false&release_status=ga&availability_types=CA&archive_type=zip&page={}&os={}&arch={}",
                page,
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

            let req = client.get(&url).send().await?;

            let pagination: Pagination = serde_json::from_str(
                req.headers()
                    .get("X-Pagination")
                    .ok_or_else(|| anyhow::anyhow!("No pagination header"))?
                    .to_str()?,
            )?;

            results.append(&mut req.json::<Vec<AzulZuluVersion>>().await?);

            if results.len() as u64 >= pagination.total {
                break;
            }

            page += 1;
        }

        Ok(results)
    }
}

#[derive(Deserialize, Debug)]
pub struct Pagination {
    total: u64,
    total_pages: u64,
    first_page: Option<u64>,
    last_page: Option<u64>,
    page: Option<u64>,
}

#[derive(Deserialize, Debug)]
pub struct AzulZuluVersion {
    package_uuid: String,
    name: String,
    java_version: Vec<u16>,
    openjdk_build_number: Option<u32>,
    latest: bool,
    download_url: String,
    product: String,
    distro_version: Vec<u8>,
    availability_type: String,
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_get_available_versions() {
        let versions = AzulAPI::get_all_versions().await.unwrap();

        assert!(!versions.is_empty());
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

    /// Regression test for the fixture-install failure that made the e2e suite
    /// unable to go green: `get_all_versions` fans out one request per
    /// `JavaOs` x `JavaArch` combination (3 x 4 = 12) and joins them with
    /// `task.await??`, which is fail-fast. With no retry on the individual
    /// request, a single transient failure among the twelve aborted the whole
    /// JRE version resolution and surfaced to the user as a failed instance
    /// install. At a 3% per-request failure rate that is `1 - 0.97^12` — about
    /// a 31% chance of failing the whole resolution, which matches the roughly
    /// one-in-three rate observed live.
    ///
    /// Asserts the transient 500 is retried rather than propagated. Serves the
    /// failure first and the success second; mockito hands each request to the
    /// first matching mock that has not yet met its expected hit count, so the
    /// retry is what reaches the second mock.
    #[tokio::test]
    async fn get_all_by_os_arch_retries_a_transient_failure() {
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
        let result =
            AzulAPI::get_all_by_os_arch(&JavaOs::Linux, &JavaArch::X86_64, &base_url).await;

        let versions = result.expect(
            "a single transient 500 must be retried, not propagated — without a retry-enabled \
             client this is the failure that aborts all twelve concurrent requests via the \
             fail-fast `task.await??` join in get_all_versions",
        );
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
                .find("\n    async fn fetch_all_versions")
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
