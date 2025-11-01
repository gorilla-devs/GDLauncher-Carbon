use crate::api::{CoreModuleStatus, update_core_module_status};

use super::api::{
    DeviceCode, DeviceCodeExpiredError, FullAccount, GetProfileError, McAccount, McAuth,
    McEntitlementMissingError, MsAuth, XboxAuth, XboxError, get_profile,
};
use super::oauth_server::{OAuthCallbackHandle, OAuthCallbackServer};
use anyhow::anyhow;
use async_trait::async_trait;
use chrono::Utc;
use futures::{future::abortable, stream::AbortHandle};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing::{error, info, trace};
use url::Url;

/// Active process of adding an account
pub struct EnrollmentTask {
    pub status: Arc<RwLock<EnrollmentStatus>>,
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
        ms_auth: MsAuth,
        mc_auth: McAuth,
        entitlements: super::api::McEntitlement,
    },
    Complete(FullAccount),
    Failed(anyhow::Result<EnrollmentError>),
}

impl EnrollmentTask {
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

        let (task, abort_handle) = abortable(task);
        tokio::task::spawn(task);

        Self {
            status,
            abort: abort_handle,
        }
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
                // Start OAuth callback server
                let oauth_server = OAuthCallbackServer::new();
                let oauth_handle = oauth_server.start().await?;

                let redirect_uri = oauth_handle.redirect_uri();
                let port = oauth_handle.port();

                // Build Microsoft OAuth authorization URL
                // Using Authorization Code Flow (RFC 6749 Section 4.1)
                let mut auth_url =
                    Url::parse("https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize")
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
                    .append_pair("prompt", "select_account");

                let auth_url = auth_url.to_string();

                info!("OAuth authorization URL: {}", auth_url);
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
                let code = oauth_handle.wait_for_code(Duration::from_secs(300)).await?;

                // Exchange authorization code for access token
                #[derive(serde::Deserialize)]
                struct TokenResponse {
                    access_token: String,
                    id_token: String,
                    refresh_token: String,
                    expires_in: i64,
                }

                info!("Exchanging authorization code for access token");
                let token_response = client
                    .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
                    .form(&[
                        ("client_id", env!("MS_AUTH_CLIENT_ID")),
                        (
                            "scope",
                            "XboxLive.signin XboxLive.offline_access profile openid email",
                        ),
                        ("code", &code),
                        ("redirect_uri", &redirect_uri),
                        ("grant_type", "authorization_code"),
                    ])
                    .send()
                    .await
                    .map_err(|e| anyhow!("Failed to send token exchange request: {}", e))?
                    .error_for_status()
                    .map_err(|e| anyhow!("Token exchange failed: {}", e))?
                    .json::<TokenResponse>()
                    .await
                    .map_err(|e| anyhow!("Failed to parse token response: {}", e))?;

                let ms_auth = MsAuth {
                    access_token: token_response.access_token,
                    id_token: token_response.id_token,
                    refresh_token: token_response.refresh_token,
                    expires_at: Utc::now() + chrono::Duration::seconds(token_response.expires_in),
                };

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

        let (task, abort_handle) = abortable(task);
        tokio::task::spawn(task);

        Self {
            status,
            abort: abort_handle,
        }
    }

    pub fn refresh(
        client: reqwest_middleware::ClientWithMiddleware,
        refresh_token: String,
        invalidate: impl InvalidateCtx + Send + Sync + 'static,
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

                update_status(EnrollmentStatus::Complete(FullAccount {
                    ms: ms_auth,
                    mc: account,
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

        let (task, abort_handle) = abortable(task);
        let handler = tokio::task::spawn(task);

        (
            Self {
                status,
                abort: abort_handle,
            },
            handler,
        )
    }
}

impl Drop for EnrollmentTask {
    fn drop(&mut self) {
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
