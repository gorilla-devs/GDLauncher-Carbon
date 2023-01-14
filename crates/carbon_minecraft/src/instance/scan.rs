use std::fs::{DirEntry, read_dir};
use std::io;
use std::path::{Path, PathBuf};
use log::trace;
use crate::try_path_fmt;

use async_trait::async_trait;
use futures::future;
use thiserror::Error;
use crate::instance::configuration::{ConfigurationFileParsingError, ConfigurationFileParser};
use crate::minecraft_package::scan::MinecraftPackageScanner;

use crate::instance::{Instance, InstanceStatus};
use crate::instance::scan::InstanceScanError::{FileStructureDoesNotMatch, FolderStructureDoesNotMatch};
use crate::minecraft_package::scan::{MinecraftPackageScanError};

type InstanceScanResult = Result<Vec<Result<Instance, InstanceScanError>>, InstanceScanError>;
type InstanceTestResult = Result<(), InstanceScanError>;

#[derive(Error, Debug)]
pub enum InstanceScanError {
    #[error("path `{path}` does not contain any valid instance at ")]
    NoInstancesInFolder { path: PathBuf, recursive_searched: bool },

    #[error("io error: {0} !\n")]
    IoError (#[from] io::Error),

    #[error("expected folder `{0}` but not found! \n")]
    FolderStructureDoesNotMatch(PathBuf),

    #[error("expected file `{0}` but not found! \n")]
    FileStructureDoesNotMatch (PathBuf),

    #[error("error happened while parsing instances configuration file: `{0}`\n")]
    ConfigurationFileParsingError(#[from] ConfigurationFileParsingError),

    #[error("error happened while scanning for minecraft package in instance : `{0}`\n")]
    MinecraftPackageScanError(#[from] MinecraftPackageScanError),

}

#[cfg(not(target_os="windows"))]
pub(crate) const SUBFOLDERS_TREE: Vec<PathBuf> = vec![
    PathBuf::from("minecraft"),
    PathBuf::from("minecraft/mods"),
    PathBuf::from("minecraft/core"),
    PathBuf::from("minecraft/save_files"),
];

#[cfg(target_os="windows")]
pub(crate) const SUBFOLDERS_TREE: Vec<PathBuf> = vec![
    PathBuf::from("minecraft"),
    PathBuf::from(r"minecraft\mods"),
    PathBuf::from(r"minecraft\core"),
    PathBuf::from(r"minecraft\save_files"),
];

pub(crate) const CONFIGURATION_FILE_RELATIVE_PATH: PathBuf = PathBuf::from(".conf.json");

pub(crate) const MINECRAFT_PACKAGE_RELATIVE_PATH: PathBuf = PathBuf::from("minecraft");

pub(crate) const FILES_TREE: Vec<PathBuf> = vec![
    CONFIGURATION_FILE_RELATIVE_PATH,
];

#[async_trait]
pub(crate) trait InstanceScanner {
    // todo : this is a factory method and the method name implies that the operation is only a scan(read), split concerns

    async fn scan_for_instances_single_directory(directory_path: Result<DirEntry, io::Error>) -> Result<Instance, InstanceScanError> {
        let directory_path = &directory_path?.path();
        trace!("scanning directory {} for instances", try_path_fmt!(directory_path.as_path()) );
        Self::check_instance_directory(directory_path).await?;
        let configuration_file_path = Path::new(directory_path).join(CONFIGURATION_FILE_RELATIVE_PATH);
        let instance_configuration_file= ConfigurationFileParser::parse_from_file(&configuration_file_path).await?;
        let minecraft_package_path = PathBuf::from(directory_path).join(crate::minecraft_package::scan::CONFIGURATION_FILE_RELATIVE_PATH);
        Ok(
            Instance {
                name: instance_configuration_file.instance_name.clone(),
                minecraft_package: MinecraftPackageScanner::scan_for_packages(&minecraft_package_path).await?,
                persistence_status: InstanceStatus::Persisted(directory_path.to_path_buf()),
            }
        )
    }


    async fn scan_for_instances<T: AsRef<Path> + Sync>(path_to_search_in: &T) -> InstanceScanResult {
        // async but mind that the scan must be executed one level at once in order to avoid possible false positives(could drive to a nasty bug for future change)
        // test whether you can recurse infinitely for a link to a self contained folder
        let path_to_search_in = path_to_search_in.as_ref();
        Ok(
            future::join_all(
                read_dir(path_to_search_in)?
                    .map(|dir_entry| { Self::scan_for_instances_single_directory(dir_entry) })
            ).await
        )
    }

    async fn check_instance_directory<T: AsRef<Path> + Sync>(target_instance_directory_path: &T) -> InstanceTestResult {
        Self::check_directory_structure(target_instance_directory_path).await
    }

    async fn check_directory_structure<T: AsRef<Path> + Sync>(target_instance_directory_path: &T) -> InstanceTestResult {
        let target_instance_directory_path = target_instance_directory_path.as_ref();

        SUBFOLDERS_TREE.iter()
            .map(|folder_path_last_part| PathBuf::from(target_instance_directory_path).join(folder_path_last_part))
            .map(|folder_path| match folder_path.exists() && folder_path.is_dir() {
                false => Err(FolderStructureDoesNotMatch(folder_path)),
                _ => Ok(())
            })
            .collect::<InstanceTestResult>()?;

        FILES_TREE.iter()
            .map(|file_path_last_part| PathBuf::from(target_instance_directory_path).join(file_path_last_part))
            .map(|file_path| match file_path.exists() && file_path.is_file() {
                false => return Err(FileStructureDoesNotMatch(file_path)),
                _ => Ok(())
            })
            .collect::<InstanceTestResult>()?;

        Ok(())
    }
}

#[cfg(test)]
mod unit_tests {

    #[test]
    fn test_instance_scan_ok() {


    }

    #[test]
    fn test_instance_scan_err() {


    }

    #[test]
    fn test_directory_structure_check_ok() {
        /*let res = InstanceScanner::check_directory_structure(&PathBuf::from("test_assets").join("instance_example")).await;
        let affirmative_check = matches!(res, Ok(_));
        assert!(affirmative_check);*/
    }

    #[test]
    fn test_directory_structure_check_err() {
        /*let res = InstanceScanner::check_directory_structure(&PathBuf::from("test_assets").join("malformed_instance_example")).await;
        let denial_check = matches!(res, Err(_)); // todo : add every error case
        assert!(denial_check);*/
    }
}

