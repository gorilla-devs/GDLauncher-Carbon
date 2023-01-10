use std::borrow::BorrowMut;
use std::collections::HashSet;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};
use serde::Deserialize;

use thiserror::Error;
use tracing::log;
use tracing::log::trace;
use uuid::Variant::Future;
use crate::instance::configuration::InstanceConfigurationFile;

use crate::instance::Instance;
use crate::instance::instances_scan::InstanceScanError::{FileStructureDoesNotMatch, FolderStructureDoesNotMatch};
use crate::minecraft_package::MinecraftPackage;

type InstanceScanResult<T> = Result<T, InstanceScanError>;
type InstanceTestResult = Result<(), InstanceScanError>;


// todo : this is a factory method and the method name implies that the operation is only a scan(read), split concerns
pub async fn scan_for_instances(path_to_search_in: impl AsRef<Path>, search_all_folders_in_depth: bool) -> InstanceScanResult<Vec<Instance>> {

    // async but mind that the scan must be executed one level per once in order to avoid possible false positives(could drive to a nasty bug for future change)
    // test whether you can recurse infinitely for a link to a self contained folder

    let path_to_search_in = path_to_search_in.as_ref();
    check_directory(path_to_search_in)?;

    let minecraft_package = MinecraftPackage::new(
        Default::default(),
        Default::default(),
        Default::default(),
    );

    let found_instances = vec![
        Instance {
            name: "".to_string(),
            minecraft_package,
        }
    ];

    Ok(found_instances)
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
        .map(|folder_path| if let false = folder_path.exists() && folder_path.is_dir() { FolderStructureDoesNotMatch(folder_path) })?;
    FILES_TREE.iter()
        .map(|file_path_last_part| PathBuf::from(target_instance_directory_path, file_path_last_part))
        .map(|file_path| if let false = file_path.exists() && file_path.is_file() { FileStructureDoesNotMatch(file_path) })?
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

#[derive(Error, Debug)]
pub enum InstanceScanError {
    #[error("path `{path}` does not contain any valid instance at ")]
    NoInstancesInFolder { path: PathBuf, recursive_searched: bool },

    #[error("path not found !\n")]
    IoError(#[from] io::Error),

    #[error("expected folder `{0}` but not found! \n")]
    FolderStructureDoesNotMatch(PathBuf),

    #[error("expected file `{0}` but not found! \n")]
    FileStructureDoesNotMatch(PathBuf),
}

#[cfg(test)]
mod unit_tests {
    use std::path::PathBuf;
    use crate::instance::instances_scan::{check_configuration_file_sanity, check_directory_structure};

    #[test]
    fn test_directory_structure_check() {
        let res = check_directory_structure(&PathBuf::from("test_assets").join("instance_example")).await;
        let affirmative_check = matches!(res, Ok(_));
        assert!(affirmative_check);
        let res = check_directory_structure(&PathBuf::from("test_assets").join("malformed_instance_example")).await;
        let denial_check = matches!(res, Err(_)); // todo : add every error case
        assert!(denial_check);
    }

    #[test]
    fn test_configuration_file_sanity_check() {
        let res = check_configuration_file_sanity(&PathBuf::from("test_assets").join("instance_example")).await;
        let affirmative_check = matches!(res, Ok(_));
        assert!(affirmative_check);
        let res = check_configuration_file_sanity(&PathBuf::from("test_assets").join("malformed_instance_example")).await;
        let denial_check = matches!(res, Err(_)); // todo : add every error case
        assert!(denial_check);
    }
}

