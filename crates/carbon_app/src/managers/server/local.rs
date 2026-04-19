use super::provider::{ServerHandle, ServerProvider};
use crate::domain::server::LaunchConfig;
use anyhow::{Context, Result};
use async_trait::async_trait;
use carbon_rt_path::ServerPath;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Notify};
use tracing::{error, info};

pub struct LocalServerProvider;

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
        log_tx: mpsc::UnboundedSender<String>,
    ) -> Result<ServerHandle> {
        let data_path = server_path.get_data_path();

        let mut cmd = tokio::process::Command::new(java_path);
        cmd.arg(format!("-Xmx{}m", xmx))
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

        if let Some(main_class) = &launch_config.main_class {
            // Modded: use classpath + main class (Forge/NeoForge pattern)
            if !launch_config.classpath.is_empty() {
                let separator = if cfg!(windows) { ";" } else { ":" };
                let classpath = launch_config.classpath.join(separator);
                cmd.arg("-cp").arg(&classpath);
            }
            cmd.arg(main_class);
        } else {
            // Vanilla or Fabric/Quilt: use -jar
            let jar_name = launch_config
                .jar_path
                .as_deref()
                .unwrap_or("server.jar");
            let jar_path = data_path.join(jar_name);

            if !jar_path.exists() {
                // Fallback to default server.jar
                let default_jar = server_path.get_server_jar_path();
                if !default_jar.exists() {
                    anyhow::bail!("Server jar not found at {}", jar_path.display());
                }
                cmd.arg("-jar").arg(&default_jar);
            } else {
                cmd.arg("-jar").arg(&jar_path);
            }
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
        let log_tx_stderr = log_tx;
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
        let exit_notify = Arc::new(Notify::new());
        let exit_notify_clone = exit_notify.clone();
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
                        Ok(s) => info!("Server process exited with status: {}", s),
                        Err(e) => error!("Error waiting for server process: {}", e),
                    }
                }
            }
            exit_notify_clone.notify_waiters();
        });

        Ok(ServerHandle {
            process_id: pid,
            kill_tx,
            stdin_tx,
            exit_notify,
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
