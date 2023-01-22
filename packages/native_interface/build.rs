use std::path::PathBuf;

use rspc::{Config, RouterBuilderLike};

fn main() {
    use napi_build::setup;

    println!("cargo:rerun-if-changed=bindings.d.ts");

    carbon_bindings::api::build_rspc_router()
        .expose()
        .config(
            Config::new().export_ts_bindings(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bindings.d.ts"),
            ),
        )
        .build();

    setup();
}
