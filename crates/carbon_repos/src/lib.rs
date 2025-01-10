#![allow(warnings)]
#![allow(dead_code)]

use rusqlite_migration::{Migrations, M};

pub mod db;
mod models;
pub mod pcr; // wip

pub fn get_migrations() -> Migrations<'static> {
    Migrations::new(vec![
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
            "/prisma/migrations/20241124163738_gdl_accounts/migration.sql"
        ))),
    ])
}
