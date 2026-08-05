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

impl DbError {
    /// True when the statement was rejected because a row it referenced does
    /// not exist — SQLite extended result code 787,
    /// `SQLITE_CONSTRAINT_FOREIGNKEY`.
    ///
    /// Worth distinguishing from other write failures because it is routinely
    /// *not* the writer's fault: a parent row can be deleted between the point
    /// a caller captured an id and the point it writes. The metadata cache
    /// does exactly that — it captures `metadataId`s, makes a network round
    /// trip, and saves afterwards, by which time `gc_orphan_metadata` may have
    /// reclaimed the row because the instance holding the mod was deleted.
    /// Callers that would otherwise treat a failed write as "this data is
    /// bad" need to tell the two apart.
    pub fn is_foreign_key_violation(&self) -> bool {
        matches!(
            self,
            DbError::Sqlite(rusqlite::Error::SqliteFailure(e, _))
                if e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY
        )
    }
}

pub type DbResult<T> = Result<T, DbError>;
