use std::collections::BTreeSet;
use std::fs::File;
use thiserror::Error;
use std::io;
use std::io::Read;
use std::path::Path;
use log::trace;
use serde::{Deserialize, Serialize};
use crate::minecraft_mod::MinecraftMod;
use crate::minecraft_package::MinecraftPackage;
use crate::try_path_fmt;

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct MinecraftPackageConfigurationFile {
    pub version: String,
    pub description: String,
    pub min_engine_version: (u8, u8, u8),
    pub mods: BTreeSet<MinecraftMod>,
}

impl From<&MinecraftPackage> for MinecraftPackageConfigurationFile {
    fn from(value: &MinecraftPackage) -> Self {
        // fixme : provisional implementation
        MinecraftPackageConfigurationFile{
            version: value.version.clone(),
            description: "".to_string(),
            min_engine_version: (0, 0, 0),
            mods: Default::default(),
        }
    }
}

//todo : boilerplate code, evaluate if is a good case to build a derive macro for the job
#[derive(Error, Debug)]
pub enum ConfigurationFileParsingError {
    #[error("error happened trying to parse json file : {0} !\n")]
    ParsingFileError(#[from] serde_json::Error),

    #[error("io error: {0} !\n")]
    IoError(#[from] io::Error),
}

pub async fn parse_from_file<T: AsRef<Path> + Sync>(configuration_file_path: &T) -> Result<MinecraftPackageConfigurationFile, ConfigurationFileParsingError> {
    // todo : evaluate if is better to use a reader
    trace!("prepare reading of minecraft package configuration file at {}", try_path_fmt!(configuration_file_path.as_ref()));
    let mut conf_file = &File::open(configuration_file_path)?;
    let mut conf_file_content = String::new();
    let bytes_read = conf_file.read_to_string(& mut conf_file_content)?;
    trace!("read {bytes_read} bytes from configuration file at {}", try_path_fmt!(configuration_file_path.as_ref()));
    serde_json::from_str(conf_file_content.as_str()).map_err(Into::into)
}

// fixme: move to new trait
pub async fn write_in_file<T: AsRef<Path> + Sync>(minecraft_package: &MinecraftPackage, configuration_file_path: &T) -> Result<MinecraftPackageConfigurationFile, ConfigurationFileParsingError> {
    // todo : evaluate if is better to use a writer
    trace!("prepare writing of minecraft package configuration file at {}", try_path_fmt!(configuration_file_path.as_ref()));
    let minecraft_package_configuration_file: MinecraftPackageConfigurationFile = minecraft_package.into();
    let minecraft_package_configuration_file_content =  serde_json::to_string_pretty(&minecraft_package_configuration_file)?;
    tokio::fs::write(configuration_file_path, minecraft_package_configuration_file_content).await?;
    trace!("wrote configuration file at {}", try_path_fmt!(configuration_file_path.as_ref()));
    Ok(minecraft_package_configuration_file)
}





