use crate::managers::vtask::Subtask;
use anyhow::{Context, Result, bail};
use carbon_rt_path::ServerPath;
use reqwest_middleware::ClientWithMiddleware;
use tracing::info;

/// Download a vanilla Minecraft server jar for the given version.
/// Uses the Mojang version manifest to find the download URL and expected size.
/// If a `progress` subtask is provided, reports download progress.
pub async fn download_vanilla_server_jar(
    client: &ClientWithMiddleware,
    game_version: &str,
    server_path: &ServerPath,
    progress: Option<&Subtask>,
) -> Result<()> {
    let jar_path = server_path.get_server_jar_path();

    // Skip if already downloaded
    if jar_path.exists() {
        info!("Server jar already exists at {}", jar_path.display());
        if let Some(p) = progress {
            p.complete_opaque();
        }
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

    // Get server download URL and expected size from manifest
    let server_download = &version_details["downloads"]["server"];
    let download_url = server_download["url"]
        .as_str()
        .context("No server download URL found for this version")?;
    let expected_size = server_download["size"].as_u64().unwrap_or(0) as u32;

    info!(
        "Downloading server jar for {} from {} (expected {} bytes)",
        game_version, download_url, expected_size
    );

    // Download the jar with streaming progress
    // Use a raw reqwest::Client (not middleware) to avoid any buffering
    let raw_client = reqwest::Client::new();
    let response = raw_client
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

    // Use manifest size (always available), fall back to Content-Length, then opaque
    let total_size = if expected_size > 0 {
        expected_size
    } else {
        response.content_length().unwrap_or(0) as u32
    };

    if let Some(p) = progress {
        if total_size > 0 {
            p.update_download(0, total_size, false);
        } else {
            p.start_opaque();
        }
    }

    let mut downloaded: u32 = 0;
    let mut last_reported: u32 = 0;
    // Only send progress updates every 0.5% to avoid flooding the WS invalidation channel
    let report_threshold = (total_size / 200).max(64 * 1024);
    let mut file = tokio::fs::File::create(&jar_path)
        .await
        .context("Failed to create server jar file")?;

    use tokio::io::AsyncWriteExt;
    let mut response = response;
    while let Some(chunk) = response
        .chunk()
        .await
        .context("Error reading server jar download chunk")?
    {
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u32;
        if let Some(p) = progress {
            if total_size > 0 && downloaded - last_reported >= report_threshold {
                p.update_download(downloaded, total_size, false);
                last_reported = downloaded;
            }
        }
    }
    file.flush().await?;

    if let Some(p) = progress {
        if total_size > 0 {
            p.complete_download();
        } else {
            p.complete_opaque();
        }
    }

    info!(
        "Server jar downloaded successfully ({} bytes)",
        downloaded
    );

    Ok(())
}
