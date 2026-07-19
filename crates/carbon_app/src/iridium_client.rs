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

                let delay = std::time::Duration::from_millis(500u64 << attempt);
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

    let client = reqwest::Client::builder()
        .user_agent(format!(
            "{} {}",
            env!("USER_AGENT_PREFIX"),
            env!("APP_VERSION")
        ))
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
}
