use std::{path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{domain::instance::info, managers::AppInner};

mod v1;

#[derive(Serialize, Deserialize)]
#[serde(tag = "_version")]
enum InstanceConfig {
    #[serde(rename = "1")]
    V1(v1::Instance),
}

pub async fn parse_and_update_instance_config(
    app: Arc<AppInner>,
    config_str: &str,
    config_path: PathBuf,
) -> anyhow::Result<info::Instance> {
    let config = parse_instance_config(config_str)?;

    let serialized = make_instance_config(config.clone())?;

    if &serialized != config_str {
        let tmpfile = app
            .settings_manager()
            .runtime_path
            .get_temp()
            .write_file_atomic(config_path, serialized.as_bytes())
            .await?;
    }

    Ok(config)
}

pub fn parse_instance_config(config_str: &str) -> Result<info::Instance, serde_json::Error> {
    let config = serde_json::from_str::<InstanceConfig>(config_str)?;

    Ok(match config {
        InstanceConfig::V1(config) => config.into(),
    })
}

pub fn make_instance_config(info: info::Instance) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&InstanceConfig::V1(info.into()))
}
