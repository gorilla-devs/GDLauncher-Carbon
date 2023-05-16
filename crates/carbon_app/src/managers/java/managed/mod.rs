use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::Arc,
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use tokio::sync::{watch::Sender, Mutex};

use crate::db::PrismaClient;

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

#[derive(Debug, EnumIter)]
pub enum Vendor {
    Azul,
}

#[derive(Debug, PartialEq, Eq, Hash, EnumIter, Clone)]
pub enum ManagedJavaOs {
    Windows,
    Linux,
    MacOs,
}

impl TryFrom<String> for ManagedJavaOs {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
            "macos" => Ok(Self::MacOs),
            _ => Err(anyhow::anyhow!("Unknown OS: {}", value)),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, EnumIter, Clone)]
pub enum ManagedJavaArch {
    X64,
    Aarch64,
}

impl TryFrom<String> for ManagedJavaArch {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "x64" => Ok(Self::X64),
            "aarch64" => Ok(Self::Aarch64),
            _ => Err(anyhow::anyhow!("Unknown arch: {}", value)),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ManagedJavaVersion {
    pub id: String,
    pub name: String,
    pub download_url: String,
}

#[derive(Debug, Clone)]
pub struct ManagedJavaArchMap(HashMap<ManagedJavaArch, Vec<ManagedJavaVersion>>);

impl Deref for ManagedJavaArchMap {
    type Target = HashMap<ManagedJavaArch, Vec<ManagedJavaVersion>>;

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
pub struct ManagedJavaOsMap(HashMap<ManagedJavaOs, ManagedJavaArchMap>);

impl Deref for ManagedJavaOsMap {
    type Target = HashMap<ManagedJavaOs, ManagedJavaArchMap>;

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
}

impl ManagedService {
    pub fn new() -> Self {
        Self {
            azul_zulu: AzulZulu::default(),
        }
    }

    pub fn get_all_os(&self) -> Vec<ManagedJavaOs> {
        ManagedJavaOs::iter().collect()
    }

    pub fn get_all_archs(&self) -> Vec<ManagedJavaArch> {
        ManagedJavaArch::iter().collect()
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
