use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("database executor is shut down")]
    Closed,
    #[error("row conversion failed: {0}")]
    Conversion(String),
}

pub type DbResult<T> = Result<T, DbError>;
