use std::path::{Path, PathBuf};
use async_trait::async_trait;
use thiserror::Error;
use crate::minecraft_package::configuration::ConfigurationFileParsingError;
use crate::minecraft_package::MinecraftPackage;

#[derive(Error, Debug)]
pub enum MinecraftPackageScanError{
    #[error("problem encountered when parsing minecraft package configuration file ! : {0} \n")]
    MinecraftPackageConfigurationFileParsingError(#[from] ConfigurationFileParsingError)

}

pub(crate) type MinecraftPackageScanResult = Result<MinecraftPackage, MinecraftPackageScanError>;

pub(crate) const CONFIGURATION_FILE_RELATIVE_PATH: PathBuf = PathBuf::from(".conf.json");

#[async_trait]
pub(crate) trait MinecraftPackageScanner {

    async fn scan_for_packages<T : AsRef<Path> + Sync>(package_dir_path: & T) -> MinecraftPackageScanResult {
        //fixme : provvisory implementation
        let package_dir_path = package_dir_path.as_ref();
        let configuration_file_path = &PathBuf::from(package_dir_path).join(CONFIGURATION_FILE_RELATIVE_PATH);
        Ok(crate::minecraft_package::configuration::ConfigurationFileParser::parse_from_file(configuration_file_path).await?.into())
    }

}



