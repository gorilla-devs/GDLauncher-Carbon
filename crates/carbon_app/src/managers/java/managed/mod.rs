use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::Arc,
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use tokio::sync::{watch::Sender, Mutex};

use crate::{
    db::PrismaClient,
    domain::java::{JavaArch, JavaOs, Vendor},
};

use self::azul_zulu::AzulZulu;

use super::java_checker::JavaChecker;

// mod adoptopenjdk;
// mod mojang;
pub mod azul_zulu;

#[derive(Debug, Default)]
pub enum Step {
    #[default]
    Idle,
    Downloading(u64, u64),
    Extracting(u64, u64),
    Done,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ManagedJavaVersion {
    pub id: String,
    pub name: String,
    pub download_url: String,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
    type VersionType;

    async fn setup<G>(
        &self,
        version: Self::VersionType,
        tmp_path: PathBuf,
        base_managed_java_path: PathBuf,
        java_checker: &G,
        db_client: &Arc<PrismaClient>,
        progress_report: Sender<Step>,
    ) -> anyhow::Result<()>
    where
        G: JavaChecker + Send + Sync;

    async fn fetch_all_versions(&self) -> anyhow::Result<ManagedJavaOsMap>;
}

pub struct ManagedService {
    azul_zulu: AzulZulu,
    setup_progress: Step,
}

impl ManagedService {
    pub fn new() -> Self {
        Self {
            azul_zulu: AzulZulu::default(),
            setup_progress: Step::Idle,
        }
    }

    pub fn get_all_os(&self) -> Vec<JavaOs> {
        JavaOs::iter().collect()
    }

    pub fn get_all_archs(&self) -> Vec<JavaArch> {
        JavaArch::iter().collect()
    }

    pub fn get_all_vendors(&self) -> Vec<Vendor> {
        Vendor::iter().collect()
    }

    pub async fn get_versions_for_vendor(
        &self,
        vendor: Vendor,
    ) -> anyhow::Result<ManagedJavaOsMap> {
        let versions = match vendor {
            Vendor::Azul => self.azul_zulu.fetch_all_versions().await?,
        };

        Ok(versions)
    }
}
