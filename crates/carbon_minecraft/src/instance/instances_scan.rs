use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::instance::Instance;

type InstanceScanResult<T> = Result<T, InstanceScanError>;


// todo : this is a factory method and the method name implies that the operation is only a scan(read), split concerns
pub async fn scan_for_instances(path_to_search_in: impl AsRef<Path>, search_all_folders_in_depth: bool) -> InstanceScanResult<Vec<Instance>> {

    // async but mind that the scan must be executed one level per once in order to avoid possible false positives(could drive to a nasty bug for future change)
    // test whether you can recurse infinitely for a link to a self contained folder ( )
    let path_to_search_in = path_to_search_in.as_ref();



    todo!()

    /*std::fs::read_dir()

    let instances_path = base_path.join("instances");

    if !instances_path.exists() {
        tokio::fs::create_dir(&instances_path).await.unwrap(); // <-- this won't be a great deal: heavy coupling and read this -> https://users.rust-lang.org/t/file-reading-async-sync-performance-differences-hyper-tokio/34696/5
    }

    let mut instances = self.inner.write().await;

    let mut dirs = tokio::fs::read_dir(instances_path).await?;

    while let Some(entry) = dirs.next_entry().await? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(name) = path.file_name().map(|name| name.to_string_lossy().to_string()) else {
            trace!("Found instance with invalid name: {:?}", path);
            continue;
        };

        let instance = instance::Instance::new(name.clone());
        instances.insert(name, instance); // impl convenience method for this
    }

    Ok(&INSTANCES)*/
}

// todo : build a chain of responsibility for testing(evaluate possible better pattern because of rust)
async fn test_directory(target_instance_directory_path: impl AsRef<Path>) -> InstanceScanResult<bool>{
    todo!()
}

async fn test_directory_structure(target_instance_directory_path: impl AsRef<Path>) -> InstanceScanResult<bool>{
    todo!()
}

#[derive(Error, Debug)]
pub enum InstanceScanError {
    #[error("path `{path}` does not contain any valid instance at ")]
    NoInstancesInFolder { path: PathBuf, recursive_searched: bool },

    #[error("path not found !\n")]
    IoError(#[from] io::Error),
}
