//! Freezes the 25 shipped (pre-floor) migrations' names and checksums as
//! string literals.
//!
//! `compat.rs`'s `checksum()` hashes a migration's raw `up_sql` bytes, and
//! that hash is the only thing `first_divergent` trusts to prove a database's
//! recorded history matches this binary's. Nothing before this test pinned
//! those 25 hashes anywhere: CI's other migration coverage (`schema_snapshot`,
//! `baseline`, `cross_version`) compares *schema*, not bytes, so an incidental
//! future edit to a shipped `migration.sql` — a reformat, an added comment, a
//! trailing newline picked up by the repo's `.editorconfig` — changes the hash
//! without changing the schema. Every other test stays green; every existing
//! install then computes a checksum that disagrees with the one recorded in
//! its own `_migrations` table when the migration first applied, which
//! `compat::MigrationSet::open` reports as `Diverged` — a fatal refusal whose
//! only effective recovery rung is "Reset Database" (wipes accounts,
//! settings, everything).
//!
//! This test closes that gap the only way that is actually enforceable: by
//! asserting the exact ordered list of `(name, sha256_hex)` this binary
//! computes right now, as literals, so any future change to a shipped
//! migration's bytes fails this test immediately, in this crate, before it
//! ships.
//!
//! Shipped migration.sql files must never be edited once released — see
//! `prisma/migrations/README.md`. `src/bin/new_migration.rs` appends the new
//! `(name, checksum)` tuple here automatically when it scaffolds/derives a new
//! migration (see its doc comment); never edit an existing tuple by hand.

use carbon_repos::get_migrations;

/// `(migration directory name, sha256 hex of its `migration.sql`/`up_sql`)`,
/// in the exact order `get_migrations()` carries them. Append new entries
/// directly above the closing bracket; never edit an existing one.
const FROZEN: &[(&str, &str)] = &[
    (
        "20240120134904_init",
        "141923835027ae95faa0f4b9c0b9fba02189d2c0f26e9317a5977788c6a2aac7",
    ),
    (
        "20240123180711_launcher_action_on_game_launch_game_resolution",
        "eceed46d9a7ab5c767cd3708259c890b090675dcf1d804896108b1704323f077",
    ),
    (
        "20240126072544_update_modpacks",
        "9b0d93f04bdde8f3f1d4ee0e5d8a189213d7cac3a7c82fe6be05b0708c52b7a7",
    ),
    (
        "20240127230211_add_meta_cache",
        "728ca7a4de6ab61b52a4ec3dcf35bb923c7ae48141875709f01c4c1becead11e",
    ),
    (
        "20240204033019_add_instances_settings",
        "714dfdbce5bc71314e916334f9030930e6d07c52551164e07e669f7484f6d895",
    ),
    (
        "20240206064454_downloaddeps",
        "a4b0ee49ebcf640b989e3a7d0c5b5b6d31da686d3ffce58b62feedb7400e8075",
    ),
    (
        "20240206225900_add_hooks",
        "78605fff22243080b55d5b283b9869d57ac3895c7c3b538edaa0b43005730173",
    ),
    (
        "20240212215946_fix_java_profiles",
        "4ebe2af59b8164f771d84a183415b39567b58b2beab87bf9184d9313482bbf9d",
    ),
    (
        "20240220223507_rename_auto_manage_java_for_system_profiles",
        "ad0d26d0030b594ed94e1f617272b324f96b7ac291327da89ffd4d0037e1a7af",
    ),
    (
        "20240403131726_add_show_app_close_warning_option",
        "a775ccef85c16de04ccf1f8297c61604a5d262e7602d917225a5f88fa0abac06",
    ),
    (
        "20240410205605_add_last_app_version_and_updated_at",
        "7928d975eb0d8c0133974ebe0f7b553fead6f070e052819c450b1ca69da4466e",
    ),
    (
        "20241124163738_gdl_accounts",
        "1b9430ac9553115e8b67a607124d7e350cd37f79ad8976652a9396c1fa9fca32",
    ),
    (
        "20250608012843_add_addon_type_to_mod_file_cache",
        "2dda7ff0130272e7776355b89c2d4b3e12a98e6ca9c3916d3170aa4b74048675",
    ),
    (
        "20250902113747_remove_show_news_setting",
        "26e3bd3ceaa9fed3082a404c2136b9ca7f9a116e33f55bd1b57c96d2fa037548",
    ),
    (
        "20251024094741_hashed_email",
        "2d858a808b37018a06c9f39d7e1a94e539f5449b958d1d66ef4b65228ec8617c",
    ),
    (
        "20251122000000_remove_hashed_email_accepted",
        "236595908f9e33b103388b7af245497026a25e962ae6f062e4495cf83eaea32a",
    ),
    (
        "20251207000000_default_sort_by_created_desc",
        "6ad0b0c60468f472447290945803729f6fd8f3d629e5d39b8a0f8eae6e9efa81",
    ),
    (
        "20260102000000_add_frontend_preference",
        "9f909d2f898afb841f785b9542d36d5aec2975a88c5f49799f60f440d570fab3",
    ),
    (
        "20260120212303_add_gdl_token",
        "9a3729c1e281f71d4422810316d347b5faebd7ced6fbc25f64604090c2448ec5",
    ),
    (
        "20260122000000_add_library_position",
        "5233a54689a9e8d8d3ea26045d1c3f75f127eeba62dc707db6a7c77666ffcc6f",
    ),
    (
        "20260124000000_unify_library_view_modes",
        "4b75abf12629204f8db971fd606fadd2f17457e4bd0e0e6803d8dc3b101555ae",
    ),
    (
        "20260223000000_add_servers",
        "357345d7cda14ba14a8e0935880942d107dcc59dc5adb6327b503d677515b3e2",
    ),
    (
        "20260325000000_add_server_icon_revision",
        "36e644977bbc2ad597840fdf75c57132d3552d8bc5329a07f9ff4aa3f053f05b",
    ),
    (
        "20260328000000_add_server_modloader_and_addons",
        "d6e626bdee317ecaa025329b97c32a49b645d235e9affaa03cd0b2450108791f",
    ),
    (
        "20260410000000_add_server_modpack_info",
        "0c1160f20ea909a746ad9716360e71f29df538e4116d6cf8a3a4aa75c54c55d3",
    ),
    // new-migration:anchor — new_migration appends the new (name, checksum) tuple directly above this line
];

#[test]
fn shipped_migration_checksums_are_frozen() {
    let (set, count) = get_migrations();
    assert_eq!(
        count as usize,
        FROZEN.len(),
        "get_migrations() now carries a different number of migrations than FROZEN. \
         If this is a new migration, append its (name, checksum) tuple to FROZEN (never \
         edit an existing one) — new_migration does this automatically when it scaffolds \
         or derives a migration."
    );

    for (i, def) in set.migrations.iter().enumerate() {
        let (frozen_name, frozen_checksum) = FROZEN[i];
        assert_eq!(
            def.name,
            frozen_name,
            "migration at position {} was renamed or reordered (expected `{}`). Shipped \
             migrations must never be renamed or reordered once released.",
            i + 1,
            frozen_name
        );
        let actual_checksum = set.checksum((i + 1) as i32);
        assert_eq!(
            actual_checksum, frozen_checksum,
            "migration `{}`'s up_sql bytes no longer match the checksum frozen at release. \
             Every existing install recorded the old checksum in its own _migrations table \
             when this migration first applied, so it would now compute a mismatch and \
             compat::MigrationSet::open would refuse the database as Diverged — the only \
             recovery rung offered from there is Reset Database (a full wipe). Shipped \
             migration.sql files are immutable; see prisma/migrations/README.md.",
            def.name
        );
    }
}
