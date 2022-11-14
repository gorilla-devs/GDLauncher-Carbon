use std::str::FromStr;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    ConnectOptions, SqlitePool,
};

#[napi]
pub async fn init_global_storage() -> Result<()> {
    // Ensure the data folder exists first 🤡
    std::fs::create_dir_all("./data")?;
    let pool = SqliteConnectOptions::from_str("sqlite://data/_store.db")
        .map_err(|err| Error::new(Status::GenericFailure, err.to_string()))?
        .journal_mode(SqliteJournalMode::Wal)
        .create_if_missing(true)
        .read_only(false)
        .connect()
        .await
        .map_err(|err| Error::new(Status::GenericFailure, err.to_string()))?;

    Ok(())
}
