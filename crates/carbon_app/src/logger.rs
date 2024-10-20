use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter,
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
        "reqwest::async_impl::client=warn",
        "hyper::client::connect::http=warn",
        "hyper::client::connect::dns=warn",
        "quaint::pooled::manager=warn",
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

pub async fn setup_logger(runtime_path: &Path) -> Option<WorkerGuard> {
    let logs_path = runtime_path.join("__gdl_logs__");

    println!("Logs path: {}", logs_path.display());

    if !logs_path.exists() {
        tokio::fs::create_dir_all(&logs_path).await.unwrap();
    }

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
