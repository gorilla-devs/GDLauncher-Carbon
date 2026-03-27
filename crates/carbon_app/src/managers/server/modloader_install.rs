use crate::domain::server::LaunchConfig;
use anyhow::{Context, Result, bail};
use carbon_rt_path::ServerPath;
use reqwest_middleware::ClientWithMiddleware;
use tracing::info;

/// Install a modloader for a server. Returns the LaunchConfig to use for launching.
///
/// Currently supports Fabric and Quilt (self-contained server jars).
/// Forge and NeoForge require processor execution and are more complex.
pub async fn install_modloader(
    reqwest_client: &ClientWithMiddleware,
    server_path: &ServerPath,
    game_version: &str,
    modloader_type: &str,
    modloader_version: &str,
) -> Result<LaunchConfig> {
    match modloader_type {
        "fabric" => install_fabric(reqwest_client, server_path, game_version, modloader_version).await,
        "quilt" => install_quilt(reqwest_client, server_path, game_version, modloader_version).await,
        "forge" => install_forge(reqwest_client, server_path, game_version, modloader_version).await,
        "neoforge" => install_neoforge(reqwest_client, server_path, game_version, modloader_version).await,
        other => bail!("Unsupported modloader type: {}", other),
    }
}

/// Install Fabric server.
/// Fabric provides a self-contained server launcher jar that includes everything needed.
/// Download from: https://meta.fabricmc.net/v2/versions/loader/{game_version}/{loader_version}/server/jar
async fn install_fabric(
    reqwest_client: &ClientWithMiddleware,
    server_path: &ServerPath,
    game_version: &str,
    loader_version: &str,
) -> Result<LaunchConfig> {
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/server/jar",
        game_version, loader_version
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
    let jar_path = server_path.get_data_path().join("fabric-server-launch.jar");
    tokio::fs::write(&jar_path, &bytes)
        .await
        .context("Failed to write Fabric server jar")?;

    // Create mods directory
    tokio::fs::create_dir_all(server_path.get_mods_path())
        .await
        .context("Failed to create mods directory")?;

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
/// Quilt also provides a self-contained server launcher jar.
/// Download from: https://meta.quiltmc.org/v3/versions/loader/{game_version}/{loader_version}/server/jar
async fn install_quilt(
    reqwest_client: &ClientWithMiddleware,
    server_path: &ServerPath,
    game_version: &str,
    loader_version: &str,
) -> Result<LaunchConfig> {
    let url = format!(
        "https://meta.quiltmc.org/v3/versions/loader/{}/{}/server/jar",
        game_version, loader_version
    );

    info!("Downloading Quilt server jar from {}", url);

    let response = reqwest_client
        .get(&url)
        .send()
        .await
        .context("Failed to download Quilt server jar")?;

    if !response.status().is_success() {
        bail!(
            "Failed to download Quilt server jar: HTTP {}",
            response.status()
        );
    }

    let bytes = response.bytes().await?;
    let jar_path = server_path.get_data_path().join("quilt-server-launch.jar");
    tokio::fs::write(&jar_path, &bytes)
        .await
        .context("Failed to write Quilt server jar")?;

    // Create mods directory
    tokio::fs::create_dir_all(server_path.get_mods_path())
        .await
        .context("Failed to create mods directory")?;

    info!("Quilt server jar installed successfully");

    Ok(LaunchConfig {
        jar_path: Some("quilt-server-launch.jar".to_string()),
        main_class: None,
        classpath: Vec::new(),
        extra_jvm_args: Vec::new(),
        extra_game_args: Vec::new(),
    })
}

/// Install Forge server.
/// Forge uses an installer jar that must be executed to set up the server.
/// For now, downloads the installer and runs it.
async fn install_forge(
    reqwest_client: &ClientWithMiddleware,
    server_path: &ServerPath,
    game_version: &str,
    forge_version: &str,
) -> Result<LaunchConfig> {
    // Forge installer URL format: https://maven.minecraftforge.net/net/minecraftforge/forge/{mc}-{forge}/forge-{mc}-{forge}-installer.jar
    let version_string = format!("{}-{}", game_version, forge_version);
    let url = format!(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/{}/forge-{}-installer.jar",
        version_string, version_string
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

    // Run the installer with --installServer
    info!("Running Forge installer...");
    let data_path = server_path.get_data_path();
    let status = tokio::process::Command::new("java")
        .arg("-jar")
        .arg("forge-installer.jar")
        .arg("--installServer")
        .current_dir(&data_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .status()
        .await
        .context("Failed to run Forge installer")?;

    if !status.success() {
        bail!("Forge installer failed with exit code: {:?}", status.code());
    }

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
    let args_file = data_path.join("libraries").join("net").join("minecraftforge");

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

    // Run the installer with --installServer
    info!("Running NeoForge installer...");
    let data_path = server_path.get_data_path();
    let status = tokio::process::Command::new("java")
        .arg("-jar")
        .arg("neoforge-installer.jar")
        .arg("--installServer")
        .current_dir(&data_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .status()
        .await
        .context("Failed to run NeoForge installer")?;

    if !status.success() {
        bail!(
            "NeoForge installer failed with exit code: {:?}",
            status.code()
        );
    }

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
