use super::provider::{ServerHandle, ServerProvider, exit_signal, remove_pid_file, write_pid_file};
use crate::domain::server::LaunchConfig;
use anyhow::{Context, Result};
use async_trait::async_trait;
use carbon_rt_path::ServerPath;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tracing::{error, info};

pub struct LocalServerProvider;

/// Clamp the server's configured heap settings (MB) into values safe to hand
/// the JVM. Saturating instead of a raw `as u16` cast avoids e.g. 66000 MB
/// silently wrapping into a tiny heap; flooring both at 1 MB and capping xms
/// to xmx keeps a zero/negative/inverted setting from making the JVM refuse
/// to start outright, which would otherwise crash the server on every boot
/// (and feed the auto-restart loop).
fn clamp_heap_mb(xms: i32, xmx: i32) -> (u16, u16) {
    let xmx = xmx.clamp(1, u16::MAX as i32) as u16;
    let xms = xms.clamp(1, xmx as i32) as u16;
    (xms, xmx)
}

#[async_trait]
impl ServerProvider for LocalServerProvider {
    async fn start(
        &self,
        java_path: &Path,
        server_path: &ServerPath,
        xmx: i32,
        xms: i32,
        extra_args: &str,
        launch_config: &LaunchConfig,
        modloader_type: Option<&str>,
        log_tx: mpsc::UnboundedSender<String>,
    ) -> Result<ServerHandle> {
        let data_path = server_path.get_data_path();
        let server_root = server_path.get_root();

        let (xms, xmx) = clamp_heap_mb(xms, xmx);

        let mut cmd = tokio::process::Command::new(java_path);
        // Without this, a process still running when the app exits without an
        // orderly per-server shutdown (crash, force-quit) is never signalled and
        // is orphaned as a live JVM.
        cmd.kill_on_drop(true)
            .arg(format!("-Xmx{}m", xmx))
            .arg(format!("-Xms{}m", xms));

        // Add extra JVM args from modloader config
        for arg in &launch_config.extra_jvm_args {
            cmd.arg(arg);
        }

        // Add user extra Java args
        if !extra_args.is_empty() {
            if let Some(args) = shlex::split(extra_args) {
                for arg in args {
                    cmd.arg(arg);
                }
            }
        }

        if let Some(args_file) = &launch_config.args_file {
            // Modern Forge/NeoForge: hand the argument file to the JVM and let
            // it expand the tokens, exactly as the installer's own run.sh does.
            // The file supplies the module path, main class and game args.
            let args_file_path = data_path.join(args_file);
            if !args_file_path.exists() {
                anyhow::bail!(
                    "Modloader argument file not found at {}. The server install may be incomplete — try reinstalling it.",
                    args_file_path.display()
                );
            }
            cmd.arg(format!("@{}", args_file));
        } else if let Some(main_class) = &launch_config.main_class {
            // Modded: use classpath + main class (Forge/NeoForge pattern)
            if !launch_config.classpath.is_empty() {
                let separator = if cfg!(windows) { ";" } else { ":" };
                let classpath = launch_config.classpath.join(separator);
                cmd.arg("-cp").arg(&classpath);
            }
            cmd.arg(main_class);
        } else if let Some(jar_name) = &launch_config.jar_path {
            // Fabric/Quilt: a loader-specific launcher jar was resolved by name.
            let jar_path = data_path.join(jar_name);

            if !jar_path.exists() {
                // The resolved jar has since gone missing. For a modded server
                // this must not silently fall back to the untouched vanilla
                // server.jar — refuse instead of masking a broken install.
                if let Some(modloader) = modloader_type {
                    anyhow::bail!(
                        "Modded server has no valid launch configuration for {modloader} (expected {} to exist) — reinstall the server to repair it.",
                        jar_path.display()
                    );
                }

                let default_jar = server_path.get_server_jar_path();
                if !default_jar.exists() {
                    anyhow::bail!("Server jar not found at {}", jar_path.display());
                }
                cmd.arg("-jar").arg(&default_jar);
            } else {
                cmd.arg("-jar").arg(&jar_path);
            }
        } else {
            // Nothing loader-specific was resolved at all (no args_file, no
            // main_class, no jar_path). For a genuine vanilla server this is
            // the normal case — launch server.jar. For a modded server
            // (modloader_type set) it means the launch config never got
            // populated (interrupted install, or a config predating the
            // args-file lookup): server.jar is always present (it's
            // downloaded unconditionally at create time) regardless of
            // modloader, so silently launching it here would boot a vanilla
            // server that modded clients cannot join. Refuse instead.
            if let Some(modloader) = modloader_type {
                anyhow::bail!(
                    "Modded server has no valid launch configuration for {modloader} — reinstall the server to repair it."
                );
            }

            let default_jar = server_path.get_server_jar_path();
            if !default_jar.exists() {
                anyhow::bail!("Server jar not found at {}", default_jar.display());
            }
            cmd.arg("-jar").arg(&default_jar);
        }

        // Add extra game args from modloader config
        for arg in &launch_config.extra_game_args {
            cmd.arg(arg);
        }

        cmd.arg("nogui")
            .current_dir(&data_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().context("Failed to spawn server process")?;
        let pid = child.id().unwrap_or(0);

        info!("Server process started with PID {}", pid);

        // Best-effort: record the pid so a future `load_servers` pass can
        // detect and kill this JVM if the core exits without going through
        // `stop`/`kill` first (crash, force-quit, Windows TerminateProcess —
        // none of which run the kill/wait task below). Never blocks or fails
        // the launch on write failure.
        write_pid_file(&server_root, pid).await;

        // Set up stdin channel
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(64);
        let mut stdin = child.stdin.take().expect("Failed to take stdin");

        tokio::spawn(async move {
            while let Some(command) = stdin_rx.recv().await {
                let line = if command.ends_with('\n') {
                    command
                } else {
                    format!("{}\n", command)
                };
                if let Err(e) = stdin.write_all(line.as_bytes()).await {
                    error!("Failed to write to server stdin: {}", e);
                    break;
                }
                let _ = stdin.flush().await;
            }
        });

        // Set up stdout reading
        let stdout = child.stdout.take().expect("Failed to take stdout");
        let log_tx_stdout = log_tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if log_tx_stdout.send(line).is_err() {
                    break;
                }
            }
        });

        // Set up stderr reading
        let stderr = child.stderr.take().expect("Failed to take stderr");
        let log_tx_stderr = log_tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if log_tx_stderr.send(format!("[STDERR] {}", line)).is_err() {
                    break;
                }
            }
        });

        // Set up kill channel and exit notification
        let (kill_tx, mut kill_rx) = mpsc::channel::<()>(1);
        let (exited_tx, exited) = exit_signal();
        let log_tx_exit = log_tx;
        let exit_server_root = server_root.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = kill_rx.recv() => {
                    info!("Kill signal received, terminating server process");
                    let _ = child.kill().await;
                    // Wait for process to fully exit after kill
                    let _ = child.wait().await;
                }
                status = child.wait() => {
                    match status {
                        Ok(s) => {
                            info!("Server process exited with status: {}", s);
                            // Surface to in-app console so users (and bug reports)
                            // see the actual exit status, not just "exited unexpectedly".
                            let _ = log_tx_exit
                                .send(format!("[GDLauncher] Server process exited: {}", s));
                        }
                        Err(e) => {
                            error!("Error waiting for server process: {}", e);
                            let _ = log_tx_exit
                                .send(format!("[GDLauncher] Error waiting for server process: {}", e));
                        }
                    }
                }
            }
            // Best-effort: the process is gone either way (killed or exited
            // on its own), so the pidfile no longer refers to anything a
            // future `load_servers` pass needs to clean up.
            remove_pid_file(&exit_server_root).await;
            let _ = exited_tx.send(true);
        });

        Ok(ServerHandle {
            process_id: pid,
            kill_tx,
            stdin_tx,
            exited,
        })
    }

    async fn stop(&self, handle: &ServerHandle) -> Result<()> {
        handle
            .stdin_tx
            .send("stop".to_string())
            .await
            .context("Failed to send stop command to server")?;
        Ok(())
    }

    async fn kill(&self, handle: &ServerHandle) -> Result<()> {
        handle
            .kill_tx
            .send(())
            .await
            .context("Failed to send kill signal to server")?;
        Ok(())
    }

    async fn send_command(&self, handle: &ServerHandle, command: &str) -> Result<()> {
        handle
            .stdin_tx
            .send(command.to_string())
            .await
            .context("Failed to send command to server")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heap_clamp_saturates_and_orders() {
        // Absurdly large values saturate instead of wrapping (a raw `as u16`
        // cast would turn 66000 into 464).
        assert_eq!(clamp_heap_mb(1024, 66000), (1024, u16::MAX));
        // xms is capped to xmx rather than left inverted.
        assert_eq!(clamp_heap_mb(8192, 1024), (1024, 1024));
        // Zero/negative settings are floored at 1 MB so the JVM never refuses
        // to start on a nonsensical heap.
        assert_eq!(clamp_heap_mb(0, 0), (1, 1));
        assert_eq!(clamp_heap_mb(-100, -50), (1, 1));
        // A normal, already-sane pair passes through unchanged.
        assert_eq!(clamp_heap_mb(1024, 4096), (1024, 4096));
    }

    #[tokio::test]
    async fn modded_server_with_no_launch_config_refuses_to_boot_vanilla() {
        // Regression: a modded server whose launch config resolved to nothing
        // (no args_file, no main_class, no jar_path) used to fall through to
        // `-jar server.jar` — silently booting vanilla instead of surfacing
        // the broken install. The vanilla server.jar always exists (it's
        // downloaded unconditionally at create time), so this must be caught
        // before ever reaching `cmd.spawn()`.
        let dir = tempfile::tempdir().unwrap();
        let server_path = ServerPath::new(dir.path().to_path_buf());
        std::fs::create_dir_all(server_path.get_data_path()).unwrap();
        std::fs::write(server_path.get_server_jar_path(), b"jar").unwrap();

        let (log_tx, _log_rx) = mpsc::unbounded_channel();
        let provider = LocalServerProvider;
        let result = provider
            .start(
                // Never reached if the guard fires correctly — spawning this
                // would fail anyway (ENOENT), but with a different message.
                Path::new("/nonexistent/gdl-test-java-binary"),
                &server_path,
                1024,
                1024,
                "",
                &LaunchConfig::vanilla(),
                Some("neoforge"),
                log_tx,
            )
            .await;

        let err = result.expect_err("expected the modded-vanilla-fallback guard to fire");
        let msg = err.to_string();
        assert!(
            msg.contains("Modded server has no valid launch configuration"),
            "got: {msg}"
        );
    }

    #[tokio::test]
    async fn vanilla_server_without_modloader_type_still_attempts_to_launch() {
        // A genuine vanilla server (modloader_type: None) must still reach the
        // spawn attempt instead of being caught by the modded-only guard.
        let dir = tempfile::tempdir().unwrap();
        let server_path = ServerPath::new(dir.path().to_path_buf());
        std::fs::create_dir_all(server_path.get_data_path()).unwrap();
        std::fs::write(server_path.get_server_jar_path(), b"jar").unwrap();

        let (log_tx, _log_rx) = mpsc::unbounded_channel();
        let provider = LocalServerProvider;
        let result = provider
            .start(
                Path::new("/nonexistent/gdl-test-java-binary"),
                &server_path,
                1024,
                1024,
                "",
                &LaunchConfig::vanilla(),
                None,
                log_tx,
            )
            .await;

        let err = result.expect_err("spawning a nonexistent binary must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("Failed to spawn server process"),
            "expected a spawn failure (proving the vanilla path was allowed through), got: {msg}"
        );
    }
}
