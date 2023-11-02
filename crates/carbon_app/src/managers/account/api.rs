use std::time::Duration;

use anyhow::{anyhow, bail};
use chrono::{DateTime, Utc};
use jsonwebtoken::{errors::ErrorKind, Algorithm, DecodingKey, Validation};
use reqwest::StatusCode;
use reqwest_middleware::ClientWithMiddleware;
use rspc::Type;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tracing::info;

use crate::error::request::{
    censor_error, MalformedResponseDetails, RequestContext, RequestError,
    RequestErrorDetails,
};

#[derive(Debug, Clone)]
pub struct DeviceCode {
    pub user_code: String,
    device_code: String,
    pub verification_uri: String,
    pub polling_interval: Duration,
    pub expires_at: DateTime<Utc>,
}

impl DeviceCode {
    pub async fn request_code(
        client: &ClientWithMiddleware,
    ) -> anyhow::Result<Self> {
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
                ("client_id", env!("MS_AUTH_CLIENT_ID")),
                (
                    "scope",
                    "XboxLive.signin XboxLive.offline_access profile openid email",
                ),
            ])
            .header("content-length", "0")
            .send()
            .await
            .map_err(censor_error)?
            .error_for_status()
            .map_err(reqwest::Error::without_url)?
            .json::<DeviceCodeResponse>()
            .await
            .map_err(reqwest::Error::without_url)?;

        Ok(Self {
            user_code: response.user_code,
            device_code: response.device_code,
            verification_uri: response.verification_uri,
            // polling_interval: Duration::from_secs(response.interval.into()),
            polling_interval: Duration::from_secs(1),
            expires_at: Utc::now()
                + chrono::Duration::seconds(response.expires_in),
        })
    }

    pub async fn poll_ms_auth(
        &self,
        client: &ClientWithMiddleware,
    ) -> anyhow::Result<Result<MsAuth, DeviceCodeExpiredError>> {
        loop {
            tokio::time::sleep(self.polling_interval).await;

            let response = client
                .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
                .form(&[
                    ("client_id", env!("MS_AUTH_CLIENT_ID")),
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

                    let error = response
                        .json::<BadRequestError>()
                        .await
                        .map_err(RequestError::from_error)?
                        .error;

                    match &error as &str {
                        "authorization_pending" => continue,
                        "expired_token" => {
                            return Ok(Err(DeviceCodeExpiredError))
                        }
                        _ => bail!(RequestError {
                            context: RequestContext::none(),
                            error: RequestErrorDetails::UnexpectedStatus {
                                status: StatusCode::BAD_REQUEST,
                                details: Some(error),
                            },
                        }),
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

                    break Ok(Ok(MsAuth {
                        access_token: response.access_token,
                        //id_token: response.id_token,
                        refresh_token: response.refresh_token,
                        expires_at: Utc::now()
                            + chrono::Duration::seconds(response.expires_in),
                    }));
                }
                _ => bail!(RequestError::from_status(&response,)),
            }
        }
    }
}

#[derive(Error, Debug)]
#[error("pending device code expired")]
pub struct DeviceCodeExpiredError;

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
    pub async fn refresh(
        client: &ClientWithMiddleware,
        refresh_token: &str,
    ) -> anyhow::Result<Self> {
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
                ("client_id", env!("MS_AUTH_CLIENT_ID")),
                ("refresh_token", refresh_token),
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

        Ok(Self {
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            expires_at: Utc::now()
                + chrono::Duration::seconds(response.expires_in),
        })
    }
}

pub struct XboxAuth {
    xsts_token: String,
    userhash: String,
}

impl XboxAuth {
    /// Obtain an Xbox account from a MS account (without refreshing it)
    pub async fn from_ms(
        ms_auth: &MsAuth,
        client: &ClientWithMiddleware,
    ) -> anyhow::Result<Result<Self, XboxError>> {
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

                Ok(Ok(Self {
                    xsts_token: response.token,
                    userhash: response
                        .display_claims
                        .xui
                        .get(0)
                        .ok_or_else(|| {
                            anyhow!(RequestError {
                                context: RequestContext::none(),
                                error: RequestErrorDetails::MalformedResponse {
                                    details: MalformedResponseDetails::UnknownDecodeError
                                },
                            })
                        })?
                        .uhs
                        .clone(),
                }))
            }
            StatusCode::UNAUTHORIZED => {
                #[derive(Deserialize)]
                struct XstsError {
                    #[serde(rename = "XErr")]
                    xerr: u32,
                }

                let xsts_err = response.json::<XstsError>().await?;

                Ok(Err(XboxError::from_xerr(xsts_err.xerr)))
            }
            _ => Err(anyhow!(RequestError::from_status(&response))),
        }
    }
}

#[derive(Error, Debug, Clone, Copy, Type, Serialize)]
#[serde(rename_all = "camelCase")]
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
    Unknown(u32),
}

impl XboxError {
    // error code from an XErr code returned by the XSTS auth endpoint
    fn from_xerr(xerr: u32) -> Self {
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
    pub async fn auth_ms(
        xbox_auth: XboxAuth,
        client: &ClientWithMiddleware,
    ) -> anyhow::Result<Self> {
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
            expires_at: Utc::now()
                + chrono::Duration::seconds(response.expires_in),
        })
    }

    pub async fn get_entitlement(
        &self,
        client: &ClientWithMiddleware,
    ) -> anyhow::Result<Result<McEntitlement, McEntitlementMissingError>> {
        #[derive(Deserialize)]
        struct EntitlementResponse {
            signature: String,
        }

        let response = client
            .get("https://api.minecraftservices.com/entitlements/mcstore")
            .bearer_auth(&self.access_token)
            .send()
            .await?
            .error_for_status()?
            .json::<EntitlementResponse>()
            .await?;

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
                    ErrorKind::InvalidSignature
                    | ErrorKind::ImmatureSignature => {
                        McEntitlementError::InvalidSignature
                    }
                    ErrorKind::InvalidToken
                    | ErrorKind::MissingRequiredClaim(_) => {
                        McEntitlementError::InvalidData
                    }
                    ErrorKind::MissingAlgorithm => McEntitlementError::Outdated,
                    _ => McEntitlementError::Jwt(e),
                };

                bail!(error);
            }
        };

        info!("Entitlements: {entitlements:#?}");

        // likely will not work for gamepass
        let owns_game = entitlements
            .entitlements
            .iter()
            .any(|SignedEntitlement { name }| name == "product_minecraft");

        match owns_game {
            true => Ok(Ok(McEntitlement::Owned)),
            false => Ok(Err(McEntitlementMissingError)),
        }
    }
}

pub async fn get_profile(
    client: &ClientWithMiddleware,
    access_token: &str,
) -> anyhow::Result<Result<McProfile, GetProfileError>> {
    let response = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(access_token)
        .send()
        .await?;

    match response.status() {
        StatusCode::UNAUTHORIZED => Ok(Err(GetProfileError::AuthTokenInvalid)),
        StatusCode::NOT_FOUND => Ok(Err(GetProfileError::GameProfileMissing)),
        StatusCode::OK => {
            #[derive(Debug, Deserialize)]
            struct McProfileResponse {
                id: String,
                name: String,
                skins: Vec<Skin>,
            }

            #[derive(Debug, Deserialize)]
            struct Skin {
                id: String,
                state: String, // unknown possible states,
                url: String,
            }

            let response = response
                .json::<McProfileResponse>()
                .await
                .map_err(RequestError::from_error)?;

            let skin = response
                .skins
                .into_iter()
                .find(|skin| skin.state == "ACTIVE")
                .map(|skin| McSkin {
                    id: skin.id,
                    url: skin.url,
                });

            Ok(Ok(McProfile {
                uuid: response.id,
                username: response.name,
                skin,
            }))
        }
        _ => bail!(RequestError::from_status(&response)),
    }
}

#[derive(Debug, Clone)]
pub enum McEntitlement {
    Owned,
    XboxGamepass,
}

impl McEntitlement {
    fn mojang_jwt_key() -> DecodingKey {
        // The test at the bottom of this file makes sure this unwrap is fine.
        DecodingKey::from_rsa_pem(include_bytes!("mojang_jwt_signature.pem"))
            .unwrap()
    }
}

#[derive(Error, Debug)]
pub enum GetProfileError {
    #[error("missing game profile")]
    GameProfileMissing,
    #[error("authentication token invalid")]
    AuthTokenInvalid,
}

#[derive(Error, Debug)]
#[error("no game entitlement")]
pub struct McEntitlementMissingError;

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
    pub skin: Option<McSkin>,
}

#[derive(Debug, Clone)]
pub struct McSkin {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct McAccount {
    pub auth: McAuth,
    pub entitlement: McEntitlement,
    pub profile: McProfile,
}

#[derive(Debug, Clone)]
pub struct FullAccount {
    pub ms: MsAuth,
    pub mc: McAccount,
}

#[derive(Debug, Clone)]
pub enum AccessTokenStatus {
    Ok,
    Invalid,
    XboxMultiplayerDisabled,
    BannedFromMultiplayer,
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
