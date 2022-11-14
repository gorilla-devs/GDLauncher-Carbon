use std::str::FromStr;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    ConnectOptions, SqlitePool,
};

use super::{checker::find_java_paths, mc_java::fetch_java_manifest, JAVA_MANIFEST};

#[napi]
pub async fn init_java() -> Result<()> {
    let javas = find_java_paths().await;
    let java_manifest = fetch_java_manifest()
        .await
        .map_err(|err| Error::new(Status::GenericFailure, err.to_string()))?;

    let _ = JAVA_MANIFEST.set(java_manifest);

    let pool = SqliteConnectOptions::from_str("sqlite://data/db.db")
        .map_err(|err| Error::new(Status::GenericFailure, err.to_string()))?
        .journal_mode(SqliteJournalMode::Wal)
        .create_if_missing(true)
        .read_only(false)
        .connect()
        .await
        .map_err(|err| Error::new(Status::GenericFailure, err.to_string()))?;

    Ok(())
}
