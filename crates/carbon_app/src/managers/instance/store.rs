use crate::try_path_fmt::try_path_fmt;
use carbon_domain::instance::{Instance, InstanceStatus};
use log::trace;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Default)]
pub(in crate::managers::instance) struct InstanceStore {
    instances_pool: RwLock<BTreeSet<Instance>>,
    instances_by_id: RwLock<BTreeMap<u128, Instance>>,
    instances_by_path: RwLock<BTreeMap<PathBuf, Instance>>,
}

#[derive(Error, Debug)]
pub enum InstanceStoreError {
    #[error("found instance with same path, path collision not supported! candidate : {candidate:?} , already_indexed : {already_indexed:?}")]
    InstanceBreakPathIntegrityRule {
        candidate: Instance,
        already_indexed: Instance,
    },
}

impl InstanceStore {
    pub async fn get_next_available_id(&self) -> u128 {
        Uuid::new_v4().as_u128()
    }

    pub async fn get_all_instances(&self) -> BTreeSet<Instance> {
        trace!("retrieving all instances from instances store");
        self.instances_pool.read().await.iter().cloned().collect()
    }

    pub async fn get_instance_by_id(&self, id: u128) -> Option<Instance> {
        trace!("trying to retrieve instance with id {id} from instances store");
        self.instances_by_id.read().await.get(&id).map(Clone::clone)
    }

    pub async fn delete_instance_by_id(&self, id: &u128) -> Option<Instance> {
        trace!("trying to remove instance with id {id} from instances store");
        let deindexed_instance = self.instances_by_id.write().await.remove(id);
        match &deindexed_instance {
            Some(instance) => self.instances_pool.write().await.remove(instance),
            None => false,
        };
        trace!("instance with id {id} correctly removed from instances store");
        deindexed_instance
    }

    pub async fn save_instance(&self, instance: Instance) -> Result<Instance, InstanceStoreError> {
        trace!(
            "trying to save instance {} into instances store",
            serde_json::to_string(&instance).unwrap_or("<<unrepresentable instance!>>".to_string())
        );
        let instance_id = &instance.id;
        match instance.persistence_status {
            InstanceStatus::Installing(ref path) | InstanceStatus::Ready(ref path) => {
                match self.instances_by_path.read().await.get(path) {
                    Some(found_instance) => {
                        trace!("found instance with same path, path collision not supported!");
                        return Err(InstanceStoreError::InstanceBreakPathIntegrityRule {
                            candidate: instance.clone(),
                            already_indexed: found_instance.clone(),
                        });
                    }
                    _ => (),
                };
                self.instances_by_path
                    .write()
                    .await
                    .insert(path.to_path_buf(), instance.clone());
                trace!(
                    "added instance with id {instance_id} to path index with path {}",
                    try_path_fmt!(path)
                )
            }
            _ => trace!(
                "instance with id {instance_id} not saved in path index since is not persisted"
            ),
        }
        if let true = self.instances_by_id.read().await.contains_key(instance_id) {
            trace!("found instance with id {instance_id}, removing in order to replace with the new one");
            self.delete_instance_by_id(instance_id).await;
        };
        self.instances_by_id
            .write()
            .await
            .insert(instance_id.clone(), instance.clone());
        match self.instances_pool.write().await.insert(instance.clone()) {
            true => trace!("updated instance with id {instance_id}"),
            false => trace!("saved new instance with id {instance_id}"),
        };
        return Ok(instance);
    }
}
