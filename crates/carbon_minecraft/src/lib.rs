use std::{collections::HashMap, path::PathBuf, sync::Arc};
use std::any::Any;

use anyhow::Result;
use lazy_static::lazy_static;
use tokio::sync::RwLock;
use tracing::trace;
use uuid::Uuid;

mod mc;

mod instance;
mod app;
mod minecraft_package;
mod minecraft_mod;
mod modloader;
mod package_file;

type TaskProgressListener = impl Fn();


macro_rules! try_path_fmt{
    () => {

    };
}


trait UUIDIndexed { //
    fn get_uuid(&self) -> uuid;
}

trait Validable {
    fn validate() -> bool;
}

/*
//
// pub struct Instances {
//     inner: Arc<RwLock<HashMap<String, instance::Instance>>>,
// }
//
// impl Instances {
//     pub async fn scan_for_instances(&mut self, base_path: PathBuf) -> Result<&Instances> {
//         let instances_path = base_path.join("instances");
//
//         if !instances_path.exists() {
//             tokio::fs::create_dir(&instances_path).await.unwrap();
//         }
//
//         let mut instances = self.inner.write().await;
//
//         let mut dirs = tokio::fs::read_dir(instances_path).await?;
//
//         while let Some(entry) = dirs.next_entry().await? {
//             let path = entry.path();
//             if !path.is_dir() {
//                 continue;
//             }
//
//             let Some(name) = path.file_name().map(|name| name.to_string_lossy().to_string()) else {
//                 trace!("Found instance with invalid name: {:?}", path);
//                 continue;
//             };
//
//             let instance = instance::Instance::new(name.clone());
//             instances.insert(name, instance); // impl convenience method for this
//         }
//
//         Ok(&INSTANCES)
//     }
// }
//
// lazy_static! {
//     static ref INSTANCES: Instances = Instances {
//         inner: Arc::new(RwLock::new(HashMap::new()))
//     };
// }
*/