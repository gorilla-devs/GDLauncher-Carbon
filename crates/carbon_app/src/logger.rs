use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    EnvFilter, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};

/// Hard cap for a single log file (launcher session logs and per-instance game logs).
/// A log-spamming game previously grew one session file past 100 GB; once the cap is
/// reached, further output is dropped after a single truncation notice.
pub const MAX_LOG_FILE_SIZE: u64 = 256 * 1024 * 1024;

pub const LOG_TRUNCATION_NOTICE: &[u8] = b"\n[log truncated: file size cap reached]\n";

/// `Write` wrapper that drops output past [`MAX_LOG_FILE_SIZE`].
#[cfg_attr(debug_assertions, allow(dead_code))]
struct SizeCappedWriter<W: std::io::Write> {
    inner: W,
    written: u64,
    truncated: bool,
}

impl<W: std::io::Write> SizeCappedWriter<W> {
    #[cfg_attr(debug_assertions, allow(dead_code))]
    fn new(inner: W) -> Self {
        Self {
            inner,
            written: 0,
            truncated: false,
        }
    }
}

impl<W: std::io::Write> std::io::Write for SizeCappedWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.truncated {
            // Report the bytes as written so tracing doesn't treat this as an error.
            return Ok(buf.len());
        }

        if self.written + buf.len() as u64 > MAX_LOG_FILE_SIZE {
            self.truncated = true;
            let _ = self.inner.write_all(LOG_TRUNCATION_NOTICE);
            let _ = self.inner.flush();
            return Ok(buf.len());
        }

        let n = self.inner.write(buf)?;
        self.written += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn generate_logs_filters() -> String {
    #[cfg(debug_assertions)]
    let app_level = "carbon_app=trace";
    #[cfg(not(debug_assertions))]
    let app_level = "carbon_app=debug";

    let filters = &[
        "debug",
        app_level,
        "hyper::client::pool=warn",
        "reqwest::connect=warn",
        "hyper::proto::h1::conn=warn",
        "hyper::proto::h1::io=warn",
        "hyper::proto::h1::decode=warn",
        "hyper_util::client::legacy::pool=warn",
        "hyper_util::client::legacy::connect::http=warn",
        "hyper_util::client::legacy::connect::dns=warn",
        "hyper_util::client::legacy::client=warn",
        "reqwest::async_impl::client=warn",
        "hyper::client::connect::http=warn",
        "hyper::client::connect::dns=warn",
        "rustls::client::hs=warn",
        "rustls::client::tls13=warn",
        "h2::client=warn",
        "rustls::client::common=warn",
        "h2::codec::framed_read=warn",
        "h2::codec::framed_write=warn",
        "h2::proto::settings=warn",
        "tungstenite::protocol=warn",
    ];

    filters.to_vec().join(",")
}

/// Cleanup old log files, keeping only the most recent `keep_count` files.
/// Reused by the cache-cleanup dialog so the launcher's "don't blanket-
/// wipe logs" policy is enforced from a single place.
pub fn cleanup_old_logs(logs_path: &Path, keep_count: usize) {
    let Ok(read_dir) = std::fs::read_dir(logs_path) else {
        return;
    };

    let mut entries: Vec<_> = read_dir
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "log"))
        .collect();

    // Sort by modified time, newest first
    entries.sort_by(|a, b| {
        let time_a = a.metadata().ok().and_then(|m| m.modified().ok());
        let time_b = b.metadata().ok().and_then(|m| m.modified().ok());
        time_b.cmp(&time_a)
    });

    // Delete all but the newest `keep_count` files
    for entry in entries.into_iter().skip(keep_count) {
        if let Err(e) = std::fs::remove_file(entry.path()) {
            eprintln!("Failed to delete old log file {:?}: {}", entry.path(), e);
        }
    }
}

pub async fn setup_logger(runtime_path: &Path) -> Option<WorkerGuard> {
    let logs_path = runtime_path.join("__gdl_logs__");

    println!("Logs path: {}", logs_path.display());

    if !logs_path.exists() {
        tokio::fs::create_dir_all(&logs_path).await.unwrap();
    }

    // Keep only the last 10 log files. Same retention as the cache-cleanup
    // dialog enforces — recent logs are useful for debugging crashes that
    // happened a few launches ago, so we don't blanket-wipe them.
    cleanup_old_logs(&logs_path, 10);

    let filter = EnvFilter::builder();

    // We need to check if the env is present, because, although
    // `EnvFilter::from_env()` says in it's docs that it will return an error
    // if the env is not set, reading the source of the method reveals this is
    // not true :(
    let filter = if std::env::var("RUST_LOG").is_ok() {
        println!("loaded logger directives from `RUST_LOG` env");

        filter.from_env().expect("logger directives are invalid")
    } else {
        let directives = generate_logs_filters();

        println!(
            "loaded default logger directives, to override, set `RUST_LOG` env var\n\
             RUST_LOG=\"{directives}\""
        );

        filter.parse(directives).unwrap()
    };

    // let processor = tracing_forest::Printer::new()
    //     .formatter(tracing_forest::printer::Pretty)
    //     // .formatter(serde_json::to_string_pretty)
    //     .writer(non_blocking);
    // let layer = tracing_forest::ForestLayer::from(processor);

    #[cfg(debug_assertions)]
    {
        let printer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_ansi(true)
            .pretty()
            .with_thread_names(true);

        tracing_subscriber::registry()
            .with(printer)
            .with(filter)
            .init();

        None
    }
    #[cfg(not(debug_assertions))]
    {
        let file_name = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let file_appender =
            tracing_appender::rolling::never(logs_path, format!("{}.log", file_name));

        let (non_blocking, guard) =
            tracing_appender::non_blocking(SizeCappedWriter::new(file_appender));

        let printer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_ansi(true)
            .pretty()
            .with_thread_names(false);

        tracing_subscriber::registry()
            .with(printer.with_writer(non_blocking))
            .with(filter)
            .init();

        tracing::trace!("Logger initialized");
        return Some(guard);
    }
}

#[cfg(test)]
mod test {
    use std::io::Write;

    #[test]
    fn size_capped_writer_stops_at_cap() {
        let mut writer = super::SizeCappedWriter {
            inner: Vec::new(),
            written: super::MAX_LOG_FILE_SIZE - 10,
            truncated: false,
        };

        writer.write_all(b"0123456789").unwrap();
        assert_eq!(writer.inner, b"0123456789");

        // This write crosses the cap: dropped, notice appended, no error reported
        writer.write_all(b"overflow").unwrap();
        assert!(writer.truncated);
        let expected = [b"0123456789" as &[u8], super::LOG_TRUNCATION_NOTICE].concat();
        assert_eq!(writer.inner, expected);

        // Subsequent writes are silently dropped
        writer.write_all(b"more").unwrap();
        assert_eq!(writer.inner, expected);
    }
}
