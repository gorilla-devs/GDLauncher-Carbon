use std::path::PathBuf;

use rspc::{Config, RouterBuilderLike};

fn main() {
    use napi_build::setup;
    println!("a");

    carbon_bindings::api::build_router()
        .expose()
        .config(
            Config::new().export_ts_bindings(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bindings.d.ts"),
            ),
        )
        .build();

    setup();
}
