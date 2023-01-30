use std::path::Path;

use thiserror::Error;

use crate::db::{self, PrismaClient};

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("error raised while trying to build the client for DB: {0}")]
    ClientError(#[from] prisma_client_rust::NewClientError),
    #[error("error while trying to migrate the database")]
    MigrationError(#[from] prisma_client_rust::migrations::MigrateDeployError),
}

pub(super) async fn load_and_migrate() -> Result<PrismaClient, DatabaseError> {
    let db_client = db::new_client_with_url(&format!(
        "file:{}",
        std::env::current_dir()
            .unwrap()
            .join("gdl_conf.db")
            .to_str()
            .unwrap()
    ))
    .await
    .map_err(DatabaseError::ClientError)?;

    db_client
        ._migrate_deploy()
        .await
        .map_err(DatabaseError::MigrationError)?;

    // Add default settings if they don't exist
    if db_client
        .app_configuration()
        .count(vec![])
        .exec()
        .await
        .unwrap()
        == 0
    {
        db_client
            .app_configuration()
            .create(vec![])
            .exec()
            .await
            .unwrap();
    }

    Ok(db_client)
}
