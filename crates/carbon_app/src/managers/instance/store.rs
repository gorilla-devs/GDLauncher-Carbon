use crate::try_path_fmt::try_path_fmt;
use carbon_domain::instance::InstanceStatus::*;
use carbon_domain::instance::{Instance, InstanceStatus};
use log::{error, trace};
use std::collections::{BTreeMap, BTreeSet};
use std::num::ParseIntError;
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
    #[error("unable to convert string {0} to u128")]
    UuidParseError(#[from] ParseIntError),
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
        match self.instances_by_id.read().await.get(&id).map(Clone::clone) {
            Some(found_instance) => {
                trace!("instance with id {id} correctly retrieved from store, instance is {found_instance:?}");
                Some(found_instance)
            }
            None => {
                trace!("instance with id {id} not exist in store");
                None
            }
        }
    }

    pub async fn delete_instance_by_id(&self, id: &u128) -> Option<Instance> {
        trace!("trying to remove instance with id {id} from instances store");
        let deindexed_instance = self.instances_by_id.write().await.remove(id); // deadlock
        match &deindexed_instance {
            Some(instance) => self.instances_pool.write().await.remove(instance),
            None => false,
        };
        match &deindexed_instance {
            Some(Instance {
                status: Ready(path),
                ..
            })
            | Some(Instance {
                status: Installing(path),
                ..
            }) => {
                self.instances_by_path.write().await.remove(path);
            }
            _ => (),
        }
        trace!("instance with id {id} correctly removed from instances store");
        deindexed_instance
    }

    pub async fn save_instance(&self, instance: Instance) -> Result<Instance, InstanceStoreError> {
        trace!(
            "trying to save instance {} into instances store",
            serde_json::to_string(&instance).unwrap_or("<<unrepresentable instance!>>".to_string())
        );
        let instance_id = &instance.uuid.parse()?;
        match (
            self.instances_by_id.read().await.get(instance_id).cloned(),
            instance.status.clone(),
        ) {
            (
                Some(Instance {
                    status: Ready(ref old_path) | Installing(ref old_path),
                    ..
                }),
                Ready(ref new_path) | Installing(ref new_path),
            ) => {
                trace!("found instance with same id, going to merge them");
                self.instances_by_path.write().await.remove(old_path);
                self.instances_by_path
                    .write()
                    .await
                    .insert(new_path.to_path_buf(), instance.clone());
                trace!(
                    "change path in index for instance with id {instance_id} from {} to {}",
                    try_path_fmt!(old_path),
                    try_path_fmt!(new_path)
                )
            }
            (
                Some(Instance {
                    status: Ready(ref old_path) | Installing(ref old_path),
                    ..
                }),
                NotPersisted,
            ) => {
                self.instances_by_path.write().await.remove(old_path);
                trace!("removed path {} from index since instance with id {instance_id} is not persisted anymore", try_path_fmt!(old_path))
            }
            (None, Ready(ref new_path) | Installing(ref new_path))
                if self.instances_by_path.read().await.contains_key(new_path) =>
            {
                let found_instance = self.instances_by_path.read().await.get(new_path).cloned();
                error!("found instance with same path, path collision not supported!");
                Err(InstanceStoreError::InstanceBreakPathIntegrityRule {
                    candidate: instance.clone(),
                    already_indexed: found_instance
                        .expect("expected instance in instance by path index !!!"),
                })?
            }
            (_, _) => (),
        }

        match self
            .instances_by_id
            .write()
            .await
            .insert(*instance_id, instance.clone())
        {
            Some(old_instance) => {
                trace!("replacing old instance {old_instance:?} with new {instance:?}");
                self.instances_pool.write().await.remove(&instance);
            }
            None => trace!("adding brand new instance {instance:?}"),
        }
        self.instances_pool.write().await.insert(instance.clone());
        trace!("instance correctly added to store : {instance:?}");
        Ok(instance)
    }
}
