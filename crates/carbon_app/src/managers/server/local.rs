use super::provider::{ServerHandle, ServerProvider};
use anyhow::{Context, Result};
use async_trait::async_trait;
use carbon_rt_path::ServerPath;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
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
        log_tx: mpsc::UnboundedSender<String>,
    ) -> Result<ServerHandle> {
        let data_path = server_path.get_data_path();
        let jar_path = server_path.get_server_jar_path();

        if !jar_path.exists() {
            anyhow::bail!("Server jar not found at {}", jar_path.display());
        }

        // Accept EULA automatically
        let eula_path = server_path.get_eula_path();
        if !eula_path.exists() {
            tokio::fs::write(&eula_path, "eula=true\n")
                .await
                .context("Failed to write eula.txt")?;
        }

        let mut cmd = tokio::process::Command::new(java_path);
        cmd.arg(format!("-Xmx{}m", xmx))
            .arg(format!("-Xms{}m", xms));

        // Add extra Java args if present
        if !extra_args.is_empty() {
            if let Some(args) = shlex::split(extra_args) {
                for arg in args {
                    cmd.arg(arg);
                }
            }
        }

        cmd.arg("-jar")
            .arg(&jar_path)
            .arg("nogui")
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

        // Set up kill channel
        let (kill_tx, mut kill_rx) = mpsc::channel::<()>(1);
        tokio::spawn(async move {
            tokio::select! {
                _ = kill_rx.recv() => {
                    info!("Kill signal received, terminating server process");
                    let _ = child.kill().await;
                }
                status = child.wait() => {
                    match status {
                        Ok(s) => info!("Server process exited with status: {}", s),
                        Err(e) => error!("Error waiting for server process: {}", e),
                    }
                }
            }
        });

        Ok(ServerHandle {
            process_id: pid,
            kill_tx,
            stdin_tx,
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
