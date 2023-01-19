use std::path::PathBuf;

use rspc::{Config, RouterBuilderLike};

fn main() {
    use napi_build::setup;

    let (_, invalidation_receiver) = tokio::sync::broadcast::channel(200);

    carbon_bindings::api::build_rspc_router(invalidation_receiver)
        .expose()
        .config(
            Config::new().export_ts_bindings(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bindings.d.ts"),
            ),
        )
        .build();

    setup();
}
