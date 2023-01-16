use std::path::{Path, PathBuf};
use crate::instance::{Instance, InstanceStatus};
use futures::future;
use log::trace;
use thiserror::Error;
use crate::{instance, minecraft_package, try_path_fmt};

#[derive(Error, Debug)]
pub enum InstanceWriteError{

    #[error("error happened while trying to write configuration file for instance : {0}\n")]
    InstanceConfigurationWritingError(#[from] instance::configuration::ConfigurationFileParsingError),

    #[error("error happened while trying to write configuration file for minecraft package : {0}\n")]
    MinecraftPackageConfigurationWritingError(#[from] minecraft_package::configuration::ConfigurationFileParsingError),

    #[error("io error raised while writing instance : {0}\n")]
    IoError(#[from] std::io::Error),

}

type InstanceWriteResult = Result<Instance, InstanceWriteError>;

/*
pub async fn write(instance: Instance) -> InstanceWriteResult {
    match &instance.persistence_status {
        InstanceStatus::Persisted(path) => write_at(instance, path).await,
        InstanceStatus::NotPersisted => Err(InstanceWriteError::PathNotSpecified),
    }
}
*/

pub async fn write_at<T: AsRef<Path> + Sync>(instance: Instance, path: &T) -> InstanceWriteResult {
    let base_path = path.as_ref();
    trace!("writing instance at {}", try_path_fmt!(base_path));
    future::try_join_all(
        instance::consts::SUBFOLDERS_TREE.iter()
            .map(|dir_entry| tokio::fs::create_dir(PathBuf::from(base_path).join(dir_entry)))
    ).await?;

    let instance_configuration_file_path = PathBuf::from(base_path).join(instance::consts::CONFIGURATION_FILE_RELATIVE_PATH);
    instance::configuration::write_in_file(&instance, &instance_configuration_file_path).await?;

    let minecraft_package_configuration_file_path = PathBuf::from(base_path)
        .join(instance::consts::MINECRAFT_PACKAGE_RELATIVE_PATH)
        .join(minecraft_package::scan::CONFIGURATION_FILE_RELATIVE_PATH);
    minecraft_package::configuration::write_in_file(&instance.minecraft_package, &minecraft_package_configuration_file_path).await?;

    Ok(instance.mutate_persistence_status(InstanceStatus::Persisted(base_path.to_path_buf())))
}

