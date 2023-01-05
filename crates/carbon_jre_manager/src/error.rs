use std::backtrace::Backtrace;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum JREError {
    #[error("There is not a valid version of AdoptOpenJDK for your system")]
    NoAdoptOpenJDKMetaValidVersion,
    #[error("Failed to make network request {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("I/O Error {0}")]
    IOError(#[from] std::io::Error),
    #[error("Download checksum mismatch")]
    ChecksumMismatch,
    #[error("Failed to decompress file: {0}")]
    CompressionError(#[from] carbon_compression::CompressionError),
    #[error("Failed to serialise/deserialize JSON: {0}")]
    JSONError(#[from] serde_json::Error),
    #[error("No architecture found in java output")]
    NoArchInJavaOutput,
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
    #[error("Failed to parse int: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Failed to parse String: {0}")]
    ParseStringError(#[from] std::string::ParseError),
    #[error("Failed to parse version")]
    ParseVersionError,
    #[error("Environment variable {0} not found")]
    EnvVarNotFound(#[from] std::env::VarError),
    #[error("Java binary invalid or not found")]
    JavaBinaryInvalidOrNotFound,
    #[error("Failed to parse UTF-8: {0}")]
    ParseUTF8Error(#[from] std::string::FromUtf8Error),
}
