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
    domain::java::{JavaArch, JavaOs},
    managers::java::{java_checker::JavaChecker, scan_and_sync::add_java_component_to_db},
};

use super::{Managed, ManagedJavaArchMap, ManagedJavaOsMap, ManagedJavaVersion, Step};

#[derive(Debug, Default)]
pub struct AzulZulu;

#[async_trait::async_trait]
impl Managed for AzulZulu {
    type VersionType = AzulZuluVersion;

    async fn setup<G: JavaChecker + Send + Sync>(
        &self,
        version: AzulZuluVersion,
        tmp_path: PathBuf,
        base_managed_java_path: PathBuf,
        java_checker: &G,
        db_client: &Arc<PrismaClient>,
        progress_report: Sender<Step>,
    ) -> anyhow::Result<()> {
        let progress_report = Arc::new(progress_report);

        let download_temp_path = tmp_path.join(&version.name);
        let download_url = version.download_url;

        let content_length = reqwest::get(&download_url).await?.content_length();

        let downloadable = if let Some(content_length) = content_length {
            Downloadable::new(download_url, download_temp_path).with_size(content_length)
        } else {
            Downloadable::new(download_url, download_temp_path)
        };

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

        carbon_net::download_file(&downloadable, Some(p_sender)).await?;

        progress_proxy.await??;

        let file_handle = std::fs::File::open(&downloadable.path)?;
        let mut archive = zip::ZipArchive::new(file_handle)?;

        let java_managed_path = base_managed_java_path.join(&version.name);

        tokio::fs::create_dir_all(&java_managed_path).await?;

        let progress_report_clone = progress_report.clone();
        spawn_blocking(move || {
            let total_archive_files = archive.len() as u64;
            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                let outpath = match file.enclosed_name() {
                    Some(path) => Path::new(&java_managed_path).join(path),
                    None => continue,
                };

                if (*file.name()).ends_with('/') {
                    std::fs::create_dir_all(&outpath)?;
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            std::fs::create_dir_all(p)?;
                        }
                    }
                    let mut outfile = std::fs::File::create(&outpath)?;
                    std::io::copy(&mut file, &mut outfile)?;
                }

                // Get and Set permissions
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;

                    if let Some(mode) = file.unix_mode() {
                        std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
                    }
                }

                progress_report_clone.send(Step::Extracting(i as u64, total_archive_files))?;
            }

            Ok::<(), anyhow::Error>(())
        })
        .await??;

        progress_report.send(Step::Done)?;

        let java_bin_path = PathBuf::new();

        let java_component = java_checker
            .get_bin_info(
                &java_bin_path,
                crate::domain::java::JavaComponentType::Managed,
            )
            .await?;

        add_java_component_to_db(db_client, java_component).await?;

        Ok(())
    }

    async fn fetch_all_versions(&self) -> anyhow::Result<ManagedJavaOsMap> {
        let results = AzulAPI::get_all_versions().await?;
        Ok(results)
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
        let mut page = 0;

        loop {
            let url = format!(
                "{AZUL_BASE_URL}?java_package_type=jre&javafx_bundled=false&release_status=ga&availability_types=CA&archive_type=zip&include_fields=os%2C%20arch&page={}&os={}&arch={}",
                page,
                match os {
                    JavaOs::Windows => "windows",
                    JavaOs::Linux => "linux",
                    JavaOs::MacOs => "macos",
                },
                match arch {
                    JavaArch::X64 => "amd64",
                    JavaArch::X86 => "x86",
                    JavaArch::Aarch64 => "aarch64",
                }
            );

            let req = reqwest::get(&url).await?;

            let pagination: Pagination = serde_json::from_str(
                req.headers()
                    .get("x-pagination")
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
    first_page: u64,
    last_page: u64,
    page: u64,
}

#[derive(Deserialize, Debug)]
pub struct AzulZuluVersion {
    package_uuid: String,
    name: String,
    java_version: Vec<u16>,
    openjdk_build_number: u32,
    latest: bool,
    download_url: String,
    product: String,
    distro_version: Vec<u8>,
    availability_type: String,
    os: String,
    arch: String,
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
