use crate::managers::UnsafeAppRef;
use anyhow::anyhow;
use axum::http::Extensions;
use carbon_repos::db::{
    http_cache::{SetParam, WhereParam},
    read_filters::StringFilter,
};
use chrono::{DateTime, Duration, Utc};
use reqwest::{Method, Request, Response, ResponseBuilderExt, StatusCode, Url, header::HeaderMap};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, Middleware, Next, Result};

/// Responses larger than this are never stored in the cache.
/// Large responses are rarely worth caching (they're either downloads in
/// disguise or bulk aggregations that change often) and they're what
/// actually bloats gdl_conf.db.
const MAX_CACHEABLE_BODY_BYTES: usize = 1024 * 1024; // 1 MB

pub fn new_client(app: UnsafeAppRef, client_builder: ClientBuilder) -> ClientWithMiddleware {
    client_builder.with(CacheMiddleware { app }).build()
}

struct CacheMiddleware {
    app: UnsafeAppRef,
}

#[async_trait::async_trait]
impl Middleware for CacheMiddleware {
    async fn handle(
        &self,
        mut req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        let headers = req.headers_mut();
        if let Some(_) = headers.remove("avoid-caching") {
            return next.run(req, extensions).await;
        }

        // SAFETY: Requests cannot be made before the appref is initialized
        let app = unsafe { self.app.upgrade() };

        // Rebuilds a `reqwest::Response` for the caller. Must preserve:
        // - the URL, or reqwest falls back to "http://no.url.provided.local"
        //   and downstream errors lose attribution
        // - the original headers (minus wire-encoding ones we'd be lying
        //   about on the rebuilt body), so consumers still see things like
        //   Retry-After and Content-Type
        fn build_cached(
            status: i32,
            body: Vec<u8>,
            headers: &HeaderMap,
            url: &Url,
            cached: bool,
        ) -> std::result::Result<Response, ()> {
            let mut builder = hyper::Response::builder()
                .status(StatusCode::from_u16(status.try_into().map_err(|_| ())?).map_err(|_| ())?)
                .url(url.clone());

            if let Some(dst) = builder.headers_mut() {
                for (name, value) in headers {
                    // Body was already decoded by reqwest and we're setting a
                    // fresh one, so the wire-level framing/encoding headers
                    // would no longer describe reality.
                    if matches!(
                        name.as_str(),
                        "content-length" | "content-encoding" | "transfer-encoding" | "connection"
                    ) {
                        continue;
                    }
                    dst.append(name.clone(), value.clone());
                }
            }

            if cached {
                builder = builder.header("Cached", "true");
            }

            Ok(builder.body(body).map_err(|_| ())?.into())
        }

        let method = req.method().clone();
        let req_url = req.url().clone();
        let req_url_str = req_url.to_string();

        let mut cached = if method != Method::GET {
            None
        } else {
            app.prisma_client
                .http_cache()
                .find_first(vec![WhereParam::Url(StringFilter::Equals(
                    req_url_str.clone(),
                ))])
                .exec()
                .await
                .map_err(|e| reqwest_middleware::Error::Middleware(anyhow!(e)))?
        };

        // We don't persist original response headers in HTTPCache, so
        // responses served from the DB come back with only the synthetic
        // "Cached: true" header.
        let empty_headers = HeaderMap::new();

        // return the cached value if fresh
        if let Some(expires) = cached.as_ref().and_then(|c| c.expires_at) {
            if expires > Utc::now() {
                let cached = cached.take().expect("cached was just asserted to be Some");
                if let Ok(response) = build_cached(
                    cached.status_code,
                    cached.data,
                    &empty_headers,
                    &req_url,
                    true,
                ) {
                    return Ok(response);
                }
            }
        }

        let response = next.run(req, extensions).await;
        let Ok(response) = response else {
            return response;
        };
        let headers = response.headers();

        'use_cache: {
            if let Some(cached) = cached {
                if let (Some(cached_etag), Some(etag)) = (cached.etag, headers.get("etag")) {
                    if Some(&cached_etag as &str) == etag.to_str().ok() {
                        match build_cached(
                            cached.status_code,
                            cached.data,
                            &empty_headers,
                            &req_url,
                            true,
                        ) {
                            Ok(response) => return Ok(response),
                            Err(_) => break 'use_cache,
                        }
                    }
                }

                if let (Some(cached_last_modified), Some(last_modified)) =
                    (cached.last_modified, headers.get("last-modified"))
                {
                    if Some(&cached_last_modified as &str) == last_modified.to_str().ok() {
                        match build_cached(
                            cached.status_code,
                            cached.data,
                            &empty_headers,
                            &req_url,
                            true,
                        ) {
                            Ok(response) => return Ok(response),
                            Err(_) => break 'use_cache,
                        }
                    }
                }
            }
        }

        // Caching non-success responses persists errors (e.g. 429 rate-limit
        // bodies served with Cache-Control) past their natural recovery
        // window, and rebuilding them strips Retry-After/X-RateLimit headers
        // that retry logic needs. Let the original response through.
        if method != Method::GET || !response.status().is_success() {
            return Ok(response);
        }

        let mut expires = None::<DateTime<Utc>>;

        if let Some(cache_control) = headers
            .get("cache-control")
            .and_then(|header| header.to_str().ok())
        {
            let directives = cache_control.split(',').map(|s| s.trim());

            let mut max_age = None::<u32>;
            let mut no_store = false;

            for directive in directives {
                let (directive, value) = match directive.split_once('=') {
                    Some((d, v)) => (d, Some(v)),
                    None => (directive, None),
                };

                match (directive, value) {
                    ("max-age", Some(value)) => {
                        max_age = value.parse::<u32>().ok();
                    }
                    ("no-store", None) => {
                        no_store = true;
                    }
                    _ => {}
                }
            }

            if !no_store {
                expires = max_age.map(|offset| Utc::now() + Duration::seconds(offset as i64));
            }
        }

        expires = expires.or_else(|| {
            headers
                .get("expires")
                .and_then(|header| header.to_str().ok())
                .and_then(|header| httpdate::parse_http_date(header).ok())
                .map(DateTime::<Utc>::from)
        });

        let etag = headers
            .get("etag")
            .and_then(|header| header.to_str().ok())
            .map(String::from);

        let last_modified = headers
            .get("last-modified")
            .and_then(|header| header.to_str().ok())
            .map(String::from);

        // ignoring `Vary`

        if expires.is_some() || etag.is_some() || last_modified.is_some() {
            let url = response.url().clone();
            let url_str = url.to_string();
            let status = response.status().as_u16() as i32;
            let response_headers = response.headers().clone();
            let body = response.bytes().await?;

            // Skip persisting responses larger than the cap. This prevents
            // occasional multi-MB blobs (bulk aggregations, accidental file
            // downloads) from ever entering HTTPCache, which is what caused
            // the 60-80GB db bloat reports.
            if body.len() <= MAX_CACHEABLE_BODY_BYTES {
                let _ = app
                    .prisma_client
                    ._batch((
                        app.prisma_client.http_cache().delete_many(vec![
                            // will not fail when not found
                            WhereParam::Url(StringFilter::Equals(url_str.clone())),
                        ]),
                        app.prisma_client.http_cache().create(
                            url_str,
                            status,
                            body.to_vec(),
                            vec![
                                SetParam::SetExpiresAt(expires.map(Into::into)),
                                SetParam::SetLastModified(last_modified),
                                SetParam::SetEtag(etag),
                            ],
                        ),
                    ))
                    .await;
            }

            match build_cached(status, body.to_vec(), &response_headers, &url, false) {
                Ok(response) => return Ok(response),
                Err(_) => {
                    return Err(reqwest_middleware::Error::Middleware(anyhow!(
                        "could not return cached response"
                    )));
                }
            }
        }

        Ok(response)
    }
}

#[cfg(test)]
mod test {
    use std::time::SystemTime;

    use axum::{Router, http::header, routing::get};
    use chrono::{Duration, Utc};
    use tokio::net::TcpListener;

    use crate::managers::App;

    macro_rules! launch_server {
        [$($headers:expr),*] => {{
            let tcp_listener = TcpListener::bind("127.0.0.1:0").await
                .expect("Failed to bind test server to localhost");
            let port = tcp_listener.local_addr()
                .expect("Failed to get local address of test server")
                .port();

            let server = Router::new()
                .route("/", get(|| async { ([$($headers),*], "test") }));

            tokio::spawn(async {
                axum::serve(tcp_listener, server.into_make_service())
                    .await
                    .expect("Test server failed to start");
            });

            // let the server start
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

            port
        }}
    }

    async fn launch_body_server(body_size: usize) -> u16 {
        let tcp_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind test server to localhost");
        let port = tcp_listener
            .local_addr()
            .expect("Failed to get local address of test server")
            .port();

        let body = vec![b'x'; body_size];

        let server = Router::new().route(
            "/",
            get(move || {
                let body = body.clone();
                async move { ([(header::CACHE_CONTROL, "max-age=60")], body) }
            }),
        );

        tokio::spawn(async {
            axum::serve(tcp_listener, server.into_make_service())
                .await
                .expect("Test server failed to start");
        });

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        port
    }

    async fn request_cached(app: &App, port: u16) -> bool {
        app.reqwest_client
            .get(format!("http://127.0.0.1:{port}/"))
            .send()
            .await
            .expect("Failed to send test request")
            .headers()
            .get("Cached")
            .is_some()
    }

    #[tokio::test]
    async fn test_expires() {
        let app = crate::setup_managers_for_test().await;

        let port = launch_server![(
            header::EXPIRES,
            httpdate::fmt_http_date(SystemTime::from(Utc::now() + Duration::seconds(2)))
        )];

        assert!(!request_cached(&app, port).await);
        assert!(request_cached(&app, port).await);
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        assert!(!request_cached(&app, port).await);
    }

    #[tokio::test]
    async fn test_max_age() {
        let app = crate::setup_managers_for_test().await;

        let port = launch_server![(header::CACHE_CONTROL, "max-age=1")];

        assert!(!request_cached(&app, port).await);
        assert!(request_cached(&app, port).await);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        assert!(!request_cached(&app, port).await);
    }

    #[tokio::test]
    async fn test_no_store() {
        let app = crate::setup_managers_for_test().await;

        let port = launch_server![(header::CACHE_CONTROL, "no-store")];

        assert!(!request_cached(&app, port).await);
        assert!(!request_cached(&app, port).await);
    }

    #[tokio::test]
    async fn test_etag() {
        let app = crate::setup_managers_for_test().await;

        let port = launch_server![(header::ETAG, "test_etag")];

        assert!(!request_cached(&app, port).await);
        assert!(request_cached(&app, port).await);
    }

    #[tokio::test]
    async fn test_last_modified() {
        let app = crate::setup_managers_for_test().await;

        let port = launch_server![(header::LAST_MODIFIED, "test_last_modified")];

        assert!(!request_cached(&app, port).await);
        assert!(request_cached(&app, port).await);
    }

    #[tokio::test]
    async fn test_body_under_cap_is_cached() {
        let app = crate::setup_managers_for_test().await;

        // 512 KB, below the 1 MB cap
        let port = launch_body_server(512 * 1024).await;

        assert!(!request_cached(&app, port).await);
        assert!(request_cached(&app, port).await);
    }

    #[tokio::test]
    async fn test_body_over_cap_is_not_cached() {
        let app = crate::setup_managers_for_test().await;

        // 2 MB, above the 1 MB cap
        let port = launch_body_server(2 * 1024 * 1024).await;

        assert!(!request_cached(&app, port).await);
        // still uncached on the second request because the first was never persisted
        assert!(!request_cached(&app, port).await);
    }
}
