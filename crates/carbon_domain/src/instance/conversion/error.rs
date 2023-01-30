use thiserror::Error;

#[derive(Error, Debug)]
pub enum InstanceConversionError {}

trait InstanceConverter {}
