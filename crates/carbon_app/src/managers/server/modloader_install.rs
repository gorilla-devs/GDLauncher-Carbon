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
        main_class: None,
        classpath: Vec::new(),
        extra_jvm_args: Vec::new(),
        extra_game_args: Vec::new(),
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
        main_class: None,
        classpath: Vec::new(),
        extra_jvm_args: Vec::new(),
        extra_game_args: Vec::new(),
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
async fn latest_fabric_installer_version(
    reqwest_client: &ClientWithMiddleware,
) -> Result<String> {
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
async fn latest_quilt_installer(
    reqwest_client: &ClientWithMiddleware,
) -> Result<QuiltInstaller> {
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

    // Modern Forge (1.17+) uses a run script / @libraries approach.
    // The installer creates various files. We look for the forge server jar or run args.
    // Check for run.sh/run.bat or forge-*-server.jar
    let args_file = data_path
        .join("libraries")
        .join("net")
        .join("minecraftforge");

    // For modern Forge, check if there are unix_args.txt or win_args.txt
    let unix_args_path = data_path.join("unix_args.txt");
    let win_args_path = data_path.join("win_args.txt");

    if unix_args_path.exists() || win_args_path.exists() {
        // Modern Forge: parse the args file to get classpath and main class
        let args_path = if cfg!(unix) {
            &unix_args_path
        } else {
            &win_args_path
        };

        if args_path.exists() {
            let content = tokio::fs::read_to_string(args_path).await?;
            return parse_forge_args(&content);
        }
    }

    // Legacy Forge (pre-1.17): look for forge-*-server.jar
    let mut forge_jar = None;
    let mut entries = tokio::fs::read_dir(&data_path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("forge-") && name.ends_with(".jar") && !name.contains("installer") {
            forge_jar = Some(name);
            break;
        }
    }

    if let Some(jar) = forge_jar {
        return Ok(LaunchConfig {
            jar_path: Some(jar),
            main_class: None,
            classpath: Vec::new(),
            extra_jvm_args: Vec::new(),
            extra_game_args: Vec::new(),
        });
    }

    // Fallback: assume the installer set things up with default naming
    Ok(LaunchConfig {
        jar_path: None,
        main_class: None,
        classpath: Vec::new(),
        extra_jvm_args: Vec::new(),
        extra_game_args: Vec::new(),
    })
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

    // NeoForge also uses unix_args.txt / win_args.txt for modern versions
    let unix_args_path = data_path.join("unix_args.txt");
    let win_args_path = data_path.join("win_args.txt");

    let args_path = if cfg!(unix) {
        &unix_args_path
    } else {
        &win_args_path
    };

    if args_path.exists() {
        let content = tokio::fs::read_to_string(args_path).await?;
        return parse_forge_args(&content);
    }

    // Fallback
    Ok(LaunchConfig::vanilla())
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

/// Parse Forge/NeoForge unix_args.txt or win_args.txt into a LaunchConfig.
/// The format is typically lines with JVM args, then @libraries/..., then main class.
fn parse_forge_args(content: &str) -> Result<LaunchConfig> {
    let mut jvm_args = Vec::new();
    let mut classpath = Vec::new();
    let mut main_class = None;
    let mut game_args = Vec::new();
    let mut past_main_class = false;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with("@") {
            // @libraries file reference - this is the classpath file
            // Read it if it exists, or treat as a JVM arg
            jvm_args.push(line.to_string());
            continue;
        }

        if line.starts_with("-") {
            if past_main_class {
                game_args.push(line.to_string());
            } else {
                jvm_args.push(line.to_string());
            }
            continue;
        }

        // If it looks like a class name (contains dots, no dashes)
        if line.contains('.') && !line.starts_with('-') && main_class.is_none() {
            main_class = Some(line.to_string());
            past_main_class = true;
            continue;
        }

        if past_main_class {
            game_args.push(line.to_string());
        }
    }

    Ok(LaunchConfig {
        jar_path: None, // Forge/NeoForge use -cp + main class, not -jar
        main_class,
        classpath,
        extra_jvm_args: jvm_args,
        extra_game_args: game_args,
    })
}
