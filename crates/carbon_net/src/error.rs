use thiserror::Error;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("Failed to download {0}")]
    GenericDownload(String),
    #[error("I/O Error {0}")]
    IOError(#[from] std::io::Error),
    #[error("Failed to make network request {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Failed to make network request {0}")]
    NetworkError2(#[from] reqwest_middleware::Error),
    #[error("Join error {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Send error {0}")]
    SendError(#[from] tokio::sync::watch::error::SendError<crate::Progress>),
    #[error("Size mismatch")]
    SizeMismatch { expected: u64, actual: u64 },
    #[error("Checksum mismatch")]
    ChecksumMismatch { expected: String, actual: String },
    #[error("Non 200 status code {0}")]
    Non200StatusCode(u16),
}
