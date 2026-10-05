use crate::api::{CoreModuleStatus, update_core_module_status};
use crate::error::request::{RequestError, RequestErrorDetails};

use super::api::{
    DeviceCode, DeviceCodeExpiredError, FullAccount, GetProfileError, McAccount, McAuth,
    McEntitlementMissingError, MsAuth, UnexpectedStatus, XboxAuth, XboxError, get_profile,
};
use super::endpoints;
use super::gdl_account::GDLAccountTask;
use super::oauth_server::{OAuthCallbackHandle, OAuthCallbackServer};
use anyhow::{Context, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use futures::{future::abortable, stream::AbortHandle};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing::{error, info, trace, warn};
use url::Url;

/// Active process of adding an account
pub struct EnrollmentTask {
    pub status: Arc<RwLock<EnrollmentStatus>>,
    pub id: String,
    pub cancel: tokio_util::sync::CancellationToken,
    abort: AbortHandle,
}

#[derive(Debug)]
pub enum EnrollmentStatus {
    RefreshingMSAuth,
    RequestingCode,
    PollingCode(DeviceCode),
    WaitingForBrowser {
        auth_url: String,
        redirect_uri: String,
        expires_at: chrono::DateTime<chrono::Utc>,
    },
    McLogin,
    XboxAuth,
    MCEntitlements,
    McProfile,
    NeedsProfileCreation {
        access_token: String,
        verification_retry: bool,
        ms_auth: MsAuth,
        mc_auth: McAuth,
        entitlements: super::api::McEntitlement,
    },
    Complete(FullAccount),
    Failed(anyhow::Result<EnrollmentError>),
}

impl EnrollmentTask {
    #[cfg(test)]
    pub(super) fn inert(status: EnrollmentStatus) -> Self {
        let (abort, _) = AbortHandle::new_pair();
        Self {
            status: Arc::new(RwLock::new(status)),
            abort,
            id: uuid::Uuid::new_v4().to_string(),
            cancel: tokio_util::sync::CancellationToken::new(),
        }
    }

    fn spawn(
        status: Arc<RwLock<EnrollmentStatus>>,
        task: impl std::future::Future<Output = ()> + Send + 'static,
    ) -> (
        Self,
        tokio::task::JoinHandle<Result<(), futures::future::Aborted>>,
    ) {
        let (task, abort) = abortable(task);
        let handle = tokio::spawn(task);
        (
            Self {
                status,
                abort,
                id: uuid::Uuid::new_v4().to_string(),
                cancel: tokio_util::sync::CancellationToken::new(),
            },
            handle,
        )
    }
    #[cfg(test)]
    pub(super) fn running_for_test(
        task: impl std::future::Future<Output = ()> + Send + 'static,
    ) -> (
        Self,
        tokio::task::JoinHandle<Result<(), futures::future::Aborted>>,
    ) {
        Self::spawn(
            Arc::new(RwLock::new(EnrollmentStatus::RequestingCode)),
            task,
        )
    }

    /// Begin account enrollment. `invalidate_fn` will be called
    /// whenever the task's status updates.
    pub fn begin(
        client: reqwest_middleware::ClientWithMiddleware,
        invalidate: impl InvalidateCtx + Send + Sync + 'static,
    ) -> Self {
        let status = Arc::new(RwLock::new(EnrollmentStatus::RequestingCode));
        let task_status = status.clone();

        let task = async move {
            let update_status = |status: EnrollmentStatus| async {
                *task_status.write().await = status;
                invalidate.invalidate().await;
            };

            let task = || async {
                // request device code
                let device_code = DeviceCode::request_code(&client).await?;

                // poll ms auth
                update_status(EnrollmentStatus::PollingCode(device_code.clone())).await;
                let ms_auth = device_code.poll_ms_auth().await??;

                update_status(EnrollmentStatus::XboxAuth).await;

                // authenticate with XBox
                let xbox_auth = XboxAuth::from_ms(&ms_auth, &client).await??;

                update_status(EnrollmentStatus::McLogin).await;
                // authenticate with MC
                let mc_auth = McAuth::auth_ms(xbox_auth, &client).await?;

                update_status(EnrollmentStatus::MCEntitlements).await;

                let entitlements = mc_auth.get_entitlement(&client).await??;

                update_status(EnrollmentStatus::McProfile).await;

                // Handle profile creation for new accounts
                let mc_profile = match get_profile(&client, &mc_auth.access_token).await? {
                    Ok(profile) => profile,
                    Err(GetProfileError::GameProfileMissing) => {
                        // Set status to NeedsProfileCreation instead of failing
                        update_status(EnrollmentStatus::NeedsProfileCreation {
                            access_token: mc_auth.access_token.clone(),
                            verification_retry: false,
                            ms_auth: ms_auth.clone(),
                            mc_auth: mc_auth.clone(),
                            entitlements: entitlements.clone(),
                        })
                        .await;

                        // Don't complete the enrollment yet - wait for profile creation
                        return Ok(());
                    }
                    Err(GetProfileError::AuthTokenInvalid) => {
                        return Err(GetProfileError::AuthTokenInvalid.into());
                    }
                };

                let account = McAccount {
                    entitlement: entitlements.clone(),
                    profile: mc_profile,
                    auth: mc_auth,
                };

                update_status(EnrollmentStatus::Complete(FullAccount {
                    ms: ms_auth,
                    mc: account,
                    gdl_token: None, // Will be exchanged after enrollment
                }))
                .await;

                Ok(())
            };

            publish_result(
                &task_status,
                &invalidate,
                SignInMethod::DeviceCode,
                task().await,
            )
            .await;
        };

        Self::spawn(status, task).0
    }

    /// Begin browser-based account enrollment using OAuth authorization code flow.
    ///
    /// This starts a local HTTP server to receive the OAuth callback and opens
    /// the user's browser to Microsoft's login page.
    pub fn begin_browser(
        client: reqwest_middleware::ClientWithMiddleware,
        invalidate: impl InvalidateCtx + Send + Sync + 'static,
        open_browser: bool,
    ) -> Self {
        let status = Arc::new(RwLock::new(EnrollmentStatus::RequestingCode));
        let task_status = status.clone();

        let task = async move {
            let update_status = |status: EnrollmentStatus| async {
                *task_status.write().await = status;
                invalidate.invalidate().await;
            };

            let task = || async {
                // Generate per-flow `state` (CSRF) and PKCE values (RFC 8252 §6).
                let state_token = format!(
                    "{}{}",
                    uuid::Uuid::new_v4().simple(),
                    uuid::Uuid::new_v4().simple()
                );
                let code_verifier = format!(
                    "{}{}",
                    uuid::Uuid::new_v4().simple(),
                    uuid::Uuid::new_v4().simple()
                );
                let code_challenge = {
                    use base64::Engine;
                    use sha2::Digest;
                    let digest = sha2::Sha256::digest(code_verifier.as_bytes());
                    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
                };

                // Start OAuth callback server
                let oauth_server = OAuthCallbackServer::new(state_token.clone());
                let oauth_handle = oauth_server.start().await?;

                let redirect_uri = oauth_handle.redirect_uri();
                let port = oauth_handle.port();

                // Build Microsoft OAuth authorization URL
                // Using Authorization Code Flow (RFC 6749 Section 4.1) + PKCE (RFC 7636).
                let mut auth_url = Url::parse(&endpoints::ms_authorize_url())
                    .map_err(|e| anyhow!("Failed to parse OAuth URL: {}", e))?;
                auth_url
                    .query_pairs_mut()
                    .append_pair("client_id", env!("MS_AUTH_CLIENT_ID"))
                    .append_pair("response_type", "code")
                    .append_pair("redirect_uri", &redirect_uri)
                    .append_pair(
                        "scope",
                        "XboxLive.signin XboxLive.offline_access profile openid email",
                    )
                    .append_pair("response_mode", "query")
                    .append_pair("prompt", "select_account")
                    .append_pair("state", &state_token)
                    .append_pair("code_challenge", &code_challenge)
                    .append_pair("code_challenge_method", "S256");

                let auth_url = auth_url.to_string();

                // Don't log the full URL — it contains the state token which
                // shouldn't appear in logs. Log just the host/path/port.
                info!(
                    "Starting OAuth flow against Microsoft on callback port {}",
                    port
                );
                info!("Waiting for OAuth callback on port {}", port);

                // Update status to waiting for browser
                let expires_at = Utc::now() + chrono::Duration::minutes(5);
                update_status(EnrollmentStatus::WaitingForBrowser {
                    auth_url: auth_url.clone(),
                    redirect_uri: redirect_uri.clone(),
                    expires_at,
                })
                .await;

                // Open browser if requested
                if open_browser {
                    if let Err(e) = opener::open(&auth_url) {
                        error!("Failed to open browser: {}", e);
                        // Don't fail the enrollment, user can still open manually
                    }
                }

                // Wait for authorization code (5 minute timeout)
                // Exchange authorization code for access token
                #[derive(serde::Deserialize)]
                struct TokenResponse {
                    access_token: String,
                    id_token: String,
                    refresh_token: String,
                    expires_in: i64,
                }

                let ms_auth = exchange_browser_code(
                    oauth_handle.wait_for_code(Duration::from_secs(300)),
                    |code| {
                        let client = &client;
                        let redirect_uri = &redirect_uri;
                        let code_verifier = &code_verifier;
                        async move {
                            info!("Exchanging authorization code for access token");
                            let token_response = client
                    .post(endpoints::ms_token_url())
                    .form(&[
                        ("client_id", env!("MS_AUTH_CLIENT_ID")),
                        (
                            "scope",
                            "XboxLive.signin XboxLive.offline_access profile openid email",
                        ),
                        ("code", &code),
                        ("redirect_uri", &redirect_uri),
                        ("grant_type", "authorization_code"),
                        ("code_verifier", &code_verifier),
                    ])
                    .send()
                    .await
                    .context("Failed to send token exchange request")?
                    .error_for_status()
                    .context("Token exchange failed")?
                    .json::<TokenResponse>()
                    .await
                    .context("Failed to parse token response")?;

                            Ok(MsAuth {
                                access_token: token_response.access_token,
                                id_token: token_response.id_token,
                                refresh_token: token_response.refresh_token,
                                expires_at: Utc::now()
                                    + chrono::Duration::seconds(token_response.expires_in),
                            })
                        }
                    },
                )
                .await?;

                // Shutdown the OAuth server
                oauth_handle.shutdown().await;

                info!("Successfully obtained Microsoft access token via browser flow");

                // Continue with Xbox/Minecraft authentication (same as device code flow)
                update_status(EnrollmentStatus::XboxAuth).await;

                // authenticate with XBox
                let xbox_auth = XboxAuth::from_ms(&ms_auth, &client).await??;

                update_status(EnrollmentStatus::McLogin).await;
                // authenticate with MC
                let mc_auth = McAuth::auth_ms(xbox_auth, &client).await?;

                update_status(EnrollmentStatus::MCEntitlements).await;

                let entitlements = mc_auth.get_entitlement(&client).await??;

                update_status(EnrollmentStatus::McProfile).await;

                // Handle profile creation for new accounts
                let mc_profile = match get_profile(&client, &mc_auth.access_token).await? {
                    Ok(profile) => profile,
                    Err(GetProfileError::GameProfileMissing) => {
                        // Set status to NeedsProfileCreation instead of failing
                        update_status(EnrollmentStatus::NeedsProfileCreation {
                            access_token: mc_auth.access_token.clone(),
                            verification_retry: false,
                            ms_auth: ms_auth.clone(),
                            mc_auth: mc_auth.clone(),
                            entitlements: entitlements.clone(),
                        })
                        .await;

                        // Don't complete the enrollment yet - wait for profile creation
                        return Ok(());
                    }
                    Err(GetProfileError::AuthTokenInvalid) => {
                        return Err(GetProfileError::AuthTokenInvalid.into());
                    }
                };

                let account = McAccount {
                    entitlement: entitlements.clone(),
                    profile: mc_profile,
                    auth: mc_auth,
                };

                update_status(EnrollmentStatus::Complete(FullAccount {
                    ms: ms_auth,
                    mc: account,
                    gdl_token: None, // Will be exchanged after enrollment
                }))
                .await;

                Ok(())
            };

            publish_result(
                &task_status,
                &invalidate,
                SignInMethod::Browser,
                task().await,
            )
            .await;
        };

        Self::spawn(status, task).0
    }

    pub fn refresh(
        client: reqwest_middleware::ClientWithMiddleware,
        refresh_token: String,
        invalidate: impl InvalidateCtx + Send + Sync + 'static,
        gdl_account_task: GDLAccountTask,
    ) -> (
        Self,
        tokio::task::JoinHandle<Result<(), futures::future::Aborted>>,
    ) {
        let status = Arc::new(RwLock::new(EnrollmentStatus::RequestingCode));
        let task_status = status.clone();

        let task = async move {
            let update_status = |status: EnrollmentStatus| async {
                *task_status.write().await = status;
                invalidate.invalidate().await;
            };

            let task = || async {
                update_status(EnrollmentStatus::RefreshingMSAuth).await;

                trace!("Refreshing MsAuth with refresh token");
                // attempt to refresh token
                let ms_auth = MsAuth::refresh(&client, &refresh_token).await?;
                update_core_module_status(CoreModuleStatus::RefreshMSAuth);

                trace!("Successfully refreshed MsAuth with refresh token");

                // Spawn GDL token exchange in parallel with Xbox/MC auth
                let gdl_id_token = ms_auth.id_token.clone();
                let gdl_handle = tokio::spawn(async move {
                    update_core_module_status(CoreModuleStatus::ExchangingGdlToken);
                    match gdl_account_task.exchange_token(&gdl_id_token).await {
                        Ok(response) => Some(response.access_token),
                        Err(e) => {
                            warn!("Failed to exchange GDL token: {}", e);
                            None
                        }
                    }
                });

                update_status(EnrollmentStatus::XboxAuth).await;
                trace!("Authenticating with XBox");

                // authenticate with XBox
                let xbox_auth = XboxAuth::from_ms(&ms_auth, &client).await??;

                update_core_module_status(CoreModuleStatus::XboxAuth);
                trace!("Successfully authenticated with XBox");

                trace!("Authenticating with MC");

                update_status(EnrollmentStatus::McLogin).await;
                // authenticate with MC
                let mc_auth = McAuth::auth_ms(xbox_auth, &client).await?;

                update_core_module_status(CoreModuleStatus::McLogin);
                trace!("Successfully authenticated with MC");

                update_status(EnrollmentStatus::MCEntitlements).await;
                let entitlements = mc_auth.get_entitlement(&client).await??;
                update_core_module_status(CoreModuleStatus::MCEntitlements);

                update_status(EnrollmentStatus::McProfile).await;

                // Handle profile creation for new accounts
                let mc_profile = match get_profile(&client, &mc_auth.access_token).await? {
                    Ok(profile) => profile,
                    Err(GetProfileError::GameProfileMissing) => {
                        // Set status to NeedsProfileCreation instead of failing
                        update_status(EnrollmentStatus::NeedsProfileCreation {
                            access_token: mc_auth.access_token.clone(),
                            verification_retry: false,
                            ms_auth: ms_auth.clone(),
                            mc_auth: mc_auth.clone(),
                            entitlements: entitlements.clone(),
                        })
                        .await;

                        // Don't complete the enrollment yet - wait for profile creation
                        return Ok(());
                    }
                    Err(GetProfileError::AuthTokenInvalid) => {
                        return Err(GetProfileError::AuthTokenInvalid.into());
                    }
                };

                update_core_module_status(CoreModuleStatus::McProfile);

                let account = McAccount {
                    entitlement: entitlements.clone(),
                    profile: mc_profile,
                    auth: mc_auth,
                };

                // Await GDL token exchange result before completing
                let gdl_token = gdl_handle.await.ok().flatten();

                update_status(EnrollmentStatus::Complete(FullAccount {
                    ms: ms_auth,
                    mc: account,
                    gdl_token,
                }))
                .await;

                Ok(())
            };

            match task().await {
                Ok(()) => {}
                Err(EnrollmentErrorOrAnyhow::EnrollmentError(e)) => {
                    update_status(EnrollmentStatus::Failed(Ok(e))).await
                }
                Err(EnrollmentErrorOrAnyhow::Anyhow(e)) => {
                    update_status(EnrollmentStatus::Failed(Err(e))).await
                }
            };
        };

        Self::spawn(status, task)
    }
}

pub(super) async fn exchange_browser_code<W, X, F>(waiter: W, exchange: X) -> anyhow::Result<MsAuth>
where
    W: std::future::Future<Output = anyhow::Result<String>>,
    X: FnOnce(String) -> F,
    F: std::future::Future<Output = anyhow::Result<MsAuth>>,
{
    // A terminal callback error stops here, before constructing a token request.
    exchange(waiter.await?).await
}

#[derive(Debug, Clone, Copy)]
pub(super) enum SignInMethod {
    DeviceCode,
    Browser,
}

impl std::fmt::Display for SignInMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::DeviceCode => "device code",
            Self::Browser => "browser",
        })
    }
}

/// Publishes how a sign-in ended. A failure's cause goes to the core log
/// here: the renderer only gets a typed error or a [`FailureCategory`],
/// never the cause's text, which can carry provider details.
pub(super) async fn publish_result(
    status: &RwLock<EnrollmentStatus>,
    invalidate: &impl InvalidateCtx,
    method: SignInMethod,
    result: Result<(), EnrollmentErrorOrAnyhow>,
) {
    let failure = match result {
        Ok(()) => return,
        Err(EnrollmentErrorOrAnyhow::EnrollmentError(error)) => {
            info!("Microsoft sign-in ({method}) stopped: {error}");
            Ok(error)
        }
        Err(EnrollmentErrorOrAnyhow::Anyhow(error)) => {
            warn!(
                category = ?FailureCategory::of(&error),
                "Microsoft sign-in ({method}) failed: {error:#}"
            );
            Err(error)
        }
    };
    *status.write().await = EnrollmentStatus::Failed(failure);
    invalidate.invalidate().await;
}

/// What the sign-in screen may say about a failure without a typed
/// [`EnrollmentError`]. Only this category reaches the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCategory {
    /// No answer came back: offline, DNS, a proxy, VPN or antivirus in the
    /// way, or a timeout.
    Network,
    /// Microsoft, Xbox or Minecraft answered that it is busy or failing.
    ServiceUnavailable,
    Other,
}

impl FailureCategory {
    pub fn of(error: &anyhow::Error) -> Self {
        error
            .chain()
            .find_map(Self::of_cause)
            .unwrap_or(Self::Other)
    }

    fn of_cause(cause: &(dyn std::error::Error + 'static)) -> Option<Self> {
        if let Some(error) = cause.downcast_ref::<reqwest::Error>() {
            if error.is_connect() || error.is_timeout() {
                return Some(Self::Network);
            }
            return error.status().and_then(Self::of_status);
        }
        if let Some(error) = cause.downcast_ref::<RequestError>() {
            return match &error.error {
                RequestErrorDetails::Timeout | RequestErrorDetails::ConnectionFailed => {
                    Some(Self::Network)
                }
                RequestErrorDetails::UnexpectedStatus { status, .. } => Self::of_status(*status),
                _ => None,
            };
        }
        if let Some(error) = cause.downcast_ref::<UnexpectedStatus>() {
            return Self::of_status(error.status);
        }
        if let Some(error) = cause.downcast_ref::<std::io::Error>() {
            use std::io::ErrorKind::*;
            return matches!(
                error.kind(),
                ConnectionRefused
                    | ConnectionReset
                    | ConnectionAborted
                    | NotConnected
                    | BrokenPipe
                    | TimedOut
                    | HostUnreachable
                    | NetworkUnreachable
                    | NetworkDown
            )
            .then_some(Self::Network);
        }
        None
    }

    fn of_status(status: reqwest::StatusCode) -> Option<Self> {
        (status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error())
            .then_some(Self::ServiceUnavailable)
    }
}

impl Drop for EnrollmentTask {
    fn drop(&mut self) {
        self.cancel.cancel();
        self.abort.abort()
    }
}

#[async_trait]
pub trait InvalidateCtx {
    async fn invalidate(&self);
}

#[derive(Error, Debug, Clone)]
pub enum EnrollmentError {
    #[error("device code expired")]
    DeviceCodeExpired,
    #[error("xbox error: {0}")]
    XboxError(#[from] XboxError),
    #[error("game entitlement missing")]
    EntitlementMissing,
    #[error("game profile missing")]
    GameProfileMissing,
}

impl EnrollmentError {
    /// Get a user-friendly title for the error
    pub fn title(&self) -> &'static str {
        match self {
            Self::DeviceCodeExpired => "Authentication Code Expired",
            Self::XboxError(e) => e.title(),
            Self::EntitlementMissing => "Minecraft Not Owned",
            Self::GameProfileMissing => "Profile Creation Required",
        }
    }

    /// Get a detailed, user-friendly description of the error
    pub fn description(&self) -> String {
        match self {
            Self::DeviceCodeExpired =>
                "The authentication code has expired. Authentication codes are only valid for a limited time.".to_string(),
            Self::XboxError(e) => e.description().to_string(),
            Self::EntitlementMissing =>
                "Your Microsoft account does not own Minecraft: Java Edition. You need to purchase the game to continue.".to_string(),
            Self::GameProfileMissing =>
                "Your account needs a Minecraft profile to continue. This is a one-time setup.".to_string(),
        }
    }

    /// Get suggested recovery steps for the user
    pub fn recovery_steps(&self) -> Vec<String> {
        match self {
            Self::DeviceCodeExpired => vec![
                "Click the button to start the authentication process again".to_string(),
                "Complete the authentication more quickly this time".to_string(),
                "If you continue to have issues, try the browser authentication method".to_string(),
            ],
            Self::XboxError(e) => e.recovery_steps().iter().map(|s| s.to_string()).collect(),
            Self::EntitlementMissing => vec![
                "Visit minecraft.net to purchase Minecraft: Java Edition".to_string(),
                "Check if you're signed in with the correct Microsoft account".to_string(),
                "If you recently purchased the game, wait a few minutes and try again".to_string(),
                "Contact Minecraft Support if you believe this is an error".to_string(),
            ],
            Self::GameProfileMissing => vec![
                "Choose a username for your Minecraft profile".to_string(),
                "Make sure the username follows Minecraft's guidelines".to_string(),
                "The profile creation happens automatically - no additional steps needed"
                    .to_string(),
            ],
        }
    }

    /// Get a support link for this error
    pub fn support_link(&self) -> &'static str {
        match self {
            Self::DeviceCodeExpired => {
                "https://help.minecraft.net/hc/en-us/articles/4409159214605-Microsoft-Authentication-Issues"
            }
            Self::XboxError(e) => e.support_link(),
            Self::EntitlementMissing => "https://www.minecraft.net/get-minecraft",
            Self::GameProfileMissing => {
                "https://help.minecraft.net/hc/en-us/articles/4409152531341-Minecraft-Profile-Troubleshooting"
            }
        }
    }
}

pub enum EnrollmentErrorOrAnyhow {
    EnrollmentError(EnrollmentError),
    Anyhow(anyhow::Error),
}

impl From<DeviceCodeExpiredError> for EnrollmentErrorOrAnyhow {
    fn from(_: DeviceCodeExpiredError) -> Self {
        Self::EnrollmentError(EnrollmentError::DeviceCodeExpired)
    }
}

impl From<XboxError> for EnrollmentErrorOrAnyhow {
    fn from(value: XboxError) -> Self {
        Self::EnrollmentError(EnrollmentError::XboxError(value))
    }
}

impl From<McEntitlementMissingError> for EnrollmentErrorOrAnyhow {
    fn from(_: McEntitlementMissingError) -> Self {
        Self::EnrollmentError(EnrollmentError::EntitlementMissing)
    }
}

impl From<GetProfileError> for EnrollmentErrorOrAnyhow {
    fn from(value: GetProfileError) -> Self {
        match value {
            GetProfileError::GameProfileMissing => {
                Self::EnrollmentError(EnrollmentError::GameProfileMissing)
            }
            GetProfileError::AuthTokenInvalid => {
                Self::Anyhow(anyhow!(GetProfileError::AuthTokenInvalid))
            }
        }
    }
}

impl From<anyhow::Error> for EnrollmentErrorOrAnyhow {
    fn from(value: anyhow::Error) -> Self {
        Self::Anyhow(value)
    }
}

#[cfg(test)]
mod sign_in_failure_tests {
    use super::*;
    use crate::error::request::RequestContext;
    use anyhow::Context as _;

    fn client() -> reqwest_middleware::ClientWithMiddleware {
        reqwest_middleware::ClientBuilder::new(
            reqwest::Client::builder()
                .timeout(Duration::from_millis(300))
                .build()
                .unwrap(),
        )
        .build()
    }

    /// A request the way the Xbox and Minecraft steps send it.
    async fn request(url: &str) -> anyhow::Error {
        let result: anyhow::Result<reqwest::Response> = async {
            Ok(client()
                .get(url)
                .send()
                .await
                .context("Failed to authenticate Xbox account")?
                .error_for_status()?)
        }
        .await;
        result.expect_err("synthetic request unexpectedly succeeded")
    }

    async fn status(code: usize) -> anyhow::Error {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/")
            .with_status(code)
            .create_async()
            .await;
        request(&format!("{}/", server.url())).await
    }

    #[tokio::test]
    async fn refused_connection_is_a_network_failure() {
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let error = request(&format!("http://127.0.0.1:{port}/")).await;
        assert_eq!(FailureCategory::of(&error), FailureCategory::Network);
    }

    #[tokio::test]
    async fn unanswered_request_is_a_network_failure() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let error = request(&url).await;
        drop(listener);
        assert_eq!(FailureCategory::of(&error), FailureCategory::Network);
    }

    #[tokio::test]
    async fn busy_or_failing_service_is_unavailable_and_other_statuses_are_generic() {
        for code in [429, 500, 503] {
            assert_eq!(
                FailureCategory::of(&status(code).await),
                FailureCategory::ServiceUnavailable,
                "status {code}"
            );
        }
        for code in [400, 403, 404] {
            assert_eq!(
                FailureCategory::of(&status(code).await),
                FailureCategory::Other,
                "status {code}"
            );
        }
    }

    #[test]
    fn typed_request_errors_keep_their_category_under_context() {
        let typed = |error| {
            anyhow::Error::new(RequestError {
                context: RequestContext::none(),
                error,
            })
            .context("Failed to get game profile")
        };
        assert_eq!(
            FailureCategory::of(&typed(RequestErrorDetails::ConnectionFailed)),
            FailureCategory::Network
        );
        assert_eq!(
            FailureCategory::of(&typed(RequestErrorDetails::Timeout)),
            FailureCategory::Network
        );
        assert_eq!(
            FailureCategory::of(&typed(RequestErrorDetails::UnexpectedStatus {
                status: reqwest::StatusCode::BAD_GATEWAY,
                details: None,
            })),
            FailureCategory::ServiceUnavailable
        );
        assert_eq!(
            FailureCategory::of(&typed(RequestErrorDetails::UnexpectedStatus {
                status: reqwest::StatusCode::FORBIDDEN,
                details: None,
            })),
            FailureCategory::Other
        );
        let reset = anyhow::Error::new(std::io::Error::from(std::io::ErrorKind::ConnectionReset));
        assert_eq!(FailureCategory::of(&reset), FailureCategory::Network);
        let local = anyhow::Error::new(std::io::Error::from(std::io::ErrorKind::AddrInUse));
        assert_eq!(FailureCategory::of(&local), FailureCategory::Other);
    }

    #[test]
    fn denials_and_text_only_failures_stay_generic() {
        for error in [
            anyhow!("access_denied: Cancelled by user"),
            anyhow!("Timeout waiting for OAuth callback"),
            anyhow!("error sending request: connection refused"),
        ] {
            assert_eq!(FailureCategory::of(&error), FailureCategory::Other);
        }
    }

    struct Invalidations(std::sync::atomic::AtomicUsize);
    #[async_trait]
    impl InvalidateCtx for Invalidations {
        async fn invalidate(&self) {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn a_failed_sign_in_logs_its_cause_and_still_publishes_it() {
        for method in [SignInMethod::DeviceCode, SignInMethod::Browser] {
            let status = RwLock::new(EnrollmentStatus::RequestingCode);
            let invalidate = Invalidations(Default::default());
            let error = anyhow!("synthetic dns failure for {method}")
                .context("Failed to authenticate Xbox account");
            publish_result(
                &status,
                &invalidate,
                method,
                Err(EnrollmentErrorOrAnyhow::Anyhow(error)),
            )
            .await;
            assert!(matches!(
                &*status.read().await,
                EnrollmentStatus::Failed(Err(_))
            ));
            assert_eq!(invalidate.0.load(std::sync::atomic::Ordering::SeqCst), 1);
            assert!(logs_contain(&format!(
                "Microsoft sign-in ({method}) failed: Failed to authenticate Xbox account: synthetic dns failure for {method}"
            )));
        }
        let status = RwLock::new(EnrollmentStatus::RequestingCode);
        publish_result(
            &status,
            &Invalidations(Default::default()),
            SignInMethod::DeviceCode,
            Err(EnrollmentErrorOrAnyhow::EnrollmentError(
                EnrollmentError::EntitlementMissing,
            )),
        )
        .await;
        assert!(logs_contain(
            "Microsoft sign-in (device code) stopped: game entitlement missing"
        ));
    }
}

/*
mod test {
    use std::sync::Arc;

    use async_trait::async_trait;
    use tokio::sync::RwLock;

    use crate::managers::account::enroll::EnrollmentStatus;

    use super::InvalidateCtx;

    use super::EnrollmentTask;

    #[tokio::test]
    async fn test_mc_auth() {
        let enrollment = Arc::new(RwLock::new(Option::<EnrollmentTask>::None));

        struct Printer {
            enrollment: Arc<RwLock<Option<EnrollmentTask>>>,
        }

        #[async_trait]
        impl InvalidateCtx for Printer {
            async fn invalidate(&self) {
                let enrollment1 = self.enrollment.read().await;
                let enrollment = enrollment1.as_ref().unwrap().status.read().await;
                if let EnrollmentStatus::Failed(e) = &*enrollment {
                    println!("{e}");
                }
                println!("Invalidate: {enrollment:#?}",);
            }
        }

        *enrollment.write().await = Some(EnrollmentTask::begin(
            reqwest::Client::new(),
            Printer {
                enrollment: enrollment.clone(),
            },
        ));

        tokio::time::sleep(std::time::Duration::from_secs(10000)).await
    }
}*/
