use std::sync::Arc;

use markdown::{CompileOptions, Options};
use reqwest_middleware::ClientWithMiddleware;
use serde::Serialize;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentType {
    Metrics,
    TermsAndPrivacy,
}

/// How long a start waits for the check of the terms in force. The launcher
/// shows its loading screen for all of it, on every start without an answer.
pub const STARTUP_CONSENT_CHECK_BUDGET: std::time::Duration = std::time::Duration::from_secs(3);

pub struct TermsAndPrivacy {
    http_client: ClientWithMiddleware,
    gdl_base_api: String,
}

impl TermsAndPrivacy {
    pub fn new(http_client: ClientWithMiddleware, gdl_base_api: String) -> Self {
        Self {
            http_client,
            gdl_base_api,
        }
    }

    #[tracing::instrument(skip(self, secret))]
    pub async fn record_consent<'b>(
        &self,
        consent_type: ConsentType,
        consented: bool,
        user_id: &'b Uuid,
        secret: &'b Vec<u8>,
    ) -> anyhow::Result<String> {
        tracing::info!("Recording consent");

        #[derive(Debug, Serialize)]
        pub struct Body<'c> {
            pub user_id: &'c Uuid,
            pub secret: &'c Vec<u8>,
            pub consent_type: ConsentType,
            pub consented: bool,
        }

        let consent_url = format!("{}/v1/record_consent", self.gdl_base_api);
        let body = Body {
            secret,
            user_id,
            consent_type,
            consented,
        };

        let resp = self
            .http_client
            .post(&consent_url)
            .body(reqwest::Body::from(serde_json::to_string(&body)?))
            .header("Content-Type", "application/json")
            .send()
            .await;

        let accepted_sha = match resp {
            Ok(res) => {
                if res.status().is_success() {
                    tracing::info!("Consent recorded successfully");

                    let sha = res.text().await?;
                    tracing::info!("Accepted sha: {}", sha);

                    sha
                } else {
                    tracing::error!("Failed to record consent: {:?}", res);
                    return Err(anyhow::anyhow!("Failed to record consent"));
                }
            }
            Err(e) => {
                tracing::error!("Failed to record consent: {:?}", e);
                return Err(anyhow::anyhow!("Failed to record consent"));
            }
        };

        Ok(accepted_sha)
    }

    pub async fn fetch_terms_of_service_body(&self) -> anyhow::Result<String> {
        let response = self
            .http_client
            .get(format!("{}/v1/terms_of_service_md", self.gdl_base_api))
            .send()
            .await?
            .text()
            .await?;

        Ok(parse_markdown_document(&response))
    }

    pub async fn fetch_privacy_statement_body(&self) -> anyhow::Result<String> {
        let response = self
            .http_client
            .get(format!("{}/v1/privacy_statement_md", self.gdl_base_api))
            .send()
            .await?
            .text()
            .await?;

        Ok(parse_markdown_document(&response))
    }

    /// The checksum of the terms in force, asked once.
    ///
    /// The start waits for this before it opens the database, so nothing is
    /// retried: a connection that is refused answers at once, and the caller
    /// bounds one that never answers.
    #[tracing::instrument]
    pub async fn get_latest_consent_sha(gdl_base_api: &str) -> anyhow::Result<String> {
        let client = crate::iridium_client::single_attempt_client();

        let url = format!("{}/v1/latest_consent_checksum", gdl_base_api);

        let latest_consent_sha = client.get(&url).send().await?;

        if latest_consent_sha.status() != 200 {
            return Err(anyhow::anyhow!(
                "Failed to fetch latest consent sha: {}",
                latest_consent_sha.status()
            ));
        }

        Ok(latest_consent_sha.text().await?)
    }

    /// What the start knows about the terms in force when it opens the
    /// database: their checksum, or `None` when the API gave none within
    /// `budget`. Without one the stored acceptance is left as it is, and the
    /// next start asks again.
    pub async fn latest_consent_sha_within(
        gdl_base_api: &str,
        budget: std::time::Duration,
    ) -> Option<String> {
        match tokio::time::timeout(budget, Self::get_latest_consent_sha(gdl_base_api)).await {
            Ok(Ok(sha)) => Some(sha),
            Ok(Err(error)) => {
                tracing::warn!("Could not fetch the latest terms and privacy checksum: {error:#}");
                None
            }
            Err(_) => {
                tracing::warn!(
                    "No answer within {budget:?} fetching the latest terms and privacy checksum"
                );
                None
            }
        }
    }
}

fn parse_markdown_document(markdown: &str) -> String {
    markdown::to_html_with_options(
        markdown,
        &Options {
            compile: CompileOptions {
                allow_dangerous_html: true,
                ..CompileOptions::gfm()
            },
            ..Options::gfm()
        },
    )
    .expect("This is guaranteed to work with gfm")
}

#[cfg(test)]
mod startup_check_tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// Longer than any of these tests may take: what ends them is the API's
    /// answer, or the lack of a retry, not this.
    const PATIENT: Duration = Duration::from_secs(60);

    #[tokio::test]
    async fn the_answer_of_the_api_is_the_checksum() {
        let mut api = mockito::Server::new_async().await;
        let asked = api
            .mock("GET", "/v1/latest_consent_checksum")
            .with_status(200)
            .with_body("3f5a")
            .expect(1)
            .create_async()
            .await;

        let checksum = TermsAndPrivacy::latest_consent_sha_within(&api.url(), PATIENT).await;

        assert_eq!(checksum.as_deref(), Some("3f5a"));
        asked.assert_async().await;
    }

    #[tokio::test]
    async fn an_api_that_fails_is_asked_once() {
        let mut api = mockito::Server::new_async().await;
        let asked = api
            .mock("GET", "/v1/latest_consent_checksum")
            .with_status(503)
            .expect(1)
            .create_async()
            .await;

        let checksum = TermsAndPrivacy::latest_consent_sha_within(&api.url(), PATIENT).await;

        assert_eq!(checksum, None);
        asked.assert_async().await;
    }

    /// What a start without a network looks like. With the shared client's
    /// retries this took 3.5 seconds of backoff on every such start.
    #[tokio::test]
    async fn a_refused_connection_is_not_tried_again() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let api = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);

        let started = Instant::now();
        let checksum = TermsAndPrivacy::latest_consent_sha_within(&api, PATIENT).await;

        assert_eq!(checksum, None);
        // Windows itself takes about two seconds to report a refused local
        // connection; a second attempt would add as much again and its wait.
        let one_attempt = Duration::from_millis(if cfg!(windows) { 3000 } else { 400 });
        assert!(
            started.elapsed() < one_attempt,
            "took {:?}: the first retry alone waits 500ms",
            started.elapsed()
        );
    }

    /// A captive portal, a firewall that drops packets, a VPN kill switch:
    /// the connection opens and nothing comes back.
    #[tokio::test]
    async fn an_api_that_never_answers_costs_the_budget_and_no_more() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let api = format!("http://{}", listener.local_addr().unwrap());
        let silent = tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((connection, _)) = listener.accept().await {
                held.push(connection);
            }
        });

        let budget = Duration::from_millis(300);
        let started = Instant::now();
        let checksum = TermsAndPrivacy::latest_consent_sha_within(&api, budget).await;
        let took = started.elapsed();

        assert_eq!(checksum, None);
        assert!(took >= budget, "gave up after {took:?}, before {budget:?}");
        assert!(took < budget + Duration::from_secs(2), "took {took:?}");
        silent.abort();
    }

    /// The loading screen stays for all of this on a network that drops
    /// packets; it was 90 seconds.
    #[test]
    fn a_start_waits_three_seconds_for_the_terms_at_most() {
        assert!(STARTUP_CONSENT_CHECK_BUDGET <= Duration::from_secs(3));
        assert!(STARTUP_CONSENT_CHECK_BUDGET >= Duration::from_secs(1));
    }
}
