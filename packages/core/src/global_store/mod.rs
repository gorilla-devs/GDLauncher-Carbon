use anyhow::{Context, Result};
use lazy_static::lazy_static;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use tokio::{spawn, sync::Mutex};

use self::{accounts::Accounts, settings::GDLSettings};

mod accounts;
mod settings;

lazy_static! {
    pub static ref GLOBAL_STORE: Arc<Mutex<Option<GlobalStore>>> = Arc::new(Mutex::new(None));
    pub static ref DATA_DIR: Arc<Mutex<PathBuf>> = Arc::new(Mutex::new(
        directories::ProjectDirs::from("com", "GorillaDevs", "GDL")
            .expect("Failed to get data directory")
            .data_dir()
            .to_path_buf()
    ));
    pub static ref STORE_PATH: Arc<Mutex<PathBuf>> = Arc::new(Mutex::new(
        directories::ProjectDirs::from("com", "GorillaDevs", "GDL")
            .expect("Failed to get data directory for STORE_PATH")
            .data_dir()
            .to_path_buf()
            .join("_global_store")
    ));
}

#[napi]
pub async fn init_global_storage() -> napi::Result<()> {
    init_inner_global_storage()
        .await
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, format!("{:?}", e)))?;
    store_save_loop();
    Ok(())
}

async fn init_inner_global_storage() -> Result<()> {
    let store_path = STORE_PATH.lock().await;
    let data_dir = DATA_DIR.lock().await;
    tokio::fs::create_dir_all(data_dir.to_string_lossy().to_string())
        .await
        .context("Can't create data dir")?;
    drop(data_dir);

    let mut store = GLOBAL_STORE.lock().await;
    match store_path.exists() {
        true => {
            let file = std::fs::File::open(store_path.to_string_lossy().to_string())?;
            let reader = std::io::BufReader::new(file);
            // deserialize bincode
            let store_new: GlobalStore = bincode::deserialize_from(reader)?;
            *store = Some(store_new);
        }
        false => {
            let global_store = GlobalStore::new();
            let serialized_data = bincode::serialize(&global_store)?;
            std::fs::write(store_path.to_string_lossy().to_string(), serialized_data)?;
            *store = Some(global_store);
        }
    }

    Ok(())
}

fn store_save_loop() -> () {
    spawn(async move {
        let store_path = STORE_PATH.lock().await;
        let store_path_clone = store_path.clone();
        drop(store_path);
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let store = GLOBAL_STORE.lock().await;
            if let Some(store) = store.as_ref() {
                let serialized_data = bincode::serialize(store).unwrap();

                match std::fs::write(&store_path_clone, serialized_data) {
                    Ok(_) => {}
                    Err(error) => {
                        eprintln!("Failed to save global store: {}", error);
                    }
                }
            }
        }
    });
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
        let store_path = super::STORE_PATH.lock().await;
        // Cleanup if the db is already there
        if store_path.exists() {
            std::fs::remove_file(store_path.to_string_lossy().to_string()).unwrap();
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
        std::fs::write(store_path.to_string_lossy().to_string(), serialized_data).unwrap();

        drop(store_lock);

        assert!(std::fs::File::open(store_path.to_string_lossy().to_string()).is_ok() == true);

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
        std::fs::remove_file(store_path.to_string_lossy().to_string()).unwrap();
    }
}
