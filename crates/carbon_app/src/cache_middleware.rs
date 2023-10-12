use anyhow::anyhow;
use chrono::{DateTime, Duration, Utc};
use reqwest::{Method, Request, Response, StatusCode};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, Middleware, Next, Result};
use task_local_extensions::Extensions;

use crate::{
    db::{
        http_cache::{SetParam, WhereParam},
        read_filters::StringFilter,
    },
    managers::UnsafeAppRef,
};

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

        fn build_cached(
            status: i32,
            body: Vec<u8>,
            cached: bool,
        ) -> std::result::Result<Response, ()> {
            let mut response = hyper::Response::builder()
                .status(StatusCode::from_u16(status.try_into().map_err(|_| ())?).map_err(|_| ())?);

            if cached {
                response = response.header("Cached", "true");
            }

            Ok(response.body(body).map_err(|_| ())?.into())
        }

        let method = req.method().clone();

        let mut cached = if method != Method::GET {
            None
        } else {
            app.prisma_client
                .http_cache()
                .find_first(vec![WhereParam::Url(StringFilter::Equals(
                    req.url().to_string(),
                ))])
                .exec()
                .await
                .map_err(|e| reqwest_middleware::Error::Middleware(anyhow!(e)))?
        };

        // return the cached value if fresh
        if let Some(expires) = cached.as_ref().and_then(|c| c.expires_at) {
            if expires > Utc::now() {
                let cached = cached.take().expect("cached was just asserted to be Some");
                if let Ok(response) = build_cached(cached.status_code, cached.data, true) {
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
                        match build_cached(cached.status_code, cached.data, true) {
                            Ok(response) => return Ok(response),
                            Err(_) => break 'use_cache,
                        }
                    }
                }

                if let (Some(cached_last_modified), Some(last_modified)) =
                    (cached.last_modified, headers.get("last-modified"))
                {
                    if Some(&cached_last_modified as &str) == last_modified.to_str().ok() {
                        match build_cached(cached.status_code, cached.data, true) {
                            Ok(response) => return Ok(response),
                            Err(_) => break 'use_cache,
                        }
                    }
                }
            }
        }

        if method == Method::GET {
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
                let url = response.url().to_string();
                let status = response.status().as_u16() as i32;
                let body = response.bytes().await?;

                let _ = app
                    .prisma_client
                    ._batch((
                        app.prisma_client
                            .http_cache()
                            // will not fail when not found
                            .delete_many(vec![WhereParam::Url(StringFilter::Equals(url.clone()))]),
                        app.prisma_client.http_cache().create(
                            url,
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

                match build_cached(status, body.to_vec(), false) {
                    Ok(response) => return Ok(response),
                    Err(_) => {
                        return Err(reqwest_middleware::Error::Middleware(anyhow!(
                            "could not return cached response"
                        )))
                    }
                }
            }
        }

        Ok(response)
    }
}

#[cfg(test)]
mod test {
    use std::{net::TcpListener, time::SystemTime};

    use axum::{http::header, routing::get, Router};
    use chrono::{Duration, Utc};

    use crate::managers::App;

    macro_rules! launch_server {
        [$($headers:expr),*] => {{
            let tcp_listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = tcp_listener.local_addr().unwrap().port();

            let server = Router::new()
                .route("/", get(|| async { ([$($headers),*], "test") }));

            tokio::spawn(async {
                axum::Server::from_tcp(tcp_listener).unwrap()
                    .serve(server.into_make_service())
                    .await
                    .unwrap();
            });

            // let the server start
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

            port
        }}
    }

    async fn request_cached(app: &App, port: u16) -> bool {
        app.reqwest_client
            .get(format!("http://127.0.0.1:{port}/"))
            .send()
            .await
            .unwrap()
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
}
