mod delete;
mod instance_configuration;
mod play;
pub mod representation;
mod scan;
mod store;
mod tests;
mod write;

use crate::app::instance::delete::InstanceDeleteError;
use crate::app::instance::representation::CreateInstanceDto;
use crate::app::instance::scan::InstanceScanError;
use crate::app::instance::store::InstanceStore;
use crate::app::instance::write::InstanceWriteError;
use crate::app::App;
use carbon_domain::instance::{Instance, InstanceStatus};
use carbon_domain::minecraft_package::{MinecraftPackage, MinecraftPackageStatus};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Arc, Weak};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Error, Debug)]
pub enum InstanceManagerError {
    #[error("app reference not found")]
    AppNotFoundError,
    #[error("instance with id {0} not found")]
    InstanceWithGivenIdNotFound(u128),
    #[error("unable to delete instance : {0}")]
    InstanceDeleteError(#[from] InstanceDeleteError),
    #[error("unable to write instance : {0} ")]
    InstanceWriteError(#[from] InstanceWriteError),
    #[error("unable to scan directory for instances : {0} ")]
    InstanceScanError(#[from] InstanceScanError),
}

pub(crate) struct InstanceManager {
    app: Weak<RwLock<App>>,
    instance_store: RwLock<InstanceStore>,
}

impl InstanceManager {
    pub fn make_for_app(app: &Arc<RwLock<App>>) -> InstanceManager {
        InstanceManager {
            app: Arc::downgrade(app),
            instance_store: Default::default(),
        }
    }

    pub async fn get_all_instances(&self) -> BTreeSet<Instance> {
        self.instance_store.read().await.get_all_instances().await
    }

    pub async fn get_instance_by_id(&self, id: u128) -> Result<Instance, InstanceManagerError> {
        self.instance_store
            .read()
            .await
            .get_instance_by_id(id)
            .await
            .ok_or(InstanceManagerError::InstanceWithGivenIdNotFound(id))
    }

    pub async fn add_instance(
        &self,
        dto: CreateInstanceDto,
    ) -> Result<Instance, InstanceManagerError> {
        let instance = Instance {
            name: dto.name,
            id: self
                .instance_store
                .read()
                .await
                .get_next_available_id()
                .await,
            played_time: Duration::default(),
            last_played: None,
            minecraft_package: MinecraftPackage {
                version: dto.minecraft_version,
                mods: Default::default(),
                description: "".to_string(),
                mod_loaders: Default::default(),
                status: MinecraftPackageStatus::NotPersisted,
            },
            persistence_status: InstanceStatus::NotPersisted,
            notes: "".to_string(),
        };
        let instance = self
            .instance_store
            .read()
            .await
            .save_instance(instance)
            .await;
        //todo: handle path collision between instances
        let instance = if let Some(path_to_write_in) = dto.path_to_save_at {
            self.write_at(instance, &path_to_write_in).await?
        } else {
            instance
        };
        Ok(self
            .instance_store
            .read()
            .await
            .save_instance(instance)
            .await)
    }

    pub async fn delete_instance_by_id(
        &self,
        id: u128,
        remove_from_fs: bool,
    ) -> Result<Instance, InstanceManagerError> {
        let deleted_instance = self
            .instance_store
            .read()
            .await
            .delete_instance_by_id(&id)
            .await
            .ok_or(InstanceManagerError::InstanceWithGivenIdNotFound(id))?;
        Ok(self
            .delete_from_fs(deleted_instance, !remove_from_fs)
            .await?)
    }

    pub async fn read_instances_from_directory(
        &self,
        path: &impl AsRef<Path>,
    ) -> Result<Vec<Instance>, InstanceManagerError> {
        let found_instances = self
            .scan_for_instances(path.as_ref())
            .await?
            .into_iter()
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        for instance in found_instances.clone() {
            self.instance_store
                .read()
                .await
                .save_instance(instance)
                .await;
        }
        Ok(found_instances)
    }

    pub async fn start_instance_by_id(&self, id: String) -> Result<Instance, InstanceManagerError> {
        unimplemented!()
    }

    pub async fn stop_instance_by_id(&self, id: u128) -> Result<Instance, InstanceManagerError> {
        unimplemented!()
    }
}
