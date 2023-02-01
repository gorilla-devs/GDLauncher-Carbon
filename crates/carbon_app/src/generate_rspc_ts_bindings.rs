use std::path::PathBuf;

use rspc::{Config, RouterBuilderLike};

#[cfg(debug_assertions)]
pub fn generate(dir: PathBuf) {
    crate::api::build_rspc_router()
        .expose()
        .config(Config::new().export_ts_bindings(dir.join("bindings.d.ts")))
        .build();
}
