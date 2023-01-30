use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Weak};
use std::time::SystemTime;
use rspc::Type;
use serde::Deserialize;
use thiserror::Error;
use tokio::sync::RwLock;
use carbon_minecraft::instance::{Instance, InstanceStatus};
use carbon_minecraft::minecraft_package::MinecraftPackage;
use crate::app::App;

#[derive(Error, Debug)]
pub enum InstanceManagerError {
    #[error("app reference not found")]
    AppNotFoundError,

}


pub(crate) struct InstanceManager {
    app: Weak<RwLock<App>>,
    instances_pool: RwLock<BTreeSet<Instance>>,
    instances_by_id: RwLock<BTreeMap<u128, Instance>>,
}

#[derive(Type, Deserialize)]
pub struct CreateInstanceDto {
    name: String,
    minecraft_version: String,
}

impl InstanceManager {
    pub fn make_for_app(app: &Arc<RwLock<App>>) -> InstanceManager {
        InstanceManager {
            app: Arc::downgrade(app),
            instances_pool: Default::default(),
            instances_by_id: Default::default(),
        }
    }

    pub async fn get_all_instances(&self) -> BTreeSet<Instance> {
        self.instances_pool.read().await.iter().map(|i| i.clone()).collect()
    }

    pub async fn get_instance_by_id(&self, id: u128) -> Option<Instance> {
        self.instances_by_id.read().await.get(&id).map(Clone::clone)
    }

    pub async fn delete_instance_by_id(&self, id: String, remove_from_fs: bool) -> Option<Instance> {
        self.instances_pool.read().await.iter().map(|i| i.clone()).collect()
    }

    pub async fn start_instance_by_id(&self, id: String) -> Option<Instance> {
        self.instances_pool.read().await.iter().map(|i| i.clone()).collect()
    }

    pub async fn stop_instance_by_id(&self, id: String) -> Option<Instance> {
        self.instances_pool.read().await.iter().map(|i| i.clone()).collect()
    }

    pub async fn add_instance(&self, dto: CreateInstanceDto) -> Result<Instance, InstanceManagerError> {
        Ok(Instance {
            name: "".to_string(),
            id: 0,
            played_time: Default::default(),
            last_played: SystemTime::now(),
            minecraft_package: MinecraftPackage {
                version: "".to_string(),
                mods: Default::default(),
                description: "".to_string(),
                modloaders: Default::default(),
                status: carbon_minecraft::minecraft_package::MinecraftPackageStatus::NotPersisted,
            },
            persistence_status: InstanceStatus::NotPersisted,
            notes: "".to_string(),
        })
    }
}
