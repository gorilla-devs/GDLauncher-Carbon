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

/// Read the launch config, re-deriving it from what is installed on disk when
/// it names nothing to launch but the server is configured for a modloader.
///
/// Servers installed before the loader argument-file lookup was corrected hold
/// a config whose launch fields are all null. That file parses cleanly, so the
/// launch path would otherwise keep starting a modded server vanilla, and a
/// server not created from a modpack has no reinstall path to repair it. A
/// re-derived config is written back so the lookup happens once.
pub async fn resolve_launch_config(
    server_path: &ServerPath,
    modloader_type: Option<&str>,
    modloader_version: Option<&str>,
) -> Result<LaunchConfig> {
    let stored = get_launch_config(server_path).await?;

    let names_something_to_launch =
        stored.args_file.is_some() || stored.jar_path.is_some() || stored.main_class.is_some();
    if names_something_to_launch {
        return Ok(stored);
    }

    let Some(loader) = modloader_type else {
        return Ok(stored);
    };

    // No installed loader to derive from: keep the stored config so a genuinely
    // broken install still surfaces as one instead of being papered over.
    let Some(derived) = super::modloader_install::existing_install_launch_config(
        server_path,
        loader,
        modloader_version,
    )
    .await
    else {
        return Ok(stored);
    };

    save_launch_config(server_path, &derived).await?;
    Ok(derived)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::server::LaunchConfig;

    /// The layout `neoforge-<ver>-installer.jar --installServer` produces.
    fn write_neoforge_install(data_path: &std::path::Path, version: &str) {
        let loader_dir = data_path
            .join("libraries/net/neoforged/neoforge")
            .join(version);
        std::fs::create_dir_all(&loader_dir).unwrap();
        std::fs::write(loader_dir.join("unix_args.txt"), "-p libraries/a.jar").unwrap();
        std::fs::write(loader_dir.join("win_args.txt"), "-p libraries/a.jar").unwrap();
    }

    fn server_with_data(dir: &tempfile::TempDir) -> ServerPath {
        let server_path = ServerPath::new(dir.path().to_path_buf());
        std::fs::create_dir_all(server_path.get_data_path()).unwrap();
        server_path
    }

    #[tokio::test]
    async fn rederives_a_config_that_carries_no_launch_override() {
        // A server created before the args-file lookup was fixed has NeoForge
        // installed on disk but a config naming nothing to launch, which parses
        // cleanly and therefore starts the server vanilla.
        let dir = tempfile::tempdir().unwrap();
        let server_path = server_with_data(&dir);
        write_neoforge_install(&server_path.get_data_path(), "21.1.77");
        save_launch_config(&server_path, &LaunchConfig::vanilla())
            .await
            .unwrap();

        let config = resolve_launch_config(&server_path, Some("neoforge"), Some("21.1.77"))
            .await
            .unwrap();

        assert!(
            config.args_file.is_some(),
            "expected the installed argument file, got {config:?}"
        );
    }

    #[tokio::test]
    async fn rederived_config_is_persisted() {
        // Healing must stick, so the cost is paid once rather than every launch.
        let dir = tempfile::tempdir().unwrap();
        let server_path = server_with_data(&dir);
        write_neoforge_install(&server_path.get_data_path(), "21.1.77");
        save_launch_config(&server_path, &LaunchConfig::vanilla())
            .await
            .unwrap();

        resolve_launch_config(&server_path, Some("neoforge"), Some("21.1.77"))
            .await
            .unwrap();

        let reread = get_launch_config(&server_path).await.unwrap();
        assert!(
            reread.args_file.is_some(),
            "the healed config should be written back, got {reread:?}"
        );
    }

    #[tokio::test]
    async fn leaves_a_usable_config_alone() {
        let dir = tempfile::tempdir().unwrap();
        let server_path = server_with_data(&dir);
        write_neoforge_install(&server_path.get_data_path(), "21.1.77");
        let existing = LaunchConfig {
            jar_path: Some("custom-launch.jar".to_string()),
            ..LaunchConfig::vanilla()
        };
        save_launch_config(&server_path, &existing).await.unwrap();

        let config = resolve_launch_config(&server_path, Some("neoforge"), Some("21.1.77"))
            .await
            .unwrap();

        assert_eq!(config.jar_path.as_deref(), Some("custom-launch.jar"));
        assert!(config.args_file.is_none());
    }

    #[tokio::test]
    async fn leaves_a_vanilla_server_vanilla() {
        let dir = tempfile::tempdir().unwrap();
        let server_path = server_with_data(&dir);
        save_launch_config(&server_path, &LaunchConfig::vanilla())
            .await
            .unwrap();

        let config = resolve_launch_config(&server_path, None, None)
            .await
            .unwrap();

        assert!(config.args_file.is_none() && config.jar_path.is_none());
    }

    #[tokio::test]
    async fn leaves_config_alone_when_nothing_is_installed() {
        // Nothing on disk to derive from: keep the stored config rather than
        // inventing one, so a genuinely broken install still reports as such.
        let dir = tempfile::tempdir().unwrap();
        let server_path = server_with_data(&dir);
        save_launch_config(&server_path, &LaunchConfig::vanilla())
            .await
            .unwrap();

        let config = resolve_launch_config(&server_path, Some("neoforge"), Some("21.1.77"))
            .await
            .unwrap();

        assert!(config.args_file.is_none() && config.jar_path.is_none());
    }
}
