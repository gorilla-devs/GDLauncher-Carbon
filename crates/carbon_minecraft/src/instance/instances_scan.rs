use std::backtrace::Backtrace;
use std::borrow::BorrowMut;
use std::collections::HashSet;
use std::fs::{DirEntry, File, read_dir};
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};
use camino::Utf8Path;
use cairo::Cairo;
use serde::Deserialize;

use thiserror::Error;
use tracing::log;
use tracing::log::trace;
use uuid::Variant::Future;
use crate::instance::configuration::InstanceConfigurationFile;
use crate::minecraft_package::package_scan::MinecraftPackageScanner;

use crate::instance::Instance;
use crate::instance::instances_scan::InstanceScanError::{FileStructureDoesNotMatch, FolderStructureDoesNotMatch, IoError, NotRepresentablePath};
use crate::minecraft_package::MinecraftPackage;
use crate::minecraft_package::package_scan::MinecraftPackageScanResult;

type InstanceScanResult = Vec<Result<Instance, InstanceScanError>>;
type InstanceTestResult = Result<(), InstanceScanError>;

#[derive(Error, Debug)]
pub enum InstanceScanError {
    #[error("path `{path}` does not contain any valid instance at ")]
    NoInstancesInFolder { path: PathBuf, recursive_searched: bool },

    #[error("io error: {0} !\n")]
    IoError { #[from] error: io::Error, backtrace: Backtrace },

    #[error("expected folder `{0}` but not found! \n")]
    FolderStructureDoesNotMatch(PathBuf),

    #[error("expected file `{0}` but not found! \n")]
    FileStructureDoesNotMatch(PathBuf),

    #[error("path not representable\n")]
    NotRepresentablePath
}

pub(crate) trait InstanceScanner {
    // todo : this is a factory method and the method name implies that the operation is only a scan(read), split concerns
    async fn scan_for_instances(path_to_search_in: impl AsRef<Path>, search_all_folders_in_depth: bool) -> InstanceScanResult {

        // async but mind that the scan must be executed one level at once in order to avoid possible false positives(could drive to a nasty bug for future change)
        // test whether you can recurse infinitely for a link to a self contained folder

        let path_to_search_in = path_to_search_in.as_ref();

        async fn scan_for_instances_single_directory(directory_path : Result<DirEntry, io::Error>) -> Result<Instance, InstanceScanError>{
            let directory_path = &directory_path?.path();
            trace!("scanning directory {} for instances", Utf8Path::from_path(directory_path.as_path()).map(ToString::to_string).unwrap_or("<<unrepresentable path!>>".to_string()););
            check_instance_directory(directory_path)?;
            Ok(
                Instance{
                    name: Utf8Path::from_path(directory_path.as_path()).ok_or(NotRepresentablePath )?.to_string() ,
                    minecraft_package: MinecraftPackageScanner::scan_for_packages(directory_path).await?,
                }
            )
        }

        read_dir(path_to_search_in)
            .map_err(Into::into)?
            .map(|dir_entry|scan_for_instances_single_directory(dir_entry).await)
            .collect::<Vec<_>>()

    }
}

// todo : build a chain of responsibility for testing(evaluate possible better pattern because of rust)
pub async fn check_instance_directory(target_instance_directory_path: impl AsRef<Path>) -> InstanceTestResult {
    check_directory_structure(path_to_search_in)?;
    check_configuration_file_sanity(path_to_search_in)?
}


const SUBFOLDERS_TREE: Vec<PathBuf> = vec![
    PathBuf::from("minecraft"),
    PathBuf::from("minecraft", "mods"),
];

const FILES_TREE: Vec<PathBuf> = vec![
    PathBuf::from(".conf.json"),
];

async fn check_directory_structure(target_instance_directory_path: &impl AsRef<Path>) -> InstanceTestResult {
    let target_instance_directory_path = target_instance_directory_path.as_ref();
    SUBFOLDERS_TREE.iter()
        .map(|folder_path_last_part| PathBuf::from(target_instance_directory_path, folder_path_last_part))
        .map(|folder_path| if let false = folder_path.exists() && folder_path.is_dir() { Err(FolderStructureDoesNotMatch(folder_path)) })?;
    FILES_TREE.iter()
        .map(|file_path_last_part| PathBuf::from(target_instance_directory_path, file_path_last_part))
        .map(|file_path| if let false = file_path.exists() && file_path.is_file() { Err(FileStructureDoesNotMatch(file_path)) })?
}

const CONFIGURATION_FILE_RELATIVE_PATH: PathBuf = PathBuf::from(".conf.json");

async fn check_configuration_file_sanity(target_instance_directory_path: impl AsRef<Path>) -> InstanceTestResult {
    // todo : evaluate if is better to use a reader
    let mut conf_file_path = PathBuf::from(target_instance_directory_path).join(CONFIGURATION_FILE_RELATIVE_PATH);
    let mut conf_file = &File::open(conf_file_path)?;
    let mut conf_file_content = &String::new();
    let bytes_read = conf_file.read_to_string(conf_file_content.borrow_mut())?;
    trace!("read {bytes_read} bytes from configuration file"); //fixme  pathbuff doesn't implement Display trait, use cairo path to try conversion and so show the conf file path
    serde_json::from_str::<'_, InstanceConfigurationFile>(conf_file_content)?;
    Ok(())
}

#[cfg(test)]
mod unit_tests {
    use std::path::PathBuf;
    use crate::instance::instances_scan::{check_configuration_file_sanity, check_directory_structure};

    #[test]
    fn test_instance_scan_ok() {

    }

    #[test]
    fn test_instance_scan_err() {

    }

    #[test]
    fn test_directory_structure_check_ok() {
        let res = check_directory_structure(&PathBuf::from("test_assets").join("instance_example")).await;
        let affirmative_check = matches!(res, Ok(_));
        assert!(affirmative_check);
    }

    #[test]
    fn test_directory_structure_check_err() {
        let res = check_directory_structure(&PathBuf::from("test_assets").join("malformed_instance_example")).await;
        let denial_check = matches!(res, Err(_)); // todo : add every error case
        assert!(denial_check);
    }

    #[test]
    fn test_configuration_file_sanity_check_ok() {
        let res = check_configuration_file_sanity(&PathBuf::from("test_assets").join("instance_example")).await;
        let affirmative_check = matches!(res, Ok(_));
        assert!(affirmative_check);
    }

    #[test]
    fn test_configuration_file_sanity_check_fail() {
        let res = check_configuration_file_sanity(&PathBuf::from("test_assets").join("malformed_instance_example")).await;
        let denial_check = matches!(res, Err(_)); // todo : add every error case
        assert!(denial_check);
    }
}

