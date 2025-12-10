use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    EnvFilter, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};

fn generate_logs_filters() -> String {
    let filters = &[
        "debug",
        "carbon_app=trace",
        "hyper::client::pool=warn",
        "reqwest::connect=warn",
        "hyper::proto::h1::conn=warn",
        "hyper::proto::h1::io=warn",
        "hyper::proto::h1::decode=warn",
        "quaint::connector::metrics=warn",
        "hyper_util::client::legacy::pool=warn",
        "hyper_util::client::legacy::connect::http=warn",
        "hyper_util::client::legacy::connect::dns=warn",
        "hyper_util::client::legacy::client=warn",
        "reqwest::async_impl::client=warn",
        "hyper::client::connect::http=warn",
        "hyper::client::connect::dns=warn",
        "quaint::pooled::manager=warn",
        "query_core::interactive_transactions::actors=warn",
        "query_core::executor::interpreting_executor=warn",
        "rustls::client::hs=warn",
        "rustls::client::tls13=warn",
        "h2::client=warn",
        "rustls::client::common=warn",
        "h2::codec::framed_read=warn",
        "h2::codec::framed_write=warn",
        "h2::proto::settings=warn",
        "tungstenite::protocol=warn",
        "mobc=trace",
    ];

    filters.to_vec().join(",")
}

/// Cleanup old log files, keeping only the most recent `keep_count` files
fn cleanup_old_logs(logs_path: &Path, keep_count: usize) {
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

    // Keep only the last 5 log files to prevent disk space issues
    cleanup_old_logs(&logs_path, 5);

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

        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

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
