use std::borrow::BorrowMut;
use std::io;
use std::path::Path;
use log::trace;
use thiserror::Error;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use crate::instance::Instance;
use crate::try_path_fmt;

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct InstanceConfigurationFile {
    pub instance_name: String,
}

impl From<&Instance> for InstanceConfigurationFile {
    fn from(value: &Instance) -> Self {
        InstanceConfigurationFile{ instance_name: value.name.clone() }
    }
}

#[derive(Error, Debug)]
pub enum ConfigurationFileParsingError {

    #[error("error happened trying to parse json file : {0} !\n")]
    ParsingFileError(#[from] serde_json::Error),

    #[error("io error: {0} !\n")]
    IoError(#[from] io::Error),

}

#[async_trait]
pub(crate) trait ConfigurationFileParser {
    async fn parse_from_file<T : AsRef<Path> + Sync>(configuration_file_path: &T) -> Result<InstanceConfigurationFile, ConfigurationFileParsingError> {
        // todo : evaluate if is better to use a reader
        let mut conf_file = & tokio::fs::File::open(configuration_file_path).await?;
        let mut conf_file_content = &String::new();
        let bytes_read = conf_file.read_to_string(conf_file_content.borrow_mut()).await?;
        trace!("read {bytes_read} bytes from configuration file at {}", try_path_fmt!(configuration_file_path.as_ref()));
        serde_json::from_str(conf_file_content).map_err(Into::into)
    }

    // fixme: move to new trait
    async fn write_in_file<T : AsRef<Path> + Sync>(instance : &Instance, configuration_file_path: &T) -> Result<InstanceConfigurationFile, ConfigurationFileParsingError> {
        // todo : evaluate if is better to use a writer
        let mut conf_file = & tokio::fs::File::create(configuration_file_path).await?;
        let mut conf_file_content = &String::new();
        let instance_configuration_file : InstanceConfigurationFile = instance.into();
        let instance_configuration_file_content =  serde_json::to_string_pretty(&instance_configuration_file)?;
        tokio::fs::write(configuration_file_path, instance_configuration_file_content).await?;
        trace!("wrote configuration file at {}", try_path_fmt!(configuration_file_path.as_ref()));
        Ok(instance_configuration_file)
    }

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
