use std::path::PathBuf;

use crate::db::{self, PrismaClient};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("error raised while trying to build the client for DB: {0}")]
    ClientError(#[from] prisma_client_rust::NewClientError),
    #[error("error while trying to migrate the database")]
    MigrationError(#[from] prisma_client_rust::migrations::MigrateDeployError),
}

pub(super) async fn load_and_migrate(runtime_path: PathBuf) -> Result<PrismaClient, DatabaseError> {
    let db_client = db::new_client_with_url(&format!(
        "file:{}",
        runtime_path.join("gdl_conf.db").to_str().unwrap()
    ))
    .await
    .map_err(DatabaseError::ClientError)?;

    let try_migrate = db_client._migrate_deploy().await;

    #[cfg(not(feature = "production"))]
    {
        if try_migrate.is_err() {
            db_client
                ._db_push()
                .accept_data_loss()
                .force_reset()
                .await
                .unwrap();
        }
    }
    #[cfg(feature = "production")]
    {
        try_migrate.map_err(DatabaseError::MigrationError)?;
    }

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
