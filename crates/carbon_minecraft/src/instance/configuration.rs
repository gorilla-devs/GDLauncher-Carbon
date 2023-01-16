use std::io;
use std::path::Path;
use log::trace;
use thiserror::Error;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use crate::instance::Instance;
use crate::try_path_fmt;

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct InstanceConfiguration {
    pub instance_name: String,
}

impl From<&Instance> for InstanceConfiguration {
    fn from(value: &Instance) -> Self {
        InstanceConfiguration { instance_name: value.name.clone() }
    }
}

#[derive(Error, Debug)]
pub enum ConfigurationFileParsingError {

    #[error("error happened trying to parse json file : {0} !\n")]
    ParsingFileError(#[from] serde_json::Error),

    #[error("io error: {0} !\n")]
    IoError(#[from] io::Error),

}

pub async fn parse_from_file<T : AsRef<Path> + Sync>(configuration_file_path: &T) -> Result<InstanceConfiguration, ConfigurationFileParsingError> {
    // todo : evaluate if is better to use a reader
    trace!("prepare reading of instance configuration file at {}", try_path_fmt!(configuration_file_path.as_ref()));
    let conf_file = &mut tokio::fs::File::open(configuration_file_path).await?;
    let mut conf_file_content = String::new();
    let bytes_read = conf_file.read_to_string(& mut conf_file_content).await?;
    trace!("read {bytes_read} bytes from configuration file at {}", try_path_fmt!(configuration_file_path.as_ref()));
    let instance_configuration : InstanceConfiguration = serde_json::from_str(conf_file_content.as_str())?;
    Ok(instance_configuration)
}

// fixme: move to new trait
pub async fn write_in_file<T : AsRef<Path> + Sync>(instance : &Instance, configuration_file_path: &T) -> Result<InstanceConfiguration, ConfigurationFileParsingError> {
    // todo : evaluate if is better to use a writer
    trace!("prepare writing of instance configuration file at {}", try_path_fmt!(configuration_file_path.as_ref()));
    let instance_configuration_file : InstanceConfiguration = instance.into();
    let instance_configuration_file_content =  serde_json::to_string_pretty(&instance_configuration_file)?;
    tokio::fs::write(configuration_file_path, instance_configuration_file_content).await?;
    trace!("wrote instance configuration file at {}", try_path_fmt!(configuration_file_path.as_ref()));
    Ok(instance_configuration_file)
}


#[cfg(test)]
mod unit_tests {

    #[test]
    fn test_configuration_file_parsing_ok() {
        /* let res = check_configuration_file_sanity(&PathBuf::from("test_assets").join("instance_example")).await;
         let affirmative_check = matches!(res, Ok(_));
         assert!(affirmative_check);*/
    }

    #[test]
    fn test_configuration_file_parsing_fail() {
        /* let res = check_configuration_file_sanity(&PathBuf::from("test_assets").join("malformed_instance_example")).await;
         let denial_check = matches!(res, Err(_)); // todo : add every error case
         assert!(denial_check);*/
    }
}
