use crate::managers::instance::instance_configuration::consts::{
    CONFIGURATION_FILE_RELATIVE_PATH, MINECRAFT_PACKAGE_RELATIVE_PATH,
};
use crate::managers::instance::instance_configuration::ConfigurationFileParsingError;
use crate::managers::instance::write::InstanceWriteError::{
    InstanceAlreadyPresent, InstanceIsInstalling,
};
use crate::managers::instance::InstanceManager;
use crate::try_path_fmt::try_path_fmt;
use carbon_domain::instance::{Instance, InstanceStatus};
use log::trace;
use std::future::Future;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InstanceWriteError {
    #[error("error happened while trying to write configuration file for instance : {0}\n")]
    InstanceConfigurationWritingError(#[from] ConfigurationFileParsingError),

    #[error("instance already exist at : {0}")]
    InstanceAlreadyPresent(PathBuf),

    #[error("instance is installing so we cannot write now : {0:?}")]
    InstanceIsInstalling(Instance),

    #[error("minecraft package already exist and is a file at {0}")]
    MinecraftPackageIsAFile(PathBuf),

    #[error("minecraft package already exist")]
    MinecraftPackageAlreadyExist(PathBuf),

    #[error("io error raised while writing instance : {0}")]
    IoError(#[from] std::io::Error),
}

type InstanceWriteResult = Result<Instance, InstanceWriteError>;

impl InstanceManager {
    pub(super) async fn write_at<T: AsRef<Path> + Sync>(
        &self,
        instance: Instance,
        path: &T,
    ) -> InstanceWriteResult {
        let base_path = path.as_ref();
        trace!("writing instance at {}", try_path_fmt!(base_path));

        if (self.check_instance_directory_sanity(&base_path).await).is_ok() {
            Err(InstanceAlreadyPresent(base_path.to_path_buf()))?
        };

        if !base_path.exists() {
            trace!(
                "folder {} does not exist, making ",
                try_path_fmt!(base_path)
            );
            tokio::fs::create_dir_all(base_path).await?
        }

        let minecraft_package_folder =
            &PathBuf::from(base_path).join(MINECRAFT_PACKAGE_RELATIVE_PATH);

        async fn make_minecraft_package_folder_at(folder_path: &Path) -> std::io::Result<()> {
            trace!(
                "making minecraft package folder at {}",
                try_path_fmt!(folder_path)
            );
            tokio::fs::create_dir(folder_path).await
        }

        let minecraft_package_folder_exist = minecraft_package_folder.exists();
        let minecraft_package_folder_is_dir = minecraft_package_folder.is_dir();

        match (
            &instance.persistence_status,
            minecraft_package_folder_exist,
            minecraft_package_folder_is_dir,
        ) {
            (InstanceStatus::Ready(instance_path), false, false)
                if !instance_path.starts_with(minecraft_package_folder) =>
            {
                let old_instance_minecraft_package_dir =
                    instance_path.join(MINECRAFT_PACKAGE_RELATIVE_PATH);
                trace!(
                    "coping old instance minecraft package dir from {} to {}",
                    try_path_fmt!(old_instance_minecraft_package_dir),
                    try_path_fmt!(minecraft_package_folder)
                );
                tokio::fs::copy(
                    old_instance_minecraft_package_dir.clone(),
                    minecraft_package_folder,
                )
                .await?;
                tokio::fs::remove_dir_all(old_instance_minecraft_package_dir).await?;
            }
            (_, false, false) => make_minecraft_package_folder_at(minecraft_package_folder).await?,
            (InstanceStatus::Installing(instance_path), _, _) => {
                Err(InstanceIsInstalling(instance.clone()))?
            }
            (_, true, false) => Err(InstanceWriteError::MinecraftPackageIsAFile(
                minecraft_package_folder.clone(),
            ))?,
            (_, true, true) => Err(InstanceWriteError::MinecraftPackageAlreadyExist(
                minecraft_package_folder.clone(),
            ))?,
            _ => (),
        }

        let instance_configuration_file_path =
            PathBuf::from(base_path).join(CONFIGURATION_FILE_RELATIVE_PATH);
        self.write_configuration_in_file(&instance, &instance_configuration_file_path)
            .await?;

        Ok(instance.mutate_persistence_status(InstanceStatus::Ready(base_path.to_path_buf())))
    }
}
