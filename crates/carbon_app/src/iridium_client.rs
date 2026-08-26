/// Builds the shared client's transport config. Split out so tests can
/// inject sub-second `connect`/`read` durations against a local server
/// instead of waiting on the real multi-second production budgets.
fn shared_client_builder(
    connect: std::time::Duration,
    read: std::time::Duration,
) -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .user_agent(format!(
            "{} {}",
            env!("USER_AGENT_PREFIX"),
            env!("APP_VERSION")
        ))
        .connect_timeout(connect)
        // Bounds the gap between reads, not the whole transfer: a stalled
        // connection dies fast while a slow, still-moving large body
        // survives. Server-side downloads (server packs and mrpacks in
        // server/modpack.rs, modloader installer jars in
        // server/modloader_install.rs) stream multi-hundred-MB bodies
        // straight through this client, so the budget must tolerate
        // transfers that take minutes as long as bytes keep arriving.
        // Vanilla server.jar bytes do not go through this client -- they
        // stream through carbon_net's own downloader instead, and
        // server/jars.rs only uses this client for two small JSON metadata
        // GETs (the version manifest and version details).
        .read_timeout(read)
}

pub fn get_client(gdl_base_api: String) -> reqwest_middleware::ClientBuilder {
    use reqwest::{Request, Response};
    use reqwest_middleware::{Middleware, Next};

    use crate::managers::modplatforms::modrinth::MODRINTH_API_BASE;

    /// Retries requests that fail transiently (5xx, 429, connect/timeout
    /// errors) with exponential backoff. Only replayable requests are
    /// retried: GETs, and the read-only POST queries of the mod platform
    /// APIs. Streaming uploads can't be cloned and pass through untouched.
    struct TransientRetryMiddleware {
        max_retries: u32,
        retryable_post_hosts: Vec<String>,
    }

    #[async_trait::async_trait]
    impl Middleware for TransientRetryMiddleware {
        async fn handle(
            &self,
            req: Request,
            extensions: &mut axum::http::Extensions,
            next: Next<'_>,
        ) -> reqwest_middleware::Result<Response> {
            let host_allows_post = req
                .url()
                .host_str()
                .is_some_and(|host| self.retryable_post_hosts.iter().any(|h| h == host));
            let replayable = req.method() == reqwest::Method::GET
                || (req.method() == reqwest::Method::POST && host_allows_post);

            let mut attempt = 0u32;
            loop {
                let attempt_req = if replayable { req.try_clone() } else { None };
                let Some(attempt_req) = attempt_req else {
                    return next.run(req, extensions).await;
                };

                let result = next.clone().run(attempt_req, extensions).await;
                let transient = match &result {
                    Ok(response) => {
                        let status = response.status();
                        status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS
                    }
                    Err(reqwest_middleware::Error::Reqwest(error)) => {
                        error.is_connect() || error.is_timeout()
                    }
                    Err(_) => false,
                };

                if !transient || attempt >= self.max_retries {
                    return result;
                }

                // A 429 that names its window is obeyed rather than guessed at:
                // both platforms limit per minute, so the plain backoff below
                // retries inside the same window and is refused again.
                let declared_wait = match &result {
                    Ok(response) if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS => {
                        rate_limit_wait(response.headers())
                    }
                    _ => None,
                };
                if exceeds_honoured_rate_limit_wait(declared_wait) {
                    tracing::warn!(
                        "rate limited by {} for longer than we will wait; returning the response",
                        req.url()
                    );
                    return result;
                }
                let delay = declared_wait
                    .unwrap_or_else(|| std::time::Duration::from_millis(500u64 << attempt));
                match &result {
                    Ok(response) => tracing::warn!(
                        "transient {} from {}, retrying in {:?}",
                        response.status(),
                        req.url(),
                        delay
                    ),
                    Err(error) => tracing::warn!(
                        "transient network error from {} ({}), retrying in {:?}",
                        req.url(),
                        error,
                        delay
                    ),
                }
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
        }
    }

    struct AddHeaderMiddleware {
        gdl_api_base_host: url::Url,
    };

    let gdl_api_base_host =
        url::Url::parse(&gdl_base_api).expect("Failed to parse GDLauncher API base URL");

    #[async_trait::async_trait]
    impl Middleware for AddHeaderMiddleware {
        async fn handle(
            &self,
            mut req: Request,
            _extensions: &mut axum::http::Extensions,
            next: Next<'_>,
        ) -> reqwest_middleware::Result<Response> {
            let curseforge_api_base = url::Url::parse(env!(
                "CURSEFORGE_API_BASE",
                "missing curseforge env api base"
            ))
            .expect("Failed to parse CURSEFORGE_API_BASE environment variable");

            if req.url().host_str() == curseforge_api_base.host_str() {
                let api_key = option_env!("CURSEFORGE_API_KEY")
                    .expect("CURSEFORGE_API_KEY environment variable not set. Please set it to use CurseForge features.");

                let api_key_header = api_key
                    .parse()
                    .expect("Failed to parse CURSEFORGE_API_KEY as header value");

                req.headers_mut().insert("x-api-key", api_key_header);

                req.headers_mut().insert(
                    "Content-Type",
                    "application/json"
                        .parse()
                        .expect("Failed to parse Content-Type header"),
                );

                req.headers_mut().insert(
                    "Accept",
                    "application/json"
                        .parse()
                        .expect("Failed to parse Accept header"),
                );
            }

            let modrinth_api_base =
                url::Url::parse(MODRINTH_API_BASE).expect("Failed to parse MODRINTH_API_BASE URL");

            if req.url().host_str() == modrinth_api_base.host_str() {
                req.headers_mut().insert(
                    "Content-Type",
                    "application/json"
                        .parse()
                        .expect("Failed to parse Content-Type header"),
                );

                req.headers_mut().insert(
                    "Accept",
                    "application/json"
                        .parse()
                        .expect("Failed to parse Accept header"),
                );
            }

            // Continue with the modified request.
            next.run(req, _extensions).await
        }
    }

    let client = shared_client_builder(
        std::time::Duration::from_secs(30),
        std::time::Duration::from_secs(60),
    )
    .build()
    .expect("Failed to build HTTP client");

    let retryable_post_hosts = [
        url::Url::parse(MODRINTH_API_BASE).ok(),
        option_env!("CURSEFORGE_API_BASE").and_then(|base| url::Url::parse(base).ok()),
    ]
    .into_iter()
    .flatten()
    .filter_map(|url| url.host_str().map(str::to_string))
    .collect();

    reqwest_middleware::ClientBuilder::new(client)
        .with(TransientRetryMiddleware {
            max_retries: 3,
            retryable_post_hosts,
        })
        .with(AddHeaderMiddleware { gdl_api_base_host })
}

#[cfg(test)]
mod test {
    #[tokio::test]
    async fn get_retries_transient_503() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/flaky")
            .with_status(503)
            .expect(4) // initial attempt + 3 retries
            .create_async()
            .await;

        let client = super::get_client(server.url()).build();
        let response = client
            .get(format!("{}/flaky", server.url()))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn get_success_is_not_retried() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/ok")
            .with_status(200)
            .with_body("fine")
            .expect(1)
            .create_async()
            .await;

        let client = super::get_client(server.url()).build();
        let response = client
            .get(format!("{}/ok", server.url()))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn post_to_unknown_host_is_not_retried() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/upload")
            .with_status(503)
            .expect(1)
            .create_async()
            .await;

        let client = super::get_client(server.url()).build();
        let response = client
            .post(format!("{}/upload", server.url()))
            .body("payload")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn stalled_response_times_out() {
        use tokio::io::AsyncWriteExt;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind test server");
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept failed");
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\n")
                .await
                .expect("failed to write headers");
            // Never write the body: the connection just sits open.
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        });

        let client = super::shared_client_builder(
            std::time::Duration::from_secs(5),
            std::time::Duration::from_millis(500),
        )
        .build()
        .expect("failed to build client");

        let result = client
            .get(format!("http://{addr}/"))
            .send()
            .await
            .expect("headers should arrive")
            .bytes()
            .await;

        let err = result.expect_err("a stalled body must time out");
        assert!(err.is_timeout(), "expected a timeout error, got {err:?}");
    }

    #[tokio::test]
    async fn slow_stream_is_not_capped() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        const CHUNK: &[u8] = b"0123456789";
        const CHUNK_COUNT: usize = 30;

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind test server");
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept failed");
            // Drain the request before responding. Nothing here needs its
            // contents, but leaving it unread means this socket still holds
            // received data when the task ends and drops it, and closing a
            // socket with unread data in its receive buffer sends an RST
            // rather than a FIN. That RST discards what the peer has not yet
            // consumed, so the client surfaces `ConnectionReset` partway
            // through decoding instead of the complete body asserted below —
            // deterministically so on Windows, where the reset wins the race
            // against the last chunks every time.
            let mut request = [0u8; 2048];
            let _ = socket.read(&mut request).await;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                        CHUNK.len() * CHUNK_COUNT
                    )
                    .as_bytes(),
                )
                .await
                .expect("failed to write headers");
            // A chunk every 100ms for 3s. What is under test is that the
            // budget applies per read gap and not to the transfer as a whole,
            // so the total has to exceed it while no single gap comes close.
            // The gap budget below is 2s against a 100ms cadence: a loaded CI
            // box has to stall a sleep by 20x before this reports a cap that
            // did not happen. At 500ms it only took a 5x stall, which a
            // Windows runner under a full parallel suite does hit.
            for _ in 0..CHUNK_COUNT {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                socket
                    .write_all(CHUNK)
                    .await
                    .expect("failed to write chunk");
            }
        });

        let client = super::shared_client_builder(
            std::time::Duration::from_secs(10),
            std::time::Duration::from_secs(2),
        )
        .build()
        .expect("failed to build client");

        let body = client
            .get(format!("http://{addr}/"))
            .send()
            .await
            .expect("headers should arrive")
            .bytes()
            .await
            .expect("a slow but steady stream must not be capped by the read gap");

        assert_eq!(body.len(), CHUNK.len() * CHUNK_COUNT);
    }
}

/// How long a server-declared rate-limit window may be before the request is
/// handed back instead of waited out. Both platforms limit per minute, so a
/// genuine window never exceeds ~60s; beyond that, holding the caller is
/// worse than surfacing the 429.
const MAX_HONOURED_RATE_LIMIT_WAIT: std::time::Duration = std::time::Duration::from_secs(70);

/// Separate from the middleware so the policy can be asserted without
/// sleeping through a real window.
fn exceeds_honoured_rate_limit_wait(declared_wait: Option<std::time::Duration>) -> bool {
    declared_wait.is_some_and(|wait| wait > MAX_HONOURED_RATE_LIMIT_WAIT)
}

/// The wait a 429 response asks for, from `Retry-After` (delta-seconds) or
/// Modrinth's `X-Ratelimit-Reset` (seconds until the window rolls over).
///
/// Both platforms rate-limit on a per-minute window, so a fixed sub-second
/// backoff lands inside the same window and is spent for nothing: the retry is
/// refused too, the caller still ends up with a 429, and the extra traffic
/// pushes the window further out.
///
/// The two headers are tried in order, and the first one that actually
/// parses as a plain integer wins -- not just the first one present. This
/// matters because `Retry-After` may be sent in HTTP-date form (RFC 9110),
/// which is not parsed here: if a response carries an HTTP-date
/// `Retry-After` alongside a numeric `X-Ratelimit-Reset`, the reset header is
/// now the one honoured instead of the pair reading as no declared wait at
/// all.
///
/// A parsed value is normally delta-seconds, but some servers reuse the same
/// field for an absolute reset instant (Unix epoch seconds) instead. The two
/// readings are told apart by size: `EPOCH_LIKE_THRESHOLD` (10^6 seconds,
/// ~11.5 days) is far beyond any rate-limit window either platform actually
/// declares, so a value that large can only be a timestamp, never a genuine
/// relative wait -- and it's converted to a delta against the current time
/// and clamped to `MAX_HONOURED_RATE_LIMIT_WAIT`. Values at or under that
/// line are always taken as a relative wait exactly as before, even ones
/// that exceed `MAX_HONOURED_RATE_LIMIT_WAIT`: that excess is handled by the
/// caller, which gives up and surfaces the response rather than waiting.
fn rate_limit_wait(headers: &reqwest::header::HeaderMap) -> Option<std::time::Duration> {
    /// Below this, a parsed header value is taken as a relative wait in
    /// seconds, however large; at or above it, it can only be a Unix epoch
    /// timestamp -- no real rate-limit window is anywhere close to 10^6
    /// seconds (~11.5 days), so this can never misclassify a legitimate
    /// relative wait as an absolute one.
    const EPOCH_LIKE_THRESHOLD: u64 = 1_000_000;

    let parsed = ["retry-after", "x-ratelimit-reset"]
        .iter()
        .filter_map(|name| headers.get(*name))
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.trim().parse::<u64>().ok())
        .next()?;

    if parsed < EPOCH_LIKE_THRESHOLD {
        return Some(std::time::Duration::from_secs(parsed));
    }

    let now_epoch_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let delta = parsed.saturating_sub(now_epoch_secs);

    Some(std::time::Duration::from_secs(
        delta.min(MAX_HONOURED_RATE_LIMIT_WAIT.as_secs()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderMap;
    use std::time::Duration;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(
                reqwest::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                value.parse().unwrap(),
            );
        }
        map
    }

    #[test]
    fn reads_retry_after_seconds() {
        assert_eq!(
            rate_limit_wait(&headers(&[("retry-after", "12")])),
            Some(Duration::from_secs(12))
        );
    }

    #[test]
    fn reads_the_modrinth_reset_header() {
        assert_eq!(
            rate_limit_wait(&headers(&[("x-ratelimit-reset", "45")])),
            Some(Duration::from_secs(45))
        );
    }

    #[test]
    fn retry_after_wins_over_the_reset_header() {
        assert_eq!(
            rate_limit_wait(&headers(&[
                ("x-ratelimit-reset", "45"),
                ("retry-after", "5")
            ])),
            Some(Duration::from_secs(5))
        );
    }

    #[test]
    fn absent_or_unparseable_values_read_as_no_declared_wait() {
        assert_eq!(rate_limit_wait(&HeaderMap::new()), None);
        // The HTTP-date form is not parsed, and nothing else is present to
        // fall back to.
        assert_eq!(
            rate_limit_wait(&headers(&[(
                "retry-after",
                "Wed, 21 Oct 2015 07:28:00 GMT"
            )])),
            None
        );
    }

    #[test]
    fn http_date_retry_after_falls_through_to_a_parseable_reset_header() {
        // Regression: picking the first *present* header rather than the
        // first *parseable* one used to make this pair read as no declared
        // wait at all, even though x-ratelimit-reset is right there and
        // perfectly usable.
        assert_eq!(
            rate_limit_wait(&headers(&[
                ("retry-after", "Wed, 21 Oct 2015 07:28:00 GMT"),
                ("x-ratelimit-reset", "45"),
            ])),
            Some(Duration::from_secs(45))
        );
    }

    #[test]
    fn both_headers_garbage_reads_as_no_declared_wait() {
        assert_eq!(
            rate_limit_wait(&headers(&[
                ("retry-after", "not-a-number"),
                ("x-ratelimit-reset", "also-not-a-number"),
            ])),
            None
        );
    }

    fn current_epoch_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[test]
    fn epoch_style_reset_yields_a_delta_from_now() {
        // Some servers send an absolute reset instant instead of
        // delta-seconds. A value this far above any real rate-limit window
        // can only be a Unix timestamp, so it must convert to a sane
        // (small, non-negative) wait rather than being handed to the caller
        // as a multi-decade delay.
        let reset_at = current_epoch_secs() + 5;
        let wait = rate_limit_wait(&headers(&[("x-ratelimit-reset", &reset_at.to_string())]))
            .expect("an epoch-style reset must still be honoured");

        assert!(
            wait <= Duration::from_secs(5),
            "expected a delta close to 5s, got {wait:?}"
        );
    }

    #[test]
    fn a_window_length_rate_limit_is_waited_out_not_surfaced() {
        assert!(!exceeds_honoured_rate_limit_wait(Some(
            Duration::from_secs(45)
        )));
        assert!(!exceeds_honoured_rate_limit_wait(Some(
            Duration::from_secs(60)
        )));
    }

    #[test]
    fn a_wait_beyond_a_full_window_is_still_surfaced() {
        assert!(exceeds_honoured_rate_limit_wait(Some(Duration::from_secs(
            600
        ))));
    }

    #[test]
    fn no_declared_wait_is_not_treated_as_too_long() {
        assert!(!exceeds_honoured_rate_limit_wait(None));
    }

    #[test]
    fn epoch_style_reset_far_in_the_future_clamps_to_the_max_honoured_wait() {
        let reset_at = current_epoch_secs() + 3600;
        let wait = rate_limit_wait(&headers(&[("x-ratelimit-reset", &reset_at.to_string())]))
            .expect("an epoch-style reset must still be honoured");

        assert_eq!(wait, MAX_HONOURED_RATE_LIMIT_WAIT);
    }

    #[test]
    fn large_relative_wait_under_the_epoch_threshold_is_not_treated_as_a_timestamp() {
        // 600,000 seconds (~6.9 days) is a huge wait, but it is well under
        // the 10^6-second epoch-like threshold, so it must still read as a
        // plain relative delta -- not get reinterpreted as a Unix timestamp
        // and collapse to a near-zero or saturated delta.
        assert_eq!(
            rate_limit_wait(&headers(&[("retry-after", "600000")])),
            Some(Duration::from_secs(600000))
        );
    }
}
