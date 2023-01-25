use crate::instance::consts::TEMP_CONFIG_FILE_PREFIX;
use crate::instance::Instance;
use crate::try_path_fmt;
use log::trace;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, SystemTimeError};
use thiserror::Error;
use tokio::io::AsyncReadExt;

use super::InstanceConfigurationFile;

#[derive(Error, Debug)]
pub enum ConfigurationFileParsingError {
    #[error("error happened trying to parse json file : {0} !\n")]
    ParsingFileError(#[from] serde_json::Error),

    #[error("error raised while trying to retrieve system time : {0} !\n")]
    SystemTimeError(#[from] SystemTimeError),

    #[error("io error: {0} !\n")]
    IoError(#[from] io::Error),
}

pub async fn parse_from_file<T: AsRef<Path> + Sync>(
    configuration_file_path: &T,
) -> Result<InstanceConfigurationFile, ConfigurationFileParsingError> {
    trace!(
        "prepare reading of instance configuration file at {}",
        try_path_fmt!(configuration_file_path.as_ref())
    );
    let conf_file = &mut tokio::fs::File::open(configuration_file_path).await?;
    let mut conf_file_content = String::new();
    let bytes_read = conf_file.read_to_string(&mut conf_file_content).await?;
    trace!(
        "read {bytes_read} bytes from configuration file at {}",
        try_path_fmt!(configuration_file_path.as_ref())
    );
    let instance_configuration: InstanceConfigurationFile =
        serde_json::from_str(conf_file_content.as_str())?;
    Ok(instance_configuration)
}

pub async fn write_in_file<T: AsRef<Path> + Sync>(
    instance: &Instance,
    configuration_file_path: &T,
) -> Result<InstanceConfigurationFile, ConfigurationFileParsingError> {
    trace!(
        "prepare writing of instance configuration file at {}",
        try_path_fmt!(configuration_file_path.as_ref())
    );
    let configuration_file_path = configuration_file_path.as_ref();
    let instance_configuration_file: InstanceConfigurationFile = instance.into();
    let instance_configuration_file_content =
        serde_json::to_string_pretty(&instance_configuration_file)?;
    let duration_since_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let timestamp_nanos = duration_since_epoch.as_nanos();
    let temporary_configuration_file_path = PathBuf::from(configuration_file_path)
        .with_file_name(format!("{TEMP_CONFIG_FILE_PREFIX}-{timestamp_nanos}"));
    trace!(
        "writing temporary instance configuration file at {}",
        try_path_fmt!(temporary_configuration_file_path)
    );
    tokio::fs::write(
        &temporary_configuration_file_path,
        instance_configuration_file_content,
    )
    .await?;
    trace!(
        "renaming configuration file at {} in {}",
        try_path_fmt!(temporary_configuration_file_path),
        try_path_fmt!(configuration_file_path)
    );
    tokio::fs::rename(temporary_configuration_file_path, configuration_file_path).await?;
    trace!(
        "wrote instance configuration file at {}",
        try_path_fmt!(configuration_file_path)
    );
    Ok(instance_configuration_file)
}

#[cfg(test)]
mod unit_tests {
    #[test]
    fn test_configuration_file_parsing_ok() {
        /* let res = check_configuration_file_sanity(&PathBuf::from("fixtures").join("instance_example")).await;
        let affirmative_check = matches!(res, Ok(_));
        assert!(affirmative_check);*/
    }

    #[test]
    fn test_configuration_file_parsing_fail() {
        /* let res = check_configuration_file_sanity(&PathBuf::from("fixtures").join("malformed_instance_example")).await;
        let denial_check = matches!(res, Err(_)); // todo : add every error case
        assert!(denial_check);*/
    }
}
