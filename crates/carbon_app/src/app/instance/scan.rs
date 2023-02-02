use crate::app::instance::instance_configuration::consts::{
    CONFIGURATION_FILE_RELATIVE_PATH, MINECRAFT_PACKAGE_RELATIVE_PATH,
};
use crate::app::instance::instance_configuration::ConfigurationFileParsingError;
use crate::app::instance::scan::InstanceScanError::{
    FileStructureDoesNotMatch, FolderStructureDoesNotMatch,
};
use crate::app::instance::InstanceManager;
use crate::try_path_fmt;
use carbon_domain::instance::{Instance, InstanceStatus};
use carbon_domain::minecraft_package::{MinecraftPackage, MinecraftPackageStatus};
use log::trace;
use std::path::{Path, PathBuf};
use std::{future, io};
use thiserror::Error;
use tokio::fs::{read_dir, DirEntry};
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;

#[derive(Error, Debug)]
pub enum InstanceScanError {
    #[error("path `{path}` does not contain any valid instance at ")]
    NoInstancesInFolder {
        path: PathBuf,
        recursive_searched: bool,
    },

    #[error("path `{0}` does not point to a directory ")]
    PathNotIsNotPointingToAFolder(PathBuf),

    #[error("io error: {0} !\n")]
    IoError(#[from] io::Error),

    #[error("expected folder `{0}` but not found! \n")]
    FolderStructureDoesNotMatch(PathBuf),

    #[error("expected file `{0}` but not found! \n")]
    FileStructureDoesNotMatch(PathBuf),

    #[error("error happened while parsing instances configuration file: `{0}`\n")]
    ConfigurationFileParsingError(#[from] ConfigurationFileParsingError),
}

type InstanceScanResult = Result<Vec<Result<Instance, InstanceScanError>>, InstanceScanError>;
type InstanceTestResult = Result<(), InstanceScanError>;

impl InstanceManager {
    pub(in crate::app::instance) async fn scan_for_instances(
        &mut self,
        folder_path: impl AsRef<Path>,
    ) -> InstanceScanResult {
        let folder_path = folder_path.as_ref();
        trace!(
            "scanning directory {} for instances",
            try_path_fmt!(folder_path)
        );
        let aaa = futures::future::join_all(
            ReadDirStream::new(read_dir(folder_path).await?)
                .map(|dir_entry| async move {
                    self.scan_for_instances_single_directory_entry(dir_entry)
                        .await
                })
                .into(),
        );

        /*let aaa = futures::future::join_all(
            ReadDirStream::new(read_dir(folder_path).await?)
                .map(|dir_entry| self.scan_for_instances_single_directory_entry(dir_entry)),
        );*/

        let res = match folder_path.is_dir() {
            true => Ok(future::join_all(
                ReadDirStream::new(read_dir(folder_path).await?)
                    .map(self.scan_for_instances_single_directory_entry)
                    .collect::<Vec<_>>()
                    .await,
            )
            .await),
            false => {
                trace!(
                    "path {} is not pointing to a directory! aborting instance scan process ...",
                    try_path_fmt!(folder_path)
                );
                Err(InstanceScanError::PathNotIsNotPointingToAFolder(
                    folder_path.to_path_buf(),
                ))
            }
        };

        if let Ok(instances) = res {
            for instance in instances.into_iter() {
                if let Ok(instance) = instance {
                    self.instances.read().await.save_instance(instance);
                }
            }
        }
    }

    pub(in crate::app::instance) async fn scan_for_instances_single_directory_entry(
        &self,
        directory_path: Result<DirEntry, io::Error>,
    ) -> Result<Instance, InstanceScanError> {
        let directory_path = &directory_path?.path();
        trace!(
            "scanning directory {} for instance",
            try_path_fmt!(directory_path.as_path())
        );
        self.check_instance_directory_sanity(directory_path).await?;
        let configuration_file_path =
            Path::new(directory_path).join(CONFIGURATION_FILE_RELATIVE_PATH);
        let instance_configuration_file = self
            .parse_configuration_from_file(&configuration_file_path)
            .await?;
        let instance_id = self.instances.read().await.get_next_available_id().await;
        Ok(Instance {
            name: instance_configuration_file.instance_name,
            id: instance_id,
            played_time: instance_configuration_file.played_time,
            last_played: instance_configuration_file.last_played,
            notes: instance_configuration_file.notes,
            minecraft_package: MinecraftPackage {
                version: instance_configuration_file
                    .minecraft_package_configuration
                    .version,
                mods: Default::default(),
                description: "".to_string(),
                mod_loaders: Default::default(),
                status: MinecraftPackageStatus::NotPersisted, // todo add probing method for the mc pakcage in relative manager
            },
            persistence_status: InstanceStatus::Ready(directory_path.clone()),
        })
    }

    pub(in crate::app::instance) async fn check_instance_directory_sanity<T: AsRef<Path> + Sync>(
        &self,
        target_instance_directory_path: &T,
    ) -> InstanceTestResult {
        let target_instance_directory_path = target_instance_directory_path.as_ref();
        trace!(
            "start check process for folder {}",
            try_path_fmt!(target_instance_directory_path)
        );
        let minecraft_package_path =
            PathBuf::from(target_instance_directory_path).join(MINECRAFT_PACKAGE_RELATIVE_PATH);
        let instance_configuration_file_path =
            PathBuf::from(target_instance_directory_path).join(CONFIGURATION_FILE_RELATIVE_PATH);
        let instance_configuration_file_exist =
            instance_configuration_file_path.exists() && instance_configuration_file_path.is_file();
        let minecraft_package_folder_exist =
            minecraft_package_path.exists() && minecraft_package_path.is_dir();
        match (
            instance_configuration_file_exist,
            minecraft_package_folder_exist,
        ) {
            (false, _) => Err(FolderStructureDoesNotMatch(
                instance_configuration_file_path,
            )),
            (_, false) => Err(FileStructureDoesNotMatch(minecraft_package_path)),
            _ => Ok(()),
        }
    }
}
