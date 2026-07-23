use crate::domain::server::LaunchConfig;
use crate::managers::vtask::Subtask;
use anyhow::{Context, Result, bail};
use carbon_rt_path::ServerPath;
use reqwest_middleware::ClientWithMiddleware;
use std::path::Path;
use tracing::info;

/// Install a modloader for a server. Returns the LaunchConfig to use for launching.
///
/// Fabric uses the self-contained server launcher jar served by the fabric meta API.
/// Quilt, Forge and NeoForge download their installer jar and run it with Java.
///
/// If `progress` is provided, reports staged item-based progress:
/// - Fabric: 2 stages (download, write)
/// - Quilt: 2 stages (download installer, run installer)
/// - Forge/NeoForge: 3 stages (download installer, run installer, finalize)
pub async fn install_modloader(
    reqwest_client: &ClientWithMiddleware,
    server_path: &ServerPath,
    game_version: &str,
    modloader_type: &str,
    modloader_version: &str,
    java_path: &Path,
    progress: Option<&Subtask>,
) -> Result<LaunchConfig> {
    match modloader_type {
        "fabric" => {
            install_fabric(
                reqwest_client,
                server_path,
                game_version,
                modloader_version,
                progress,
            )
            .await
        }
        "quilt" => {
            install_quilt(
                reqwest_client,
                server_path,
                game_version,
                modloader_version,
                java_path,
                progress,
            )
            .await
        }
        "forge" => {
            install_forge(
                reqwest_client,
                server_path,
                game_version,
                modloader_version,
                java_path,
                progress,
            )
            .await
        }
        "neoforge" => {
            install_neoforge(
                reqwest_client,
                server_path,
                game_version,
                modloader_version,
                java_path,
                progress,
            )
            .await
        }
        other => bail!("Unsupported modloader type: {}", other),
    }
}

/// Install Fabric server.
/// Fabric provides a self-contained server launcher jar that includes everything needed.
/// Download from: https://meta.fabricmc.net/v2/versions/loader/{game_version}/{loader_version}/{installer_version}/server/jar
/// The installer version path segment is required; without it the endpoint returns 404.
async fn install_fabric(
    reqwest_client: &ClientWithMiddleware,
    server_path: &ServerPath,
    game_version: &str,
    loader_version: &str,
    progress: Option<&Subtask>,
) -> Result<LaunchConfig> {
    if let Some(p) = progress {
        p.update_items(0, 2);
    }

    let installer_version = latest_fabric_installer_version(reqwest_client)
        .await
        .context("Failed to determine Fabric installer version")?;

    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/{}/server/jar",
        game_version, loader_version, installer_version
    );

    info!("Downloading Fabric server jar from {}", url);

    let response = reqwest_client
        .get(&url)
        .send()
        .await
        .context("Failed to download Fabric server jar")?;

    if !response.status().is_success() {
        bail!(
            "Failed to download Fabric server jar: HTTP {}",
            response.status()
        );
    }

    let bytes = response.bytes().await?;
    if let Some(p) = progress {
        p.update_items(1, 2);
    }
    let jar_path = server_path.get_data_path().join("fabric-server-launch.jar");
    tokio::fs::write(&jar_path, &bytes)
        .await
        .context("Failed to write Fabric server jar")?;

    // Create mods directory
    tokio::fs::create_dir_all(server_path.get_mods_path())
        .await
        .context("Failed to create mods directory")?;

    if let Some(p) = progress {
        p.update_items(2, 2);
    }
    info!("Fabric server jar installed successfully");

    Ok(LaunchConfig {
        jar_path: Some("fabric-server-launch.jar".to_string()),
        ..LaunchConfig::vanilla()
    })
}

/// Install Quilt server.
/// Quilt's meta API has no bundled server jar endpoint. Instead it serves the
/// quilt-installer jar, which is run with `install server` to generate
/// quilt-server-launch.jar and its libraries in the server directory. The
/// generated launcher picks up the vanilla `server.jar` already downloaded
/// next to it.
async fn install_quilt(
    reqwest_client: &ClientWithMiddleware,
    server_path: &ServerPath,
    game_version: &str,
    loader_version: &str,
    java_path: &Path,
    progress: Option<&Subtask>,
) -> Result<LaunchConfig> {
    if let Some(p) = progress {
        p.update_items(0, 2);
    }

    let installer = latest_quilt_installer(reqwest_client)
        .await
        .context("Failed to determine Quilt installer version")?;

    info!(
        "Downloading Quilt installer {} from {}",
        installer.version, installer.url
    );

    let response = reqwest_client
        .get(&installer.url)
        .send()
        .await
        .context("Failed to download Quilt installer")?;

    if !response.status().is_success() {
        bail!(
            "Failed to download Quilt installer: HTTP {}",
            response.status()
        );
    }

    let bytes = response.bytes().await?;
    let data_path = server_path.get_data_path();
    let installer_path = data_path.join("quilt-installer.jar");
    tokio::fs::write(&installer_path, &bytes)
        .await
        .context("Failed to write Quilt installer")?;

    if let Some(p) = progress {
        p.update_items(1, 2);
    }

    info!("Running Quilt installer...");
    let output = tokio::process::Command::new(java_path)
        .arg("-jar")
        .arg("quilt-installer.jar")
        .arg("install")
        .arg("server")
        .arg(game_version)
        .arg(loader_version)
        .arg(format!("--install-dir={}", data_path.display()))
        .current_dir(&data_path)
        .output()
        .await
        .context("Failed to run Quilt installer")?;

    // Clean up installer
    let _ = tokio::fs::remove_file(&installer_path).await;

    if !output.status.success() {
        bail!(
            "Quilt installer exited with code {:?}.\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let launch_jar = data_path.join("quilt-server-launch.jar");
    if !launch_jar.exists() {
        bail!(
            "Quilt installer did not produce quilt-server-launch.jar in {}",
            data_path.display()
        );
    }

    // Create mods directory
    tokio::fs::create_dir_all(server_path.get_mods_path())
        .await
        .context("Failed to create mods directory")?;

    if let Some(p) = progress {
        p.update_items(2, 2);
    }
    info!("Quilt server installed successfully");

    Ok(LaunchConfig {
        jar_path: Some("quilt-server-launch.jar".to_string()),
        ..LaunchConfig::vanilla()
    })
}

#[derive(serde::Deserialize)]
struct FabricInstallerVersion {
    version: String,
    #[serde(default)]
    stable: bool,
}

/// Fetch the newest stable Fabric installer version from the fabric meta API,
/// newest first. The server jar endpoint requires it as a path segment.
async fn latest_fabric_installer_version(reqwest_client: &ClientWithMiddleware) -> Result<String> {
    let versions: Vec<FabricInstallerVersion> = reqwest_client
        .get("https://meta.fabricmc.net/v2/versions/installer")
        .send()
        .await
        .context("Failed to fetch Fabric installer versions")?
        .json()
        .await
        .context("Failed to parse Fabric installer versions")?;

    versions
        .iter()
        .find(|v| v.stable)
        .or_else(|| versions.first())
        .map(|v| v.version.clone())
        .ok_or_else(|| anyhow::anyhow!("Fabric meta returned no installer versions"))
}

#[derive(serde::Deserialize)]
struct QuiltInstaller {
    version: String,
    url: String,
}

/// Fetch the newest Quilt installer (version and jar url) from the quilt meta
/// API, which lists installers newest first.
async fn latest_quilt_installer(reqwest_client: &ClientWithMiddleware) -> Result<QuiltInstaller> {
    let mut installers: Vec<QuiltInstaller> = reqwest_client
        .get("https://meta.quiltmc.org/v3/versions/installer")
        .send()
        .await
        .context("Failed to fetch Quilt installer versions")?
        .json()
        .await
        .context("Failed to parse Quilt installer versions")?;

    if installers.is_empty() {
        bail!("Quilt meta returned no installer versions");
    }
    Ok(installers.remove(0))
}

/// Install Forge server.
/// Forge uses an installer jar that must be executed to set up the server.
/// For now, downloads the installer and runs it.
async fn install_forge(
    reqwest_client: &ClientWithMiddleware,
    server_path: &ServerPath,
    game_version: &str,
    forge_version: &str,
    java_path: &Path,
    progress: Option<&Subtask>,
) -> Result<LaunchConfig> {
    // The Forge maven requires the full `{mc}-{loader}` version in the path,
    // e.g. `1.20.1-47.4.10`. Different callers pass the version in different
    // shapes (modpacks sometimes give only the loader part, the creation modal
    // gives only the loader part too), so normalize here.
    let full_version = if forge_version.starts_with(&format!("{}-", game_version)) {
        forge_version.to_string()
    } else {
        format!("{}-{}", game_version, forge_version)
    };
    let url = format!(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/{}/forge-{}-installer.jar",
        full_version, full_version
    );

    info!("Downloading Forge installer from {}", url);

    let response = reqwest_client
        .get(&url)
        .send()
        .await
        .context("Failed to download Forge installer")?;

    if !response.status().is_success() {
        bail!(
            "Failed to download Forge installer: HTTP {}",
            response.status()
        );
    }

    let bytes = response.bytes().await?;
    let installer_path = server_path.get_data_path().join("forge-installer.jar");
    tokio::fs::write(&installer_path, &bytes)
        .await
        .context("Failed to write Forge installer")?;

    // Run the installer with --installServer using the managed Java, streaming stdout
    // to track processor progress. The Forge installer emits lines like:
    //   "Considering library xxx"
    //   "Processing: xxx" / "  MainClass: xxx" / "  Output: xxx"
    // We count "Processing:" lines as processor steps.
    info!("Running Forge installer...");
    let data_path = server_path.get_data_path();
    run_installer_with_progress(java_path, &data_path, "forge-installer.jar", progress).await?;

    // Clean up installer
    let _ = tokio::fs::remove_file(&installer_path).await;
    // Also clean up installer log
    let log_path = data_path.join("forge-installer.jar.log");
    let _ = tokio::fs::remove_file(&log_path).await;

    // Create mods directory
    tokio::fs::create_dir_all(server_path.get_mods_path())
        .await
        .context("Failed to create mods directory")?;

    info!("Forge server installed successfully");

    // Forge ships a self-contained launcher jar (the "shim" on modern versions,
    // the universal jar on pre-1.17) that knows how to bootstrap itself. Prefer
    // it when present — it needs no argument-file handling at all.
    if let Some(jar) = find_forge_launcher_jar(&data_path).await {
        info!("Using Forge launcher jar {}", jar);
        return Ok(LaunchConfig {
            jar_path: Some(jar),
            ..LaunchConfig::vanilla()
        });
    }

    // Modern Forge (1.17+) without a shim jar: launch through the argument file
    // the installer wrote under libraries/.
    if let Some(args_file) =
        find_loader_args_file(&data_path, "net/minecraftforge/forge", None).await
    {
        info!("Using Forge argument file {}", args_file);
        return Ok(LaunchConfig::from_args_file(args_file));
    }

    // A vanilla fallback here would boot the server without Forge and leave
    // clients unable to join, so surface the failure instead.
    bail!(
        "Forge {} installer completed but produced neither a launcher jar nor a {}",
        forge_version,
        platform_args_file_name()
    )
}

/// Install NeoForge server.
/// NeoForge uses a similar installer approach to Forge.
async fn install_neoforge(
    reqwest_client: &ClientWithMiddleware,
    server_path: &ServerPath,
    _game_version: &str,
    neoforge_version: &str,
    java_path: &Path,
    progress: Option<&Subtask>,
) -> Result<LaunchConfig> {
    // NeoForge installer URL: https://maven.neoforged.net/releases/net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar
    let url = format!(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/{}/neoforge-{}-installer.jar",
        neoforge_version, neoforge_version
    );

    info!("Downloading NeoForge installer from {}", url);

    let response = reqwest_client
        .get(&url)
        .send()
        .await
        .context("Failed to download NeoForge installer")?;

    if !response.status().is_success() {
        bail!(
            "Failed to download NeoForge installer: HTTP {}",
            response.status()
        );
    }

    let bytes = response.bytes().await?;
    let installer_path = server_path.get_data_path().join("neoforge-installer.jar");
    tokio::fs::write(&installer_path, &bytes)
        .await
        .context("Failed to write NeoForge installer")?;

    // Run the installer with --installServer using the managed Java, streaming stdout
    info!("Running NeoForge installer...");
    let data_path = server_path.get_data_path();
    run_installer_with_progress(java_path, &data_path, "neoforge-installer.jar", progress).await?;

    // Clean up installer
    let _ = tokio::fs::remove_file(&installer_path).await;

    // Create mods directory
    tokio::fs::create_dir_all(server_path.get_mods_path())
        .await
        .context("Failed to create mods directory")?;

    info!("NeoForge server installed successfully");

    // NeoForge always launches through an argument file under libraries/.
    if let Some(args_file) =
        find_loader_args_file(&data_path, "net/neoforged/neoforge", Some(neoforge_version)).await
    {
        info!("Using NeoForge argument file {}", args_file);
        return Ok(LaunchConfig::from_args_file(args_file));
    }

    // Falling back to a vanilla config here would silently boot the server
    // without NeoForge, which clients then fail to join with a confusing
    // "server is not running NeoForge" error. Fail the install instead.
    bail!(
        "NeoForge {} installer completed but produced no {} — cannot determine how to launch the server",
        neoforge_version,
        platform_args_file_name()
    )
}

/// Run a Forge/NeoForge installer jar with --installServer, streaming stdout to track
/// processor progress. The installer emits "Processing: ..." lines for each processor
/// it runs. We count these to report granular item-based progress.
///
/// We use a two-phase report:
/// - Before any "Processing:" line is seen, we show item progress 1/N (started, no processors yet).
/// - Once processors start appearing, we update to (1 + seen)/estimated_total and adjust
///   estimated_total if `seen` exceeds it.
async fn run_installer_with_progress(
    java_path: &Path,
    data_path: &Path,
    jar_name: &str,
    progress: Option<&Subtask>,
) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    // Rough initial estimate — most Forge/NeoForge installers run 30-60 processors.
    // We auto-expand if we go over.
    let initial_estimate: u32 = 50;

    if let Some(p) = progress {
        p.update_items(0, initial_estimate);
    }

    let mut child = Command::new(java_path)
        .arg("-jar")
        .arg(jar_name)
        .arg("--installServer")
        .current_dir(data_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn installer")?;

    let stdout = child
        .stdout
        .take()
        .context("Failed to capture installer stdout")?;
    let stderr = child
        .stderr
        .take()
        .context("Failed to capture installer stderr")?;

    // Read stderr to buffer in case of failure
    let stderr_handle = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr);
        let mut buf = String::new();
        let mut full = String::new();
        while reader.read_line(&mut buf).await.unwrap_or(0) > 0 {
            full.push_str(&buf);
            buf.clear();
        }
        full
    });

    // Read stdout line by line, updating progress on "Processing:" lines.
    // Also keep a tail of recent lines so we can include context in error messages.
    let mut reader = BufReader::new(stdout).lines();
    let mut processors_seen: u32 = 0;
    let mut total_estimate: u32 = initial_estimate;
    let mut stdout_tail: std::collections::VecDeque<String> =
        std::collections::VecDeque::with_capacity(30);
    while let Some(line) = reader
        .next_line()
        .await
        .context("Error reading installer stdout")?
    {
        // Forge: "Processing: <library>"
        // NeoForge: also uses "Processing: <library>"
        if line.starts_with("Processing:")
            || line.starts_with("  Processing:")
            || line.contains("] Processing:")
        {
            processors_seen += 1;
            if processors_seen > total_estimate {
                total_estimate = processors_seen + 10;
            }
            if let Some(p) = progress {
                p.update_items(processors_seen, total_estimate);
            }
        }

        // Keep the last 30 lines of stdout for error context
        if stdout_tail.len() >= 30 {
            stdout_tail.pop_front();
        }
        stdout_tail.push_back(line);
    }

    let status = child.wait().await.context("Failed to wait for installer")?;
    let stderr_output = stderr_handle.await.unwrap_or_default();

    if !status.success() {
        let stdout_tail_str: String = stdout_tail.iter().cloned().collect::<Vec<_>>().join("\n");
        let stderr_trimmed = stderr_output.trim();
        let combined = if stderr_trimmed.is_empty() {
            stdout_tail_str
        } else if stdout_tail_str.is_empty() {
            stderr_trimmed.to_string()
        } else {
            format!(
                "stderr:\n{}\n\nstdout (last lines):\n{}",
                stderr_trimmed, stdout_tail_str
            )
        };
        bail!(
            "Installer exited with code {:?}.\n{}",
            status.code(),
            combined
        );
    }

    if let Some(p) = progress {
        p.complete_items();
    }

    Ok(())
}

/// Find Forge's self-contained launcher jar at the data root — the "shim" jar on
/// modern versions, the universal jar on pre-1.17. Excludes the installer jar,
/// which is a different thing that must be *run*, not launched.
async fn find_forge_launcher_jar(data_path: &Path) -> Option<String> {
    let mut entries = tokio::fs::read_dir(data_path).await.ok()?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("forge-") && name.ends_with(".jar") && !name.contains("installer") {
            return Some(name);
        }
    }
    None
}

/// Build a launch config for a modloader that a server pack already ships
/// pre-installed, letting us skip downloading and running the installer
/// entirely. Returns `None` when the loader still has to be installed.
///
/// CurseForge server packs are usually distributed with the loader already
/// unpacked (that is what makes them "server packs" rather than client
/// modpacks), so this is the common path for modpack-created servers.
pub async fn existing_install_launch_config(
    server_path: &ServerPath,
    modloader_type: &str,
    modloader_version: Option<&str>,
) -> Option<LaunchConfig> {
    let data_path = server_path.get_data_path();

    match modloader_type {
        "neoforge" => {
            find_loader_args_file(&data_path, "net/neoforged/neoforge", modloader_version)
                .await
                .map(LaunchConfig::from_args_file)
        }
        "forge" => {
            if let Some(jar) = find_forge_launcher_jar(&data_path).await {
                return Some(LaunchConfig {
                    jar_path: Some(jar),
                    ..LaunchConfig::vanilla()
                });
            }
            find_loader_args_file(&data_path, "net/minecraftforge/forge", modloader_version)
                .await
                .map(LaunchConfig::from_args_file)
        }
        "fabric" | "quilt" => {
            let jar = format!("{modloader_type}-server-launch.jar");
            data_path.join(&jar).exists().then(|| LaunchConfig {
                jar_path: Some(jar),
                ..LaunchConfig::vanilla()
            })
        }
        _ => None,
    }
}

/// Name of the Forge/NeoForge argument file for the current platform. Shared
/// with `modpack`'s pack-detection scan so it requires the same evidence
/// (the argfile actually present) that the launch-time lookup does.
pub(crate) fn platform_args_file_name() -> &'static str {
    if cfg!(windows) {
        "win_args.txt"
    } else {
        "unix_args.txt"
    }
}

/// Locate a Forge/NeoForge argument file, returning its path relative to the
/// server data dir with forward slashes — portable across platforms and safe to
/// hand to the JVM as `@<path>`.
///
/// Modern installers (Forge 1.17+, every NeoForge) write this file under
/// `libraries/<vendor_path>/<loader_version>/`, not at the data root. Callers
/// pass `preferred_version` when the loader version is known so a data dir
/// holding more than one installed version resolves unambiguously.
async fn find_loader_args_file(
    data_path: &Path,
    vendor_path: &str,
    preferred_version: Option<&str>,
) -> Option<String> {
    let file_name = platform_args_file_name();

    let vendor_dir = vendor_path
        .split('/')
        .fold(data_path.join("libraries"), |acc, segment| {
            acc.join(segment)
        });

    let mut versions: Vec<String> = Vec::new();
    if let Ok(mut entries) = tokio::fs::read_dir(&vendor_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if !entry.path().join(file_name).exists() {
                continue;
            }
            if let Some(version) = entry.file_name().to_str() {
                versions.push(version.to_string());
            }
        }
    }

    let chosen = match preferred_version {
        Some(wanted) if versions.iter().any(|v| v == wanted) => Some(wanted.to_string()),
        _ => {
            versions.sort();
            versions.pop()
        }
    };

    if let Some(version) = chosen {
        return Some(format!("libraries/{vendor_path}/{version}/{file_name}"));
    }

    // Legacy layout: some old installers dropped the file at the data root.
    if data_path.join(file_name).exists() {
        return Some(file_name.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build the directory layout that `neoforge-<ver>-installer.jar
    /// --installServer` actually produces: the argument files live under
    /// `libraries/net/neoforged/neoforge/<ver>/`, never at the data root.
    fn write_neoforge_install(data_path: &Path, version: &str) {
        let loader_dir = data_path
            .join("libraries/net/neoforged/neoforge")
            .join(version);
        std::fs::create_dir_all(&loader_dir).unwrap();
        std::fs::write(loader_dir.join("unix_args.txt"), "-p libraries/a.jar").unwrap();
        std::fs::write(loader_dir.join("win_args.txt"), "-p libraries/a.jar").unwrap();
    }

    #[tokio::test]
    async fn finds_neoforge_args_file_under_libraries() {
        // Regression: this lookup used to check only the data root, so it never
        // matched and the server silently fell back to a vanilla launch config.
        let dir = tempfile::tempdir().unwrap();
        write_neoforge_install(dir.path(), "21.1.77");

        let found = find_loader_args_file(dir.path(), "net/neoforged/neoforge", Some("21.1.77"))
            .await
            .expect("argument file should be found under libraries/");

        assert_eq!(
            found,
            format!(
                "libraries/net/neoforged/neoforge/21.1.77/{}",
                platform_args_file_name()
            )
        );
    }

    #[tokio::test]
    async fn args_file_lookup_prefers_the_requested_version() {
        let dir = tempfile::tempdir().unwrap();
        write_neoforge_install(dir.path(), "21.1.77");
        write_neoforge_install(dir.path(), "21.1.80");

        let found = find_loader_args_file(dir.path(), "net/neoforged/neoforge", Some("21.1.77"))
            .await
            .unwrap();

        assert!(found.contains("21.1.77"), "got {found}");
    }

    #[tokio::test]
    async fn preinstalled_neoforge_pack_skips_the_installer() {
        let dir = tempfile::tempdir().unwrap();
        let server_path = ServerPath::new(dir.path().to_path_buf());
        std::fs::create_dir_all(server_path.get_data_path()).unwrap();
        write_neoforge_install(&server_path.get_data_path(), "21.1.77");

        let config = existing_install_launch_config(&server_path, "neoforge", Some("21.1.77"))
            .await
            .expect("a pre-installed pack should yield a launch config");

        assert_eq!(
            config.args_file.as_deref(),
            Some(
                format!(
                    "libraries/net/neoforged/neoforge/21.1.77/{}",
                    platform_args_file_name()
                )
                .as_str()
            )
        );
        // Nothing else should be set — the argument file supplies main class,
        // module path and game args on its own.
        assert_eq!(config.jar_path, None);
        assert_eq!(config.main_class, None);
    }

    #[tokio::test]
    async fn bare_pack_reports_nothing_preinstalled() {
        let dir = tempfile::tempdir().unwrap();
        let server_path = ServerPath::new(dir.path().to_path_buf());
        std::fs::create_dir_all(server_path.get_data_path()).unwrap();

        assert!(
            existing_install_launch_config(&server_path, "neoforge", Some("21.1.77"))
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn preinstalled_forge_prefers_its_launcher_jar() {
        let dir = tempfile::tempdir().unwrap();
        let server_path = ServerPath::new(dir.path().to_path_buf());
        let data_path = server_path.get_data_path();
        std::fs::create_dir_all(&data_path).unwrap();
        std::fs::write(data_path.join("forge-1.20.1-47.4.10-shim.jar"), b"jar").unwrap();
        // The installer jar must never be picked as the thing to launch.
        std::fs::write(data_path.join("forge-1.20.1-47.4.10-installer.jar"), b"jar").unwrap();

        let config = existing_install_launch_config(&server_path, "forge", Some("1.20.1-47.4.10"))
            .await
            .unwrap();

        assert_eq!(
            config.jar_path.as_deref(),
            Some("forge-1.20.1-47.4.10-shim.jar")
        );
    }
}
