use std::time::Duration;

use chrono::{DateTime, Utc};
use jsonwebtoken::{errors::ErrorKind, Algorithm, DecodingKey, Validation};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::json;
use thiserror::Error;

const MS_KEY: &str = "221e73fa-365e-4263-9e06-7a0a1f277960";

#[derive(Debug, Clone)]
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
                        _ => break Err(DeviceCodePollError::BadRequest(error.error)),
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
                    });
                }
                status => break Err(DeviceCodePollError::UnexpectedResponse(status)),
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

struct XboxAuth {
    xsts_token: String,
    userhash: String,
}

impl XboxAuth {
    /// Obtain an Xbox account from a MS account (without refreshing it)
    pub async fn from_ms(ms_auth: &MsAuth, client: &Client) -> Result<Self, XboxAuthError> {
        let xbl_token = {
            #[derive(Deserialize)]
            struct XblToken {
                #[serde(rename = "Token")]
                token: String,
            }

            let json = json!({
                "Properties": {
                    "AuthMethod": "RPS",
                    "SiteName":   "user.auth.xboxlive.com",
                    "RpsTicket": format!("d={}", &ms_auth.access_token),
                },
                "RelyingParty": "http://auth.xboxlive.com",
                "TokenType":    "JWT",
            });

            let response = client
                .post("https://user.auth.xboxlive.com/user/authenticate")
                .header("Accept", "application/json")
                .json(&json)
                .send()
                .await?
                .error_for_status()?
                .json::<XblToken>()
                .await?;

            response.token
        };

        // get xsts token

        let json = json!({
            "Properties": {
                "SandboxId":  "RETAIL",
                "UserTokens": [xbl_token],
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType":    "JWT",
        });

        let response = client
            .post("https://xsts.auth.xboxlive.com/xsts/authorize")
            .header("Content-Type", "application/json")
            .json(&json)
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                #[derive(Deserialize)]
                struct XstsToken {
                    #[serde(rename = "Token")]
                    token: String,
                    #[serde(rename = "DisplayClaims")]
                    display_claims: DisplayClaims,
                }

                #[derive(Deserialize)]
                struct DisplayClaims {
                    xui: Vec<Xui>,
                }

                #[derive(Deserialize)]
                struct Xui {
                    uhs: String,
                }

                let response = response.json::<XstsToken>().await?;
                Ok(Self {
                    xsts_token: response.token,
                    userhash: response
                        .display_claims
                        .xui
                        .get(0)
                        .ok_or(XboxAuthError::MissingUserhash)?
                        .uhs
                        .clone(),
                })
            }
            StatusCode::UNAUTHORIZED => {
                #[derive(Deserialize)]
                struct XstsError {
                    #[serde(rename = "XErr")]
                    xerr: u64,
                }

                let xsts_err = response.json::<XstsError>().await?;
                Err(XboxAuthError::from_xerr(xsts_err.xerr))
            }
            status => Err(XboxAuthError::UnexpectedResponse(status)),
        }
    }
}

#[derive(Error, Debug)]
pub enum XboxAuthError {
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("missing userhash")]
    MissingUserhash,

    #[error("unexpected response: {0}")]
    UnexpectedResponse(StatusCode),

    #[error("no xbox account is associated with this microsoft account")]
    NoAccount,

    #[error("xbox live is not availible in this country")]
    XboxServicesBanned,

    #[error("this xbox account must be verified as an adult")]
    AdultVerificationRequired,

    #[error("this xbox account is a child account, and must have a family associated")]
    ChildAccount,

    #[error("unknown xbox error code: {0}")]
    Unknown(u64),
}

impl XboxAuthError {
    // error code from an XErr code returned by the XSTS auth endpoint
    fn from_xerr(xerr: u64) -> Self {
        match xerr {
            2148916233 => Self::NoAccount,
            2148916235 => Self::XboxServicesBanned,
            2148916236 | 2148916237 => Self::AdultVerificationRequired,
            2148916238 => Self::ChildAccount,
            xerr => Self::Unknown(xerr),
        }
    }
}

#[derive(Debug)]
pub struct McAuth {
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
}

impl McAuth {
    /// Authenticate with a MS account (without refreshing it)
    pub async fn auth_ms(ms_auth: &MsAuth, client: &Client) -> Result<Self, McAuthError> {
        let xbox_auth = XboxAuth::from_ms(ms_auth, client).await?;

        let json = json!({
            "identityToken": format!("XBL3.0 x={};{}", xbox_auth.userhash, xbox_auth.xsts_token)
        });

        #[derive(Deserialize)]
        struct McAuthResponse {
            access_token: String,
            expires_in: i64,
        }

        let response = client
            .post("https://api.minecraftservices.com/authentication/login_with_xbox")
            .header("Accept", "application/json")
            .json(&json)
            .send()
            .await?
            .error_for_status()?
            .json::<McAuthResponse>()
            .await?;

        Ok(Self {
            access_token: response.access_token,
            expires_at: Utc::now() + chrono::Duration::seconds(response.expires_in),
        })
    }
}

#[derive(Error, Debug)]
pub enum McAuthError {
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("error getting xbox auth: {0}")]
    Xbox(#[from] XboxAuthError),
}

pub enum McEntitlements {
    None,
    Owned,
    XboxGamepass,
}

impl McEntitlements {
    fn mojang_jwt_key() -> DecodingKey {
        // The test at the bottom of this file makes sure this unwrap is fine.
        DecodingKey::from_rsa_pem(include_bytes!("mojang_jwt_signature.pem")).unwrap()
    }

    pub async fn check_entitlements(
        mc_auth: &McAuth,
        client: &Client,
    ) -> Result<McEntitlements, McEntitlementCheckError> {
        #[derive(Deserialize)]
        struct EntitlementResponse {
            signature: String,
        }

        let response = client
            .get("https://api.minecraftservices.com/entitlements/mcstore")
            .bearer_auth(&mc_auth.access_token)
            .send()
            .await?
            .error_for_status()?
            .json::<EntitlementResponse>()
            .await?;

        println!("Reponse: {}", &response.signature);

        #[derive(Debug, Deserialize)]
        struct SignedEntitlements {
            entitlements: Vec<SignedEntitlement>,
        }

        #[derive(Debug, Deserialize)]
        struct SignedEntitlement {
            name: String,
        }

        // The only part of the response we use is the JWT signature part,
        // as its data is confirmed to be signed by mojang, and contains
        // everything we actually need to check game ownership.
        let entitlements = jsonwebtoken::decode::<SignedEntitlements>(
            &response.signature,
            &Self::mojang_jwt_key(),
            &Validation::new(Algorithm::RS256),
        );

        let entitlements = match entitlements {
            Ok(jwt) => jwt.claims,
            Err(e) => {
                let error = match e.kind() {
                    ErrorKind::InvalidSignature | ErrorKind::ImmatureSignature => {
                        McEntitlementCheckError::InvalidSignature
                    }
                    ErrorKind::InvalidToken | ErrorKind::MissingRequiredClaim(_) => {
                        McEntitlementCheckError::InvalidData
                    }
                    ErrorKind::MissingAlgorithm => McEntitlementCheckError::Outdated,
                    _ => McEntitlementCheckError::Jwt(e),
                };

                return Err(error);
            }
        };

        println!("Entitlements: {entitlements:#?}");

        todo!("parse signed entitlements")
    }
}

#[derive(Error, Debug)]
pub enum McEntitlementCheckError {
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("response data was not valid")]
    InvalidData,

    #[error("response data could not be verified")]
    InvalidSignature,

    #[error("GDLauncher account verifcation checks are outdated")]
    Outdated,

    #[error("JWT error: {0}")]
    Jwt(jsonwebtoken::errors::Error),
}

#[cfg(test)]
mod test {
    use super::McEntitlements;

    /// Make sure it's possible to get a JWT decoding key from
    /// the saved public key.
    #[test]
    fn valid_mojang_account_sig() {
        // unwrap performed inside
        let _ = McEntitlements::mojang_jwt_key();
    }
}
