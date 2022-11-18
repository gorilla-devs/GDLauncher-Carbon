use std::sync::Arc;

use anyhow::Result;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use self::{accounts::Accounts, settings::GDLSettings};

mod accounts;
mod napi;
mod settings;

lazy_static! {
    pub static ref GLOBAL_STORE: Arc<Mutex<Option<GlobalStore>>> = Arc::new(Mutex::new(None));
}

const GLOBAL_STORE_FILE: &str = "./data/_global_store";

async fn init_global_storage() -> Result<()> {
    tokio::fs::create_dir_all("./data").await?;

    let mut store = GLOBAL_STORE.lock().await;
    match std::path::Path::new(GLOBAL_STORE_FILE).exists() {
        true => {
            let file = std::fs::File::open(GLOBAL_STORE_FILE)?;
            let reader = std::io::BufReader::new(file);
            // deserialize bincode
            let store_new: GlobalStore = bincode::deserialize_from(reader)?;
            *store = Some(store_new);
        }
        false => {
            let global_store = GlobalStore::new();
            let serialized_data = bincode::serialize(&global_store)?;
            std::fs::write(GLOBAL_STORE_FILE, serialized_data)?;
            *store = Some(global_store);
        }
    }

    Ok(())
}

async fn store_save_loop() -> () {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let store = GLOBAL_STORE.lock().await;
        if let Some(store) = store.as_ref() {
            let serialized_data = bincode::serialize(store).unwrap();
            match std::fs::write(GLOBAL_STORE_FILE, serialized_data) {
                Ok(_) => {}
                Err(e) => {
                    // TODO: Log error
                    // log::error!("Failed to save global store: {}", e);
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct GlobalStore {
    pub settings: settings::GDLSettings,
    pub accounts: Accounts,
}

impl GlobalStore {
    pub fn new() -> Self {
        Self {
            settings: GDLSettings::new(),
            accounts: Accounts::new(),
        }
    }
}

mod tests {
    #[tokio::test]
    async fn test_init_global_storage() {
        // Cleanup if the db is already there
        if std::path::Path::new(super::GLOBAL_STORE_FILE).exists() {
            std::fs::remove_file(super::GLOBAL_STORE_FILE).unwrap();
        }

        let store_lock = super::GLOBAL_STORE.lock().await;
        assert!(store_lock.is_none() == true);
        drop(store_lock);

        let init = super::init_global_storage().await;
        assert!(init.is_ok() == true);

        // Try to modify value, write to disk and see if next load it's loaded correctly
        let mut store_lock = super::GLOBAL_STORE.lock().await;
        assert!(store_lock.is_some() == true);
        assert!(store_lock.as_ref().unwrap().settings.discord_rpc.enabled == true);
        store_lock.as_mut().unwrap().settings.discord_rpc.enabled = false;

        let serialized_data = bincode::serialize(&store_lock.as_ref().unwrap()).unwrap();
        std::fs::write(super::GLOBAL_STORE_FILE, serialized_data).unwrap();

        drop(store_lock);

        assert!(std::fs::File::open(super::GLOBAL_STORE_FILE).is_ok() == true);

        let mut store_lock = super::GLOBAL_STORE.lock().await;
        *store_lock = None;
        drop(store_lock);

        // Test to initialize the database when it's already there
        let init = super::init_global_storage().await;
        assert!(init.is_ok() == true);

        let store_lock = super::GLOBAL_STORE.lock().await;
        assert!(store_lock.is_some() == true);
        assert!(store_lock.as_ref().unwrap().settings.discord_rpc.enabled == false);

        drop(store_lock);

        // Final cleanup
        std::fs::remove_file(super::GLOBAL_STORE_FILE).unwrap();
    }
}
