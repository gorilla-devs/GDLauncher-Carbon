pub mod account;
pub mod active_downloads;
pub mod app_configuration;
pub mod frontend_preference;
pub mod http_cache;
pub mod instance;
pub mod java;
pub mod mod_file_cache;
pub mod mod_metadata;
pub mod modpack_cache;
pub mod server;
pub mod skin;
pub mod version_meta;

/// Every registered `QueryCheck` across every repo module above — the single
/// shared source of truth for "every query the checker must validate".
///
/// Callers that need the full set (the in-process checker tests in
/// `tests/query_checker.rs`, and `src/bin/compat_probe.rs`'s cross-version
/// harness entrypoint) call this instead of each hand-maintaining their own
/// copy of this module list: two independently-maintained lists are two
/// places a newly added repo module can be forgotten, silently escaping the
/// checker. `tests/module_registration.rs` mechanically asserts this
/// function's source still extends every module declared above, so even a
/// forgotten `all.extend(...)` line here — the one remaining place a module
/// could still be missed — fails the build rather than passing silently.
pub fn all_queries() -> Vec<crate::registry::QueryCheck> {
    let mut all: Vec<crate::registry::QueryCheck> = Vec::new();
    all.extend(account::all_queries());
    all.extend(active_downloads::all_queries());
    all.extend(app_configuration::all_queries());
    all.extend(frontend_preference::all_queries());
    all.extend(http_cache::all_queries());
    all.extend(instance::all_queries());
    all.extend(java::all_queries());
    all.extend(mod_file_cache::all_queries());
    all.extend(mod_metadata::all_queries());
    all.extend(modpack_cache::all_queries());
    all.extend(server::all_queries());
    all.extend(skin::all_queries());
    all.extend(version_meta::all_queries());
    all
}
