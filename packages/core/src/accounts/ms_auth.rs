use std::sync::Arc;

use anyhow::{bail, Result};
use chrono::{Duration, Local, NaiveDateTime};
use jsonwebtoken::{TokenData, DecodingKey};
use lazy_static::lazy_static;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::Mutex;

pub const MS_KEY: &str = "221e73fa-365e-4263-9e06-7a0a1f277960";
pub const AZ_OPENID_URL: &str =
    "https://login.microsoftonline.com/common/.well-known/openid-configuration";

#[derive(Deserialize)]
pub struct OpenIdResponse {
    pub token_endpoint: String,
    pub device_authorization_endpoint: String,
    pub jwks_uri: String,
}

impl OpenIdResponse {
    pub async fn get() -> Result<Self> {
        let client = Client::new();
        let response = client.get(AZ_OPENID_URL).send().await?;

        if response.status() != StatusCode::OK {
            bail!("Failed to get OpenID response");
        }

        let response = response.json::<OpenIdResponse>().await?;

        Ok(response)
    }

    pub async fn get_public_keys(&self) -> Result<Vec<Jwk>> {
        let client = Client::new();
        let response = client.get(&self.jwks_uri).send().await?;

        if response.status() != StatusCode::OK {
            bail!("Failed to get public keys");
        }

        let response = response.json::<JwkSet>().await?;

        Ok(response.keys)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct JwkSet {
    keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Jwk {
    pub kid: String,
    n: String,
    e: String,
}

impl Jwk {
    fn modulus(&self) -> &str {
        &self.n
    }

    fn exponent(&self) -> &str {
        &self.e
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AzureData {
    pub token_endpoint: String,
    pub device_authorization_endpoint: String,
    pub jwks_uri: String,
    pub public_keys: Option<Vec<Jwk>>,
    pub last_key_refresh: Option<NaiveDateTime>,
    pub exp_hours: i64,
    retry_count: i32,
}

impl AzureData {
    pub async fn new() -> Result<Self> {
        let response = OpenIdResponse::get().await?;
        let public_keys = response.get_public_keys().await?;

        Ok(AzureData {
            token_endpoint: response.token_endpoint,
            device_authorization_endpoint: response.device_authorization_endpoint,
            jwks_uri: response.jwks_uri,
            public_keys: Some(public_keys),
            last_key_refresh: None,
            exp_hours: 24,
            retry_count: 0,
        })
    }

    async fn refresh_pub_keys(&mut self) -> Result<()> {
        let response = OpenIdResponse::get().await?;
        let public_keys = response.get_public_keys().await?;
        self.last_key_refresh = Some(Local::now().naive_local());
        self.public_keys = Some(public_keys);
        Ok(())
    }

    fn are_keys_valid(&self) -> bool {
        match self.last_key_refresh {
            None => false,
            Some(lr) => (Local::now().naive_local() - lr) <= Duration::hours(self.exp_hours),
        }
    }

    #[async_recursion::async_recursion]
    pub async fn validate_token(&mut self, token: &str) -> Result<TokenData<AzureJwtClaims>> {
        let mut validator = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);

        // exp, nbf, iat is set to validate as default
        validator.leeway = 60;
        validator.set_audience(&[&MS_KEY]);
        let decoded: TokenData<AzureJwtClaims> =
            self.validate_token_authenticity(token, &validator).await.unwrap();

        Ok(decoded)
    }

    async fn validate_token_authenticity(
        &mut self,
        token: &str,
        validator: &jsonwebtoken::Validation,
    ) -> Result<TokenData<AzureJwtClaims>> {
        // If public keys are expired, refresh them
        if !self.are_keys_valid() {
            self.refresh_pub_keys().await.unwrap();
        }

        let decoded = jsonwebtoken::decode_header(token).unwrap();
        let key = match &self.public_keys {
            None => bail!("No public keys found"),
            Some(keys) => match &decoded.kid {
                None => bail!("No kid found in token"),
                Some(kid) => keys.iter().find(|k| k.kid == *kid),
            },
        };

        let auth_key = match key {
            None => {
                // the first time this happens let's go and refresh the keys and try once more.
                // It could be that our keys are out of date. Limit to once in an hour.
                if self.retry_count == 0 {
                    self.refresh_pub_keys().await?;
                    self.retry_count += 1;
                    return self.validate_token(token).await;
                } else {
                    self.retry_count = 0;
                    bail!("Invalid token. Could not verify authenticity");
                }
            }
            Some(key) => {
                self.retry_count = 0;
                key
            }
        };

        let key = DecodingKey::from_rsa_components(auth_key.modulus(), auth_key.exponent());
        let valid: TokenData<AzureJwtClaims> = jsonwebtoken::decode(token, &key, &validator)?;

        Ok(valid)
    }
}

lazy_static! {
    pub static ref AZURE_DATA: Arc<Mutex<Option<AzureData>>> = Arc::new(Mutex::new(None));
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct McProfile {
    pub id: String,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct McAuth {
    pub access_token: String,
    pub expires_in: i64,
    //#[serde(skip)]
    //pub expires_after: i64,
}

impl McAuth {
    pub async fn get_mc_profile(&self, client: &Client) -> anyhow::Result<McProfile> {
        let pr_resp = client
            .get("https://api.minecraftservices.com/minecraft/profile")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await?
            .json()
            .await?;

        Ok(pr_resp)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct DisplayClaims {
    xui: Vec<Xui>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Xui {
    uhs: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XstsAuth {
    token: String,
    display_claims: DisplayClaims,
}

impl XstsAuth {
    async fn auth_mc(&self, client: &Client) -> anyhow::Result<McAuth> {
        let json = json!({
            "identityToken": format!("XBL3.0 x={};{}", self.display_claims.xui[0].uhs, self.token)
        });

        let mc_resp = client
            .post("https://api.minecraftservices.com/authentication/login_with_xbox")
            .header("Accept", "application/json")
            .json(&json)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        //mc_auth.expires_after = mc_auth.expires_in + chrono::Utc::now().timestamp();
        Ok(mc_resp)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XblAuth {
    token: String,
}

impl XblAuth {
    async fn auth_xsts(&self, client: &Client) -> anyhow::Result<XstsAuth> {
        let json = json!({
            "Properties": {
                "SandboxId":  "RETAIL",
                "UserTokens": [self.token]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType":    "JWT",
        });

        let xsts_resp = client
            .post("https://xsts.auth.xboxlive.com/xsts/authorize")
            .header("Content-Type", "application/json")
            .json(&json)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(xsts_resp)
    }
}

pub enum AuthError {
    Error,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MsAuthRefresh {
    expires_in: i64,
    access_token: String,
    refresh_token: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MsAuth {
    expires_in: i64,
    pub access_token: String,
    pub id_token: String,
    refresh_token: String,
    #[serde(skip)]
    expires_after: i64,
}

pub struct AzureJwtHeader {
    /// Indicates that the token is a JWT.
    pub typ: String,
    /// Indicates the algorithm that was used to sign the token. Example: "RS256"
    pub alg: String,
    /// Thumbprint for the public key used to sign this token. Emitted in both
    /// v1.0 and v2.0 id_tokens
    pub kid: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AzureJwtClaims {
    /// dentifies the intended recipient of the token. In id_tokens, the audience
    /// is your app's Application ID, assigned to your app in the Azure portal.
    /// Your app should validate this value, and reject the token if the value
    /// does not match.
    pub aud: String,

    /// The application ID of the client using the token. The application can
    /// act as itself or on behalf of a user. The application ID typically
    /// represents an application object, but it can also represent a service
    /// principal object in Azure AD.
    pub azp: Option<String>,

    /// Indicates how the client was authenticated. For a public client, the
    /// value is "0". If client ID and client secret are used, the value is "1".
    /// If a client certificate was used for authentication, the value is "2".
    pub azpacr: Option<String>,

    /// Identifies the security token service (STS) that constructs and returns
    /// the token, and the Azure AD tenant in which the user was authenticated.
    /// If the token was issued by the v2.0 endpoint, the URI will end in /v2.0.
    /// The GUID that indicates that the user is a consumer user from a Microsoft
    /// account is 9188040d-6c67-4c5b-b112-36a304b66dad.
    ///
    /// Your app should use the GUID portion of the claim to restrict the set of
    /// tenants that can sign in to the app, if applicable.
    pub iss: String,

    /// Unix timestamp. "Issued At" indicates when the authentication for this
    /// token occurred.
    pub iat: u64,

    /// Records the identity provider that authenticated the subject of the token.
    /// This value is identical to the value of the Issuer claim unless the user
    /// account not in the same tenant as the issuer - guests, for instance. If
    /// the claim isn't present, it means that the value of iss can be used
    /// instead. For personal accounts being used in an organizational context
    /// (for instance, a personal account invited to an Azure AD tenant), the idp
    /// claim may be 'live.com' or an STS URI containing the Microsoft account
    /// tenant 9188040d-6c67-4c5b-b112-36a304b66dad
    pub idp: Option<String>,

    /// Unix timestamp. The "nbf" (not before) claim identifies the time before
    /// which the JWT MUST NOT be accepted for processing.
    pub nbf: u64,

    /// Unix timestamp. he "exp" (expiration time) claim identifies the
    /// expiration time on or after which the JWT MUST NOT be accepted for
    /// processing. It's important to note that a resource may reject the token
    /// before this time as well - if, for example, a change in authentication
    /// is required or a token revocation has been detected.
    pub exp: u64,

    /// The code hash is included in ID tokens only when the ID token is issued
    /// with an OAuth 2.0 authorization code. It can be used to validate the
    /// authenticity of an authorization code. For details about performing this
    /// validation, see the OpenID Connect specification.
    pub c_hash: Option<String>,

    /// The access token hash is included in ID tokens only when the ID token is
    /// issued with an OAuth 2.0 access token. It can be used to validate the
    /// authenticity of an access token. For details about performing this
    /// validation, see the OpenID Connect specification.
    pub at_hash: Option<String>,

    /// The email claim is present by default for guest accounts that have an
    /// email address. Your app can request the email claim for managed users
    /// (those from the same tenant as the resource) using the email optional
    /// claim. On the v2.0 endpoint, your app can also request the email OpenID
    /// Connect scope - you don't need to request both the optional claim and
    /// the scope to get the claim. The email claim only supports addressable
    /// mail from the user's profile information.
    pub preferred_username: Option<String>,

    /// The name claim provides a human-readable value that identifies the
    /// subject of the token. The value isn't guaranteed to be unique, it is
    /// mutable, and it's designed to be used only for display purposes. The
    /// profile scope is required to receive this claim.
    pub name: Option<String>,

    /// The nonce matches the parameter included in the original /authorize
    /// request to the IDP. If it does not match, your application should reject
    /// the token.
    pub nonce: Option<String>,

    /// Guid. The immutable identifier for an object in the Microsoft identity system,
    /// in this case, a user account. This ID uniquely identifies the user
    /// across applications - two different applications signing in the same
    /// user will receive the same value in the oid claim. The Microsoft Graph
    /// will return this ID as the id property for a given user account. Because
    /// the oid allows multiple apps to correlate users, the profile scope is
    /// required to receive this claim. Note that if a single user exists in
    /// multiple tenants, the user will contain a different object ID in each
    /// tenant - they're considered different accounts, even though the user
    /// logs into each account with the same credentials.
    pub oid: String,

    /// The set of roles that were assigned to the user who is logging in.
    pub roles: Option<Vec<String>>,

    /// The set of scopes exposed by your application for which the client
    /// application has requested (and received) consent. Your app should verify
    /// that these scopes are valid ones exposed by your app, and make authorization
    /// decisions based on the value of these scopes. Only included for user tokens.
    pub scp: Option<String>,

    /// The principal about which the token asserts information, such as the
    /// user of an app. This value is immutable and cannot be reassigned or
    /// reused. The subject is a pairwise identifier - it is unique to a
    /// particular application ID. If a single user signs into two different
    /// apps using two different client IDs, those apps will receive two
    /// different values for the subject claim. This may or may not be wanted
    /// depending on your architecture and privacy requirements.
    pub sub: String,

    /// A GUID that represents the Azure AD tenant that the user is from.
    /// For work and school accounts, the GUID is the immutable tenant ID of
    /// the organization that the user belongs to. For personal accounts,
    /// the value is 9188040d-6c67-4c5b-b112-36a304b66dad. The profile scope is
    /// required to receive this claim.
    pub tid: String,

    /// Provides a human readable value that identifies the subject of the
    /// token. This value isn't guaranteed to be unique within a tenant and
    /// should be used only for display purposes. Only issued in v1.0 id_tokens.
    pub unique_name: Option<String>,

    /// Indicates the version of the id_token. Either 1.0 or 2.0.
    pub ver: String,
}

impl MsAuth {
    /// Checks if the access token is still valid and refreshes it if it isn't.
    pub async fn refresh(&mut self, cid: &str, client: &Client) -> anyhow::Result<bool> {
        if self.expires_after <= chrono::Utc::now().timestamp() {
            let resp: MsAuthRefresh = client
                .post("https://login.live.com/oauth20_token.srf")
                .form(&[
                    ("client_id", cid),
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
                .json()
                .await?;

            self.access_token = resp.access_token;
            self.refresh_token = resp.refresh_token;
            self.expires_after = resp.expires_in + chrono::Utc::now().timestamp();
            return Ok(true);
        }
        Ok(false)
    }

    async fn auth_xbl(&self, client: &Client) -> anyhow::Result<XblAuth> {
        let json = json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName":   "user.auth.xboxlive.com",
                "RpsTicket":  &(String::from("d=") + &self.access_token) as &str,
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType":    "JWT",
        });

        let xbl_resp = client
            .post("https://user.auth.xboxlive.com/user/authenticate")
            .header("Accept", "application/json")
            .json(&json)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(xbl_resp)
    }

    pub async fn finalize_auth(&self, client: &Client) -> anyhow::Result<McAuth> {
        let xbl_auth = self.auth_xbl(client).await?;
        let xsts_auth = xbl_auth.auth_xsts(client).await?;
        let mc_auth = xsts_auth.auth_mc(client).await?;

        Ok(mc_auth)
    }

    pub async fn get_id_token_claims(&self) -> anyhow::Result<AzureJwtClaims> {
        let token = jsonwebtoken::decode::<AzureJwtClaims>(
            &self.id_token,
            &jsonwebtoken::DecodingKey::from_secret("".as_ref()),
            &jsonwebtoken::Validation::default(),
        )?;
        Ok(token.claims)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Auth {
    pub name: String,
    pub uuid: String,
    pub token: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceCodeInner {
    pub user_code: String,
    device_code: String,
    pub verification_uri: String,
    expires_in: i64,
    interval: u64,
    pub message: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MsAuthError {
    error: String,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct DeviceCode {
    pub inner: Option<DeviceCodeInner>,
}

impl DeviceCode {
    /// Step 1: Get device code
    ///
    /// Step 2: Poll for a MsAuth
    ///
    /// Step 3: XBL Auth
    ///
    /// Step 4: XSTS Auth
    ///
    /// Step 5: MC Auth
    ///
    /// Step 6: MC Profile
    pub async fn new(client: &Client) -> anyhow::Result<Self> {
        let device_code: DeviceCode;
        let device_code_inner: Option<DeviceCodeInner>;
        let device_resp = client
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
            .json()
            .await?;

        device_code_inner = Some(device_resp);
        device_code = DeviceCode {
            inner: device_code_inner,
        };
        Ok(device_code)
    }

    pub async fn poll_device_code_auth(&self, client: &Client) -> anyhow::Result<MsAuth> {
        match &self.inner {
            Some(inner) => loop {
                std::thread::sleep(std::time::Duration::from_secs(inner.interval));

                let code_resp = client
                    .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
                    .form(&[
                        ("client_id", MS_KEY),
                        (
                            "scope",
                            "XboxLive.signin XboxLive.offline_access profile openid email",
                        ),
                        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                        ("device_code", &inner.device_code),
                    ])
                    .send()
                    .await?;

                match code_resp.status() {
                    StatusCode::BAD_REQUEST => {
                        let ms_auth: MsAuthError = code_resp.json().await?;
                        match &ms_auth.error as &str {
                            "authorization_pending" => continue,
                            _ => {
                                bail!("{}", ms_auth.error)
                            }
                        }
                    }
                    StatusCode::OK => {
                        let mut ms_auth: MsAuth = code_resp.json().await?;
                        ms_auth.expires_after = ms_auth.expires_in + chrono::Utc::now().timestamp();
                        return Ok(ms_auth);
                    }
                    _ => {
                        return Err(anyhow::Error::msg(format!(
                            "unexpected response code: {}",
                            code_resp.status()
                        )))
                    }
                }
            },
            None => bail!("device code not initialized"),
        }
    }
}
