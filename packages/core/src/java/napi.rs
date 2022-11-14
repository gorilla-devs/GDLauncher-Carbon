use napi::bindgen_prelude::*;
use napi_derive::napi;

use super::{checker::find_java_paths, mc_java::fetch_java_manifest, JAVA_MANIFEST};

#[napi]
pub async fn init_java() -> Result<()> {
    let javas = find_java_paths().await;
    let java_manifest = fetch_java_manifest()
        .await
        .map_err(|err| Error::new(Status::GenericFailure, err.to_string()))?;

    let _ = JAVA_MANIFEST.set(java_manifest);

    Ok(())
}
