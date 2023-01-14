use std::path::{Path, PathBuf};
use crate::instance::{Instance, InstanceStatus};
use async_trait::async_trait;
use futures::future;
use thiserror::Error;
use crate::{instance, minecraft_package};

#[derive(Error, Debug)]
pub enum InstanceWriteError{

    #[error("")]
    PathNotSpecified,

    #[error("")]
    InstanceConfigurationWritingError(#[from] instance::configuration::ConfigurationFileParsingError),

    #[error("")]
    MinecraftPackageConfigurationWritingError(#[from] minecraft_package::configuration::ConfigurationFileParsingError),

    #[error("")]
    IoError(#[from] std::io::Error),

}

type InstanceWriterResult = Result<Instance, InstanceWriteError>;

#[async_trait]
pub(crate) trait InstanceWriter {

    async fn write(instance: Instance) -> InstanceWriterResult {
        match &instance.persistence_status {
            InstanceStatus::Persisted(path) => Self::write_at(instance, path).await,
            InstanceStatus::NotPersisted => Err(InstanceWriteError::PathNotSpecified),
        }
    }

    async fn write_at<T: AsRef<Path> + Sync>(instance: Instance, path: &T) -> InstanceWriterResult {

        let base_path = path.as_ref();

        future::try_join_all(
            instance::scan::SUBFOLDERS_TREE.iter()
                .map(|dir_entry| tokio::fs::create_dir(PathBuf::from(base_path).join(dir_entry)))
        ).await?;

        let instance_configuration_file_path = PathBuf::from(base_path).join(instance::scan::CONFIGURATION_FILE_RELATIVE_PATH);
        instance::configuration::ConfigurationFileParser::write_in_file(&instance, &instance_configuration_file_path).await?;

        let minecraft_package_configuration_file_path = PathBuf::from(base_path)
            .join(instance::scan::MINECRAFT_PACKAGE_RELATIVE_PATH)
            .join(minecraft_package::scan::CONFIGURATION_FILE_RELATIVE_PATH);
        minecraft_package::configuration::ConfigurationFileParser::write_in_file(&instance.minecraft_package, &minecraft_package_configuration_file_path).await?;

        Ok(instance.mutate_persistence_status(InstanceStatus::Persisted(base_path.to_path_buf())))
    }

}


