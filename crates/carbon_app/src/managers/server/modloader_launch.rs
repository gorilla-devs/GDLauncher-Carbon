use crate::domain::server::LaunchConfig;
use anyhow::{Context, Result};
use carbon_rt_path::ServerPath;

/// Read a LaunchConfig from the server's modloader_config.json.
/// Returns vanilla defaults if the file doesn't exist.
pub async fn get_launch_config(server_path: &ServerPath) -> Result<LaunchConfig> {
    let config_path = server_path.get_modloader_config_path();

    if !config_path.exists() {
        return Ok(LaunchConfig::vanilla());
    }

    let content = tokio::fs::read_to_string(&config_path)
        .await
        .context("Failed to read modloader_config.json")?;

    let config: LaunchConfig =
        serde_json::from_str(&content).context("Failed to parse modloader_config.json")?;

    Ok(config)
}

/// Save a LaunchConfig to the server's modloader_config.json.
pub async fn save_launch_config(server_path: &ServerPath, config: &LaunchConfig) -> Result<()> {
    let config_path = server_path.get_modloader_config_path();
    let content =
        serde_json::to_string_pretty(config).context("Failed to serialize LaunchConfig")?;

    tokio::fs::write(&config_path, content)
        .await
        .context("Failed to write modloader_config.json")?;

    Ok(())
}
