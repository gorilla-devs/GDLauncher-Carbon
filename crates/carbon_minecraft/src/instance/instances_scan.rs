use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;
use uuid::Variant::Future;

use crate::instance::Instance;
use crate::instance::instances_scan::InstanceScanError::{FileStructureDoesNotMatch, FolderStructureDoesNotMatch};

type InstanceScanResult<T> = Result<T, InstanceScanError>;
type InstanceTestResult = Result<(), InstanceScanError>;


// todo : this is a factory method and the method name implies that the operation is only a scan(read), split concerns
pub async fn scan_for_instances(path_to_search_in: impl AsRef<Path>, search_all_folders_in_depth: bool) -> InstanceScanResult<Vec<Instance>> {

    // async but mind that the scan must be executed one level per once in order to avoid possible false positives(could drive to a nasty bug for future change)
    // test whether you can recurse infinitely for a link to a self contained folder

    let path_to_search_in = path_to_search_in.as_ref();

    test_directory_structure(path_to_search_in)?;
    test_configuration_file_sanity(path_to_search_in)?;
}

// todo : build a chain of responsibility for testing(evaluate possible better pattern because of rust)
async fn test_directory(target_instance_directory_path: impl AsRef<Path>) -> InstanceScanResult<bool> {
    todo!()
}


const SUBFOLDERS_TREE: Vec<PathBuf> = vec![
    PathBuf::from("minecraft"),
    PathBuf::from("minecraft", "mods"),
];

const FILES_TREE: Vec<PathBuf> = vec![
    PathBuf::from(".conf.json"),
];

async fn test_directory_structure(target_instance_directory_path: & impl AsRef<Path>) -> InstanceTestResult {
    let target_instance_directory_path = target_instance_directory_path.as_ref();
    SUBFOLDERS_TREE.iter()
        .map(|folder_path_last_part| PathBuf::from(target_instance_directory_path, folder_path_last_part))
        .map(|folder_path| if let false = folder_path.exists() && folder_path.is_dir() {FolderStructureDoesNotMatch(folder_path)})?;
    FILES_TREE.iter()
        .map(|file_path_last_part| PathBuf::from(target_instance_directory_path, file_path_last_part))
        .map(|file_path| if let false = file_path.exists() && file_path.is_file() {FileStructureDoesNotMatch(file_path)})?
}

async fn test_configuration_file_sanity(target_instance_directory_path: impl AsRef<Path>) -> InstanceTestResult {}

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

    fn build_folder_structure(){

    }

    #[test]
    fn test_directory_structure() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }

    #[test]
    fn test_configuration_file_sanity() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }

}

