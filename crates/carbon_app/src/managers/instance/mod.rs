mod delete;
mod instance_configuration;
mod play;
pub mod representation;
mod scan;
mod store;
mod tests;
mod write;

use crate::managers::instance::delete::InstanceDeleteError;
use crate::managers::instance::representation::CreateInstanceDto;
use crate::managers::instance::scan::InstanceScanError;
use crate::managers::instance::store::InstanceStore;
use crate::managers::instance::write::InstanceWriteError;

use crate::managers::AppRef;
use carbon_domain::instance::{Instance, InstanceStatus};
use carbon_domain::minecraft_package::{MinecraftPackage, MinecraftPackageStatus};
use log::trace;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::default::Default;
use std::path::Path;
use std::time::Duration;
use thiserror::Error;

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
    #[error("error raised in instance patch process : {0} ")]
    InstancePatchError(#[from] serde_json::error::Error),
    #[error("unable to scan directory for instances : {0} ")]
    InstanceScanError(#[from] InstanceScanError),
}

pub(crate) struct InstanceManager {
    app: AppRef,
    instance_store: InstanceStore,
}

impl InstanceManager {
    pub fn new() -> Self {
        Self {
            app: AppRef::uninit(),
            instance_store: Default::default(),
        }
    }

    pub fn get_appref(&self) -> &AppRef {
        &self.app
    }

    pub async fn get_all_instances(&self) -> BTreeSet<Instance> {
        self.instance_store.get_all_instances().await
    }

    pub async fn get_instance_by_id(&self, id: u128) -> Result<Instance, InstanceManagerError> {
        self.instance_store
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
            id: self.instance_store.get_next_available_id().await,
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
        let instance = self.instance_store.save_instance(instance).await;
        //todo: handle path collision between instances
        let instance = if let Some(path_to_write_in) = dto.path_to_save_at {
            self.write_at(instance, &path_to_write_in).await?
        } else {
            instance
        };
        Ok(self.instance_store.save_instance(instance).await)
    }

    pub async fn delete_instance_by_id(
        &self,
        id: u128,
        remove_from_fs: bool,
    ) -> Result<Instance, InstanceManagerError> {
        let deleted_instance = self
            .instance_store
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
            self.instance_store.save_instance(instance).await;
        }
        Ok(found_instances)
    }

    pub async fn patch_instance_by_id(
        &self,
        id: u128,
        new_values: BTreeMap<String, Value>,
    ) -> Result<Instance, InstanceManagerError> {
        let mut new_values = new_values;
        trace!("trying to patch instance with id {id} with new values {new_values:#?}");
        let target_instance = self.get_instance_by_id(id).await?;
        let into_properties = serde_json::to_value(target_instance)?;
        let forbidden_property_keys = vec!["id"];
        for forbidden_property_key in forbidden_property_keys {
            trace!("removing property named {forbidden_property_key} from patch plan cause is forbidden to change with patch");
            new_values.remove(forbidden_property_key);
        }
        let mut properties_to_patch = into_properties.as_object().map_or(Map::new(), Clone::clone);
        for (property_key, property_value) in new_values {
            match properties_to_patch.insert(property_key.clone(), property_value.clone()){
                Some(old_value) => trace!("changed property with name  {property_key}  old : {old_value} new : {property_value} for instance with id {id}"),
                None => trace!("set initially value for property with name  {property_key} value : {property_value} for instance with id {id}")
            }
        }
        let patched_instance: Instance = serde_json::from_value(properties_to_patch.into())?;
        let new_instance = self.instance_store.save_instance(patched_instance).await;
        Ok(new_instance)
    }

    pub async fn start_instance_by_id(&self, id: String) -> Result<Instance, InstanceManagerError> {
        unimplemented!()
    }

    pub async fn stop_instance_by_id(&self, id: u128) -> Result<Instance, InstanceManagerError> {
        unimplemented!()
    }
}
