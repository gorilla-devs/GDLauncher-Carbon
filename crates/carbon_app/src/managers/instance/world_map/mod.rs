pub(crate) mod budget;
mod chunk;
mod colors;
pub(crate) mod facts;
pub(crate) mod legacy;
mod paint;
mod region;
pub(crate) mod service;
pub(crate) mod store;
#[cfg(test)]
mod test_nbt;

use std::sync::Arc;

use tokio::sync::{OwnedRwLockWriteGuard, RwLock};
use tracing::debug;

use super::{InstanceId, InstanceManager};
use crate::managers::ManagerRef;
use store::{STORE_LIMIT, Store};

impl ManagerRef<'_, InstanceManager> {
    /// The gate around every file the map opens in the instance's worlds: the map holds it
    /// shared only while it reads files, an operation that moves or removes a folder of them
    /// holds it alone. On Windows a folder with an open file under it can be neither renamed
    /// nor removed.
    pub(crate) fn world_map_files(self, id: InstanceId) -> Arc<RwLock<()>> {
        self.world_map_files.entry(id).or_default().clone()
    }

    /// Closes the instance's worlds to the map until the guard is dropped: waits for the reads
    /// in flight, which are short, not for whole paintings. Never take it while waiting on
    /// anything the map holds; the map never waits on it while holding anything.
    pub(crate) async fn close_world_map_files(self, id: InstanceId) -> OwnedRwLockWriteGuard<()> {
        self.world_map_files(id).write_owned().await
    }

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
        let task = tokio::spawn(async move {
            if let Err(error) = forgetting.await {
                debug!(
                    ?error,
                    "Instance {id}: forgetting the map of a world stopped"
                );
            }
        });
        self.keep_for_tests(task);
    }

    /// Drops, in the background, every map kept for the deleted instance `id`: its delete is
    /// never held up or failed by them.
    pub(super) fn forget_instance_maps(self, id: InstanceId) {
        let store = self.world_map_store();
        let forgetting = tokio::task::spawn_blocking(move || store.forget_instance(id));
        let task = tokio::spawn(async move {
            if let Err(error) = forgetting.await {
                debug!(?error, "Instance {id}: forgetting its maps stopped");
            }
        });
        self.keep_for_tests(task);
    }

    /// Drops, in the background, the kept maps of the instance's worlds other than
    /// `existing`, the folder names of every world it has.
    pub(super) fn forget_maps_of_missing_worlds(self, id: InstanceId, existing: Vec<String>) {
        let store = self.world_map_store();
        let forgetting =
            tokio::task::spawn_blocking(move || store.forget_missing_worlds(id, &existing));
        let task = tokio::spawn(async move {
            if let Err(error) = forgetting.await {
                debug!(
                    ?error,
                    "Instance {id}: forgetting the maps of missing worlds stopped"
                );
            }
        });
        self.keep_for_tests(task);
    }

    /// Production lets a background forgetting run on its own; tests keep it to await.
    fn keep_for_tests(self, task: tokio::task::JoinHandle<()>) {
        #[cfg(test)]
        self.forgetting
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(task);
        #[cfg(not(test))]
        drop(task);
    }

    /// Waits until every forgetting started so far has finished.
    #[cfg(test)]
    pub(crate) async fn forgetting_done(self) {
        let tasks = std::mem::take(
            &mut *self
                .forgetting
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
        for task in tasks {
            task.await.unwrap();
        }
    }
}
