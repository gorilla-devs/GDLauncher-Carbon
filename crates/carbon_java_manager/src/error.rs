use std::backtrace::Backtrace;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum JavaError {
    #[error("There is not a valid version of AdoptOpenJDK for your system")]
    NoAdoptOpenJDKMetaValidVersion,
    #[error("Failed to make network request")]
    NetworkError(#[from] reqwest::Error),
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

    #[error("Cannot run java info detect process {0}")]
    CannotRunJavaInfoDetectProcess(std::io::Error),
    #[error("Cannot write java detect file to disk {0}")]
    CannotWriteJavaDetectFileToDisk(std::io::Error),
    #[error("Cannot read registry key {0}")]
    CannotReadRegistryKey(std::io::Error),

    // AdoptOpenJDK Errors
    #[error("Cannot retrieve OpenJDK assets")]
    CannotRetrieveOpenJDKAssets(reqwest::Error),
    #[error("Cannot parse AdoptOpenJDK meta {0}")]
    CannotParseAdoptOpenJDKMeta(reqwest::Error),
    #[error("Cannot create Java OpenJDK runtime directory {0}")]
    CannotCreateJavaOpenJDKRuntimeDirectory(std::io::Error),
    #[error("Cannot download Java OpenJDK file")]
    CannotDownloadJavaOpenJDK,
    #[error("Cannot delete previously downloaded Java OpenJDK file {0}")]
    CannotDeletePreviouslyDownloadedJavaOpenJDKFile(std::io::Error),

    // Mojang JDK Errors
    #[error("Cannot retrieve Mojang JDK assets")]
    CannotRetrieveMojangJDKAssets(reqwest::Error),
    #[error("Cannot parse Mojang JDK meta {0}")]
    CannotParseMojangJDKMeta(reqwest::Error),
    #[error("Cannot create Java Mojang JDK runtime directory {0}")]
    CannotCreateJavaMojangJDKRuntimeDirectory(std::io::Error),
    #[error("Cannot create Java Mojang JDK file {0}")]
    CannotCreateJavaMojangJDKFile(std::io::Error),
    #[error("Cannot delete previously downloaded Java Mojang JDK file {0}")]
    CannotDeletePreviouslyDownloadedJavaMojangJDKFile(std::io::Error),
    #[error("No Mojang JDK available for the selected os/arch")]
    NoJavaMojangJDKAvailableForOSArch,
    #[error("Cannot retrieve Mojang JDK runtime meta")]
    CannotRetrieveMojangJDKRuntimeMeta(reqwest::Error),
    #[error("Cannot parse Mojang JDK runtime meta {0}")]
    CannotParseMojangJDKRuntimeMeta(reqwest::Error),
    #[error("Cannot create Java Mojang JDK runtime file {0}")]
    CannotCreateJavaMojangJDKRuntimeFile(std::io::Error),

    #[error("Java update date from meta is invalid {0}")]
    JavaUpdateDateFromMetaInvalid(String),
    #[error("Java auto setup version is not supported")]
    JavaAutoSetupVersionNotSupported,
    #[error("No release date provided for java component")]
    NoReleaseDateProvidedForJavaComponent,

    // Scan javas
    #[error("Cannot read Java Runtimes directory {0}")]
    CannotReadJavaRuntimesDirectory(std::io::Error),
}
