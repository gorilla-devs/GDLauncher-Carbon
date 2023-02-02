use carbon_minecraft::instance::Instance;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Default)]
pub(in crate::app::instance) struct InstanceStore {
    instances_pool: RwLock<BTreeSet<Instance>>,
    instances_by_id: RwLock<BTreeMap<u128, Instance>>,
}

impl InstanceStore {
    pub async fn get_next_available_id(&self) -> u128 {
        Uuid::new_v4().as_u128()
    }

    pub async fn get_all_instances(&self) -> BTreeSet<Instance> {
        self.instances_pool
            .read()
            .await
            .iter()
            .map(|i| i.clone())
            .collect()
    }

    pub async fn get_instance_by_id(&self, id: u128) -> Option<Instance> {
        self.instances_by_id.read().await.get(&id).map(Clone::clone)
    }

    pub async fn delete_instance_by_id(&self, id: &u128) -> Option<Instance> {
        let deindexed_instance = self.instances_by_id.write().await.remove(id);
        match &deindexed_instance {
            Some(instance) => self.instances_pool.write().await.remove(instance),
            None => false,
        };
        deindexed_instance
    }

    pub async fn save_instance(&self, instance: Instance) -> Instance {
        let instance_id = &instance.id;
        if let true = self.instances_by_id.read().await.contains_key(instance_id) {
            self.delete_instance_by_id(instance_id);
        };
        self.instances_by_id
            .write()
            .await
            .insert(instance_id.clone(), instance.clone());
        self.instances_pool.write().await.insert(instance.clone());
        return instance;
    }
}
