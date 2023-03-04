use std::time::Duration;

use chrono::{DateTime, Utc};
use jsonwebtoken::{errors::ErrorKind, Algorithm, DecodingKey, Validation};
use reqwest::{Client, StatusCode};
use rspc::Type;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use crate::error::request::{RequestContext, RequestError, RequestErrorDetails};

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
            .await
            .map_err(RequestError::from_error_censored)?
            .error_for_status()
            .map_err(RequestError::from_error_censored)?
            .json::<DeviceCodeResponse>()
            .await
            .map_err(RequestError::from_error_censored)?;

        Ok(Self {
            user_code: response.user_code,
            device_code: response.device_code,
            verification_uri: response.verification_uri,
            polling_interval: Duration::from_secs(response.interval.into()),
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
                .await
                .map_err(RequestError::from_error)?;

            match response.status() {
                StatusCode::BAD_REQUEST => {
                    #[derive(Deserialize)]
                    struct BadRequestError {
                        error: String,
                    }

                    let error = response
                        .json::<BadRequestError>()
                        .await
                        .map_err(RequestError::from_error)?
                        .error;

                    return match &error as &str {
                        "authorization_pending" => continue,
                        "expired_token" => Err(DeviceCodePollError::CodeExpired)?,
                        _ => Err(DeviceCodePollError::Request(RequestError {
                            context: RequestContext::none(),
                            error: RequestErrorDetails::UnexpectedStatus {
                                status: StatusCode::BAD_REQUEST,
                                details: Some(error),
                            },
                        })),
                    };
                }
                StatusCode::OK => {
                    #[derive(Deserialize)]
                    struct MsAuthResponse {
                        access_token: String,
                        //id_token: String,
                        refresh_token: String,
                        expires_in: i64,
                    }

                    let response = response
                        .json::<MsAuthResponse>()
                        .await
                        .map_err(RequestError::from_error)?;

                    break Ok(MsAuth {
                        access_token: response.access_token,
                        //id_token: response.id_token,
                        refresh_token: response.refresh_token,
                        expires_at: Utc::now() + chrono::Duration::seconds(response.expires_in),
                    });
                }
                _ => Err(DeviceCodePollError::Request(RequestError::from_status(
                    &response,
                )))?,
            }
        }
    }
}

#[derive(Error, Debug, Clone)]
pub enum DeviceCodeRequestError {
    #[error("{0}")]
    Request(#[from] RequestError),
}

#[derive(Error, Debug, Clone)]
pub enum DeviceCodePollError {
    #[error("request error: {0}")]
    Request(#[from] RequestError),
    #[error("device code expired")]
    CodeExpired,
}

#[derive(Debug, Clone)]
pub struct MsAuth {
    pub access_token: String,
    //pub id_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

impl MsAuth {
    /// Refresh the auth token, returning a new token if the current one
    /// has expired.
    pub async fn refresh(client: &Client, refresh_token: &str) -> Result<Self, MsAuthRefreshError> {
        #[derive(Deserialize)]
        struct RefreshResponse {
            access_token: String,
            refresh_token: String,
            expires_in: i64,
        }

        let response = client
            .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
            //.post("https://login.live.com/oauth20_token.srf")
            .form(&[
                ("client_id", MS_KEY),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
                (
                    "redirect_uri",
                    "https://login.microsoftonline.com/common/oauth2/nativeclient",
                ),
            ])
            .send()
            .await
            .map_err(RequestError::from_error)?
            .error_for_status()
            .map_err(RequestError::from_error)?
            .json::<RefreshResponse>()
            .await
            .map_err(RequestError::from_error)?;

        Ok(Self {
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            expires_at: Utc::now() + chrono::Duration::seconds(response.expires_in),
        })
    }
}

#[derive(Error, Debug, Clone)]
pub enum MsAuthRefreshError {
    #[error("request error: {0}")]
    Request(#[from] RequestError),
}

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
                .await
                .map_err(RequestError::from_error)?
                .error_for_status()
                .map_err(RequestError::from_error)?
                .json::<XblToken>()
                .await
                .map_err(RequestError::from_error)?;

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
            .await
            .map_err(RequestError::from_error)?;

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

                let response = response
                    .json::<XstsToken>()
                    .await
                    .map_err(RequestError::from_error)?;

                Ok(Self {
                    xsts_token: response.token,
                    userhash: response
                        .display_claims
                        .xui
                        .get(0)
                        .ok_or_else(|| {
                            XboxAuthError::Request(RequestError {
                                context: RequestContext::none(),
                                error: RequestErrorDetails::MalformedResponse,
                            })
                        })?
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

                let xsts_err = response
                    .json::<XstsError>()
                    .await
                    .map_err(RequestError::from_error)?;

                Err(XboxAuthError::Xbox(XboxError::from_xerr(xsts_err.xerr)))
            }
            _ => Err(XboxAuthError::Request(RequestError::from_status(&response))),
        }
    }
}

#[derive(Error, Debug, Clone, Copy, Type, Serialize)]
pub enum XboxError {
    #[error("no xbox account is associated with this microsoft account")]
    NoAccount,

    #[error("xbox live is not availible in this country")]
    XboxServicesBanned,

    #[error("this xbox account must be verified as an adult")]
    AdultVerificationRequired,

    #[error("this xbox account is a child account, and must have a family associated")]
    ChildAccount,

    #[error("xbox returned an unknown error code: {0}")]
    Unknown(u64),
}

#[derive(Error, Debug, Clone)]
pub enum XboxAuthError {
    #[error("xbox error: {0}")]
    Xbox(#[from] XboxError),

    #[error("request error: {0}")]
    Request(#[from] RequestError),
}

impl XboxError {
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

#[derive(Debug, Clone)]
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
            .await
            .map_err(RequestError::from_error)?
            .error_for_status()
            .map_err(RequestError::from_error)?
            .json::<McAuthResponse>()
            .await
            .map_err(RequestError::from_error)?;

        Ok(Self {
            access_token: response.access_token,
            expires_at: Utc::now() + chrono::Duration::seconds(response.expires_in),
        })
    }

    pub async fn get_entitlement(
        &self,
        client: &Client,
    ) -> Result<McEntitlement, McEntitlementCheckError> {
        #[derive(Deserialize)]
        struct EntitlementResponse {
            signature: String,
        }

        let response = client
            .get("https://api.minecraftservices.com/entitlements/mcstore")
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(RequestError::from_error)?
            .error_for_status()
            .map_err(RequestError::from_error)?
            .json::<EntitlementResponse>()
            .await
            .map_err(RequestError::from_error)?;

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
            &McEntitlement::mojang_jwt_key(),
            &Validation::new(Algorithm::RS256),
        );

        let entitlements = match entitlements {
            Ok(jwt) => jwt.claims,
            Err(e) => {
                let error = match e.kind() {
                    ErrorKind::InvalidSignature | ErrorKind::ImmatureSignature => {
                        McEntitlementCheckError::Entitlement(McEntitlementError::InvalidSignature)
                    }
                    ErrorKind::InvalidToken | ErrorKind::MissingRequiredClaim(_) => {
                        McEntitlementCheckError::Entitlement(McEntitlementError::InvalidData)
                    }
                    ErrorKind::MissingAlgorithm => {
                        McEntitlementCheckError::Entitlement(McEntitlementError::Outdated)
                    }
                    _ => McEntitlementCheckError::Entitlement(McEntitlementError::Jwt(e)),
                };

                return Err(error);
            }
        };

        println!("Entitlements: {entitlements:#?}");

        // likely will not work for gamepass
        let owns_game = entitlements
            .entitlements
            .iter()
            .any(|SignedEntitlement { name }| name == "product_minecraft");

        match owns_game {
            true => Ok(McEntitlement::Owned),
            false => Err(McEntitlementCheckError::Entitlement(
                McEntitlementError::NoEntitlement,
            ))?,
        }
    }

    pub async fn get_profile(&self, client: &Client) -> Result<McProfile, McProfileRequestError> {
        let response = client
            .get("https://api.minecraftservices.com/minecraft/profile")
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(RequestError::from_error)?;

        match response.status() {
            StatusCode::NOT_FOUND => {
                Err(McProfileRequestError::Profile(McProfileError::NoProfile))?
            }
            StatusCode::OK => {
                #[derive(Debug, Deserialize)]
                struct McProfileResponse {
                    id: String,
                    name: String,
                }

                let response = response
                    .json::<McProfileResponse>()
                    .await
                    .map_err(RequestError::from_error)?;

                Ok(McProfile {
                    uuid: response.id,
                    username: response.name,
                })
            }
            _ => Err(McProfileRequestError::Request(RequestError::from_status(
                &response,
            )))?,
        }
    }

    pub async fn populate(&self, client: &Client) -> Result<McAccount, McAccountPopulateError> {
        Ok(McAccount {
            auth: self.clone(),
            entitlement: self.get_entitlement(client).await?,
            profile: self.get_profile(client).await?,
        })
    }
}

#[derive(Error, Debug, Clone)]
pub enum McAuthError {
    #[error("request error: {0}")]
    Request(#[from] RequestError),

    #[error("xbox auth error: {0}")]
    Xbox(#[from] XboxAuthError),
}

#[derive(Debug, Clone)]
pub enum McEntitlement {
    Owned,
    XboxGamepass,
}

impl McEntitlement {
    fn mojang_jwt_key() -> DecodingKey {
        // The test at the bottom of this file makes sure this unwrap is fine.
        DecodingKey::from_rsa_pem(include_bytes!("mojang_jwt_signature.pem")).unwrap()
    }
}

#[derive(Error, Debug, Clone)]
pub enum McEntitlementError {
    #[error("response data was not valid")]
    InvalidData,

    #[error("response data could not be verified")]
    InvalidSignature,

    #[error("GDLauncher account verifcation checks are outdated")]
    Outdated,

    #[error("JWT error: {0}")]
    Jwt(jsonwebtoken::errors::Error),

    #[error("no game entitlement")]
    NoEntitlement,
}

#[derive(Error, Debug, Clone)]
pub enum McEntitlementCheckError {
    #[error("request error: {0}")]
    Request(#[from] RequestError),

    #[error("{0}")]
    Entitlement(#[from] McEntitlementError),
}

#[derive(Debug, Clone)]
pub struct McProfile {
    pub uuid: String,
    pub username: String,
}

#[derive(Error, Debug, Clone)]
pub enum McProfileError {
    #[error("no profile found")]
    NoProfile,
}

#[derive(Error, Debug, Clone)]
pub enum McProfileRequestError {
    #[error("reqwest error: {0}")]
    Request(#[from] RequestError),

    #[error("profile error: {0}")]
    Profile(#[from] McProfileError),
}

#[derive(Debug, Clone)]
pub struct McAccount {
    pub auth: McAuth,
    pub entitlement: McEntitlement,
    pub profile: McProfile,
}

#[derive(Error, Debug, Clone)]
pub enum McAccountPopulateError {
    #[error("entitlement check error: {0}")]
    Entitlement(#[from] McEntitlementCheckError),

    #[error("game profile error: {0}")]
    Profile(#[from] McProfileRequestError),
}

#[derive(Debug, Clone)]
pub struct FullAccount {
    pub ms: MsAuth,
    pub mc: McAccount,
}

#[cfg(test)]
mod test {
    use super::McEntitlement;

    /// Make sure it's possible to get a JWT decoding key from
    /// the saved public key.
    #[test]
    fn valid_mojang_account_sig() {
        // unwrap performed inside
        let _ = McEntitlement::mojang_jwt_key();
    }
}
