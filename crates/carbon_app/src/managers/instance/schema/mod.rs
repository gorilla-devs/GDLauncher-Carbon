use serde::{Deserialize, Serialize};

use crate::domain::instance::info;

mod v1;

#[derive(Serialize, Deserialize)]
#[serde(tag = "_version")]
enum InstanceConfig {
    #[serde(rename = "1")]
    V1(v1::Instance),
}

pub fn parse_instance_config(config: &str) -> Result<info::Instance, serde_json::Error> {
    let config = serde_json::from_str::<InstanceConfig>(config)?;

    Ok(match config {
        InstanceConfig::V1(config) => config.into(),
    })
}

pub fn make_instance_config(info: info::Instance) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&InstanceConfig::V1(info.into()))
}
