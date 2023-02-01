use std::path::PathBuf;

fn main() {
    use napi_build::setup;
    setup();

    println!("cargo:rerun-if-changed=../../crates");

    carbon_app::generate_rspc_ts_bindings::generate(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
}
