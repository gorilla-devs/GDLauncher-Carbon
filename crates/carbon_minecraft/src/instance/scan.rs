use std::fs::{DirEntry, read_dir};
use std::io;
use std::path::{Path, PathBuf};
use log::trace;
use crate::{minecraft_package, try_path_fmt};

use futures::future;
use thiserror::Error;
use crate::instance::configuration::{ConfigurationFileParsingError, parse_from_file};
use crate::instance::{Instance, InstanceStatus};
use crate::instance::consts::{CONFIGURATION_FILE_RELATIVE_PATH, FILES_TREE, MINECRAFT_PACKAGE_RELATIVE_PATH, SUBFOLDERS_TREE};
use crate::instance::scan::InstanceScanError::{FileStructureDoesNotMatch, FolderStructureDoesNotMatch, PathNotIsNotPointingToAFolder};
use crate::minecraft_package::scan::{MinecraftPackageScanError};

type InstanceScanResult = Result<Vec<Result<Instance, InstanceScanError>>, InstanceScanError>;
type InstanceTestResult = Result<(), InstanceScanError>;

#[derive(Error, Debug)]
pub enum InstanceScanError {
    #[error("path `{path}` does not contain any valid instance at ")]
    NoInstancesInFolder { path: PathBuf, recursive_searched: bool },

    #[error("path `{0}` does not point to a directory ")]
    PathNotIsNotPointingToAFolder(PathBuf),

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


pub async fn scan_for_instances<T: AsRef<Path> + Sync>(path_to_search_in: &T) -> InstanceScanResult {
    // todo : add recursive mode
    let path_to_search_in = path_to_search_in.as_ref();
    trace!("scanning directory {} for instances", try_path_fmt!(path_to_search_in));
    match path_to_search_in.is_dir() {
        true => Ok(
            future::join_all(
                read_dir(path_to_search_in)?
                    //.filter(|dir_entry| dir_entry. map_or(true, |dir_entry|dir_entry.path().is_dir())) todo : filter only directory
                    .map(|dir_entry| { scan_for_instances_single_directory(dir_entry) })
            ).await
        ),
        false => {
            trace!("path {} is not pointing to a directory! aborting instance scan process ...", try_path_fmt!(path_to_search_in) );
            Err(PathNotIsNotPointingToAFolder(path_to_search_in.to_path_buf()))
        }
    }



}

pub async fn check_instance_directory<T: AsRef<Path> + Sync>(target_instance_directory_path: &T) -> InstanceTestResult {
    trace!("start check process for folder {}", try_path_fmt!(target_instance_directory_path.as_ref()));
    check_directory_structure(target_instance_directory_path).await
}

async fn check_directory_structure<T: AsRef<Path> + Sync>(target_instance_directory_path: &T) -> InstanceTestResult {
    let target_instance_directory_path = target_instance_directory_path.as_ref();
    trace!("checking instance directory structure at {}", try_path_fmt!(target_instance_directory_path));
    SUBFOLDERS_TREE.iter()
        .map(|folder_path_last_part| PathBuf::from(target_instance_directory_path).join(folder_path_last_part))
        .try_for_each(|folder_path| match folder_path.exists() && folder_path.is_dir() {
            false => Err(FolderStructureDoesNotMatch(folder_path)),
            _ => Ok(())
        })?;

    FILES_TREE.iter()
        .map(|file_path_last_part| PathBuf::from(target_instance_directory_path).join(file_path_last_part))
        .try_for_each(|file_path| match file_path.exists() && file_path.is_file() {
            false => Err(FileStructureDoesNotMatch(file_path)),
            _ => Ok(())
        })?;

    Ok(())
}

async fn scan_for_instances_single_directory(directory_path: Result<DirEntry, io::Error>) -> Result<Instance, InstanceScanError> {
    let directory_path = &directory_path?.path();
    trace!("scanning directory {} for instance", try_path_fmt!(directory_path.as_path()) );
    check_instance_directory(directory_path).await?;
    let configuration_file_path = Path::new(directory_path).join(CONFIGURATION_FILE_RELATIVE_PATH);
    let instance_configuration_file = parse_from_file(&configuration_file_path).await?;
    let minecraft_package_path = PathBuf::from(directory_path).join(MINECRAFT_PACKAGE_RELATIVE_PATH);
    Ok(
        Instance {
            name: instance_configuration_file.instance_name.clone(),
            minecraft_package: minecraft_package::scan::scan_for_packages(&minecraft_package_path).await?,
            persistence_status: InstanceStatus::Persisted(directory_path.to_path_buf()),
        }
    )
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

