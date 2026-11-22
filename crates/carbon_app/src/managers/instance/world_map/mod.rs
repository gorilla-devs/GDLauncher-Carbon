mod chunk;
mod colors;
pub(crate) mod facts;
mod legacy;
mod paint;
mod region;
pub(crate) mod store;
#[cfg(test)]
mod test_nbt;

use std::sync::Arc;

use tracing::debug;

use super::{InstanceId, InstanceManager};
use crate::managers::ManagerRef;
use store::{STORE_LIMIT, Store};

impl ManagerRef<'_, InstanceManager> {
    /// The launcher's one store of finished map pieces.
    pub(crate) fn world_map_store(self) -> Arc<Store> {
        self.world_map_store
            .get_or_init(|| {
                let root = self.app.settings_manager().runtime_path.get_world_maps();
                Arc::new(Store::new(root, STORE_LIMIT))
            })
            .clone()
    }

    /// Drops, in the background, the map kept for the world folder `world`, which no world
    /// has any more: a delete or rename is never held up or failed by its map.
    pub(super) fn forget_world_map(self, id: InstanceId, world: String) {
        let store = self.world_map_store();
        let forgetting = tokio::task::spawn_blocking(move || store.forget_world(id, &world));
        tokio::spawn(async move {
            if let Err(error) = forgetting.await {
                debug!(
                    ?error,
                    "Instance {id}: forgetting the map of a world stopped"
                );
            }
        });
    }

    /// Drops, in the background, the kept maps of the instance's worlds other than
    /// `existing`, the folder names of every world it has.
    pub(super) fn forget_maps_of_missing_worlds(self, id: InstanceId, existing: Vec<String>) {
        let store = self.world_map_store();
        let forgetting =
            tokio::task::spawn_blocking(move || store.forget_missing_worlds(id, &existing));
        tokio::spawn(async move {
            if let Err(error) = forgetting.await {
                debug!(
                    ?error,
                    "Instance {id}: forgetting the maps of missing worlds stopped"
                );
            }
        });
    }
}
