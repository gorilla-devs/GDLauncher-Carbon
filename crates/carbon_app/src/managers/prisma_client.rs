use std::path::PathBuf;

use crate::{
    app_version::APP_VERSION,
    db::{self, app_configuration, PrismaClient},
};
use prisma_client_rust::raw;
use ring::rand::SecureRandom;
use rusqlite_migration::{Migrations, M};
use serde::Deserialize;
use sysinfo::{System, SystemExt};
use thiserror::Error;
use tracing::{debug, error, instrument, trace};

use super::{java::JavaManager, settings::terms_and_privacy::TermsAndPrivacy};

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("error raised while trying to build the client for DB: {0}")]
    Client(#[from] prisma_client_rust::NewClientError),
    #[error("error while trying to migrate the database")]
    MigrationConn(#[from] rusqlite::Error),
    #[error("error while trying to migrate the database")]
    Migration(#[from] rusqlite_migration::Error),
    #[error("error while trying to query db")]
    Query(#[from] prisma_client_rust::QueryError),
    #[error("error while ensuring java profiles in db")]
    EnsureProfiles(anyhow::Error),
    #[error("error while fetching latest terms and privacy checksum")]
    TermsAndPrivacy(anyhow::Error),
}

#[instrument]
pub(super) async fn load_and_migrate(runtime_path: PathBuf) -> Result<PrismaClient, anyhow::Error> {
    let runtime_path = dunce::simplified(&runtime_path);

    let db_uri = format!(
        "file:{}?connection_limit=1",
        runtime_path.join("gdl_conf.db").to_str().unwrap()
    );

    let migrations = vec![
        M::up(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/prisma/migrations/20240120134904_init/migration.sql"
        ))),
        M::up(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/prisma/migrations/20240123180711_launcher_action_on_game_launch_game_resolution/migration.sql"
        ))),
        M::up(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/prisma/migrations/20240126072544_update_modpacks/migration.sql"
        ))),
        M::up(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/prisma/migrations/20240127230211_add_meta_cache/migration.sql"
        ))),
        M::up(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/prisma/migrations/20240204033019_add_instances_settings/migration.sql"
        ))),
        M::up(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/prisma/migrations/20240206064454_downloaddeps/migration.sql"
        ))),
        M::up(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/prisma/migrations/20240206225900_add_hooks/migration.sql"
        ))),
        M::up(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/prisma/migrations/20240212215946_fix_java_profiles/migration.sql"
        ))),
        M::up(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/prisma/migrations/20240220223507_rename_auto_manage_java_for_system_profiles/migration.sql"
        ))),
        M::up(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/prisma/migrations/20240403131726_add_show_app_close_warning_option/migration.sql"
        ))),
        M::up(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/prisma/migrations/20240410205605_add_last_app_version_and_updated_at/migration.sql"
        ))),
        M::up(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/prisma/migrations/20240802210146_gdl_accounts/migration.sql"
        ))),
    ];

    let migrations = Migrations::new(migrations);

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

async fn seed_init_db(db_client: &PrismaClient) -> Result<(), anyhow::Error> {
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
