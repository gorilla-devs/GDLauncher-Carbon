pub mod v1;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "_version")]
pub enum VersionedManifest {
    #[serde(rename = "1")]
    V1(v1::Instance),
}

pub fn parse_instance_config(config_str: &str) -> Result<v1::Instance, serde_json::Error> {
    let config = serde_json::from_str::<VersionedManifest>(config_str)?;

    Ok(match config {
        VersionedManifest::V1(config) => config,
    })
}

pub fn make_instance_config(info: v1::Instance) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&VersionedManifest::V1(info))
}
