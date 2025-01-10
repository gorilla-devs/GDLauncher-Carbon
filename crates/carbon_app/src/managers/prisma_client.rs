use super::{java::JavaManager, settings::terms_and_privacy::TermsAndPrivacy};
use crate::app_version::APP_VERSION;
use carbon_repos::db::PrismaClient;
use carbon_repos::db::{self, app_configuration};
use carbon_repos::db::{
    http_cache::{SetParam, WhereParam},
    read_filters::StringFilter,
};
use carbon_repos::pcr::raw;
use ring::rand::SecureRandom;
use serde::Deserialize;
use std::path::PathBuf;
use sysinfo::System;
use thiserror::Error;
use tracing::{debug, error, instrument, trace};

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("error raised while trying to build the client for DB: {0}")]
    Client(#[from] carbon_repos::pcr::NewClientError),
    #[error("error while trying to migrate the database")]
    MigrationConn(#[from] rusqlite::Error),
    #[error("error while trying to migrate the database")]
    Migration(#[from] rusqlite_migration::Error),
    #[error("error while trying to query db")]
    Query(#[from] carbon_repos::pcr::QueryError),
    #[error("error while ensuring java profiles in db")]
    EnsureProfiles(anyhow::Error),
    #[error("error while fetching latest terms and privacy checksum")]
    TermsAndPrivacy(anyhow::Error),
}

#[instrument]
pub(super) async fn load_and_migrate(
    runtime_path: PathBuf,
    gdl_base_api: String,
) -> Result<PrismaClient, anyhow::Error> {
    let runtime_path = dunce::simplified(&runtime_path);

    let db_uri = format!(
        "file:{}?connection_limit=1",
        runtime_path.join("gdl_conf.db").to_str().unwrap()
    );

    let migrations = carbon_repos::get_migrations();

    debug!("db uri: {}", db_uri);

    debug!("Starting migration procedure");

    let mut conn = rusqlite::Connection::open(&db_uri)?;

    let results: Result<i32, _> =
        conn.query_row("SELECT COUNT(*) FROM _prisma_migrations", [], |row| {
            row.get(0)
        });

    let already_existing_migration_count = match results {
        Ok(value) => Some(value),
        Err(err) => None,
    };

    debug!(
        "Found {:?} migrations from prisma. Converting them",
        already_existing_migration_count
    );

    conn.pragma_update(None, "journal_mode", &"WAL").unwrap();

    if let Some(already_existing_migration_count) = already_existing_migration_count {
        conn.pragma_update(None, "user_version", &already_existing_migration_count)?;
    }

    let _ = conn.execute("DROP TABLE IF EXISTS _prisma_migrations", []);

    debug!("Migrating database");

    migrations.to_latest(&mut conn)?;

    debug!("Closing migration connection");

    conn.close().unwrap();

    debug!("Starting prisma connection");

    let db_client = db::new_client_with_url(&db_uri)
        .await
        .map_err(DatabaseError::Client)?;

    #[derive(Deserialize)]
    struct Whatever {}

    let _: Vec<Whatever> = db_client
        ._query_raw(raw!("PRAGMA journal_mode=WAL;"))
        .exec()
        .await
        .unwrap();
    let _: Vec<Whatever> = db_client
        ._query_raw(raw!("PRAGMA synchronous=normal;"))
        .exec()
        .await
        .unwrap();
    let _: Vec<Whatever> = db_client
        ._query_raw(raw!("PRAGMA temp_store=MEMORY;"))
        .exec()
        .await
        .unwrap();
    let _: Vec<Whatever> = db_client
        ._query_raw(raw!("PRAGMA mmap_size = 30000000000;"))
        .exec()
        .await
        .unwrap();

    seed_init_db(&db_client, gdl_base_api).await?;

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

async fn seed_init_db(db_client: &PrismaClient, gdl_base_api: String) -> Result<(), anyhow::Error> {
    let release_channel = match APP_VERSION {
        v if v.contains("alpha") => "alpha",
        v if v.contains("beta") => "beta",
        _ => "stable",
    }
    .to_string();

    // Create base app config
    if db_client.app_configuration().count(vec![]).exec().await? == 0 {
        trace!("No app configuration found. Creating default one");

        db_client
            .app_configuration()
            .create(
                release_channel.clone(),
                find_appropriate_default_xmx().await,
                vec![app_configuration::last_app_version::set(Some(
                    APP_VERSION.to_string(),
                ))],
            )
            .exec()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create default app configuration: {e}"))?;

        trace!("Created default app configuration");
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
        updates.push(app_configuration::release_channel::set(String::from(
            release_channel,
        )));
    }

    let latest_tos_privacy_checksum = TermsAndPrivacy::get_latest_consent_sha(gdl_base_api)
        .await
        .map_err(DatabaseError::TermsAndPrivacy);

    match latest_tos_privacy_checksum {
        Ok(latest_tos_privacy_checksum) => {
            let mut should_empty_tos_privacy = false;

            if app_config.terms_and_privacy_accepted_checksum
                != Some(latest_tos_privacy_checksum.clone())
            {
                should_empty_tos_privacy = true;
            }

            tracing::info!(
                    "Should empty tos_privacy: {}, latest tos_privacy checksum: {}, current tos_privacy checksum: {:?}",
                    should_empty_tos_privacy,
                    latest_tos_privacy_checksum,
                    app_config.terms_and_privacy_accepted_checksum
                );

            if should_empty_tos_privacy {
                updates.push(app_configuration::terms_and_privacy_accepted::set(false));
                updates.push(app_configuration::terms_and_privacy_accepted_checksum::set(
                    None,
                ));
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

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_migrate_tos_privacy_should_reset_status_200() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock = server
            .mock("GET", "/v1/latest_consent_checksum")
            .with_status(200)
            .with_body("1234567890")
            .expect(1)
            .create_async()
            .await;

        let temp_dir = tempdir::TempDir::new("carbon_app_test").unwrap();
        let temp_path = dunce::canonicalize(temp_dir.into_path()).unwrap();

        let db_client = load_and_migrate(temp_path.clone(), mock_url.to_string())
            .await
            .unwrap();

        db_client
            .app_configuration()
            .update(
                db::app_configuration::id::equals(0),
                vec![
                    db::app_configuration::terms_and_privacy_accepted_checksum::set(Some(
                        "something else".to_string(),
                    )),
                ],
            )
            .exec()
            .await
            .unwrap();

        drop(db_client);

        let db_client = load_and_migrate(temp_path, mock_url.to_string())
            .await
            .unwrap();

        assert_eq!(
            db_client
                .app_configuration()
                .find_unique(db::app_configuration::id::equals(0))
                .exec()
                .await
                .unwrap()
                .unwrap()
                .terms_and_privacy_accepted_checksum,
            None
        );
    }

    #[tokio::test]
    async fn test_migrate_tos_privacy_should_not_reset_status_500() {
        let mut server_500 = mockito::Server::new_async().await;
        let mock_url_500 = server_500.url();
        let mut server_200 = mockito::Server::new_async().await;
        let mock_url_200 = server_200.url();

        server_500
            .mock("GET", "/v1/latest_consent_checksum")
            .with_status(500)
            .with_body("1234567890")
            .expect(1)
            .create_async()
            .await;

        server_200
            .mock("GET", "/v1/latest_consent_checksum")
            .with_status(200)
            .with_body("1234567890")
            .expect(1)
            .create_async()
            .await;

        let temp_dir = tempdir::TempDir::new("carbon_app_test").unwrap();
        let temp_path = dunce::canonicalize(temp_dir.into_path()).unwrap();

        let db_client = load_and_migrate(temp_path.clone(), mock_url_200)
            .await
            .unwrap();

        db_client
            .app_configuration()
            .update(
                db::app_configuration::id::equals(0),
                vec![
                    db::app_configuration::terms_and_privacy_accepted_checksum::set(Some(
                        "something else".to_string(),
                    )),
                ],
            )
            .exec()
            .await
            .unwrap();

        drop(db_client);

        // Since it's a 500 we should not reset the status
        let db_client = load_and_migrate(temp_path, mock_url_500).await.unwrap();

        assert_eq!(
            db_client
                .app_configuration()
                .find_unique(db::app_configuration::id::equals(0))
                .exec()
                .await
                .unwrap()
                .unwrap()
                .terms_and_privacy_accepted_checksum,
            Some("something else".to_string())
        );
    }
}
