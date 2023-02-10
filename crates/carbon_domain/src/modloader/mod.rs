use super::instance::Instance;
use async_trait::async_trait;
use rspc::Type;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::{fmt::Debug, sync::Weak};
use tokio::sync::{watch::Sender, RwLock};

pub(crate) mod fabric;
pub(crate) mod forge;
pub(crate) mod vanilla;

pub trait ModLoaderError: std::error::Error + Send + Sync + 'static {}

enum ModLoaderInstanceStatus {
    NotPersisted,
    Queued,
    Ready,
}

struct ModLoaderInstance {}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, Ord, PartialOrd, Type)]
pub enum ModLoader {
    Vanilla,
    Forge,
    Fabric,
    LiteLoader,
    Quilt,
}

impl ModLoader {
    pub fn get_version(&self) -> String {
        "".to_string()
    }
}

impl Display for ModLoader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let enum_name = match self {
            ModLoader::Vanilla => "vanilla",
            ModLoader::Forge => "forge",
            ModLoader::Fabric => "fabric",
            ModLoader::LiteLoader => "lite-loader",
            ModLoader::Quilt => "quilt",
        };
        write!(f, "{}", enum_name)
    }
}

impl Default for ModLoader {
    fn default() -> Self {
        ModLoader::Vanilla
    }
}

pub type ModloaderVersion = String;

#[derive(Debug)]
pub struct InstallProgress<T>
where
    T: Debug,
{
    pub count_progress: Option<(u64, u64)>,
    pub size_progress: Option<(u64, u64)>,
    pub stage: Option<T>,
}

#[async_trait]
pub trait ModLoaderHandler
where
    Self: Sized,
{
    type Error: ModLoaderError;
    type Stages: Debug;

    fn new(mod_loader_version: ModloaderVersion, instance: Weak<RwLock<Instance>>) -> Self
    where
        Self: Sized;

    async fn install(
        &self,
        progress_send: Sender<InstallProgress<Self::Stages>>,
    ) -> Result<(), Self::Error>;

    fn remove(&self) -> Result<(), Self::Error>;

    fn verify(&self) -> Result<(), Self::Error>;

    fn get_version(&self) -> ModloaderVersion;
}
