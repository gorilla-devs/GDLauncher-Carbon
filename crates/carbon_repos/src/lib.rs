// Lets the `FromRow` derive emit `carbon_repos::` paths that resolve both
// inside this crate's own modules and in its `tests/` integration crates.
extern crate self as carbon_repos;

// Re-exported so the `#[macro_export]`ed `queries!` muncher can name `paste`
// through `$crate::paste` — integration-test crates that invoke `queries!` see
// only `carbon_repos`'s own items, not its private dependencies, so the path
// must route through the crate itself.
#[doc(hidden)]
pub use paste;

use compat::{MigrationDef, MigrationKind, MigrationSet};

pub mod checker;
pub mod compat;
pub mod db_error;
pub mod db_exec;
pub mod dbtypes;
pub mod downgen;
pub mod fk;
pub mod from_row;
pub mod manifest;
pub mod registry;
pub mod repos;
pub mod schema_dump;

/// Expands to a [`MigrationDef`] for one historical migration directory. The
/// 25 migrations that predate the compatibility floor carry no stored down and
/// are recorded conservatively as `Breaking`: they were authored before down
/// derivation existed, so no older binary ever steps back through them.
macro_rules! historical_migration {
    ($name:literal) => {
        MigrationDef {
            name: $name,
            up_sql: include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/prisma/migrations/",
                $name,
                "/migration.sql"
            )),
            down_sql: None,
            kind: MigrationKind::Breaking,
            data_down: "full",
        }
    };
}

pub fn get_migrations() -> (MigrationSet, i32) {
    let migrations = vec![
        historical_migration!("20240120134904_init"),
        historical_migration!(
            "20240123180711_launcher_action_on_game_launch_game_resolution"
        ),
        historical_migration!("20240126072544_update_modpacks"),
        historical_migration!("20240127230211_add_meta_cache"),
        historical_migration!("20240204033019_add_instances_settings"),
        historical_migration!("20240206064454_downloaddeps"),
        historical_migration!("20240206225900_add_hooks"),
        historical_migration!("20240212215946_fix_java_profiles"),
        historical_migration!("20240220223507_rename_auto_manage_java_for_system_profiles"),
        historical_migration!("20240403131726_add_show_app_close_warning_option"),
        historical_migration!("20240410205605_add_last_app_version_and_updated_at"),
        historical_migration!("20241124163738_gdl_accounts"),
        historical_migration!("20250608012843_add_addon_type_to_mod_file_cache"),
        historical_migration!("20250902113747_remove_show_news_setting"),
        historical_migration!("20251024094741_hashed_email"),
        historical_migration!("20251122000000_remove_hashed_email_accepted"),
        historical_migration!("20251207000000_default_sort_by_created_desc"),
        historical_migration!("20260102000000_add_frontend_preference"),
        historical_migration!("20260120212303_add_gdl_token"),
        historical_migration!("20260122000000_add_library_position"),
        historical_migration!("20260124000000_unify_library_view_modes"),
        historical_migration!("20260223000000_add_servers"),
        historical_migration!("20260325000000_add_server_icon_revision"),
        historical_migration!("20260328000000_add_server_modloader_and_addons"),
        historical_migration!("20260410000000_add_server_modpack_info"),
        // new-migration:anchor — the tool inserts new MigrationDef entries directly above this line
    ];
    let count = migrations.len() as i32;
    (MigrationSet { migrations }, count)
}
