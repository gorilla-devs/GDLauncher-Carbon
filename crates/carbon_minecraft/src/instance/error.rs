use thiserror::Error;
use crate::instance::configuration::ConfigurationFileParsingError;
use crate::instance::conversion::error::InstanceConversionError;
use crate::instance::delete::InstanceDeleteError;
use crate::instance::scan::InstanceScanError;
use crate::instance::write::InstanceWriteError;

#[derive(Error, Debug)]
pub enum InstanceError {

    #[error("error happened trying to parse json file : {0} !\n")]
    InstanceConversionError(#[from] InstanceConversionError),

    #[error("error happened trying to parse json file : {0} !\n")]
    InstanceConfigurationFileParsingError(#[from] ConfigurationFileParsingError),

    #[error("error happened trying to parse json file : {0} !\n")]
    InstanceScanError(#[from] InstanceScanError),

    #[error("error happened trying to parse json file : {0} !\n")]
    InstanceWriteError(#[from] InstanceWriteError),

    #[error("error happened trying to parse json file : {0} !\n")]
    InstanceDeleteError(#[from] InstanceDeleteError),

}