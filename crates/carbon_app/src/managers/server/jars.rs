use crate::managers::vtask::Subtask;
use anyhow::{Context, Result};
use carbon_net::{Checksum, DownloadOptions, Downloadable};
use carbon_rt_path::ServerPath;
use reqwest_middleware::ClientWithMiddleware;
use std::path::Path;
use tracing::info;

/// Build the `Downloadable` for a server jar from the version manifest's
/// `downloads.server` object, carrying over its checksum/size so carbon_net's
/// downloader can skip an already-valid file and verify a freshly downloaded
/// one actually landed intact instead of trusting mere existence.
fn server_jar_downloadable(
    server_download: &serde_json::Value,
    jar_path: &Path,
) -> Result<Downloadable> {
    let download_url = server_download["url"]
        .as_str()
        .context("No server download URL found for this version")?;
    let expected_size = server_download["size"].as_u64();
    let expected_sha1 = server_download["sha1"].as_str();

    let mut downloadable = Downloadable::new(download_url, jar_path)
        .with_checksum(expected_sha1.map(|s| Checksum::Sha1(s.to_string())));
    if let Some(size) = expected_size {
        downloadable = downloadable.with_size(size);
    }

    Ok(downloadable)
}

/// Download a vanilla Minecraft server jar for the given version.
///
/// Uses the Mojang version manifest to find the download URL, expected size and
/// SHA1 checksum, then downloads through carbon_net's shared downloader — the
/// same retry/connect-timeout/checksum machinery every other download in the
/// app goes through, instead of an unsupervised bare `reqwest::Client` that
/// never verified what actually landed on disk. If a `progress` subtask is
/// provided, reports download progress.
pub async fn download_vanilla_server_jar(
    client: &ClientWithMiddleware,
    game_version: &str,
    server_path: &ServerPath,
    progress: Option<&Subtask>,
) -> Result<()> {
    let jar_path = server_path.get_server_jar_path();

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

    let server_download = &version_details["downloads"]["server"];
    let downloadable = server_jar_downloadable(server_download, &jar_path)?;
    let expected_size = downloadable.size;

    info!(
        "Downloading server jar for {} from {} (expected {:?} bytes, sha1 {:?})",
        game_version,
        downloadable.url,
        expected_size,
        server_download["sha1"].as_str()
    );

    if let Some(p) = progress {
        match expected_size {
            Some(size) => p.update_download(0, size.min(u32::MAX as u64) as u32, false),
            None => p.start_opaque(),
        }
    }

    // Bridge carbon_net's byte-count progress channel into the subtask. Using
    // `tokio::join!` (rather than spawning) lets this run concurrently with
    // the download without requiring `progress` to be `'static`.
    let (progress_tx, mut progress_rx) = tokio::sync::watch::channel(carbon_net::Progress::new());

    let forward_progress = async {
        while progress_rx.changed().await.is_ok() {
            if let Some(p) = progress {
                let snapshot = progress_rx.borrow();
                p.update_download(
                    snapshot.current_size as u32,
                    snapshot.total_size as u32,
                    false,
                );
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    };

    let downloadables = [downloadable];
    let download = carbon_net::download_multiple(
        &downloadables,
        DownloadOptions::builder()
            .concurrency(1)
            .progress_sender(progress_tx)
            .build(),
    );

    let (result, ()) = tokio::join!(download, forward_progress);
    result.context("Failed to download server jar")?;

    if let Some(p) = progress {
        match expected_size {
            Some(_) => p.complete_download(),
            None => p.complete_opaque(),
        }
    }

    info!("Server jar downloaded and verified successfully");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_jar_downloadable_carries_checksum_and_size_from_manifest() {
        // Regression: this used to read only `size`, ignoring the manifest's
        // `sha1` entirely, so a truncated/corrupted download was never caught.
        let server_download = serde_json::json!({
            "sha1": "da39a3ee5e6b4b0d3255bfef95601890afd80709",
            "size": 12345,
            "url": "https://example.com/server.jar",
        });

        let downloadable =
            server_jar_downloadable(&server_download, Path::new("/tmp/server.jar")).unwrap();

        assert_eq!(downloadable.url, "https://example.com/server.jar");
        assert_eq!(downloadable.size, Some(12345));
        match downloadable.checksum {
            Some(Checksum::Sha1(hash)) => {
                assert_eq!(hash, "da39a3ee5e6b4b0d3255bfef95601890afd80709")
            }
            other => panic!("expected a Sha1 checksum, got {other:?}"),
        }
    }

    #[test]
    fn server_jar_downloadable_tolerates_a_manifest_missing_sha1_or_size() {
        // Defensive: an unexpected manifest shape should skip verification of
        // the missing field rather than fail the whole download outright.
        let server_download = serde_json::json!({
            "url": "https://example.com/server.jar",
        });

        let downloadable =
            server_jar_downloadable(&server_download, Path::new("/tmp/server.jar")).unwrap();

        assert_eq!(downloadable.size, None);
        assert!(downloadable.checksum.is_none());
    }

    #[test]
    fn server_jar_downloadable_errors_without_a_url() {
        let server_download = serde_json::json!({ "size": 123 });

        assert!(server_jar_downloadable(&server_download, Path::new("/tmp/server.jar")).is_err());
    }
}
