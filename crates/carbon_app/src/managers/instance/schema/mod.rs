use serde::{Deserialize, Serialize};

use crate::domain::instance::info;

mod v1;

#[derive(Serialize, Deserialize)]
#[serde(tag = "_version")]
enum InstanceConfig {
    #[serde(rename = "1")]
    V1(v1::Instance),
}

fn patch_creation_update_date(instance: &str) -> Result<InstanceConfig, serde_json::Error> {
    let mut _config: serde_json::Value = serde_json::from_str(instance)?;

    let _config = if _config["created_at"].is_null() {
        let mut _config = _config;
        _config["created_at"] = serde_json::Value::String(chrono::Utc::now().to_string());
        _config
    } else {
        _config
    };

    let _config = if _config["updated_at"].is_null() {
        let mut _config = _config;
        _config["updated_at"] = serde_json::Value::String(chrono::Utc::now().to_string());
        _config
    } else {
        _config
    };

    let _config = serde_json::from_value::<InstanceConfig>(_config);

    return _config;
}

pub fn parse_instance_config(config_str: &str) -> Result<info::Instance, serde_json::Error> {
    let config = serde_json::from_str::<InstanceConfig>(config_str)
        .or_else(|_| patch_creation_update_date(config_str))?;

    Ok(match config {
        InstanceConfig::V1(config) => config.into(),
    })
}

pub fn make_instance_config(info: info::Instance) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&InstanceConfig::V1(info.into()))
}
