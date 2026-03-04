use anyhow::{Context, Result, bail};
use carbon_rt_path::ServerPath;
use reqwest_middleware::ClientWithMiddleware;
use tracing::info;

/// Download a vanilla Minecraft server jar for the given version.
/// Uses the Mojang version manifest to find the download URL.
pub async fn download_vanilla_server_jar(
    client: &ClientWithMiddleware,
    game_version: &str,
    server_path: &ServerPath,
) -> Result<()> {
    let jar_path = server_path.get_server_jar_path();

    // Skip if already downloaded
    if jar_path.exists() {
        info!("Server jar already exists at {}", jar_path.display());
        return Ok(());
    }

    // Ensure data directory exists
    let data_path = server_path.get_data_path();
    tokio::fs::create_dir_all(&data_path)
        .await
        .context("Failed to create server data directory")?;

    // Fetch version manifest
    let manifest_url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
    let manifest: serde_json::Value = client
        .get(manifest_url)
        .send()
        .await
        .context("Failed to fetch version manifest")?
        .json()
        .await
        .context("Failed to parse version manifest")?;

    // Find the version entry
    let versions = manifest["versions"]
        .as_array()
        .context("Invalid manifest format")?;

    let version_entry = versions
        .iter()
        .find(|v| v["id"].as_str() == Some(game_version))
        .context(format!("Version {} not found in manifest", game_version))?;

    let version_url = version_entry["url"]
        .as_str()
        .context("Version entry missing URL")?;

    // Fetch version details
    let version_details: serde_json::Value = client
        .get(version_url)
        .send()
        .await
        .context("Failed to fetch version details")?
        .json()
        .await
        .context("Failed to parse version details")?;

    // Get server download URL
    let server_download = &version_details["downloads"]["server"];
    let download_url = server_download["url"]
        .as_str()
        .context("No server download URL found for this version")?;

    info!(
        "Downloading server jar for {} from {}",
        game_version, download_url
    );

    // Download the jar
    let response = client
        .get(download_url)
        .send()
        .await
        .context("Failed to download server jar")?;

    if !response.status().is_success() {
        bail!(
            "Failed to download server jar: HTTP {}",
            response.status()
        );
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read server jar bytes")?;

    tokio::fs::write(&jar_path, &bytes)
        .await
        .context("Failed to write server jar")?;

    info!(
        "Server jar downloaded successfully ({} bytes)",
        bytes.len()
    );

    Ok(())
}
