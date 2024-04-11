use std::path::PathBuf;

use crate::{
    app_version::APP_VERSION,
    db::{self, app_configuration, PrismaClient},
};
use ring::rand::SecureRandom;
use sysinfo::{System, SystemExt};
use thiserror::Error;
use tracing::{debug, instrument, trace};

use super::{java::JavaManager, settings::terms_and_privacy::TermsAndPrivacy};

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
    #[error("error while fetching latest terms and privacy checksum")]
    TermsAndPrivacy(anyhow::Error),
}

#[instrument]
pub(super) async fn load_and_migrate(runtime_path: PathBuf) -> Result<PrismaClient, DatabaseError> {
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
    let release_channel = match APP_VERSION {
        v if v.contains("alpha") => "alpha",
        v if v.contains("beta") => "beta",
        _ => "stable",
    }
    .to_string();

    // Create base app config
    if db_client.app_configuration().count(vec![]).exec().await? == 0 {
        trace!("No app configuration found. Creating default one");

        let mut buf = [0; 256];

        let sr = ring::rand::SystemRandom::new();
        sr.fill(&mut buf).unwrap();

        db_client
            .app_configuration()
            .create(
                release_channel.clone(),
                find_appropriate_default_xmx().await,
                Vec::from(buf),
                vec![app_configuration::last_app_version::set(Some(
                    APP_VERSION.to_string(),
                ))],
            )
            .exec()
            .await?;
    }

    let app_config = db_client
        .app_configuration()
        .find_unique(db::app_configuration::id::equals(0))
        .exec()
        .await?
        .expect("It's unreasonable to expect that the app configuration doesn't exist");

    let mut updates = vec![];

    let is_equal_to_current_version = app_config
        .last_app_version
        .as_ref()
        .map(|last_version| last_version == APP_VERSION)
        .unwrap_or(false);

    let should_force_release_channel =
        if APP_VERSION.contains("alpha") && !is_equal_to_current_version {
            true
        } else if APP_VERSION.contains("beta") && !is_equal_to_current_version {
            true
        } else {
            false
        };

    if should_force_release_channel {
        updates.push(app_configuration::release_channel::set(String::from(release_channel)));
    }

    let is_metrics_consent_too_old = app_config
        .metrics_enabled_last_update
        .map(|last_update| last_update < chrono::Utc::now() - chrono::Duration::days(365))
        .unwrap_or(true);

    let latest_tos_privacy_checksum = TermsAndPrivacy::get_latest_consent_sha()
        .await
        .map_err(DatabaseError::TermsAndPrivacy);

    match latest_tos_privacy_checksum {
        Ok(latest_tos_privacy_checksum) => {
            let mut should_empty_tos_privacy = false;
            let mut should_empty_metrics = false;

            if let Some(metrics_enabled_last_update) = app_config.metrics_enabled_last_update {
                if metrics_enabled_last_update < chrono::Utc::now() - chrono::Duration::days(365) {
                    should_empty_metrics = true;
                }
            }

            if app_config.terms_and_privacy_accepted_checksum
                != Some(latest_tos_privacy_checksum.clone())
            {
                should_empty_tos_privacy = true;
            }

            tracing::info!(
                    "Should empty tos_privacy: {}, should empty metrics: {}, latest tos_privacy checksum: {}, current tos_privacy checksum: {:?}",
                    should_empty_tos_privacy,
                    should_empty_metrics,
                    latest_tos_privacy_checksum,
                    app_config.terms_and_privacy_accepted_checksum
                );

            if should_empty_tos_privacy || should_empty_metrics {
                if should_empty_tos_privacy {
                    updates.push(app_configuration::terms_and_privacy_accepted::set(false));
                    updates.push(app_configuration::terms_and_privacy_accepted_checksum::set(
                        None,
                    ));
                }

                if should_empty_metrics {
                    updates.push(app_configuration::metrics_enabled::set(false));
                    updates.push(app_configuration::metrics_enabled_last_update::set(None));
                }
            }
        }
        Err(err) => {
            tracing::error!(
                "Error while fetching latest terms and privacy checksum: {:?}",
                err
            );
        }
    }

    db_client
        .app_configuration()
        .update(db::app_configuration::id::equals(0), updates)
        .exec()
        .await?;

    JavaManager::ensure_profiles_in_db(db_client)
        .await
        .map_err(DatabaseError::EnsureProfiles)?;

    Ok(())
}
