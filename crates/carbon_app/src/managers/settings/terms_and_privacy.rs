use std::sync::Arc;

use markdown::{CompileOptions, Options};
use serde::Serialize;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::managers::GDL_API_BASE;

const BASE_GH_API_REPO_URL: &str = "https://api.github.com/repos/gorilla-devs/ToS-Privacy";
const BASE_GH_REPO_URL: &str = "https://raw.githubusercontent.com/gorilla-devs/ToS-Privacy";

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentType {
    Metrics,
    TermsAndPrivacy,
}

pub struct TermsAndPrivacy {
    latest_sha: Arc<Mutex<Option<String>>>,
    reqwest_client: reqwest::Client,
}

impl TermsAndPrivacy {
    pub fn new() -> Self {
        Self {
            latest_sha: Arc::new(Mutex::new(None)),
            reqwest_client: reqwest::Client::builder()
                .user_agent("GDLauncher App")
                .build()
                .expect("Unreasonable to fail"),
        }
    }

    pub async fn record_consent<'a>(
        &self,
        consent_type: ConsentType,
        consented: bool,
        user_id: &'a Uuid,
        secret: &'a Vec<u8>,
    ) -> anyhow::Result<()> {
        let mut lock = self.latest_sha.lock().await;
        let latest_sha = match lock.as_ref() {
            Some(sha) => sha.to_owned(),
            None => {
                let sha = self.update_latest_commit_sha().await?;
                *lock = Some(sha.clone());
                sha
            }
        };

        #[derive(Debug, Serialize)]
        pub struct Body<'b> {
            pub user_id: &'b Uuid,
            pub secret: &'b Vec<u8>,
            pub consent_type: ConsentType,
            pub consented: bool,
            pub document_hash: String,
        }

        let consent_url = format!("{}/v1/record_consent", GDL_API_BASE);
        let body = Body {
            document_hash: latest_sha,
            secret,
            user_id,
            consent_type,
            consented,
        };

        let res = self
            .reqwest_client
            .post(&consent_url)
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            tracing::error!("Failed to record consent: {:?}", res);

            anyhow::bail!("Failed to record consent");
        }

        Ok(())
    }

    pub async fn update_latest_commit_sha(&self) -> anyhow::Result<String> {
        let response = self
            .reqwest_client
            .get(&format!("{BASE_GH_API_REPO_URL}/commits/master"))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let sha = response
            .get("sha")
            .ok_or_else(|| anyhow::anyhow!("No sha found"))?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Sha is not a string"))?;

        Ok(sha.to_owned())
    }

    pub async fn fetch_terms_of_service_body(&self) -> anyhow::Result<String> {
        let mut lock = self.latest_sha.lock().await;
        let latest_sha = match lock.as_ref() {
            Some(sha) => sha.to_owned(),
            None => {
                let sha = self.update_latest_commit_sha().await?;
                *lock = Some(sha.clone());
                sha
            }
        };

        let response = self
            .reqwest_client
            .get(format!(
                "{}/{}/terms-of-service.md",
                BASE_GH_REPO_URL, latest_sha
            ))
            .send()
            .await?
            .text()
            .await?;

        Ok(parse_markdown_document(&response))
    }

    pub async fn fetch_privacy_statement_body(&self) -> anyhow::Result<String> {
        let mut lock = self.latest_sha.lock().await;
        let latest_sha = match lock.as_ref() {
            Some(sha) => sha.to_owned(),
            None => {
                let sha = self.update_latest_commit_sha().await?;
                *lock = Some(sha.clone());
                sha
            }
        };

        let response = self
            .reqwest_client
            .get(format!(
                "{}/{}/privacy-statement.md",
                BASE_GH_REPO_URL, latest_sha
            ))
            .send()
            .await?
            .text()
            .await?;

        Ok(parse_markdown_document(&response))
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
