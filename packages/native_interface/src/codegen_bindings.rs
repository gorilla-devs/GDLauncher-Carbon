#[cfg(test)]
#[test]
fn generate() {
    use rspc::RouterBuilderLike;
    use std::path::PathBuf;
    carbon_app::api::build_rspc_router()
        .expose()
        .config(
            rspc::Config::new().export_ts_bindings(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bindings.d.ts"),
            ),
        )
        .build();
}
