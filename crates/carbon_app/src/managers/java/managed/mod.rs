use serde::Serialize;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};
use strum::IntoEnumIterator;
use tokio::sync::{watch::Sender, Mutex};

use crate::{
    api::keys::java::GET_SETUP_MANAGED_JAVA_PROGRESS,
    db::PrismaClient,
    domain::{
        java::{JavaArch, JavaOs, JavaVendor},
        runtime_path::{ManagedJavasPath, TempPath},
    },
};

use self::azul_zulu::AzulZulu;

use super::java_checker::{JavaChecker, RealJavaChecker};

// mod adoptopenjdk;
// mod mojang;
pub mod azul_zulu;

#[derive(Debug, Default, Clone)]
pub enum Step {
    #[default]
    Idle,
    Downloading(u64, u64),
    Extracting(u64, u64),
    Done,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize)]
pub struct ManagedJavaVersion {
    pub id: String,
    pub name: String,
    pub download_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManagedJavaArchMap(pub HashMap<JavaArch, Vec<ManagedJavaVersion>>);

impl Deref for ManagedJavaArchMap {
    type Target = HashMap<JavaArch, Vec<ManagedJavaVersion>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ManagedJavaArchMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ManagedJavaOsMap(pub HashMap<JavaOs, ManagedJavaArchMap>);

impl Deref for ManagedJavaOsMap {
    type Target = HashMap<JavaOs, ManagedJavaArchMap>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ManagedJavaOsMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait::async_trait]
pub trait Managed {
    async fn setup<G: JavaChecker + Send + Sync>(
        &self,
        version: &ManagedJavaVersion,
        tmp_path: TempPath,
        base_managed_java_path: ManagedJavasPath,
        java_checker: &G,
        db_client: &Arc<PrismaClient>,
        progress_report: Sender<Step>,
    ) -> anyhow::Result<()>;

    async fn fetch_all_versions(&self) -> anyhow::Result<ManagedJavaOsMap>;
}

#[derive(Debug, Default)]
pub struct ManagedService {
    azul_zulu: AzulZulu,
    pub setup_progress: Arc<Mutex<Step>>,
}

impl ManagedService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_all_os(&self) -> Vec<JavaOs> {
        JavaOs::iter().collect()
    }

    pub fn get_all_archs(&self) -> Vec<JavaArch> {
        JavaArch::iter().collect()
    }

    pub fn get_all_vendors(&self) -> Vec<JavaVendor> {
        JavaVendor::iter().collect()
    }

    pub async fn get_versions_for_vendor(
        &self,
        vendor: JavaVendor,
    ) -> anyhow::Result<ManagedJavaOsMap> {
        let versions = match vendor {
            JavaVendor::Azul => self.azul_zulu.fetch_all_versions().await?,
        };

        Ok(versions)
    }

    pub async fn setup_managed(
        &self,
        os: JavaOs,
        arch: JavaArch,
        vendor: JavaVendor,
        id: String,
        app: crate::App,
    ) -> anyhow::Result<()> {
        match vendor {
            JavaVendor::Azul => {
                let versions = self.azul_zulu.fetch_all_versions().await?;
                let version = versions
                    .get(&os)
                    .ok_or_else(|| anyhow::anyhow!("No versions for os: {:?}", os))?
                    .get(&arch)
                    .ok_or_else(|| anyhow::anyhow!("No versions for arch: {:?}", arch))?
                    .iter()
                    .find(|v| v.id == id)
                    .ok_or_else(|| anyhow::anyhow!("No version for id: {}", id))?;

                let tmp_path = app.settings_manager().runtime_path.get_temp();
                let base_managed_java_path =
                    app.settings_manager().runtime_path.get_managed_javas();
                let db_client = &app.prisma_client.clone();

                let (sender, mut recv) = tokio::sync::watch::channel(Step::Idle);

                let progress_ref = Arc::clone(&self.setup_progress);

                tokio::spawn(async move {
                    let app = app.clone();

                    while recv.changed().await.is_ok() {
                        let mut progress_ref = progress_ref.lock().await;
                        let borrowed_progress = recv.borrow().clone();
                        *progress_ref = borrowed_progress;
                        app.invalidate(GET_SETUP_MANAGED_JAVA_PROGRESS, None);
                        drop(progress_ref);
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                });

                self.azul_zulu
                    .setup(
                        version,
                        tmp_path,
                        base_managed_java_path,
                        &RealJavaChecker,
                        db_client,
                        sender,
                    )
                    .await?;
            }
        };

        Ok(())
    }
}
