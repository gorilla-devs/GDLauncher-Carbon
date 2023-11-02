use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use carbon_net::{Downloadable, Progress};
use serde::Deserialize;
use strum::IntoEnumIterator;
use tokio::{
    sync::{watch::Sender, RwLock},
    task::spawn_blocking,
};

use crate::{
    db::PrismaClient,
    domain::{
        java::{JavaArch, JavaOs, JavaVersion},
        runtime_path::{ManagedJavasPath, TempPath},
    },
    managers::java::{
        java_checker::JavaChecker, scan_and_sync::add_java_component_to_db,
    },
};

use super::{
    Managed, ManagedJavaArchMap, ManagedJavaOsMap, ManagedJavaVersion, Step,
};

#[derive(Debug, Default)]
pub struct AzulZulu {
    versions: Arc<RwLock<ManagedJavaOsMap>>,
}

#[async_trait::async_trait]
impl Managed for AzulZulu {
    async fn setup<G: JavaChecker + Send + Sync>(
        &self,
        version: &ManagedJavaVersion,
        tmp_path: TempPath,
        base_managed_java_path: ManagedJavasPath,
        java_checker: &G,
        db_client: &Arc<PrismaClient>,
        progress_report: Sender<Step>,
    ) -> anyhow::Result<String> {
        let progress_report = Arc::new(progress_report);

        let download_temp_path = tmp_path.to_path().join(&version.name);
        let download_url = &version.download_url;

        let content_length = reqwest::get(download_url).await?.content_length();

        let downloadable = if let Some(content_length) = content_length {
            Downloadable::new(download_url, download_temp_path)
                .with_size(content_length)
        } else {
            Downloadable::new(download_url, download_temp_path)
        };

        let (p_sender, mut p_recv) =
            tokio::sync::watch::channel(Progress::new());

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
            carbon_net::download_file(&downloadable, Some(p_sender)).await?;

            progress_proxy.await??;

            let file_handle = std::fs::File::open(&downloadable.path)?;
            let mut archive = zip::ZipArchive::new(file_handle)?;

            let progress_report_clone = progress_report.clone();
            let version_name = version.name.clone();
            let main_binary_path = spawn_blocking(move || {
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

                    root.components()
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("No root component"))?
                        .as_os_str()
                        .to_owned()
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

                std::fs::create_dir_all(&java_managed_path)?;

                let mut main_binary_path = None;

                for i in 0..archive.len() {
                    let mut file = archive.by_index(i)?;
                    let outpath = match file.enclosed_name() {
                        Some(path) => Path::new(&java_managed_path).join(path),
                        None => continue,
                    };

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

                main_binary_path.ok_or_else(|| anyhow::anyhow!("No main binary found"))
            })
            .await??;

            progress_report.send(Step::Done)?;

            Ok::<_, anyhow::Error>(main_binary_path)
        };

        let delete = std::fs::remove_file(&downloadable.path);

        if let Err(e) = delete {
            tracing::warn!("Could not delete downloaded file: {}", e);
        }

        let main_binary_path = {
            let tmp = result?;
            match dunce::canonicalize(&tmp) {
                Ok(p) => p,
                Err(_) => tmp,
            }
        };

        let java_component = java_checker
            .get_bin_info(
                &main_binary_path,
                crate::domain::java::JavaComponentType::Managed,
            )
            .await?;

        let java_id =
            add_java_component_to_db(db_client, java_component).await?;

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

const AZUL_BASE_URL: &str = "https://api.azul.com/metadata/v1/zulu/packages/";
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
                    let versions = Self::get_all_by_os_arch(&os, &arch).await?;

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
                                major: version
                                    .java_version
                                    .first()
                                    .cloned()
                                    .ok_or(anyhow::anyhow!(
                                        "No major version found for {}",
                                        version.name
                                    ))?,
                                minor: version
                                    .java_version
                                    .get(1)
                                    .cloned()
                                    .unwrap_or(0),
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
    ) -> anyhow::Result<Vec<AzulZuluVersion>> {
        let mut results: Vec<AzulZuluVersion> = Vec::new();
        let mut page = 1;

        loop {
            let url = format!(
                "{AZUL_BASE_URL}?java_package_type=jre&javafx_bundled=false&release_status=ga&availability_types=CA&archive_type=zip&page={}&os={}&arch={}",
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

            let req = reqwest::get(&url).await?;

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
}
