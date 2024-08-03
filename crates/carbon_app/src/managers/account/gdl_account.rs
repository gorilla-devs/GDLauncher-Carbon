use chrono::{DateTime, Utc};
use hyper::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    HeaderMap, StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::managers::GDL_API_BASE;

pub struct GDLAccountTask {
    client: reqwest_middleware::ClientWithMiddleware,
}

#[derive(Serialize)]
pub struct RegisterAccountBody {
    pub email: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GDLUser {
    pub email: String,
    pub microsoft_oid: String,
    pub microsoft_email: Option<String>,
    pub is_email_verified: bool,
}

impl GDLAccountTask {
    pub fn new(client: reqwest_middleware::ClientWithMiddleware) -> Self {
        Self { client }
    }

    pub async fn register_account(
        &self,
        body: RegisterAccountBody,
        id_token: String,
    ) -> anyhow::Result<GDLUser> {
        let url = format!("{}/v1/users/sign-up", GDL_API_BASE);

        let authorization = format!("Bearer {}", id_token);

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization.parse()?);
        headers.insert(CONTENT_TYPE, "application/json".parse()?);

        let body = serde_json::to_string(&body)?;

        let resp = self
            .client
            .post(url)
            .headers(headers)
            .body(body)
            .send()
            .await?;

        let resp = resp.error_for_status()?;

        let user: GDLUser = resp.json().await?;

        Ok(user)
    }

    pub async fn get_account(&self, id_token: String) -> anyhow::Result<Option<GDLUser>> {
        let url = format!("{}/v1/users/user", GDL_API_BASE);
        let authorization = format!("Bearer {}", id_token);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization.parse().unwrap());

        let resp = self.client.get(url).headers(headers).send().await?;

        if resp.status() == StatusCode::NO_CONTENT {
            return Ok(None);
        }

        let resp = resp.error_for_status()?;

        let user: GDLUser = resp.json().await?;

        Ok(Some(user))
    }

    pub async fn get_subscription_status(&self) {}
}
