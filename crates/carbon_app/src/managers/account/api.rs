use std::{
    future::Future,
    time::{Duration, Instant},
};

use anyhow::{Context, anyhow, bail};
use axum::routing::trace;
use chrono::{DateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, errors::ErrorKind};
use reqwest::StatusCode;
use reqwest_middleware::ClientWithMiddleware;
use serde::{Deserialize, Serialize};
use serde_json::json;
use specta::Type;
use thiserror::Error;
use tracing::{error, info, trace};

use crate::error::request::{
    MalformedResponseDetails, RequestContext, RequestError, RequestErrorDetails, censor_error,
};

use super::endpoints;

#[derive(Debug, Clone)]
pub struct DeviceCode {
    pub user_code: String,
    device_code: String,
    pub verification_uri: String,
    pub polling_interval: Duration,
    pub expires_at: DateTime<Utc>,
    deadline: Instant,
}

#[derive(Deserialize)]
struct DeviceCodeResponse {
    user_code: String,
    device_code: String,
    verification_uri: String,
    expires_in: i64,
    #[serde(default = "default_poll_interval")]
    interval: u32,
    // message: String,
}

fn default_poll_interval() -> u32 {
    5
}

impl DeviceCode {
    pub async fn request_code(client: &ClientWithMiddleware) -> anyhow::Result<Self> {
        let response = client
            .get(endpoints::device_code_url())
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

        Self::from_response(response)
    }

    fn from_response(response: DeviceCodeResponse) -> anyhow::Result<Self> {
        let lifetime = Duration::from_secs(response.expires_in.max(0) as u64);
        Ok(Self {
            user_code: response.user_code,
            device_code: response.device_code,
            verification_uri: response.verification_uri,
            polling_interval: Duration::from_secs(u64::from(response.interval.max(1))),
            expires_at: expiry_after(response.expires_in.max(0))?,
            deadline: Instant::now()
                .checked_add(lifetime)
                .ok_or_else(|| anyhow!("Invalid device-code lifetime"))?,
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn poll_ms_auth(&self) -> anyhow::Result<Result<MsAuth, DeviceCodeExpiredError>> {
        // Polling owns retries; this bare client never retries token POSTs.
        let client = reqwest::Client::new();
        let client = &client;
        self.poll_with(Instant::now, tokio::time::sleep, |remaining| async move {
            let request = async {
                let response = client
                    .post(endpoints::ms_token_url())
                    .form(&device_poll_form(
                        env!("MS_AUTH_CLIENT_ID"),
                        &self.device_code,
                    ))
                    .send()
                    .await
                    .map_err(reqwest::Error::without_url)?;
                let status = response.status();
                let body = response
                    .bytes()
                    .await
                    .map_err(reqwest::Error::without_url)?;
                Ok((status, body.to_vec()))
            };
            device_request_deadline(remaining, request).await
        })
        .await
    }

    async fn poll_with<N, S, SF, R, RF>(
        &self,
        now: N,
        sleep: S,
        mut request: R,
    ) -> anyhow::Result<Result<MsAuth, DeviceCodeExpiredError>>
    where
        N: Fn() -> Instant,
        S: Fn(Duration) -> SF,
        SF: Future<Output = ()>,
        R: FnMut(Duration) -> RF,
        RF: Future<Output = anyhow::Result<(StatusCode, Vec<u8>)>>,
    {
        let mut interval = self.polling_interval;
        loop {
            let remaining = self.deadline.saturating_duration_since(now());
            if remaining.is_zero() {
                return Ok(Err(DeviceCodeExpiredError));
            }
            sleep(interval.min(remaining)).await;
            let remaining = self.deadline.saturating_duration_since(now());
            if remaining.is_zero() {
                return Ok(Err(DeviceCodeExpiredError));
            }
            let response = request(remaining).await;
            if now() >= self.deadline {
                return Ok(Err(DeviceCodeExpiredError));
            }
            let (status, body) = match response {
                Err(e) if e.is::<DeviceCodeExpiredError>() => {
                    return Ok(Err(DeviceCodeExpiredError));
                }
                other => other?,
            };
            match decode_device_poll(status, &body)? {
                PollResponse::Pending => {}
                PollResponse::SlowDown => {
                    interval = interval.saturating_add(Duration::from_secs(5))
                }
                PollResponse::Expired => return Ok(Err(DeviceCodeExpiredError)),
                PollResponse::Complete(auth) => return Ok(Ok(auth)),
            }
        }
    }
}
async fn device_request_deadline<F>(
    remaining: Duration,
    request: F,
) -> anyhow::Result<(StatusCode, Vec<u8>)>
where
    F: Future<Output = anyhow::Result<(StatusCode, Vec<u8>)>>,
{
    match tokio::time::timeout(remaining, request).await {
        Ok(result) => result,
        Err(_) => Err(DeviceCodeExpiredError.into()),
    }
}
fn expiry_after(seconds: i64) -> anyhow::Result<DateTime<Utc>> {
    chrono::Duration::try_seconds(seconds)
        .and_then(|duration| Utc::now().checked_add_signed(duration))
        .ok_or_else(|| anyhow!("Invalid authentication lifetime"))
}
fn device_poll_form<'a>(client_id: &'a str, code: &'a str) -> [(&'static str, &'a str); 4] {
    [
        ("client_id", client_id),
        (
            "scope",
            "XboxLive.signin XboxLive.offline_access profile openid email",
        ),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("device_code", code),
    ]
}
enum PollResponse {
    Pending,
    SlowDown,
    Expired,
    Complete(MsAuth),
}
fn decode_device_poll(status: StatusCode, body: &[u8]) -> anyhow::Result<PollResponse> {
    if status == StatusCode::BAD_REQUEST {
        #[derive(Deserialize)]
        struct OAuthError {
            error: String,
        }
        let error: OAuthError = serde_json::from_slice(body)?;
        return match error.error.as_str() {
            "authorization_pending" => Ok(PollResponse::Pending),
            "slow_down" => Ok(PollResponse::SlowDown),
            "expired_token" => Ok(PollResponse::Expired),
            _ => bail!("Device authorization request failed"),
        };
    }
    if status != StatusCode::OK {
        bail!("Device authorization request returned {}", status);
    }
    #[derive(Deserialize)]
    struct Response {
        access_token: String,
        id_token: String,
        refresh_token: String,
        expires_in: i64,
    }
    let r: Response = serde_json::from_slice(body)?;
    Ok(PollResponse::Complete(MsAuth {
        access_token: r.access_token,
        id_token: r.id_token,
        refresh_token: r.refresh_token,
        expires_at: expiry_after(r.expires_in)?,
    }))
}

#[cfg(test)]
mod task19_poll_tests {
    use super::*;
    #[test]
    fn device_response_honors_interval() {
        let response: DeviceCodeResponse = serde_json::from_str(r#"{"user_code":"EXAMPLE","device_code":"dummy-code","verification_uri":"https://example.invalid","expires_in":60,"interval":5}"#).unwrap();
        let code = DeviceCode::from_response(response).unwrap();
        assert_eq!(code.polling_interval, Duration::from_secs(5));
    }
    #[test]
    fn slowdown_continues_valid_authorization() {
        assert!(
            decode_device_poll(StatusCode::BAD_REQUEST, br#"{"error":"slow_down"}"#).is_ok(),
            "slow_down aborted a still-valid enrollment"
        );
    }
    fn code(interval: u32, lifetime: i64) -> DeviceCode {
        DeviceCode::from_response(DeviceCodeResponse {
            user_code: "EXAMPLE".into(),
            device_code: "dummy-code".into(),
            verification_uri: "https://example.invalid".into(),
            expires_in: lifetime,
            interval,
        })
        .unwrap()
    }
    fn response(error: &str) -> (StatusCode, Vec<u8>) {
        (
            StatusCode::BAD_REQUEST,
            serde_json::to_vec(&json!({"error":error})).unwrap(),
        )
    }
    fn success() -> (StatusCode, Vec<u8>) {
        (StatusCode::OK,br#"{"access_token":"dummy-ms","id_token":"dummy-id","refresh_token":"dummy-refresh","expires_in":60}"#.to_vec())
    }
    #[tokio::test]
    async fn polling_waits_follow_interval_cumulative_slowdown_and_pending() {
        use std::cell::{Cell, RefCell};
        let code = code(5, 120);
        let now = Cell::new(Instant::now());
        let waits = RefCell::new(Vec::new());
        let mut responses = [
            response("authorization_pending"),
            response("slow_down"),
            response("slow_down"),
            response("authorization_pending"),
            success(),
        ]
        .into_iter();
        let auth = code
            .poll_with(
                || now.get(),
                |delay| {
                    waits.borrow_mut().push(delay.as_secs());
                    now.set(now.get() + delay);
                    std::future::ready(())
                },
                |_| std::future::ready(Ok(responses.next().unwrap())),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(*waits.borrow(), vec![5, 5, 10, 15, 15]);
        assert_eq!(auth.access_token, "dummy-ms");
        assert_eq!(auth.refresh_token, "dummy-refresh");
    }
    #[tokio::test]
    async fn expiry_before_after_wait_and_during_request_prevents_auth_completion() {
        use std::cell::Cell;
        for during_request in [false, true] {
            let mut code = code(1, 20);
            let initial = Instant::now();
            code.deadline = initial + Duration::from_secs(if during_request { 2 } else { 1 });
            let now = Cell::new(initial);
            let calls = Cell::new(0);
            let outcome = code
                .poll_with(
                    || now.get(),
                    |delay| {
                        now.set(now.get() + delay);
                        std::future::ready(())
                    },
                    |remaining| {
                        calls.set(calls.get() + 1);
                        assert_eq!(remaining, Duration::from_secs(1));
                        now.set(now.get() + remaining);
                        std::future::ready(Ok(success()))
                    },
                )
                .await
                .unwrap();
            assert!(outcome.is_err());
            assert_eq!(calls.get(), if during_request { 1 } else { 0 });
        }
        let expired = code(1, 0);
        assert!(
            expired
                .poll_with(
                    Instant::now,
                    |_| async { panic!("expired wait") },
                    |_| async { panic!("expired request") }
                )
                .await
                .unwrap()
                .is_err()
        );
    }
    #[tokio::test]
    async fn one_second_pending_expired_and_success_controls() {
        use std::cell::Cell;
        for terminal in [response("expired_token"), success()] {
            let code = code(1, 10);
            let now = Cell::new(Instant::now());
            let waits = Cell::new(0);
            let expected_success = terminal.0 == StatusCode::OK;
            let mut replies = [response("authorization_pending"), terminal].into_iter();
            let result = code
                .poll_with(
                    || now.get(),
                    |delay| {
                        assert_eq!(delay, Duration::from_secs(1));
                        waits.set(waits.get() + 1);
                        now.set(now.get() + delay);
                        std::future::ready(())
                    },
                    |_| std::future::ready(Ok(replies.next().unwrap())),
                )
                .await
                .unwrap();
            assert_eq!(result.is_ok(), expected_success);
            assert_eq!(waits.get(), 2);
        }
    }
    #[test]
    fn device_defaults_malformed_lifetimes_and_exact_request_forms() {
        let omitted:DeviceCodeResponse=serde_json::from_str(r#"{"user_code":"EXAMPLE","device_code":"dummy-code","verification_uri":"https://example.invalid","expires_in":60}"#).unwrap();
        assert_eq!(
            DeviceCode::from_response(omitted).unwrap().polling_interval,
            Duration::from_secs(5)
        );
        assert!(
            DeviceCode::from_response(DeviceCodeResponse {
                user_code: "EXAMPLE".into(),
                device_code: "dummy".into(),
                verification_uri: "https://example.invalid".into(),
                expires_in: i64::MAX,
                interval: 1
            })
            .is_err()
        );
        let request = reqwest::Client::new()
            .post("https://example.invalid/token")
            .form(&device_poll_form("dummy-client", "dummy-code"))
            .build()
            .unwrap();
        assert_eq!(
            std::str::from_utf8(request.body().unwrap().as_bytes().unwrap()).unwrap(),
            "client_id=dummy-client&scope=XboxLive.signin+XboxLive.offline_access+profile+openid+email&grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code&device_code=dummy-code"
        );
        let request = reqwest::Client::new()
            .post("https://example.invalid/token")
            .form(&refresh_form("dummy-client", "dummy-refresh"))
            .build()
            .unwrap();
        assert_eq!(
            std::str::from_utf8(request.body().unwrap().as_bytes().unwrap()).unwrap(),
            "client_id=dummy-client&refresh_token=dummy-refresh&grant_type=refresh_token&redirect_uri=https%3A%2F%2Flogin.microsoftonline.com%2Fcommon%2Foauth2%2Fnativeclient"
        );
    }

    #[tokio::test]
    async fn request_and_body_timeout_is_typed_expiration() {
        let error = device_request_deadline(Duration::from_millis(10), std::future::pending())
            .await
            .unwrap_err();
        assert!(error.is::<DeviceCodeExpiredError>());
    }
    #[tokio::test]
    async fn dropping_enrollment_cancels_poll_wait_and_injected_request() {
        use std::sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        };
        for in_request in [false, true] {
            let requests = Arc::new(AtomicUsize::new(0));
            let completed = Arc::new(AtomicUsize::new(0));
            let (entered, ready) = tokio::sync::oneshot::channel();
            let entered = Arc::new(Mutex::new(Some(entered)));
            let future = {
                let requests = requests.clone();
                let completed = completed.clone();
                async move {
                    let code = code(1, 60);
                    let _ = code
                        .poll_with(
                            Instant::now,
                            |_| {
                                if !in_request {
                                    entered.lock().unwrap().take().unwrap().send(()).unwrap();
                                }
                                async move {
                                    if !in_request {
                                        std::future::pending::<()>().await;
                                    }
                                }
                            },
                            |_| {
                                requests.fetch_add(1, Ordering::SeqCst);
                                entered.lock().unwrap().take().unwrap().send(()).unwrap();
                                std::future::pending()
                            },
                        )
                        .await;
                    completed.fetch_add(1, Ordering::SeqCst);
                }
            };
            let (enrollment, worker) =
                super::super::enroll::EnrollmentTask::running_for_test(future);
            ready.await.unwrap();
            drop(enrollment);
            assert!(
                tokio::time::timeout(Duration::from_millis(100), worker)
                    .await
                    .unwrap()
                    .unwrap()
                    .is_err()
            );
            assert_eq!(requests.load(Ordering::SeqCst), usize::from(in_request));
            assert_eq!(completed.load(Ordering::SeqCst), 0);
        }
    }
}

#[derive(Error, Debug)]
#[error("pending device code expired")]
pub struct DeviceCodeExpiredError;

#[derive(Debug, Clone)]
pub struct MsAuth {
    pub access_token: String,
    pub id_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

impl MsAuth {
    /// Refresh the auth token, returning a new token if the current one
    /// has expired.
    #[tracing::instrument(skip(client, refresh_token))]
    pub async fn refresh(
        client: &ClientWithMiddleware,
        refresh_token: &str,
    ) -> anyhow::Result<Self> {
        trace!("Refreshing auth token");

        let response = client
            .post(endpoints::ms_token_url())
            .form(&refresh_form(env!("MS_AUTH_CLIENT_ID"), refresh_token))
            .send()
            .await
            .with_context(|| format!("Failed to refresh auth token with refresh token"))?;
        let status = response.status();
        let body = response.bytes().await?;
        decode_refresh(status, &body)
    }
}

fn refresh_form<'a>(client_id: &'a str, token: &'a str) -> [(&'static str, &'a str); 4] {
    [
        ("client_id", client_id),
        ("refresh_token", token),
        ("grant_type", "refresh_token"),
        (
            "redirect_uri",
            "https://login.microsoftonline.com/common/oauth2/nativeclient",
        ),
    ]
}

#[derive(Debug, Error)]
#[error("Microsoft refresh grant was rejected; sign in again")]
pub struct RefreshRejected;
fn decode_refresh(status: StatusCode, body: &[u8]) -> anyhow::Result<MsAuth> {
    if status == StatusCode::BAD_REQUEST {
        #[derive(Deserialize)]
        struct OAuthError {
            error: String,
        }
        if serde_json::from_slice::<OAuthError>(body)
            .is_ok_and(|error| error.error == "invalid_grant")
        {
            return Err(RefreshRejected.into());
        }
    }
    if !status.is_success() {
        bail!("Microsoft refresh request returned {}", status);
    }
    #[derive(Deserialize)]
    struct Response {
        access_token: String,
        id_token: String,
        refresh_token: String,
        expires_in: i64,
    }
    let response: Response = serde_json::from_slice(body)?;
    Ok(MsAuth {
        access_token: response.access_token,
        id_token: response.id_token,
        refresh_token: response.refresh_token,
        expires_at: expiry_after(response.expires_in)?,
    })
}
#[cfg(test)]
mod task19_refresh_response_tests {
    use super::*;
    #[test]
    fn invalid_grant_is_a_permanent_refresh_rejection() {
        let error = decode_refresh(
            StatusCode::BAD_REQUEST,
            br#"{"error":"invalid_grant","error_description":"synthetic"}"#,
        )
        .unwrap_err();
        assert!(
            error.is::<RefreshRejected>(),
            "recognized invalid_grant remained an unclassified transient error"
        );
    }
    #[test]
    fn temporary_and_unrecognized_errors_never_clear_refresh_credentials() {
        for (status, body) in [
            (
                StatusCode::TOO_MANY_REQUESTS,
                br#"{"error":"invalid_grant"}"#.as_slice(),
            ),
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                br#"{"error":"invalid_grant"}"#.as_slice(),
            ),
            (StatusCode::BAD_REQUEST, b"malformed".as_slice()),
            (
                StatusCode::BAD_REQUEST,
                br#"{"error":"unrecognized"}"#.as_slice(),
            ),
            (
                StatusCode::UNAUTHORIZED,
                br#"{"error":"invalid_grant"}"#.as_slice(),
            ),
        ] {
            let error = decode_refresh(status, body).unwrap_err();
            assert!(!error.is::<RefreshRejected>());
        }
        assert_eq!(decode_refresh(StatusCode::OK,br#"{"access_token":"dummy-ms","id_token":"dummy-id","refresh_token":"dummy-refresh","expires_in":60}"#).unwrap().refresh_token,"dummy-refresh");
    }
}

pub struct XboxAuth {
    xsts_token: String,
    userhash: String,
}

impl XboxAuth {
    /// Obtain an Xbox account from a MS account (without refreshing it)
    #[tracing::instrument(skip(ms_auth, client))]
    pub async fn from_ms(
        ms_auth: &MsAuth,
        client: &ClientWithMiddleware,
    ) -> anyhow::Result<Result<Self, XboxError>> {
        trace!("Authenticating Xbox account");

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
                .post(endpoints::xbl_authenticate_url())
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .body(reqwest::Body::from(serde_json::to_string(&json)?))
                .send()
                .await
                .with_context(|| format!("Failed to authenticate Xbox account"))?
                .error_for_status()?
                .json::<XblToken>()
                .await
                .with_context(|| format!("Failed to parse Xbox auth response"))?;

            response.token
        };

        trace!("Got XBL token");

        trace!("Getting XSTS token");

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
            .post(endpoints::xsts_authorize_url())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(reqwest::Body::from(serde_json::to_string(&json)?))
            .send()
            .await
            .with_context(|| format!("Failed to get XSTS token"))?;

        match response.status() {
            StatusCode::OK => {
                trace!("Got XSTS token");

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
                    .with_context(|| format!("Failed to parse XSTS token response"))?;

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
                trace!("XSTS token request returned 401");

                #[derive(Deserialize)]
                struct XstsError {
                    #[serde(rename = "XErr")]
                    xerr: u32,
                }

                let xsts_err = response
                    .json::<XstsError>()
                    .await
                    .with_context(|| format!("Failed to parse XSTS error response"))?;

                Ok(Err(XboxError::from_xerr(xsts_err.xerr)))
            }
            _ => {
                trace!(
                    "XSTS token request returned unexpected status {:?}",
                    response.status()
                );

                Err(anyhow!(RequestError::from_status(&response)))
            }
        }
    }
}

#[derive(Error, Debug, Clone, Copy, Type, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum XboxError {
    #[error("no xbox account is associated with this microsoft account")]
    NoAccount,

    #[error("xbox live is not available in this country")]
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

    /// Get a user-friendly title for the error
    pub fn title(&self) -> &'static str {
        match self {
            Self::NoAccount => "No Xbox Account Found",
            Self::XboxServicesBanned => "Xbox Live Not Available",
            Self::AdultVerificationRequired => "Adult Verification Required",
            Self::ChildAccount => "Child Account Detected",
            Self::Unknown(_) => "Xbox Authentication Error",
        }
    }

    /// Get a detailed, user-friendly description of the error
    pub fn description(&self) -> &'static str {
        match self {
            Self::NoAccount => {
                "Your Microsoft account is not linked to an Xbox account. An Xbox account is required to play Minecraft."
            }
            Self::XboxServicesBanned => {
                "Xbox Live services are not available in your region. Unfortunately, this prevents authentication with Minecraft."
            }
            Self::AdultVerificationRequired => {
                "Your Xbox account requires adult verification before it can be used with Minecraft. This is a security requirement from Microsoft."
            }
            Self::ChildAccount => {
                "Your Xbox account is registered as a child account and must be part of a Microsoft Family before it can access Minecraft. This is a safety feature from Microsoft."
            }
            Self::Unknown(code) => {
                "An unexpected error occurred during Xbox authentication. This might be a temporary issue with Xbox Live services."
            }
        }
    }

    /// Get suggested recovery steps for the user
    pub fn recovery_steps(&self) -> Vec<&'static str> {
        match self {
            Self::NoAccount => vec![
                "Visit xbox.com and create a free Xbox account",
                "Sign in with the same Microsoft account you used here",
                "Complete the Xbox account setup process",
                "Return to GDLauncher and try signing in again",
            ],
            Self::XboxServicesBanned => vec![
                "Check if Xbox Live is available in your country",
                "Try using a VPN to connect to a region where Xbox Live is available (use at your own discretion)",
                "Contact Xbox Support for information about availability in your region",
            ],
            Self::AdultVerificationRequired => vec![
                "Visit xbox.com and sign in to your account",
                "Follow the adult verification process in your account settings",
                "You may need to provide proof of age",
                "Return to GDLauncher and try signing in again",
            ],
            Self::ChildAccount => vec![
                "Ask your parent or guardian to add your account to their Microsoft Family",
                "Visit account.microsoft.com/family to set up family management",
                "Your parent/guardian will need to approve Minecraft access",
                "Return to GDLauncher and try signing in again",
            ],
            Self::Unknown(_) => vec![
                "Wait a few minutes and try again",
                "Check Xbox Live service status at xbox.com/status",
                "Try signing out of all Microsoft services and signing back in",
                "Restart GDLauncher and try again",
            ],
        }
    }

    /// Get a support link for this error
    pub fn support_link(&self) -> &'static str {
        match self {
            Self::NoAccount => "https://www.xbox.com/signup",
            Self::XboxServicesBanned => {
                "https://support.xbox.com/help/account-profile/manage-account/xbox-live-available-countries"
            }
            Self::AdultVerificationRequired => {
                "https://support.xbox.com/help/account-profile/manage-account/verify-age-adult-account"
            }
            Self::ChildAccount => "https://account.microsoft.com/family/about",
            Self::Unknown(_) => "https://support.xbox.com/help/errors/error-code-search",
        }
    }

    /// Get the error code if available (for logging/debugging)
    pub fn error_code(&self) -> Option<u32> {
        match self {
            Self::NoAccount => Some(2148916233),
            Self::XboxServicesBanned => Some(2148916235),
            Self::AdultVerificationRequired => Some(2148916236), // or 2148916237
            Self::ChildAccount => Some(2148916238),
            Self::Unknown(code) => Some(*code),
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
    #[tracing::instrument(skip(xbox_auth, client))]
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

        trace!("Authenticating Minecraft account");

        let response = client
            .post(endpoints::mc_login_with_xbox_url())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(reqwest::Body::from(serde_json::to_string(&json)?))
            .send()
            .await
            .with_context(|| format!("Failed to authenticate Minecraft account"))?
            .error_for_status()?
            .json::<McAuthResponse>()
            .await
            .with_context(|| format!("Failed to parse Minecraft auth response"))?;

        trace!("Got Minecraft auth token");

        Ok(Self {
            access_token: response.access_token,
            expires_at: Utc::now() + chrono::Duration::seconds(response.expires_in),
        })
    }

    #[tracing::instrument(skip(self, client))]
    pub async fn get_entitlement(
        &self,
        client: &ClientWithMiddleware,
    ) -> anyhow::Result<Result<McEntitlement, McEntitlementMissingError>> {
        trace!("Checking game entitlement");

        #[derive(Deserialize)]
        struct EntitlementResponse {
            signature: String,
        }

        let response = client
            .get(endpoints::mc_entitlements_url())
            .bearer_auth(&self.access_token)
            .send()
            .await
            .with_context(|| format!("Failed to check game entitlement"))?
            .error_for_status()?
            .json::<EntitlementResponse>()
            .await
            .with_context(|| format!("Failed to parse game entitlement response"))?;

        trace!("Got game entitlement");

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
                        McEntitlementError::InvalidSignature
                    }
                    ErrorKind::InvalidToken | ErrorKind::MissingRequiredClaim(_) => {
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
        .get(endpoints::mc_profile_url())
        .bearer_auth(access_token)
        .send()
        .await
        .with_context(|| format!("Failed to get game profile"))?;

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

/// Username availability status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum UsernameAvailability {
    Available,
    Taken,
    NotAllowed,
}

/// Check if a Minecraft username is available
pub async fn check_username_available(
    client: &ClientWithMiddleware,
    access_token: &str,
    username: &str,
) -> anyhow::Result<UsernameAvailability> {
    #[derive(Debug, Deserialize)]
    struct AvailabilityResponse {
        status: String,
    }

    let response = client
        .get(endpoints::mc_name_availability_url(username))
        .bearer_auth(access_token)
        .send()
        .await
        .with_context(|| format!("Failed to check username availability"))?;

    let availability = response
        .json::<AvailabilityResponse>()
        .await
        .map_err(RequestError::from_error)?;

    let status = match availability.status.as_str() {
        "AVAILABLE" => UsernameAvailability::Available,
        "NOT_ALLOWED" => UsernameAvailability::NotAllowed,
        _ => UsernameAvailability::Taken, // DUPLICATE or any other status
    };

    Ok(status)
}

/// Create a new Minecraft profile with the given username
pub async fn create_profile(
    client: &ClientWithMiddleware,
    access_token: &str,
    username: &str,
) -> anyhow::Result<Result<McProfile, CreateProfileError>> {
    #[derive(Debug, Serialize)]
    struct CreateProfileRequest {
        #[serde(rename = "profileName")]
        profile_name: String,
    }

    let body = CreateProfileRequest {
        profile_name: username.to_string(),
    };

    let body_str = serde_json::to_string(&body)?;

    let response = client
        .post(endpoints::mc_profile_url())
        .bearer_auth(access_token)
        .header("Content-Type", "application/json")
        .body(reqwest::Body::from(body_str))
        .send()
        .await
        .with_context(|| format!("Failed to create profile"))?;

    match response.status() {
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
                state: String,
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
        StatusCode::BAD_REQUEST => Ok(Err(CreateProfileError::InvalidUsername)),
        StatusCode::FORBIDDEN => Ok(Err(CreateProfileError::NameNotAvailable)),
        _ => bail!(RequestError::from_status(&response)),
    }
}

#[derive(Debug, Clone)]
pub enum CreateProfileError {
    InvalidUsername,
    NameNotAvailable,
}

impl CreateProfileError {
    /// Get a user-friendly title for the error
    pub fn title(&self) -> &'static str {
        match self {
            Self::InvalidUsername => "Invalid Username",
            Self::NameNotAvailable => "Username Taken",
        }
    }

    /// Get a detailed, user-friendly description of the error
    pub fn description(&self) -> &'static str {
        match self {
            Self::InvalidUsername => {
                "The username you chose doesn't meet Minecraft's requirements. Usernames must be 3-16 characters long and can only contain letters, numbers, and underscores."
            }
            Self::NameNotAvailable => {
                "This username is already taken by another player. Please choose a different username."
            }
        }
    }

    /// Get suggested recovery steps for the user
    pub fn recovery_steps(&self) -> Vec<&'static str> {
        match self {
            Self::InvalidUsername => vec![
                "Make sure your username is 3-16 characters long",
                "Use only letters (a-z, A-Z), numbers (0-9), and underscores (_)",
                "Don't use spaces or special characters",
                "Try a different username",
            ],
            Self::NameNotAvailable => vec![
                "Try adding numbers to your desired username",
                "Try a variation of your desired name",
                "Be creative - make it unique!",
                "Check https://namemc.com to see if a username is available",
            ],
        }
    }

    /// Get a support link for this error
    pub fn support_link(&self) -> &'static str {
        "https://help.minecraft.net/hc/en-us/articles/4409152531341-Minecraft-Profile-Troubleshooting"
    }
}

#[derive(Debug, Clone)]
pub enum McEntitlement {
    Owned,
    XboxGamepass,
}

impl McEntitlement {
    fn mojang_jwt_key() -> DecodingKey {
        #[cfg(feature = "e2e")]
        if let Some(pem) = endpoints::e2e_entitlement_key() {
            return DecodingKey::from_rsa_pem(pem)
                .expect("the e2e entitlement key is not a valid RSA public key PEM");
        }

        // The test at the bottom of this file makes sure this unwrap is fine.
        DecodingKey::from_rsa_pem(include_bytes!("mojang_jwt_signature.pem")).unwrap()
    }
}

#[derive(Error, Debug)]
pub enum GetProfileError {
    #[error("missing game profile")]
    GameProfileMissing,
    #[error("authentication token invalid")]
    AuthTokenInvalid,
}

impl GetProfileError {
    /// Get a user-friendly title for the error
    pub fn title(&self) -> &'static str {
        match self {
            Self::GameProfileMissing => "Profile Creation Required",
            Self::AuthTokenInvalid => "Authentication Expired",
        }
    }

    /// Get a detailed, user-friendly description of the error
    pub fn description(&self) -> &'static str {
        match self {
            Self::GameProfileMissing => {
                "Your account needs a Minecraft profile to continue. This is a quick one-time setup where you choose your username."
            }
            Self::AuthTokenInvalid => {
                "Your authentication has expired or become invalid. This can happen after extended periods of time or security changes."
            }
        }
    }

    /// Get suggested recovery steps for the user
    pub fn recovery_steps(&self) -> Vec<&'static str> {
        match self {
            Self::GameProfileMissing => vec![
                "Choose a unique username for your Minecraft profile",
                "The username must be 3-16 characters long",
                "Only letters, numbers, and underscores are allowed",
                "Your profile will be created automatically once you choose a name",
            ],
            Self::AuthTokenInvalid => vec![
                "Sign out and sign back in to refresh your authentication",
                "Make sure you're using the correct Microsoft account",
                "If the problem persists, try restarting GDLauncher",
            ],
        }
    }

    /// Get a support link for this error
    pub fn support_link(&self) -> &'static str {
        match self {
            Self::GameProfileMissing => {
                "https://help.minecraft.net/hc/en-us/articles/4409152531341-Minecraft-Profile-Troubleshooting"
            }
            Self::AuthTokenInvalid => {
                "https://help.minecraft.net/hc/en-us/articles/4409159214605-Microsoft-Authentication-Issues"
            }
        }
    }
}

#[derive(Error, Debug)]
#[error("no game entitlement")]
pub struct McEntitlementMissingError;

impl McEntitlementMissingError {
    /// Get a user-friendly title for the error
    pub fn title(&self) -> &'static str {
        "Minecraft Not Owned"
    }

    /// Get a detailed, user-friendly description of the error
    pub fn description(&self) -> &'static str {
        "Your Microsoft account does not own Minecraft: Java Edition. You need to purchase the game or have an active Xbox Game Pass subscription to play."
    }

    /// Get suggested recovery steps for the user
    pub fn recovery_steps(&self) -> Vec<&'static str> {
        vec![
            "Visit minecraft.net to purchase Minecraft: Java Edition",
            "Check if you have an active Xbox Game Pass subscription",
            "Make sure you're signed in with the correct Microsoft account",
            "If you recently purchased the game, wait 10-15 minutes and try again",
            "Contact Minecraft Support if you believe you should have access",
        ]
    }

    /// Get a support link for this error
    pub fn support_link(&self) -> &'static str {
        "https://www.minecraft.net/get-minecraft"
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
    /// GDL JWT token (exchanged in parallel during refresh)
    pub gdl_token: Option<String>,
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
    use super::{Algorithm, ErrorKind, McEntitlement, Utc, Validation};

    /// Make sure it's possible to get a JWT decoding key from
    /// the saved public key.
    #[test]
    fn valid_mojang_account_sig() {
        // unwrap performed inside
        let _ = McEntitlement::mojang_jwt_key();
    }

    #[test]
    fn mojang_key_verifies_a_token_signed_by_mojang_only() {
        // A JWT signed with any other key must be rejected. Without this, an
        // entitlement-key override that leaked into a shipped build would let
        // an attacker-signed entitlement pass as genuine.
        //
        // The claims carry a live `exp`: `Validation::new` requires it by
        // default, and without it `decode` would fail on the missing claim
        // before it ever checked the signature, making this pass for the
        // wrong reason even against a matching key. Pinning the error to
        // `InvalidSignature` closes that gap.
        use jsonwebtoken::{EncodingKey, Header, encode};

        let rsa = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048)
            .expect("generating a test RSA key");
        let pem = rsa::pkcs8::EncodePrivateKey::to_pkcs8_pem(&rsa, Default::default())
            .expect("encoding the test key as PKCS#8 PEM");

        let exp = (Utc::now() + chrono::Duration::hours(1)).timestamp();
        let forged = encode(
            &Header::new(Algorithm::RS256),
            &serde_json::json!({ "entitlements": [{ "name": "product_minecraft" }], "exp": exp }),
            &EncodingKey::from_rsa_pem(pem.as_bytes()).expect("loading the test key"),
        )
        .expect("signing the forged entitlement");

        let decoded = jsonwebtoken::decode::<serde_json::Value>(
            &forged,
            &McEntitlement::mojang_jwt_key(),
            &Validation::new(Algorithm::RS256),
        );

        assert!(
            matches!(decoded.unwrap_err().kind(), ErrorKind::InvalidSignature),
            "the default key must reject a non-Mojang signature on its signature, not on its claims"
        );
    }
}
