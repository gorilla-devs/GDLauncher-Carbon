use napi::bindgen_prelude::*;
use napi_derive::napi;

use super::store_save_loop;

#[napi]
pub async fn init_global_storage() -> Result<()> {
    super::init_global_storage()
        .await
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, format!("{:?}", e)))?;
    store_save_loop();
    Ok(())
}
