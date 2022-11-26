use once_cell::sync::OnceCell;
use serde::{Serialize, Deserialize};

use self::{mc_java::{JavaManifest, fetch_java_manifest}, checker::get_available_javas};

mod checker;
mod utils;
mod mc_java;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct JavaComponent {
    pub path: String,
    pub arch: String,
    /// Indicates whether the component has manually been added by the user
    pub is_custom: bool,
    pub version: JavaVersion
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct JavaVersion {
    pub major: u8,
    pub minor: Option<u8>,
    pub patch: Option<String>,
    pub update_number: Option<String>,
    pub prerelease: Option<String>,
    pub build_metadata: Option<String>,
}

static JAVA_MANIFEST: OnceCell<JavaManifest> = OnceCell::new();

pub async fn init_java() -> Result<Vec<JavaComponent>> {
    let javas = get_available_javas().await.map_err(|e| {
        napi::Error::new(
            napi::Status::GenericFailure,
            format!("Failed to get available Java versions: {}", e),
        )
    })?;

    let java_manifest = fetch_java_manifest()
        .await
        .map_err(|err| Error::new(Status::GenericFailure, err.to_string()))?;

    let _ = JAVA_MANIFEST.set(java_manifest);

    Ok(javas)
}
