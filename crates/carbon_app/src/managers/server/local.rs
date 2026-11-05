use super::provider::{ExitSignal, ServerHandle, ServerProvider, exit_signal, write_pid_file};
use crate::domain::server::LaunchConfig;
use crate::managers::orphan_pid;
use anyhow::{Context, Result};
use async_trait::async_trait;
use carbon_rt_path::ServerPath;
use std::path::Path;
use std::process::Stdio;
use sysinfo::{Pid, ProcessesToUpdate, System};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

pub struct LocalServerProvider;

/// Converts a server's configured heap setting (MB) into a value safe to hand
/// the JVM. A zero/negative setting is a corrupt config — failing loudly here
/// beats silently booting the JVM with a nonsensical heap, which would
/// otherwise crash the server on every boot (and feed the auto-restart loop).
/// There is no upper clamp: the JVM validates its own `-Xmx`/`-Xms` bounds
/// and rejects an unreasonable value with a clear error, so a deliberately
/// large heap (e.g. 100 GB on a big host) reaches it unclamped instead of
/// being silently rewritten down to ~464 MB by a raw `as u16` cast.
fn heap_mb(v: i32) -> Result<u32> {
    if v <= 0 {
        anyhow::bail!("Server heap size must be positive, got {v} MB");
    }
    Ok(v as u32)
}

/// How long a server's stdout/stderr may take to reach EOF once its JVM has
/// exited. Everything the JVM printed is already in the pipe by then, so only a
/// helper process that inherited the pipes keeps them open longer. Kept below
/// the time `provider::drain` gives a forced exit, so Kill still confirms.
const POST_EXIT_OUTPUT_GRACE: std::time::Duration = std::time::Duration::from_secs(2);

// The exit of a killed server is confirmed only once its output closed or
// this grace ran out. With a grace as long as drain's forced-exit wait, a
// Kill of a server whose helper process holds the pipes would report its
// exit unconfirmed and leave the server Stopping, and no test would notice.
const _: () =
    assert!(POST_EXIT_OUTPUT_GRACE.as_millis() < super::provider::FORCED_EXIT_CONFIRM.as_millis());

const OUTPUT_STAYED_OPEN: &str = "[GDLauncher] Server output stayed open after exit (a helper process may still be running); later output was not captured";

/// The code page a Windows JVM uses for piped output: the ANSI code page on
/// Java 8/17, and for anything printed through `System.out` on newer Java.
#[cfg(windows)]
fn system_output_encoding() -> Option<&'static encoding_rs::Encoding> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetACP() -> u32;
    }
    // SAFETY: GetACP takes no arguments and only reads process-wide state.
    encoding_for_code_page(unsafe { GetACP() })
}

/// Unix JVMs print in the locale's charset, which is UTF-8 almost everywhere.
#[cfg(not(windows))]
fn system_output_encoding() -> Option<&'static encoding_rs::Encoding> {
    None
}

/// ANSI code pages a JVM may print in. UTF-8 (65001) and anything unusual
/// return `None`, which decodes lossily as UTF-8.
fn encoding_for_code_page(code_page: u32) -> Option<&'static encoding_rs::Encoding> {
    Some(match code_page {
        874 => encoding_rs::WINDOWS_874,
        932 => encoding_rs::SHIFT_JIS,
        936 => encoding_rs::GBK,
        949 => encoding_rs::EUC_KR,
        950 => encoding_rs::BIG5,
        1250 => encoding_rs::WINDOWS_1250,
        1251 => encoding_rs::WINDOWS_1251,
        1252 => encoding_rs::WINDOWS_1252,
        1253 => encoding_rs::WINDOWS_1253,
        1254 => encoding_rs::WINDOWS_1254,
        1255 => encoding_rs::WINDOWS_1255,
        1256 => encoding_rs::WINDOWS_1256,
        1257 => encoding_rs::WINDOWS_1257,
        1258 => encoding_rs::WINDOWS_1258,
        _ => return None,
    })
}

/// Decodes one console line without ever failing. UTF-8 lines are taken as
/// they are; any other line is decoded with `fallback` (the system code page)
/// when there is one, otherwise invalid bytes become U+FFFD. Per line, because
/// log4j on newer Java writes UTF-8 while `System.out` still uses the code page.
fn decode_output_line(line: &[u8], fallback: Option<&'static encoding_rs::Encoding>) -> String {
    let line = match line.strip_suffix(b"\n") {
        Some(line) => line.strip_suffix(b"\r").unwrap_or(line),
        None => line,
    };
    match std::str::from_utf8(line) {
        Ok(text) => text.to_owned(),
        Err(_) => match fallback {
            Some(encoding) => encoding.decode_without_bom_handling(line).0.into_owned(),
            None => String::from_utf8_lossy(line).into_owned(),
        },
    }
}

/// Publishes a server stream line by line until EOF. A byte that is not valid
/// in the expected encoding never ends the console; a read error is logged
/// and treated as the end of output, so it cannot hold up the exit.
async fn forward_output(
    reader: impl tokio::io::AsyncRead + Unpin,
    sender: mpsc::UnboundedSender<String>,
    prefix: &str,
    fallback: Option<&'static encoding_rs::Encoding>,
) {
    let mut reader = BufReader::new(reader);
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line).await {
            Ok(0) => return,
            Ok(_) => {
                let text = decode_output_line(&line, fallback);
                if sender.send(format!("{prefix}{text}")).is_err() {
                    // Nobody collects this run's console any more.
                    return;
                }
            }
            Err(error) => {
                warn!("Stopped reading server output after a read error: {error}");
                return;
            }
        }
    }
}

/// Channels of a supervised server process.
struct Supervised {
    kill_tx: mpsc::Sender<()>,
    stdin_tx: mpsc::Sender<String>,
    exited: ExitSignal,
}

/// Wires a spawned server's stdin, stdout and stderr and supervises it until
/// exit. A confirmed exit (a successful `wait`) is always published, whatever
/// state the output readers are in; only a failed `wait` leaves it unconfirmed.
fn supervise(
    mut child: tokio::process::Child,
    log_tx: mpsc::UnboundedSender<String>,
    ownership: ActivityOwnership,
    output_grace: std::time::Duration,
) -> Supervised {
    let fallback = system_output_encoding();

    // Set up stdin channel
    let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(64);
    let mut stdin = child.stdin.take().expect("Failed to take stdin");

    let stdin_task = tokio::spawn(async move {
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
    let stdout_task =
        tokio::spawn(async move { forward_output(stdout, log_tx_stdout, "", fallback).await });

    // Set up stderr reading
    let stderr = child.stderr.take().expect("Failed to take stderr");
    let log_tx_stderr = log_tx.clone();
    let stderr_task =
        tokio::spawn(
            async move { forward_output(stderr, log_tx_stderr, "[STDERR] ", fallback).await },
        );

    // Set up kill channel and exit notification
    let (kill_tx, mut kill_rx) = mpsc::channel::<()>(1);
    let (exited_tx, exited) = exit_signal();

    tokio::spawn(async move {
        let status = tokio::select! {
            Some(()) = kill_rx.recv() => {
                info!("Kill signal received, terminating server process");
                let _ = child.kill().await;
                // Wait for process to fully exit after kill
                child.wait().await
            }
            status = child.wait() => status,
        };
        let status = match status {
            Ok(status) => status,
            Err(e) => {
                error!("Error waiting for server process: {}", e);
                let _ = log_tx.send(format!(
                    "[GDLauncher] Error waiting for server process: {}",
                    e
                ));
                // The exit is unconfirmed: keep the activity and the pid
                // marker so the run stays Unknown instead of looking stopped.
                return;
            }
        };
        info!("Server process exited with status: {}", status);
        stdin_task.abort();
        let _ = stdin_task.await;
        publish_confirmed_exit(
            [stdout_task, stderr_task],
            log_tx,
            // Surface to in-app console so users (and bug reports) see the
            // actual exit status, not just "exited unexpectedly".
            format!("[GDLauncher] Server process exited: {}", status),
            &ownership,
            exited_tx,
            output_grace,
        )
        .await;
    });

    Supervised {
        kill_tx,
        stdin_tx,
        exited,
    }
}

type ActivityOwnership =
    std::sync::Arc<std::sync::Mutex<Option<crate::managers::runtime_lifecycle::RuntimeActivity>>>;

/// Runs once `wait` has confirmed the server process exited. Gives the output
/// readers `grace` to reach EOF, then aborts any still blocked (a helper that
/// inherited the pipes can hold them open indefinitely) so their pipe ends and
/// log sender clones drop. Reader errors are ignored: the exit is published
/// either way, and the pid marker and activity are retired by `finalize_run`.
async fn publish_confirmed_exit<T>(
    readers: [tokio::task::JoinHandle<T>; 2],
    log_tx: mpsc::UnboundedSender<String>,
    exit_line: String,
    ownership: &ActivityOwnership,
    exited_tx: tokio::sync::watch::Sender<bool>,
    grace: std::time::Duration,
) {
    let deadline = tokio::time::Instant::now() + grace;
    let mut stayed_open = false;
    for mut reader in readers {
        // A JoinHandle must not be awaited again once it has yielded, so only
        // a reader that missed the deadline is aborted and awaited here.
        if tokio::time::timeout_at(deadline, &mut reader)
            .await
            .is_err()
        {
            reader.abort();
            let _ = reader.await;
            stayed_open = true;
        }
    }
    if stayed_open {
        warn!(
            "Server output stayed open {grace:?} after the process exited; a helper process may still hold the pipes"
        );
        let _ = log_tx.send(OUTPUT_STAYED_OPEN.to_owned());
    }
    let _ = log_tx.send(exit_line);
    if let Some(activity) = ownership.lock().unwrap().as_ref() {
        activity.settling();
    }
    let _ = exited_tx.send(true);
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
        mut runtime_activity: crate::managers::runtime_lifecycle::RuntimeActivity,
    ) -> Result<ServerHandle> {
        let data_path = server_path.get_data_path();
        let server_root = server_path.get_root();

        let xms = heap_mb(xms)?;
        let xmx = heap_mb(xmx)?;

        let mut cmd = tokio::process::Command::new(java_path);
        // Only an explicit captured-generation kill may terminate this child.
        // Losing ownership is Unknown and leaves durable recovery evidence.
        cmd.kill_on_drop(false)
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
                    "Modloader argument file not found at {}. The server install may be incomplete — For a pack server use Repair; otherwise back up this directory and restore the matching loader files while preserving worlds, server.properties and eula.txt.",
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
                        "Modded server has no valid launch configuration for {modloader} (expected {} to exist) — For a pack server, use Repair. Otherwise back up the server directory and restore the matching loader files from its installer into this directory, preserving worlds, server.properties and eula.txt.",
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
                    "Modded server has no valid launch configuration for {modloader} — For a pack server, use Repair. Otherwise back up the server directory and restore the matching loader files from its installer into this directory, preserving worlds, server.properties and eula.txt."
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

        runtime_activity.external_child()?;
        let child = match runtime_activity.spawn(
            crate::managers::runtime_lifecycle::ActivityPolicy::Server,
            || cmd.spawn(),
        ) {
            Ok(child) => child,
            Err(error) => {
                runtime_activity.finish();
                return Err(error).context("Failed to spawn server process");
            }
        };
        // The server runs from here on: a record that cannot be written must
        // not drop it unsupervised, holding the port and the world.
        runtime_activity.record_spawned_child(child.id());
        let pid = child.id().unwrap_or(0);

        info!("Server process started with PID {}", pid);

        // Recovery records identity; startup never automatically signals this
        // process. Current destructive recovery remains an explicit operator action.
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]));
        match orphan_pid::process_start_time(&system, pid) {
            Some(start_time) => write_pid_file(&server_root, pid, start_time).await,
            None => warn!(
                "Could not determine start time for server process (pid {}); not writing a pidfile for it",
                pid
            ),
        }

        let ownership = std::sync::Arc::new(std::sync::Mutex::new(Some(runtime_activity)));
        let Supervised {
            kill_tx,
            stdin_tx,
            exited,
        } = supervise(child, log_tx, ownership.clone(), POST_EXIT_OUTPUT_GRACE);

        Ok(ServerHandle {
            generation: uuid::Uuid::new_v4(),
            log_drained: None,
            marker_root: Some(server_root),
            runtime_activity: ownership,
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

    async fn collect_lines(mut rx: mpsc::UnboundedReceiver<String>) -> Vec<String> {
        let mut lines = Vec::new();
        while let Some(line) = rx.recv().await {
            lines.push(line);
        }
        lines
    }

    #[tokio::test]
    async fn forward_output_decodes_bad_bytes_lossily_and_keeps_reading() {
        // Java on Windows prints "été" in Cp1252 as E9 74 E9. The old
        // `lines()` reader stopped the console (and, after 826b603e6, the exit
        // confirmation) at the first such byte.
        let (tx, rx) = mpsc::unbounded_channel();
        forward_output(&b"a\n\xe9t\xe9\nb\r\n"[..], tx, "", None).await;
        assert_eq!(
            collect_lines(rx).await,
            vec!["a", "\u{FFFD}t\u{FFFD}", "b"],
            "every line is delivered, the bad one lossily, and the reader returns at EOF"
        );

        // A last line without a newline is still delivered, with the prefix.
        let (tx, rx) = mpsc::unbounded_channel();
        forward_output(&b"x\nlast"[..], tx, "[STDERR] ", None).await;
        assert_eq!(collect_lines(rx).await, vec!["[STDERR] x", "[STDERR] last"]);
    }

    #[tokio::test]
    async fn forward_output_uses_the_code_page_for_lines_that_are_not_utf8() {
        let (tx, rx) = mpsc::unbounded_channel();
        forward_output(
            &b"caf\xc3\xa9 utf8\ncaf\xe9 ansi\n"[..],
            tx,
            "",
            encoding_for_code_page(1252),
        )
        .await;
        assert_eq!(collect_lines(rx).await, vec!["café utf8", "café ansi"]);

        assert_eq!(
            decode_output_line(
                b"\xcf\xf0\xe8\xe2\xe5\xf2\r\n",
                encoding_for_code_page(1251)
            ),
            "Привет"
        );
        assert_eq!(
            decode_output_line(b"\xc4\xe3\xba\xc3\n", encoding_for_code_page(936)),
            "你好"
        );
        assert!(encoding_for_code_page(65001).is_none());
        assert!(encoding_for_code_page(0).is_none());
    }

    #[tokio::test]
    async fn forward_output_ends_cleanly_on_a_read_error_or_a_closed_console() {
        struct Broken;
        impl tokio::io::AsyncRead for Broken {
            fn poll_read(
                self: std::pin::Pin<&mut Self>,
                _: &mut std::task::Context<'_>,
                _: &mut tokio::io::ReadBuf<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                std::task::Poll::Ready(Err(std::io::Error::other("injected pipe error")))
            }
        }
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            forward_output(Broken, tx, "", None),
        )
        .await
        .expect("a read error ends the reader");
        assert!(collect_lines(rx).await.is_empty());

        let (tx, rx) = mpsc::unbounded_channel();
        drop(rx);
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            forward_output(&b"unpublished\nmore\n"[..], tx, "", None),
        )
        .await
        .expect("a closed console ends the reader");
    }

    const TEST_GRACE: std::time::Duration = std::time::Duration::from_millis(100);

    #[tokio::test]
    async fn exit_is_published_when_a_reader_already_failed() {
        let (log_tx, log_rx) = mpsc::unbounded_channel();
        let (exited_tx, mut exited) = exit_signal();
        let failed = tokio::spawn(async { Err::<(), _>(std::io::Error::other("bad read")) });
        let finished = tokio::spawn(async { Ok::<(), std::io::Error>(()) });
        publish_confirmed_exit(
            [failed, finished],
            log_tx,
            "[GDLauncher] Server process exited: exit status: 0".into(),
            &Default::default(),
            exited_tx,
            TEST_GRACE,
        )
        .await;
        assert!(exited.is_confirmed());
        exited.wait_confirmed().await.unwrap();
        assert_eq!(
            collect_lines(log_rx).await,
            vec!["[GDLauncher] Server process exited: exit status: 0"]
        );
    }

    #[tokio::test]
    async fn exit_is_published_after_the_grace_when_a_reader_never_reaches_eof() {
        let (log_tx, log_rx) = mpsc::unbounded_channel();
        let (exited_tx, mut exited) = exit_signal();
        // A helper process that inherited the pipe keeps this reader, and its
        // clone of the console sender, alive forever.
        let held = log_tx.clone();
        let blocked = tokio::spawn(async move {
            let _held = held;
            std::future::pending::<()>().await
        });
        let finished = tokio::spawn(async {});
        let started = std::time::Instant::now();
        tokio::time::timeout(
            TEST_GRACE + std::time::Duration::from_secs(2),
            publish_confirmed_exit(
                [blocked, finished],
                log_tx,
                "[GDLauncher] Server process exited: exit status: 0".into(),
                &Default::default(),
                exited_tx,
                TEST_GRACE,
            ),
        )
        .await
        .expect("the exit is published after the grace, not held by the reader");
        assert!(started.elapsed() >= TEST_GRACE);
        exited.wait_confirmed().await.unwrap();
        // The console channel closes: the aborted reader dropped its sender,
        // so the log collector can report the run's output as drained.
        let lines = tokio::time::timeout(std::time::Duration::from_secs(1), collect_lines(log_rx))
            .await
            .expect("the aborted reader must release its console sender");
        assert_eq!(
            lines,
            vec![
                OUTPUT_STAYED_OPEN.to_owned(),
                "[GDLauncher] Server process exited: exit status: 0".to_owned()
            ]
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_real_server_process_with_bad_bytes_and_a_lingering_helper_still_exits() {
        let child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg("printf 'caf\\351\\n'; (sleep 30 &); exit 0")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let (log_tx, log_rx) = mpsc::unbounded_channel();
        let grace = std::time::Duration::from_millis(500);
        let supervised = supervise(child, log_tx, Default::default(), grace);
        let mut exited = supervised.exited.clone();
        tokio::time::timeout(
            grace + std::time::Duration::from_secs(5),
            exited.wait_confirmed(),
        )
        .await
        .expect("the exit is confirmed within the grace plus a margin")
        .unwrap();
        let lines = tokio::time::timeout(std::time::Duration::from_secs(2), collect_lines(log_rx))
            .await
            .expect("the console closes once the exit is published");
        assert_eq!(
            lines,
            vec![
                "caf\u{FFFD}".to_owned(),
                OUTPUT_STAYED_OPEN.to_owned(),
                "[GDLauncher] Server process exited: exit status: 0".to_owned()
            ]
        );
        // A Stop that lands after the supervisor has gone (stdin and kill
        // channels closed) sees the published exit instead of an error.
        let handle = ServerHandle {
            generation: uuid::Uuid::new_v4(),
            log_drained: None,
            marker_root: None,
            runtime_activity: Default::default(),
            process_id: 0,
            kill_tx: supervised.kill_tx,
            stdin_tx: supervised.stdin_tx,
            exited: supervised.exited,
        };
        super::super::provider::drain(&handle, false, std::time::Duration::from_secs(1))
            .await
            .unwrap();
    }

    #[test]
    fn heap_mb_honors_large_heaps_and_rejects_non_positive() {
        // A deliberately large heap must reach the JVM unclamped — the old
        // u16::MAX cap silently rewrote a 100 GB `-Xmx100000m` down to ~464 MB.
        assert_eq!(heap_mb(100_000).unwrap(), 100_000);
        assert_eq!(format!("-Xmx{}m", heap_mb(100_000).unwrap()), "-Xmx100000m");

        // A normal, already-sane value passes through unchanged.
        assert_eq!(heap_mb(1024).unwrap(), 1024);

        // Zero/negative is a corrupt config; fail loud rather than silently
        // booting the JVM with a nonsensical heap.
        assert!(heap_mb(0).is_err());
        assert!(heap_mb(-100).is_err());
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
                std::sync::Arc::new(
                    crate::managers::runtime_lifecycle::RuntimeLifecycle::default(),
                )
                .admit("test server")
                .unwrap(),
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
                std::sync::Arc::new(
                    crate::managers::runtime_lifecycle::RuntimeLifecycle::default(),
                )
                .admit("test server")
                .unwrap(),
            )
            .await;

        let err = result.expect_err("spawning a nonexistent binary must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("Failed to spawn server process"),
            "expected a spawn failure (proving the vanilla path was allowed through), got: {msg}"
        );
    }

    /// A server whose identity cannot be written into its process record
    /// once it started (the drive just filled up, say) is still started and
    /// supervised. Start used to fail there while the JVM ran on untracked,
    /// holding the port and the world, so a retry could not start it and
    /// every quit waited on the unconfirmed run. Its marker still names it
    /// and Kill still stops it.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_started_server_whose_record_cannot_be_written_stays_supervised() {
        use crate::managers::runtime_lifecycle::{RuntimeLifecycle, tests_support};
        use std::os::unix::fs::PermissionsExt;
        use std::time::{Duration, Instant};

        /// Ends the stub server however the test ends.
        struct StubServer(std::path::PathBuf);
        impl Drop for StubServer {
            fn drop(&mut self) {
                let deadline = Instant::now() + Duration::from_secs(5);
                while Instant::now() < deadline {
                    if let Some(pid) = std::fs::read_to_string(&self.0)
                        .ok()
                        .and_then(|pid| pid.trim().parse::<i32>().ok())
                    {
                        // SAFETY: signals only the stub this test started.
                        unsafe { libc::kill(pid, libc::SIGKILL) };
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let java = dir.path().join("java");
        std::fs::write(&java, STUB_SERVER).unwrap();
        std::fs::set_permissions(&java, std::fs::Permissions::from_mode(0o755)).unwrap();
        let server_path = ServerPath::new(dir.path().join("server-root"));
        let data = server_path.get_data_path();
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(server_path.get_server_jar_path(), b"jar").unwrap();
        let _stub = StubServer(data.join("pid"));
        let lifecycle = std::sync::Arc::new(RuntimeLifecycle::with_runtime_path(
            &dir.path().join("runtime"),
        ));
        lifecycle.reconciliation_finished(Ok(()));

        let (log_tx, _log_rx) = mpsc::unbounded_channel();
        let started = {
            let _fault = tests_support::fail_child_records();
            LocalServerProvider
                .start(
                    &java,
                    &server_path,
                    1024,
                    1024,
                    "",
                    &LaunchConfig::vanilla(),
                    None,
                    log_tx,
                    lifecycle.admit("server 1 run").unwrap(),
                )
                .await
        };
        let handle = started.expect("the server runs, so Start succeeds");
        assert_ne!(handle.process_id, 0);
        let marker = super::super::provider::read_process_file(&server_path.get_root())
            .await
            .unwrap()
            .expect("the server's marker names it");
        assert_eq!(marker.pid, handle.process_id);

        LocalServerProvider.kill(&handle).await.unwrap();
        handle.exited.clone().wait_confirmed().await.unwrap();
        handle.finish_publication();
        lifecycle.begin_relocation("move").unwrap();
    }

    /// Where [`forced_core_stand_in`] finds the stub server and writes.
    #[cfg(unix)]
    const FORCED_CORE_DIR: &str = "GDL_TEST_FORCED_CORE_DIR";
    /// A server "JVM": a script that, like the JVM, ignores a console that
    /// went away (SIGPIPE), and on `stop` takes a few seconds to save, logs
    /// it and exits. It leaves its pid and each step as files in its folder.
    #[cfg(unix)]
    const STUB_SERVER: &str = r#"#!/bin/sh
trap '' PIPE
echo $$ > pid
echo 'Done (0.5s)! For help, type "help"'
while IFS= read -r line; do
  if [ "$line" = stop ]; then
    : > stop-received
    sleep 3
    echo 'Saving chunks for level "world"'
    : > saved
    exit 0
  fi
done
"#;

    /// The core of `a_server_asked_to_stop_finishes_saving_after_the_core_is_killed`,
    /// run as its own process: it starts the stub server through this
    /// provider and drains it as a quit does, until it is killed. Does
    /// nothing when run on its own.
    #[cfg(unix)]
    #[tokio::test]
    #[ignore = "a stand-in core the forced-quit test starts and kills"]
    async fn forced_core_stand_in() {
        let Some(dir) = std::env::var_os(FORCED_CORE_DIR) else {
            return;
        };
        let dir = std::path::PathBuf::from(dir);
        let server_path = ServerPath::new(dir.join("server-root"));
        std::fs::write(server_path.get_server_jar_path(), b"jar").unwrap();
        let (log_tx, _log_rx) = mpsc::unbounded_channel();
        let handle = LocalServerProvider
            .start(
                &dir.join("java"),
                &server_path,
                1024,
                1024,
                "",
                &LaunchConfig::vanilla(),
                None,
                log_tx,
                std::sync::Arc::new(
                    crate::managers::runtime_lifecycle::RuntimeLifecycle::default(),
                )
                .admit("server 1 run")
                .unwrap(),
            )
            .await
            .unwrap();
        // What a quit's drain does with a running server: 'stop', then up
        // to 30 s for it to save and exit.
        let _ =
            super::super::provider::drain(&handle, false, std::time::Duration::from_secs(30)).await;
        std::future::pending::<()>().await;
    }

    /// PB2, and the settled 'Quit now' decision: a server that was asked to
    /// stop and is still saving when the user chooses Quit now (Electron
    /// sends SIGTERM, which the core refuses while its quit is pending, then
    /// SIGKILL) keeps running once the core is gone and finishes its save on
    /// its own. Nothing ties it to the core: it is spawned with
    /// kill_on_drop(false), in no group or job the core's death ends, and a
    /// console that goes away does not stop it. Windows ends the core with
    /// TerminateProcess, which never reaches its children either.
    #[cfg(unix)]
    #[test]
    fn a_server_asked_to_stop_finishes_saving_after_the_core_is_killed() {
        use std::os::unix::fs::PermissionsExt;
        use std::time::{Duration, Instant};
        let dir = tempfile::tempdir().unwrap();
        let java = dir.path().join("java");
        std::fs::write(&java, STUB_SERVER).unwrap();
        std::fs::set_permissions(&java, std::fs::Permissions::from_mode(0o755)).unwrap();
        let data = ServerPath::new(dir.path().join("server-root")).get_data_path();
        std::fs::create_dir_all(&data).unwrap();
        let wait_for = |file: &str, within: Duration| {
            let deadline = Instant::now() + within;
            while !data.join(file).exists() {
                assert!(Instant::now() < deadline, "no {file} within {within:?}");
                std::thread::sleep(Duration::from_millis(20));
            }
        };

        /// The stand-in core, ended however the test ends.
        struct Core(std::process::Child);
        impl Drop for Core {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let mut core = Core(
            std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "managers::server::local::tests::forced_core_stand_in",
                    "--exact",
                    "--ignored",
                ])
                .env(FORCED_CORE_DIR, dir.path())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        // The quit asked the server to stop, and it is saving.
        wait_for("stop-received", Duration::from_secs(60));
        // Quit now: the core's SIGTERM handler is refused while its quit is
        // pending, so it is SIGKILLed.
        core.0.kill().unwrap();
        assert!(!core.0.wait().unwrap().success());
        let pid: u32 = std::fs::read_to_string(data.join("pid"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]));
        assert!(
            system.process(Pid::from_u32(pid)).is_some() && !data.join("saved").exists(),
            "the server outlives the core, still saving"
        );
        // It finishes its save and exits on its own.
        wait_for("saved", Duration::from_secs(30));
    }

    /// The same within one process: once everything that supervises the
    /// server is dropped (its tasks, its child handle and its console pipes,
    /// as when the core's runtime goes away), a server that was asked to
    /// stop still finishes its save. Dropping the child handle must not
    /// kill it (kill_on_drop(false)).
    #[cfg(unix)]
    #[test]
    fn a_server_asked_to_stop_finishes_saving_once_its_supervisor_is_gone() {
        use std::os::unix::fs::PermissionsExt;
        use std::time::{Duration, Instant};
        let dir = tempfile::tempdir().unwrap();
        let java = dir.path().join("java");
        std::fs::write(&java, STUB_SERVER).unwrap();
        std::fs::set_permissions(&java, std::fs::Permissions::from_mode(0o755)).unwrap();
        let server_path = ServerPath::new(dir.path().join("server-root"));
        let data = server_path.get_data_path();
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(server_path.get_server_jar_path(), b"jar").unwrap();
        let wait_for = |file: &str, within: Duration| {
            let deadline = Instant::now() + within;
            while !data.join(file).exists() {
                assert!(Instant::now() < deadline, "no {file} within {within:?}");
                std::thread::sleep(Duration::from_millis(20));
            }
        };

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let (log_tx, log_rx) = mpsc::unbounded_channel();
        let handle = runtime
            .block_on(
                LocalServerProvider.start(
                    &java,
                    &server_path,
                    1024,
                    1024,
                    "",
                    &LaunchConfig::vanilla(),
                    None,
                    log_tx,
                    std::sync::Arc::new(
                        crate::managers::runtime_lifecycle::RuntimeLifecycle::default(),
                    )
                    .admit("server 1 run")
                    .unwrap(),
                ),
            )
            .unwrap();
        let drained = handle.clone();
        runtime.spawn(async move {
            super::super::provider::drain(&drained, false, Duration::from_secs(30)).await
        });
        wait_for("stop-received", Duration::from_secs(30));
        let pid: u32 = std::fs::read_to_string(data.join("pid"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        drop(handle);
        drop(log_rx);
        runtime.shutdown_timeout(Duration::from_secs(5));

        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]));
        assert!(
            system.process(Pid::from_u32(pid)).is_some() && !data.join("saved").exists(),
            "the server outlives its supervisor, still saving"
        );
        wait_for("saved", Duration::from_secs(30));
    }
}
