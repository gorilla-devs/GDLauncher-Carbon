use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode};
use serde::Deserialize;

const MS_KEY: &str = "221e73fa-365e-4263-9e06-7a0a1f277960";

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

    pub async fn poll(&self, client: &Client) -> Result<Option<MsAuth>, DeviceCodePollError> {
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

        match response {
            StatusCode::BAD_REQUEST => {
                #[derive(Deserialize)]
                struct BadRequestError {
                    error: String,
                }

                let error = response.json::<BadRequestError>().await?;
                match &error.error {
                    "authorization_pending" => Ok(None),
                    _ => Err(DeviceCodePollError::BadRequest(error.error))
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

                Ok(MsAuth {
                    access_token: response.access_token,
                    id_token: response.id_token,
                    refresh_token: response.refresh_token,
                    expires_at: Utc::now() + chrono::Duration::seconds(response.expires_in),
                })
            },
        }
        todo!()
    }
}

#[derive(Error, Debug)]
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
