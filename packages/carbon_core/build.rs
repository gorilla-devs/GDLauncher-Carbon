use std::path::PathBuf;

use rspc::Config;

fn main() {
    use napi_build::setup;

    carbon_bindings::build_router()
        .config(
            Config::new().export_ts_bindings(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bindings.d.ts"),
            ),
        )
        .build();

    setup();
}
