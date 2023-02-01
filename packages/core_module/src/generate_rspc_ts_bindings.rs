use std::path::PathBuf;

#[test]
fn generate() {
    carbon_app::generate_rspc_ts_bindings::generate(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
}
