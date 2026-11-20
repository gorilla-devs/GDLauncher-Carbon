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

#[cfg(test)]
mod tests {
    use super::*;

    const BEFORE_THE_JAVA_FLOOR: &str = r#"{
        "_version": "1",
        "name": "Old",
        "game_configuration": {
            "version": { "release": "1.20.1", "modloaders": [] },
            "java_override": { "Path": "/opt/jdk-21/bin/java" }
        }
    }"#;

    #[test]
    fn the_java_floor_persists_and_older_configs_load_without_one() {
        let mut config = parse_instance_config(BEFORE_THE_JAVA_FLOOR).unwrap();
        assert_eq!(config.game_configuration.java_floor, None);
        assert!(
            !make_instance_config(config.clone())
                .unwrap()
                .contains("java_floor")
        );

        config.game_configuration.java_floor = Some(21);
        let saved = make_instance_config(config).unwrap();
        assert_eq!(
            parse_instance_config(&saved)
                .unwrap()
                .game_configuration
                .java_floor,
            Some(21)
        );
    }
}
