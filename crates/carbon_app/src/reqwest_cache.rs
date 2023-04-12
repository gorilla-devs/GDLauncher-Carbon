use std::mem;

use anyhow::anyhow;
use chrono::{DateTime, Duration, Utc};
use http::{Method, StatusCode};
use reqwest::{Client, Request, Response};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, Middleware, Next, Result};
use task_local_extensions::Extensions;

use crate::{
    db::{
        http_cache::{SetParam, WhereParam},
        read_filters::StringFilter,
    },
    managers::UnsafeAppRef,
};

pub fn new(app: UnsafeAppRef) -> ClientWithMiddleware {
    let client = Client::builder().build().unwrap();

    ClientBuilder::new(client)
        .with(CacheMiddleware { app })
        .build()
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
            let mut response = http::Response::builder()
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
                let cached =
                    mem::replace(&mut cached, None).expect("cached was just asserted to be Some");
                if let Ok(response) = build_cached(cached.status_code, cached.data, true) {
                    return Ok(response);
                }
            }
        }

        let response = next.run(req, extensions).await;
        let Ok(response) = response else { return response };
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

// #[cfg(test)]
// mod test {
//     use std::time::SystemTime;

//     use axum::{http::header, routing::get, Router};
//     use chrono::{Duration, Utc};

//     macro_rules! launch_server {
//         [$address:literal; $($headers:expr),*] => {
//             let server = Router::new()
//                 .route("/", get(|| async { ([$($headers),*], "test") }));

//             tokio::spawn(async {
//                 axum::Server::bind(&concat!("0.0.0.0:", $address).parse().unwrap())
//                     .serve(server.into_make_service())
//                     .await
//                     .unwrap();
//             });

//             // let the server start
//             tokio::time::sleep(std::time::Duration::from_millis(10)).await;
//         }
//     }

//     macro_rules! request_cached {
//         ($app:expr, $address:literal) => {
//             $app.reqwest_client
//                 .get(concat!("http://0.0.0.0:", $address, "/"))
//                 .send()
//                 .await
//                 .unwrap()
//                 .headers()
//                 .get("Cached")
//                 .is_some()
//         };
//     }

//     #[tokio::test]
//     async fn test_expires() {
//         let app = crate::setup_managers_for_test().await;

//         launch_server![3000; (
//             header::EXPIRES,
//             httpdate::fmt_http_date(SystemTime::from(
//                 Utc::now() + Duration::seconds(1)
//             ))
//         )];

//         assert!(!request_cached!(app, 3000));
//         assert!(request_cached!(app, 3000));
//         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
//         assert!(!request_cached!(app, 3000));
//     }

//     #[tokio::test]
//     async fn test_max_age() {
//         let app = crate::setup_managers_for_test().await;

//         launch_server![3001; (
//             header::CACHE_CONTROL,
//             "max-age=1"
//         )];

//         assert!(!request_cached!(app, 3001));
//         assert!(request_cached!(app, 3001));
//         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
//         assert!(!request_cached!(app, 3001));
//     }

//     #[tokio::test]
//     async fn test_no_store() {
//         let app = crate::setup_managers_for_test().await;

//         launch_server![3002; (
//             header::CACHE_CONTROL,
//             "no-store"
//         )];

//         assert!(!request_cached!(app, 3002));
//         assert!(!request_cached!(app, 3002));
//     }

//     #[tokio::test]
//     async fn test_etag() {
//         let app = crate::setup_managers_for_test().await;

//         launch_server![3003; (
//             header::ETAG,
//             "test_etag"
//         )];

//         assert!(!request_cached!(app, 3003));
//         assert!(request_cached!(app, 3003));
//     }

//     #[tokio::test]
//     async fn test_last_modified() {
//         let app = crate::setup_managers_for_test().await;

//         launch_server![3004; (
//             header::LAST_MODIFIED,
//             "test_last_modified"
//         )];

//         assert!(!request_cached!(app, 3004));
//         assert!(request_cached!(app, 3004));
//     }
// }
