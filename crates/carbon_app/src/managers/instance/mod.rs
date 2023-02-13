mod delete;
mod instance_configuration;
mod scan;
mod store;
mod tests;
mod write;

use crate::managers::instance::delete::InstanceDeleteError;
use crate::managers::instance::scan::InstanceScanError;
use crate::managers::instance::store::{InstanceStore, InstanceStoreError};
use crate::managers::instance::write::InstanceWriteError;
use crate::managers::representation::CreateInstanceDto;

use crate::api::keys::mc::{
    DELETE_INSTANCE, OPEN_INSTANCE_FOLDER_PATH, SAVE_NEW_INSTANCE, UPDATE_INSTANCE,
};
use crate::managers::instance::InstanceManagerError::{
    InconvertibleValueToPath, InstanceMoveError, InstanceStatusPatchingNotAllowed,
    InstanceWithPathAlreadyExistError,
};
use crate::managers::AppRef;
use crate::try_path_fmt::try_path_fmt;
use carbon_domain::instance::{Instance, InstanceStatus};
use futures::future::err;
use log::{trace, warn};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::default::Default;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InstanceManagerError {
    #[error("instance with id {0} not found")]
    InstanceWithGivenIdNotFound(u128),
    #[error("instance store system error raised : {0}")]
    InstanceStoreSystemError(#[from] InstanceStoreError),
    #[error("unable to delete instance : {0}")]
    InstanceDeleteError(#[from] InstanceDeleteError),
    #[error("unable to write instance : {0} ")]
    InstanceWriteError(#[from] InstanceWriteError),
    #[error("error raised in instance patch process : {0} ")]
    InstancePatchError(#[from] serde_json::error::Error),
    #[error("unable to change instance status to {0:?} via patch")]
    InstanceStatusPatchingNotAllowed(InstanceStatus),
    #[error("error raised in instance copy process, cause : {0} ")]
    InstanceCopyError(String),
    #[error("error raised in instance move process, cause : {0} ")]
    InstanceMoveError(String),
    #[error("error raised in instance saving process, cause : {0} ")]
    InstanceSaveError(String),
    #[error("value {0} is not convertable in fs path")]
    InconvertibleValueToPath(Value),
    #[error("error raised in instance patch process : {0} ")]
    InstanceWithPathAlreadyExistError(PathBuf),
    #[error("io error raised : {0} ")]
    IoError(#[from] std::io::Error),
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
        let available_id = self.instance_store.get_next_available_id().await;
        let instance = dto.clone().into_instance_with_id(available_id).await;
        let instance = self.instance_store.save_instance(instance).await?;
        let instance = match dto.path_to_save_at {
            Some(path_to_save) => self.write_at(instance.clone(), &path_to_save).await?,
            None => instance,
        };
        let saved_instance = self.instance_store.save_instance(instance).await?;
        self.app.upgrade().invalidate(
            SAVE_NEW_INSTANCE,
            Some(serde_json::to_value(saved_instance.clone())?),
        );
        Ok(saved_instance)
    }

    pub async fn delete_instance_by_id(
        &self,
        id: u128,
        remove_from_fs: bool,
    ) -> Result<Instance, InstanceManagerError> {
        let instance_to_delete = self.get_instance_by_id(id).await?;

        let erase_instance_result = self
            .delete_from_fs(instance_to_delete.clone(), !remove_from_fs)
            .await;
        let in_memory_instance = match erase_instance_result {
            Ok(instance) => {
                trace!("removed persisted instance from fs {instance_to_delete:?}");
                self.instance_store.save_instance(instance).await?
            }
            Err(InstanceDeleteError::InstanceNotPersisted) => {
                trace!("unable to delete non persisted entity with id {id}");
                instance_to_delete.clone()
            }
            error => error?,
        };

        let deleted_instance = self
            .instance_store
            .delete_instance_by_id(&id)
            .await
            .ok_or(InstanceManagerError::InstanceWithGivenIdNotFound(id))?;
        self.app.upgrade().invalidate(
            DELETE_INSTANCE,
            Some(serde_json::to_value(instance_to_delete.clone())?),
        );
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
            self.instance_store.save_instance(instance).await?;
        }
        Ok(found_instances)
    }

    pub async fn patch_instance_by_id(
        &self,
        id: u128,
        new_values: BTreeMap<String, Value>,
    ) -> Result<Instance, InstanceManagerError> {
        let mut new_values_cloned = new_values.clone();
        trace!("trying to patch instance with id {id} with new values {new_values:#?}");
        let target_instance = self.get_instance_by_id(id).await?;
        let into_properties = serde_json::to_value(target_instance)?;
        let forbidden_property_keys = vec![
            "id",
            "minecraft_package",
            "persistence_status",
            "last_played",
            "played_time",
        ];
        for forbidden_property_key in forbidden_property_keys {
            trace!("removing property named {forbidden_property_key} from patch plan cause is forbidden to change with patch");
            new_values_cloned.remove(forbidden_property_key);
        }
        let mut properties_to_patch = into_properties.as_object().map_or(Map::new(), Clone::clone);
        for (property_key, property_value) in new_values_cloned {
            match properties_to_patch.insert(property_key.clone(), property_value.clone()){
                Some(old_value) => trace!("changed property with name  {property_key}  old : {old_value} new : {property_value} for instance with id {id}"),
                None => trace!("set initially value for property with name  {property_key} value : {property_value} for instance with id {id}")
            }
        }
        let patched_instance: Instance =
            serde_json::from_value(Value::Object(properties_to_patch.clone()))?;
        let saved_instance = match new_values.get("persistence_status") {
            Some(new_persistence_status) => {
                self.change_instance_persistence_status(
                    patched_instance,
                    new_persistence_status.clone(),
                )
                .await?
            }
            _ => self.instance_store.save_instance(patched_instance).await?,
        };
        self.app.upgrade().invalidate(
            UPDATE_INSTANCE,
            Some(serde_json::to_value(saved_instance.clone())?),
        );
        Ok(saved_instance)
    }

    async fn change_instance_persistence_status(
        &self,
        instance: Instance,
        new_persistence_value: Value,
    ) -> Result<Instance, InstanceManagerError> {
        let new_persistence_status: InstanceStatus = serde_json::from_value(new_persistence_value)?;
        let written_instance = match new_persistence_status {
            InstanceStatus::Ready(new_path) => self.write_at(instance, &new_path).await?,
            new_status => Err(InstanceStatusPatchingNotAllowed(new_status))?,
        };
        let saved_instance = self.instance_store.save_instance(written_instance).await?;
        Ok(saved_instance)
    }

    pub async fn start_instance_by_id(
        &self,
        _id: String,
    ) -> Result<Instance, InstanceManagerError> {
        unimplemented!()
    }

    pub async fn stop_instance_by_id(&self, _id: u128) -> Result<Instance, InstanceManagerError> {
        unimplemented!()
    }
}
