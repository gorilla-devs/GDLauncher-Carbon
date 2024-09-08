use chrono::{DateTime, Utc};
use hyper::{
    header::{InvalidHeaderValue, AUTHORIZATION, CONTENT_TYPE},
    HeaderMap, StatusCode,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::managers::GDL_API_BASE;

pub struct GDLAccountTask {
    client: reqwest_middleware::ClientWithMiddleware,
}

#[derive(Serialize)]
pub struct RegisterAccountBody {
    pub email: String,
}

#[derive(Serialize)]
pub struct RequestEmailChangeBody {
    pub new_email: String,
}

#[derive(Error, Debug)]
pub enum RequestNewVerificationTokenError {
    #[error("Too many requests")]
    TooManyRequests(u32),

    #[error("request failed: {0}")]
    RequestFailed(anyhow::Error),
}

#[derive(Error, Debug)]
pub enum RequestNewEmailChangeError {
    #[error("Too many requests")]
    TooManyRequests(u32),

    #[error("request failed: {0}")]
    RequestFailed(anyhow::Error),
}

#[derive(Error, Debug)]
pub enum RequestGDLAccountDeletionError {
    #[error("Too many requests")]
    TooManyRequests(u32),

    #[error("request failed: {0}")]
    RequestFailed(anyhow::Error),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GDLUser {
    pub email: String,
    pub microsoft_oid: String,
    pub microsoft_email: Option<String>,
    pub is_verified: bool,
    pub has_pending_verification: bool,
    pub verification_timeout: Option<i64>,
    pub has_pending_deletion_request: bool,
    pub deletion_timeout: Option<i64>,
    pub email_change_timeout: Option<i64>,
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

    pub async fn request_new_verification_token(
        &self,
        id_token: String,
    ) -> Result<(), RequestNewVerificationTokenError> {
        let url = format!("{}/v1/users/request-new-verification-token", GDL_API_BASE);
        let authorization = format!("Bearer {}", id_token);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization.parse().unwrap());

        let resp = self
            .client
            .post(url)
            .headers(headers)
            .send()
            .await
            .map_err(|err| RequestNewVerificationTokenError::RequestFailed(err.into()))?;

        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            let retry_after = resp
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u32>().ok());

            return Err(RequestNewVerificationTokenError::TooManyRequests(
                retry_after.unwrap_or(0),
            ));
        }

        let resp = resp
            .error_for_status()
            .map_err(|err| RequestNewVerificationTokenError::RequestFailed(err.into()))?;

        resp.bytes()
            .await
            .map_err(|err| RequestNewVerificationTokenError::RequestFailed(err.into()))?;

        Ok(())
    }

    pub async fn request_email_change(
        &self,
        id_token: String,
        email: String,
    ) -> Result<(), RequestNewEmailChangeError> {
        let body = serde_json::to_string(&RequestEmailChangeBody { new_email: email })
            .map_err(|err| RequestNewEmailChangeError::RequestFailed(err.into()))?;

        let url = format!("{}/v1/users/request-email-change", GDL_API_BASE);
        let authorization = format!("Bearer {}", id_token);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization.parse().unwrap());
        headers.insert(
            CONTENT_TYPE,
            "application/json"
                .parse()
                .map_err(|err: InvalidHeaderValue| {
                    RequestNewEmailChangeError::RequestFailed(anyhow::anyhow!(err))
                })?,
        );

        let resp = self
            .client
            .post(url)
            .body(reqwest::Body::from(body))
            .headers(headers)
            .send()
            .await
            .map_err(|err| RequestNewEmailChangeError::RequestFailed(err.into()))?;

        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            let retry_after = resp
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u32>().ok());

            return Err(RequestNewEmailChangeError::TooManyRequests(
                retry_after.unwrap_or(0),
            ));
        }

        let resp = resp
            .error_for_status()
            .map_err(|err| RequestNewEmailChangeError::RequestFailed(err.into()))?;

        resp.bytes()
            .await
            .map_err(|err| RequestNewEmailChangeError::RequestFailed(err.into()))?;

        Ok(())
    }

    pub async fn request_deletion(
        &self,
        id_token: String,
    ) -> Result<(), RequestGDLAccountDeletionError> {
        let url = format!("{}/v1/users/request-account-deletion", GDL_API_BASE);

        let authorization = format!("Bearer {}", id_token);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization.parse().unwrap());

        let resp = self
            .client
            .post(url)
            .headers(headers)
            .send()
            .await
            .map_err(|err| RequestGDLAccountDeletionError::RequestFailed(err.into()))?;

        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            let retry_after = resp
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u32>().ok());

            return Err(RequestGDLAccountDeletionError::TooManyRequests(
                retry_after.unwrap_or(0),
            ));
        }

        let resp = resp
            .error_for_status()
            .map_err(|err| RequestGDLAccountDeletionError::RequestFailed(err.into()))?;

        resp.bytes()
            .await
            .map_err(|err| RequestGDLAccountDeletionError::RequestFailed(err.into()))?;

        Ok(())
    }

    pub async fn get_subscription_status(&self) {}
}
