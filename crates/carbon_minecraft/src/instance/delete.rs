use std::path::Path;
use log::trace;
use thiserror::Error;
use tokio::task::{JoinError, spawn_blocking};
use crate::try_path_fmt;
use crate::instance::{Instance, InstanceStatus};
use crate::instance::consts::{CONFIGURATION_FILE_RELATIVE_PATH, MINECRAFT_PACKAGE_RELATIVE_PATH};
use crate::instance::delete::InstanceDeleteError::InstanceNotPersisted;
use crate::instance::scan::{check_instance_directory_sanity, InstanceScanError};


#[derive(Error, Debug)]
pub enum InstanceDeleteError {
    #[error("unable to delete a non persisted instance!")]
    InstanceNotPersisted,

    #[error("folder does not contain any instance : {0} ")]
    InstanceScanError(#[from] InstanceScanError),

    #[error("folder does not contain any instance : {0} ")]
    JoinError(#[from] JoinError),

    #[error("io-error rise while deleting instance : {0}")]
    IoError(#[from] std::io::Error),
}

type InstanceDeleterResult = Result<Instance, InstanceDeleteError>;

pub async fn delete(instance: Instance, put_in_trash_bin: bool) -> InstanceDeleterResult
{
    match &instance.persistence_status {
        InstanceStatus::Persisted(instance_path) if put_in_trash_bin => {
            trace!("checking instance directory structure at {}", try_path_fmt!(instance_path));
            check_instance_directory_sanity(instance_path).await?;
            trace!("putting in trash bin instance from fs at {}", try_path_fmt!(instance_path));
            let configuration_file_path = Path::new(instance_path).join(CONFIGURATION_FILE_RELATIVE_PATH);
            let minecraft_package_path = Path::new(instance_path).join(MINECRAFT_PACKAGE_RELATIVE_PATH);
            let _ = spawn_blocking(move || trash::delete(configuration_file_path)).await?;
            let _ = spawn_blocking(move || trash::delete(minecraft_package_path) ).await?;

            Ok(instance.mutate_persistence_status(InstanceStatus::NotPersisted))
        },
        InstanceStatus::Persisted(instance_path) if !put_in_trash_bin=> {
            trace!("checking instance directory structure at {}", try_path_fmt!(instance_path));
            check_instance_directory_sanity(instance_path).await?;
            trace!("deleting instance from fs at {}", try_path_fmt!(instance_path));
            let configuration_file_path = Path::new(instance_path).join(CONFIGURATION_FILE_RELATIVE_PATH);
            let minecraft_package_path = Path::new(instance_path).join(MINECRAFT_PACKAGE_RELATIVE_PATH);
            tokio::fs::remove_file(configuration_file_path).await?;
            tokio::fs::remove_dir_all(minecraft_package_path).await?;

            Ok(instance.mutate_persistence_status(InstanceStatus::NotPersisted))
        },
        _ => Err(InstanceNotPersisted)
    }

}

// fixme : finish refactor down here V
/*

pub async fn delete(instance: Instance, put_in_trash_bin: bool) -> InstanceDeleterResult {
    let deleter = match put_in_trash_bin {
        false =>|path| async {  tokio::fs::remove_dir_all(path).await},
        true => |path| async { spawn_blocking(move || trash::delete_all(path)).await},
    };

    match &instance.persistence_status {
        InstanceStatus::Persisted(path) => {
            deleter(path).await?;
            Ok(instance.mutate_persistence_status(InstanceStatus::NotPersisted))
        }
        _ => Err(InstanceDeleteError::InstanceNotPersisted),
    }
}


async fn delete_instance_files_at<T, R, F>(path_to_search_in: &T, remover: F) -> Result<(), InstanceDeleteError>
    where
        T: AsRef<Path> + Sync,
        R: Future<Output=InstanceDeleterResult>,
        F: Fn(&Path) -> R
{
    let path_to_search_in = path_to_search_in.as_ref();
    trace!("checking instance directory structure at {}", try_path_fmt!(path_to_search_in));
    check_instance_directory(&path_to_search_in).await?;
    trace!("checking instance directory structure at {}", try_path_fmt!(path_to_search_in));
    try_join_all(
        SUBFOLDERS_TREE.iter()
            .map(|folder_path_last_part| PathBuf::from(&path_to_search_in).join(folder_path_last_part))
            .map(|ref folder_path| remover(path_to_search_in))
    ).await?;

    try_join_all(
        FILES_TREE.iter()
            .map(|file_path_last_part| PathBuf::from(path_to_search_in).join(file_path_last_part))
            .map(|ref file_path| remover(file_path))
    ).await?;
    Ok(())
}*/