use std::path::PathBuf;

use crate::{
    app_version::APP_VERSION,
    db::{self, app_configuration, PrismaClient},
};
use ring::rand::SecureRandom;
use sysinfo::{System, SystemExt};
use thiserror::Error;
use tracing::{debug, instrument, trace};

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

#[instrument]
pub(super) async fn load_and_migrate(
    runtime_path: PathBuf,
) -> Result<PrismaClient, DatabaseError> {
    let runtime_path = dunce::simplified(&runtime_path);

    let db_uri = format!(
        "file:{}?connection_limit=1",
        runtime_path.join("gdl_conf.db").to_str().unwrap()
    );

    debug!("db uri: {}", db_uri);

    let db_client = db::new_client_with_url(&db_uri)
        .await
        .map_err(DatabaseError::Client)?;

    debug!("Trying to migrate database");
    let try_migrate = db_client._migrate_deploy().await;

    #[cfg(debug_assertions)]
    {
        if try_migrate.is_err() {
            debug!("Forcing reset of database");
            db_client
                ._db_push()
                .accept_data_loss()
                .force_reset()
                .await?;
        }
    }
    #[cfg(not(debug_assertions))]
    {
        try_migrate.map_err(DatabaseError::Migration)?;
    }

    seed_init_db(&db_client).await?;

    Ok(db_client)
}

async fn find_appropriate_default_xmx() -> i32 {
    let mut memory = System::new();
    memory.refresh_memory();

    match memory.total_memory() / 1024 / 1024 {
        0..=4096 => 1024,
        4097..=6144 => 2048,
        6145..=8192 => 3072,
        _ => 4096,
    }
}

async fn seed_init_db(db_client: &PrismaClient) -> Result<(), DatabaseError> {
    // Create base app config
    if db_client.app_configuration().count(vec![]).exec().await? == 0 {
        trace!("No app configuration found. Creating default one");

        let mut buf = [0; 256];

        let sr = ring::rand::SystemRandom::new();
        sr.fill(&mut buf).unwrap();

        let release_channel = match APP_VERSION {
            v if v.contains("alpha") => "alpha",
            v if v.contains("beta") => "beta",
            _ => "stable",
        }
        .to_string();

        db_client
            .app_configuration()
            .create(
                release_channel,
                find_appropriate_default_xmx().await,
                Vec::from(buf),
                vec![],
            )
            .exec()
            .await?;
    }

    let metrics_enabled_last_update = db_client
        .app_configuration()
        .find_unique(db::app_configuration::id::equals(0))
        .exec()
        .await?
        .expect("It's unreasonable to expect that the app configuration doesn't exist")
        .metrics_enabled_last_update;

    if let Some(metrics_enabled_last_update) = metrics_enabled_last_update {
        if metrics_enabled_last_update
            < chrono::Utc::now() - chrono::Duration::days(365)
        {
            db_client
                .app_configuration()
                .update(
                    db::app_configuration::id::equals(0),
                    vec![
                        app_configuration::terms_and_privacy_accepted::set(false),
                        app_configuration::terms_and_privacy_accepted_checksum::set(None),
                        app_configuration::metrics_enabled::set(false),
                        app_configuration::metrics_enabled_last_update::set(None),
                    ],
                )
                .exec()
                .await?;
        }
    }

    JavaManager::ensure_profiles_in_db(db_client)
        .await
        .map_err(DatabaseError::EnsureProfiles)?;

    Ok(())
}
