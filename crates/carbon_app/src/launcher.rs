//! What ties this core to the launcher that started it: the lines it tells
//! the launcher on standard output, and the watch on standard input that
//! notices the launcher is gone.

use std::io::{ErrorKind, Read, Write};

/// The launcher passes this when it starts the core with standard input as a
/// pipe it keeps open for as long as it lives. A core started any other way
/// (`pnpm watch:core`, a test harness, by hand) may have no standard input
/// at all, which reads as its end, so nothing is watched without it.
pub const EXIT_WITH_LAUNCHER_ARG: &str = "--exit_with_launcher";

/// Writes one line for the launcher to standard output.
///
/// Unlike `println!`, a launcher that is gone does not make this panic: its
/// end of the pipe is closed then, and a panic here would take down whatever
/// was telling it something, with whatever lock that held.
pub fn tell(line: impl std::fmt::Display) {
    tell_through(&mut std::io::stdout().lock(), line);
}

fn tell_through(output: &mut impl Write, line: impl std::fmt::Display) {
    let _ = writeln!(output, "{line}").and_then(|()| output.flush());
}

/// Whether the launcher that started this core is still there.
///
/// The system closes the launcher's end of the pipe when the launcher ends
/// for any reason, a crash or a kill included, and only then does a read of
/// standard input report its end. On unix nothing else ends a child with its
/// parent.
#[derive(Clone)]
pub struct Watch(Option<tokio::sync::watch::Receiver<bool>>);

impl Watch {
    /// Watches standard input when the launcher asked for it.
    pub fn from_args() -> Self {
        Self::when_requested(std::env::args(), std::io::stdin)
    }

    fn when_requested<R: Read + Send + 'static>(
        args: impl Iterator<Item = String>,
        input: impl FnOnce() -> R,
    ) -> Self {
        if requested(args) {
            Self::over(input())
        } else {
            Self(None)
        }
    }

    /// Watches nothing: the launcher is never found gone.
    #[cfg(test)]
    pub fn off() -> Self {
        Self(None)
    }

    /// A watch over a stand-in for the launcher's pipe: the launcher is gone
    /// once the returned end is dropped.
    #[cfg(test)]
    pub fn over_pipe() -> (std::sync::mpsc::Sender<std::io::Result<Vec<u8>>>, Self) {
        struct Pipe(std::sync::mpsc::Receiver<std::io::Result<Vec<u8>>>);
        impl Read for Pipe {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                match self.0.recv() {
                    Ok(Ok(bytes)) => {
                        let length = bytes.len().min(buffer.len());
                        buffer[..length].copy_from_slice(&bytes[..length]);
                        Ok(length)
                    }
                    Ok(Err(error)) => Err(error),
                    Err(std::sync::mpsc::RecvError) => Ok(0),
                }
            }
        }
        let (launcher, core) = std::sync::mpsc::channel();
        (launcher, Self::over(Pipe(core)))
    }

    /// Watches `input` until it ends or can no longer be read. Whatever
    /// arrives on it meanwhile is not looked at.
    fn over(mut input: impl Read + Send + 'static) -> Self {
        let (gone, watch) = tokio::sync::watch::channel(false);
        // A thread of its own: a blocking read of standard input cannot be
        // cancelled, and must not keep one of the runtime's threads.
        let started = std::thread::Builder::new()
            .name("launcher-watch".into())
            .spawn(move || {
                let mut buffer = [0u8; 256];
                loop {
                    match input.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(_) => {}
                        Err(error) if error.kind() == ErrorKind::Interrupted => {}
                        Err(_) => break,
                    }
                }
                let _ = gone.send(true);
            });
        match started {
            Ok(_) => Self(Some(watch)),
            Err(error) => {
                tracing::warn!("Could not watch for the launcher's end: {error}");
                Self(None)
            }
        }
    }

    /// Resolves once the launcher is gone, and again at every later call.
    /// Never resolves when nothing is watched.
    pub async fn gone(&self) {
        let Some(watch) = &self.0 else {
            return std::future::pending().await;
        };
        let mut watch = watch.clone();
        if watch.wait_for(|gone| *gone).await.is_err() {
            // The watching thread ended without saying so: nothing is known.
            std::future::pending().await
        }
    }
}

fn requested(mut args: impl Iterator<Item = String>) -> bool {
    args.any(|arg| arg == EXIT_WITH_LAUNCHER_ARG)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    fn pipe() -> (mpsc::Sender<std::io::Result<Vec<u8>>>, Watch) {
        Watch::over_pipe()
    }
    async fn is_gone(watch: &Watch) -> bool {
        tokio::time::timeout(Duration::from_millis(300), watch.gone())
            .await
            .is_ok()
    }

    #[tokio::test]
    async fn the_launcher_is_there_while_its_end_of_the_pipe_is_open() {
        let (launcher, watch) = pipe();
        assert!(!is_gone(&watch).await);
        drop(launcher);
        assert!(is_gone(&watch).await);
    }

    #[tokio::test]
    async fn bytes_from_the_launcher_do_not_end_the_watch() {
        let (launcher, watch) = pipe();
        launcher.send(Ok(b"hello\n".to_vec())).unwrap();
        launcher.send(Ok(vec![7; 1000])).unwrap();
        assert!(!is_gone(&watch).await);
        drop(launcher);
        assert!(is_gone(&watch).await);
    }

    #[tokio::test]
    async fn an_interrupted_read_is_tried_again_and_any_other_failure_ends_the_watch() {
        let (launcher, watch) = pipe();
        launcher
            .send(Err(std::io::Error::from(ErrorKind::Interrupted)))
            .unwrap();
        assert!(!is_gone(&watch).await);
        launcher
            .send(Err(std::io::Error::from(ErrorKind::BrokenPipe)))
            .unwrap();
        assert!(is_gone(&watch).await);
        // Every later look finds the launcher gone as well.
        assert!(is_gone(&watch).await);
        assert!(is_gone(&watch.clone()).await);
        drop(launcher);
    }

    #[tokio::test]
    async fn nothing_is_watched_unless_the_launcher_asked() {
        let args = |list: &[&str]| list.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        assert!(!requested(
            args(&["carbon_app", "--runtime_path", "/data"]).into_iter()
        ));
        assert!(requested(
            args(&[
                "carbon_app",
                "--runtime_path",
                "/data",
                EXIT_WITH_LAUNCHER_ARG
            ])
            .into_iter()
        ));
        // A value that merely contains the flag is not the flag.
        assert!(!requested(
            args(&["carbon_app", "--runtime_path", "/data/--exit_with_launcher"]).into_iter()
        ));
        // A core started without it may have no standard input at all,
        // which reads as its end: that is not a launcher that is gone.
        let unasked = args(&["carbon_app", "--runtime_path", "/data"]);
        assert!(!is_gone(&Watch::when_requested(unasked.into_iter(), std::io::empty)).await);
        let asked = args(&["carbon_app", EXIT_WITH_LAUNCHER_ARG]);
        assert!(is_gone(&Watch::when_requested(asked.into_iter(), std::io::empty)).await);
        // The tests themselves run without it.
        assert!(!is_gone(&Watch::from_args()).await);
        assert!(!is_gone(&Watch::off()).await);
    }

    /// `println!` panics once the launcher is gone, in whatever task runs it
    /// and with whatever lock that task holds: it poisoned the lock of the
    /// running-instances rows and ended the install that held it. Everything
    /// the core says on standard output goes through [`tell`].
    #[test]
    fn nothing_the_core_ships_prints_to_standard_output_by_itself() {
        fn sources(folder: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(folder).unwrap() {
                let path = entry.unwrap().path();
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                if path.is_dir() {
                    // Programs of their own, with their own standard output.
                    if name != "bin" {
                        sources(&path, found);
                    }
                } else if name.ends_with(".rs") && !name.contains("test") {
                    found.push(path);
                }
            }
        }
        /// The file without what is behind `#[cfg(test)]` and without comments.
        fn shipped(text: &str) -> Vec<(usize, String)> {
            let mut kept = Vec::new();
            let mut lines = text.lines().enumerate();
            let mut comment = false;
            let mut test_item = false;
            while let Some((index, line)) = lines.next() {
                let trimmed = line.trim();
                if comment || trimmed.starts_with("/*") {
                    comment = !trimmed.contains("*/");
                    continue;
                }
                if trimmed.starts_with("//") {
                    continue;
                }
                if trimmed == "#[cfg(test)]" {
                    test_item = true;
                    continue;
                }
                if !test_item {
                    kept.push((index + 1, line.to_owned()));
                    continue;
                }
                if trimmed.starts_with("#[") {
                    continue;
                }
                // The item or statement the attribute is on, to its end.
                test_item = false;
                let mut depth = 0usize;
                let mut current = line.to_owned();
                loop {
                    depth += current.matches(['{', '(']).count();
                    depth = depth.saturating_sub(current.matches(['}', ')']).count());
                    let end = current.trim_end();
                    if depth == 0
                        && (end.ends_with(';') || end.ends_with('}') || end.ends_with(','))
                    {
                        break;
                    }
                    let Some((_, next)) = lines.next() else { break };
                    current = next.to_owned();
                }
            }
            kept
        }
        let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_owned();
        let mut files = Vec::new();
        for entry in std::fs::read_dir(&crates).unwrap() {
            let src = entry.unwrap().path().join("src");
            if src.is_dir() {
                sources(&src, &mut files);
            }
        }
        assert!(files.len() > 100, "the core's sources were not found");
        let mut printing = Vec::new();
        for file in files {
            let relative = file
                .strip_prefix(&crates)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            // Runs before the watch and is shared with a program of its own
            // (bin/panic_hook_probe.rs), which has no launcher.
            if relative == "carbon_app/src/logger.rs" {
                continue;
            }
            for (number, line) in shipped(&std::fs::read_to_string(&file).unwrap()) {
                let code = line.replace("eprintln!", "").replace("eprint!", "");
                if code.contains("println!") || code.contains("print!(") || code.contains("dbg!(") {
                    printing.push(format!("{relative}:{number}: {}", line.trim()));
                }
            }
        }
        assert!(
            printing.is_empty(),
            "use crate::launcher::tell for what the launcher reads, or tracing for a log:\n{}",
            printing.join("\n")
        );
    }

    #[test]
    fn telling_a_launcher_that_is_gone_does_not_panic() {
        struct Closed;
        impl Write for Closed {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::from(ErrorKind::BrokenPipe))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::Error::from(ErrorKind::BrokenPipe))
            }
        }
        tell_through(&mut Closed, "_STATUS_:READY|1|token");

        let mut told = Vec::new();
        tell_through(&mut told, "_DRPC_:INIT");
        tell_through(&mut told, format_args!("_STATUS_:{}", 7));
        assert_eq!(told, b"_DRPC_:INIT\n_STATUS_:7\n");
    }
}
