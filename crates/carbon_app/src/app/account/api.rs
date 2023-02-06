use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use thiserror::Error;

const MS_KEY: &str = "221e73fa-365e-4263-9e06-7a0a1f277960";

#[derive(Clone)]
pub struct DeviceCode {
    pub user_code: String,
    device_code: String,
    pub verification_uri: String,
    pub polling_interval: Duration,
    pub expires_at: DateTime<Utc>,
}

impl DeviceCode {
    pub async fn request_code(client: &Client) -> Result<Self, DeviceCodeRequestError> {
        #[derive(Deserialize)]
        struct DeviceCodeResponse {
            user_code: String,
            device_code: String,
            verification_uri: String,
            expires_in: i64,
            interval: u32,
            // message: String,
        }

        let response = client
            .get("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
            .query(&[
                ("client_id", MS_KEY),
                (
                    "scope",
                    "XboxLive.signin XboxLive.offline_access profile openid email",
                ),
            ])
            .header("content-length", "0")
            .send()
            .await?
            .error_for_status()?
            .json::<DeviceCodeResponse>()
            .await?;

        Ok(Self {
            user_code: response.user_code,
            device_code: response.device_code,
            verification_uri: response.verification_uri,
            polling_interval: Duration::from_secs(2),
            expires_at: Utc::now() + chrono::Duration::seconds(response.expires_in),
        })
    }

    pub async fn poll_ms_auth(&self, client: &Client) -> Result<MsAuth, DeviceCodePollError> {
        loop {
            tokio::time::sleep(self.polling_interval).await;

            let response = client
                .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
                .form(&[
                    ("client_id", MS_KEY),
                    (
                        "scope",
                        "XboxLive.signin XboxLive.offline_access profile openid email",
                    ),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("device_code", &self.device_code),
                ])
                .send()
                .await?;

            match response.status() {
                StatusCode::BAD_REQUEST => {
                    #[derive(Deserialize)]
                    struct BadRequestError {
                        error: String,
                    }

                    let error = response.json::<BadRequestError>().await?;
                    match &error.error as &str {
                        "authorization_pending" => continue,
                        _ => break Err(DeviceCodePollError::BadRequest(error.error))
                    }
                }
                StatusCode::OK => {
                    #[derive(Deserialize)]
                    struct MsAuthResponse {
                        access_token: String,
                        id_token: String,
                        refresh_token: String,
                        expires_in: i64,
                    }

                    let response = response.json::<MsAuthResponse>().await?;

                    break Ok(MsAuth {
                        access_token: response.access_token,
                        id_token: response.id_token,
                        refresh_token: response.refresh_token,
                        expires_at: Utc::now() + chrono::Duration::seconds(response.expires_in),
                    })
                },
                status => {
                    break Err(DeviceCodePollError::UnexpectedResponse(status))
                }
            }
        }
    }
}

#[derive(Error, Debug)]
#[error("reqwest error: {0}")]
pub struct DeviceCodeRequestError(#[from] reqwest::Error);

#[derive(Error, Debug)]
pub enum DeviceCodePollError {
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("authentication server returned: bad request: {0}")]
    BadRequest(String),

    #[error("authentication server returned unexpected response: {0}")]
    UnexpectedResponse(StatusCode),
}

pub struct MsAuth {
    pub access_token: String,
    pub id_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

impl MsAuth {
    /// Refresh the auth token, returning a new token if the current one
    /// has expired.
    pub async fn refresh(&mut self, client: &Client) -> Result<bool, MsAuthRefreshError> {
        if self.expires_at > Utc::now() {
            #[derive(Deserialize)]
            struct RefreshResponse {
                access_token: String,
                refresh_token: String,
                expires_in: i64,
            }

            let response = client
                .post("https://login.live.com/oauth20_token.srf")
                .form(&[
                    ("client_id", MS_KEY),
                    ("refresh_token", &self.refresh_token),
                    ("grant_type", "refresh_token"),
                    (
                        "redirect_uri",
                        "https://login.microsoftonline.com/common/oauth2/nativeclient",
                    ),
                ])
                .send()
                .await?
                .error_for_status()?
                .json::<RefreshResponse>()
                .await?;

            self.access_token = response.access_token;
            self.refresh_token = response.refresh_token;
            self.expires_at = Utc::now() + chrono::Duration::seconds(response.expires_in);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[derive(Error, Debug)]
#[error("reqwest error: {0}")]
pub struct MsAuthRefreshError(#[from] reqwest::Error);
