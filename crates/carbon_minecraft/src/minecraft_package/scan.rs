use std::path::{Path, PathBuf};
use log::trace;
use thiserror::Error;
use crate::try_path_fmt;
use crate::minecraft_package::configuration::ConfigurationFileParsingError;
use crate::minecraft_package::MinecraftPackage;

#[derive(Error, Debug)]
pub enum MinecraftPackageScanError{
    #[error("problem encountered when parsing minecraft package configuration file ! : {0} \n")]
    MinecraftPackageConfigurationFileParsingError(#[from] ConfigurationFileParsingError)

}

pub(crate) type MinecraftPackageScanResult = Result<MinecraftPackage, MinecraftPackageScanError>;

pub(crate) const CONFIGURATION_FILE_RELATIVE_PATH: &str = ".conf.json";


pub async fn scan_for_packages<T : AsRef<Path> + Sync>(package_dir_path: & T) -> MinecraftPackageScanResult {
    let package_dir_path = package_dir_path.as_ref();
    //fixme : provvisory implementation
    trace!("scanning directory {} for minecraft package", try_path_fmt!(package_dir_path) );
    let configuration_file_path = &PathBuf::from(package_dir_path).join(CONFIGURATION_FILE_RELATIVE_PATH);
    Ok(crate::minecraft_package::configuration::parse_from_file(configuration_file_path).await?.into())
}



