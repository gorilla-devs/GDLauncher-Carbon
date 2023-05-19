use std::path::PathBuf;

use crate::db::{self, PrismaClient};
use thiserror::Error;

use super::java::JavaManager;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("error raised while trying to build the client for DB: {0}")]
    Client(#[from] prisma_client_rust::NewClientError),
    #[error("error while trying to migrate the database")]
    Migration(#[from] prisma_client_rust::migrations::MigrateDeployError),
    #[error("error while trying to push to db")]
    Push(#[from] prisma_client_rust::migrations::DbPushError),
    #[error("error while trying to query db")]
    Query(#[from] prisma_client_rust::QueryError),
    #[error("error while ensuring java profiles in db")]
    EnsureProfiles(anyhow::Error),
}

pub(super) async fn load_and_migrate(runtime_path: PathBuf) -> Result<PrismaClient, DatabaseError> {
    let db_client = db::new_client_with_url(&format!(
        "file:{}",
        runtime_path.join("gdl_conf.db").to_str().unwrap()
    ))
    .await
    .map_err(DatabaseError::Client)?;

    let try_migrate = db_client._migrate_deploy().await;

    #[cfg(not(feature = "production"))]
    {
        if try_migrate.is_err() {
            db_client
                ._db_push()
                .accept_data_loss()
                .force_reset()
                .await?;
        }
    }
    #[cfg(feature = "production")]
    {
        try_migrate.map_err(DatabaseError::Migration)?;
    }

    seed_init_db(&db_client).await?;

    Ok(db_client)
}

async fn seed_init_db(db_client: &PrismaClient) -> Result<(), DatabaseError> {
    // Create base app config
    if db_client.app_configuration().count(vec![]).exec().await? == 0 {
        db_client.app_configuration().create(vec![]).exec().await?;
    }

    JavaManager::ensure_profiles_in_db(db_client)
        .await
        .map_err(DatabaseError::EnsureProfiles)?;

    Ok(())
}
