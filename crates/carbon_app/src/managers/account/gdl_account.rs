use crate::domain::instance::info;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use hyper::{
    HeaderMap, StatusCode,
    header::{AUTHORIZATION, CONTENT_TYPE, InvalidHeaderValue},
};
use reqwest::multipart::Form;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{fs::File, sync::watch::Sender};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct GDLAccountTask {
    client: reqwest_middleware::ClientWithMiddleware,
    base_api: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RegisterAccountBody {
    pub email: String,
    pub display_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
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

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("request failed: {0}")]
    RequestFailed(anyhow::Error),
}

#[derive(Error, Debug)]
pub enum CancelGDLAccountDeletionError {
    /// Backend says there's nothing to cancel — either the user never
    /// scheduled, or the sweep already promoted the row. Treated as an
    /// info-level "already done" on the UI side.
    #[error("No scheduled deletion to cancel")]
    NoScheduledDeletion,

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("request failed: {0}")]
    RequestFailed(anyhow::Error),
}

#[derive(Error, Debug)]
pub enum ChangeDisplayNameError {
    #[error("Too many requests")]
    TooManyRequests(u32),

    #[error("request failed: {0}")]
    RequestFailed(anyhow::Error),
}

/// Error from fetching the GDL user profile.
#[derive(Error, Debug)]
pub enum GetAccountError {
    /// The backend rejected the GDL token. Recoverable — exchange a new one.
    #[error("GDL token rejected by the server")]
    Unauthorized,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<reqwest::Error> for GetAccountError {
    fn from(value: reqwest::Error) -> Self {
        Self::Other(value.into())
    }
}

impl From<reqwest_middleware::Error> for GetAccountError {
    fn from(value: reqwest_middleware::Error) -> Self {
        Self::Other(value.into())
    }
}

/// Error response structure from enderium API
#[derive(Debug, Deserialize)]
struct EnderiumErrorResponse {
    error: EnderiumErrorBody,
}

#[derive(Debug, Deserialize)]
struct EnderiumErrorBody {
    code: String,
    message: String,
}

/// Typed error for instance sharing operations
#[derive(Error, Debug)]
pub enum InstanceShareError {
    #[error("Share not found or expired")]
    ShareNotFound,
    #[error("Storage quota exceeded")]
    QuotaExceeded,
    #[error("Instance is too large to share")]
    InstanceTooLarge,
    #[error("Maximum downloads exceeded")]
    MaxDownloadsExceeded,
    #[error("Upload timed out")]
    UploadTimeout,
    #[error("Account not verified")]
    UserNotVerified,
    #[error("Too many active shares (max 50)")]
    TooManyActiveShares,
    #[error("Account is banned")]
    AccountBanned,
    #[error("Image rejected by content moderation")]
    ImageRejectedByModeration,
    #[error("Image moderation temporarily unavailable, please try again")]
    ModerationUnavailable,
    #[error("Too many image uploads, please wait a bit before trying again")]
    ModerationRateLimited,
    #[error("Image too large, maximum size is 5MB")]
    ImageTooLarge,
    #[error("Invalid image format, must be PNG, JPEG, GIF, or WebP")]
    InvalidImageFormat,
    #[error("Invalid header: {0}")]
    InvalidHeader(#[from] InvalidHeaderValue),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Network error: {0}")]
    NetworkMiddleware(#[from] reqwest_middleware::Error),
    #[error("{0}")]
    Unknown(String),
}

impl crate::error::FeErrorCode for InstanceShareError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::ShareNotFound => "SHARE_NOT_FOUND",
            Self::QuotaExceeded => "QUOTA_EXCEEDED",
            Self::InstanceTooLarge => "INSTANCE_TOO_LARGE",
            Self::MaxDownloadsExceeded => "MAX_DOWNLOADS_EXCEEDED",
            Self::UploadTimeout => "UPLOAD_TIMEOUT",
            Self::UserNotVerified => "USER_NOT_VERIFIED",
            Self::TooManyActiveShares => "TOO_MANY_ACTIVE_SHARES",
            Self::AccountBanned => "ACCOUNT_BANNED",
            Self::ImageRejectedByModeration => "IMAGE_REJECTED_BY_MODERATION",
            Self::ModerationUnavailable => "MODERATION_UNAVAILABLE",
            Self::ModerationRateLimited => "MODERATION_RATE_LIMITED",
            Self::ImageTooLarge => "IMAGE_TOO_LARGE",
            Self::InvalidImageFormat => "INVALID_IMAGE_FORMAT",
            Self::InvalidHeader(_) => "INVALID_HEADER",
            Self::Json(_) => "JSON_ERROR",
            Self::Network(_) | Self::NetworkMiddleware(_) => "NETWORK_ERROR",
            Self::Unknown(_) => "UNKNOWN_ERROR",
        }
    }
}

impl InstanceShareError {
    fn from_response(status: StatusCode, body: &str) -> Self {
        if let Ok(resp) = serde_json::from_str::<EnderiumErrorResponse>(body) {
            match resp.error.code.as_str() {
                "SHARE_NOT_FOUND" => Self::ShareNotFound,
                "QUOTA_EXCEEDED" => Self::QuotaExceeded,
                "INSTANCE_TOO_LARGE" => Self::InstanceTooLarge,
                "MAX_DOWNLOADS_EXCEEDED" => Self::MaxDownloadsExceeded,
                "UPLOAD_TIMEOUT" => Self::UploadTimeout,
                "USER_NOT_VERIFIED" => Self::UserNotVerified,
                "TOO_MANY_ACTIVE_SHARES" => Self::TooManyActiveShares,
                "ACCOUNT_BANNED" => Self::AccountBanned,
                "IMAGE_REJECTED_BY_MODERATION" => Self::ImageRejectedByModeration,
                "MODERATION_UNAVAILABLE" => Self::ModerationUnavailable,
                "MODERATION_RATE_LIMITED" => Self::ModerationRateLimited,
                "IMAGE_TOO_LARGE" => Self::ImageTooLarge,
                "INVALID_IMAGE_FORMAT" => Self::InvalidImageFormat,
                _ => Self::Unknown(resp.error.message),
            }
        } else if status == StatusCode::REQUEST_TIMEOUT {
            // Infrastructure timeouts (a proxy or tower TimeoutLayer cutting a
            // long poll) reply 408 with no error JSON. Treat them like the
            // typed UPLOAD_TIMEOUT so callers re-poll instead of failing.
            Self::UploadTimeout
        } else {
            Self::Unknown(format!("HTTP {}: {}", status.as_u16(), body))
        }
    }
}

/// Typed error for avatar upload operations. Mirrors the error codes the
/// enderium backend returns from `/v1/users/user/avatar` so the frontend can
/// show a specific toast per failure mode.
#[derive(Error, Debug)]
pub enum AvatarUploadError {
    #[error("Image rejected by content moderation")]
    ImageRejectedByModeration,
    #[error("Image moderation temporarily unavailable, please try again")]
    ModerationUnavailable,
    #[error("Too many image uploads, please wait a bit before trying again")]
    ModerationRateLimited,
    #[error("Image too large, maximum size is 5MB")]
    ImageTooLarge,
    #[error("Invalid image format, must be PNG or JPEG")]
    InvalidImageFormat,
    #[error("No file provided")]
    NoFileProvided,
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("{0}")]
    Unknown(String),
}

impl crate::error::FeErrorCode for AvatarUploadError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::ImageRejectedByModeration => "IMAGE_REJECTED_BY_MODERATION",
            Self::ModerationUnavailable => "MODERATION_UNAVAILABLE",
            Self::ModerationRateLimited => "MODERATION_RATE_LIMITED",
            Self::ImageTooLarge => "IMAGE_TOO_LARGE",
            Self::InvalidImageFormat => "INVALID_IMAGE_FORMAT",
            Self::NoFileProvided => "NO_FILE_PROVIDED",
            Self::Network(_) => "NETWORK_ERROR",
            Self::Unknown(_) => "UNKNOWN_ERROR",
        }
    }
}

impl AvatarUploadError {
    fn from_response(status: StatusCode, body: &str) -> Self {
        if let Ok(resp) = serde_json::from_str::<EnderiumErrorResponse>(body) {
            match resp.error.code.as_str() {
                "IMAGE_REJECTED_BY_MODERATION" => Self::ImageRejectedByModeration,
                "MODERATION_UNAVAILABLE" => Self::ModerationUnavailable,
                "MODERATION_RATE_LIMITED" => Self::ModerationRateLimited,
                "IMAGE_TOO_LARGE" => Self::ImageTooLarge,
                "INVALID_IMAGE_FORMAT" => Self::InvalidImageFormat,
                "NO_FILE_PROVIDED" => Self::NoFileProvided,
                _ => Self::Unknown(resp.error.message),
            }
        } else {
            Self::Unknown(format!("HTTP {}: {}", status.as_u16(), body))
        }
    }
}

/// Helper to handle instance share API responses
async fn handle_instance_share_response<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
) -> Result<T, InstanceShareError> {
    let status = resp.status();
    if status.is_success() {
        Ok(resp.json().await?)
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(InstanceShareError::from_response(status, &body))
    }
}

/// Helper to handle instance share API responses that return empty body on success
async fn handle_instance_share_response_empty(
    resp: reqwest::Response,
) -> Result<(), InstanceShareError> {
    let status = resp.status();
    if status.is_success() {
        Ok(())
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(InstanceShareError::from_response(status, &body))
    }
}

/// Helper to handle instance share API responses that return plain text
async fn handle_instance_share_response_text(
    resp: reqwest::Response,
) -> Result<String, InstanceShareError> {
    let status = resp.status();
    if status.is_success() {
        Ok(resp.text().await?)
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(InstanceShareError::from_response(status, &body))
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct DisplayNameHistoryEntry {
    pub display_name: String,
    pub changed_at: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum GDLAccountStatus {
    Valid(GDLUser),
    Invalid,
    Skipped,
    Unset,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GDLUser {
    pub email: String,
    pub microsoft_oid: String,
    pub display_name: String,
    pub friend_code: String,
    pub profile_icon_url: String,
    #[serde(default)]
    pub has_custom_avatar: bool,
    pub microsoft_email: Option<String>,
    pub is_verified: bool,
    pub has_pending_verification: bool,
    pub has_pending_deletion_request: bool,

    // Cooldown timeouts in seconds (backwards compatible)
    pub verification_timeout: Option<i64>,
    pub deletion_timeout: Option<i64>,
    pub email_change_timeout: Option<i64>,
    pub display_name_change_timeout: Option<i64>,

    // Absolute UTC timestamps when cooldown expires (ISO 8601)
    pub verification_timeout_at: Option<String>,
    pub deletion_timeout_at: Option<String>,
    pub email_change_timeout_at: Option<String>,
    pub display_name_change_timeout_at: Option<String>,

    // Set iff the user has clicked the deletion-confirm email and is
    // inside the 7-day cancel window. Presence is the whole signal —
    // the backend used to expose a parallel `has_scheduled_deletion`
    // boolean but collapsed to this single Option. Value is the
    // absolute UTC time at which the sweep will hard-delete.
    #[serde(default)]
    pub scheduled_deletion_effective_at: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RequestShareInstanceBody {
    pub file_key: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GetPresignedUploadUrlResponse {
    pub file_key: String,
    pub url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GetPresignedUploadUrlBody {
    pub content_length: u64,
    pub sha256_checksum: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_days: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_downloads: Option<i32>,
    // Instance metadata for preview
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minecraft_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modloader_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modloader_version: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mods: Vec<SharedMod>,
}

/// Individual mod data for sharing - supports both CurseForge and Modrinth
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SharedMod {
    pub name: String,
    // CurseForge data (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub curseforge_project_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub curseforge_file_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub curseforge_slug: Option<String>,
    // Modrinth data (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modrinth_project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modrinth_version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modrinth_slug: Option<String>,
}

/// Metadata for instance sharing
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ShareMetadata {
    pub minecraft_version: Option<String>,
    pub modloader_type: Option<String>,
    pub modloader_version: Option<String>,
    pub mods: Vec<SharedMod>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GetPresignedDownloadUrlBody {
    pub share_code: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WaitForShareInstanceResponse {
    pub share_code: String,
    pub expires_at: DateTime<Utc>,
}

/// Individual share info returned from the list endpoint
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ShareInfo {
    pub share_code: String,
    pub title: Option<String>,
    pub download_count: i32,
    pub max_downloads: Option<i32>,
    pub expires_at: DateTime<Utc>,
    pub size_kilobytes: i32,
    pub created_at: DateTime<Utc>,
    pub is_expired: bool,
    pub minecraft_version: Option<String>,
    pub modloader_type: Option<String>,
    pub modloader_version: Option<String>,
    #[serde(default)]
    pub mods: Vec<SharedMod>,
    pub background_url: Option<String>,
}

/// Share preview for public endpoint (no auth required)
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct SharePreview {
    pub share_code: String,
    pub title: Option<String>,
    pub minecraft_version: Option<String>,
    pub modloader_type: Option<String>,
    pub modloader_version: Option<String>,
    #[serde(default)]
    pub mods: Vec<SharedMod>,
    pub size_kilobytes: i32,
    pub background_url: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub download_count: i32,
    pub max_downloads: Option<i32>,
    pub sharer_display_name: String,
    pub sharer_friend_code: String,
}

/// Paginated response for user shares
#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct PaginatedShares {
    pub items: Vec<ShareInfo>,
    pub total_count: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Quota info for instance sharing
#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct QuotaInfo {
    pub used_kilobytes: i64,
    pub total_kilobytes: i64,
}

/// Request body for reporting a share.
/// `report_type` must be one of: "share_background", "share_title", "share_content".
#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ReportShareBody {
    pub report_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Request body for updating a share
#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct UpdateShareBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_downloads: Option<Option<i32>>,
}

/// Response for regenerating a share code
#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct RegenerateShareCodeResponse {
    pub new_share_code: String,
}

/// Response from the GDL token exchange endpoint
#[derive(Deserialize, Debug)]
pub struct TokenExchangeResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: u64,
}

/// Error for token exchange operations
#[derive(Error, Debug)]
pub enum TokenExchangeError {
    #[error("Invalid or expired Microsoft token")]
    InvalidToken,
    #[error("Token exchange service unavailable (HTTP {0})")]
    ServiceUnavailable(u16),
    #[error("Network error: {0}")]
    Network(#[from] reqwest_middleware::Error),
    #[error("Request failed (HTTP {status}): {body}")]
    RequestFailed { status: u16, body: String },
}

impl GDLAccountTask {
    pub fn new(client: reqwest_middleware::ClientWithMiddleware, base_api: String) -> Self {
        Self { client, base_api }
    }

    /// Exchange a Microsoft JWT for a GDL custom JWT.
    ///
    /// This calls POST /v1/auth/token with the Microsoft id_token
    /// and returns a GDL JWT that should be used for all GDL API calls.
    pub async fn exchange_token(
        &self,
        ms_id_token: &str,
    ) -> Result<TokenExchangeResponse, TokenExchangeError> {
        let url = format!("{}/v1/auth/token", self.base_api);

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", ms_id_token))
            .send()
            .await?;

        match resp.status() {
            StatusCode::OK => {
                let response: TokenExchangeResponse =
                    resp.json()
                        .await
                        .map_err(|e| TokenExchangeError::RequestFailed {
                            status: 200,
                            body: e.to_string(),
                        })?;
                Ok(response)
            }
            StatusCode::UNAUTHORIZED => Err(TokenExchangeError::InvalidToken),
            status @ (StatusCode::SERVICE_UNAVAILABLE | StatusCode::INTERNAL_SERVER_ERROR) => {
                Err(TokenExchangeError::ServiceUnavailable(status.as_u16()))
            }
            status => {
                let body = resp.text().await.unwrap_or_default();
                Err(TokenExchangeError::RequestFailed {
                    status: status.as_u16(),
                    body,
                })
            }
        }
    }

    pub async fn register_account(
        &self,
        body: RegisterAccountBody,
        gdl_token: String,
    ) -> anyhow::Result<GDLUser> {
        let url = format!("{}/v1/users/user", self.base_api);

        let authorization = format!("Bearer {}", gdl_token);

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

    pub async fn wait_for_account_validation(&self, gdl_token: String) -> anyhow::Result<()> {
        let url = format!("{}/v1/users/wait-for-user-verification", self.base_api);

        // Cloudflare's 524 status code is used to indicate that the request timed out
        let cloudflare_timeout_status =
            StatusCode::from_u16(524).expect("524 is a valid status code");

        loop {
            let resp = self
                .client
                .get(&url)
                .header("avoid-caching", "")
                .header("Authorization", format!("Bearer {}", gdl_token))
                .send()
                .await?;

            if resp.status() == cloudflare_timeout_status {
                tracing::warn!("Account validation timed out. Retrying...");
                continue;
            }

            resp.bytes().await?;

            return Ok(());
        }
    }

    pub async fn get_account(&self, gdl_token: String) -> Result<Option<GDLUser>, GetAccountError> {
        let url = format!("{}/v1/users/user", self.base_api);
        let authorization = format!("Bearer {}", gdl_token);
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            reqwest::header::HeaderValue::try_from(&authorization)
                .map_err(|e| anyhow::anyhow!("Invalid GDL authorization header: {e}"))?,
        );

        let resp = self.client.get(url).headers(headers).send().await?;

        if resp.status() == StatusCode::IM_A_TEAPOT {
            return Ok(None);
        }

        // Surfaced separately so callers can re-mint the token and retry: the
        // token may have been rejected for reasons no local `exp` check can
        // see, such as the backend rotating its JWT signing key.
        if resp.status() == StatusCode::UNAUTHORIZED {
            return Err(GetAccountError::Unauthorized);
        }

        let resp = resp.error_for_status()?;

        let user: GDLUser = resp.json().await?;

        Ok(Some(user))
    }

    pub async fn request_new_verification_token(
        &self,
        gdl_token: String,
    ) -> Result<(), RequestNewVerificationTokenError> {
        let url = format!("{}/v1/users/request-new-verification-token", self.base_api);
        let authorization = format!("Bearer {}", gdl_token);
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            reqwest::header::HeaderValue::try_from(&authorization).map_err(|e| {
                RequestNewVerificationTokenError::RequestFailed(anyhow::anyhow!(
                    "Invalid GDL authorization header: {e}"
                ))
            })?,
        );

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
        gdl_token: String,
        email: String,
    ) -> Result<(), RequestNewEmailChangeError> {
        let body = serde_json::to_string(&RequestEmailChangeBody { new_email: email })
            .map_err(|err| RequestNewEmailChangeError::RequestFailed(err.into()))?;

        let url = format!("{}/v1/users/request-email-change", self.base_api);
        let authorization = format!("Bearer {}", gdl_token);
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            reqwest::header::HeaderValue::try_from(&authorization).map_err(|e| {
                RequestNewEmailChangeError::RequestFailed(anyhow::anyhow!(
                    "Invalid GDL authorization header: {e}"
                ))
            })?,
        );
        headers.insert(
            CONTENT_TYPE,
            "application/json"
                .parse()
                .expect("failed to parse content type"),
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
        gdl_token: String,
    ) -> Result<(), RequestGDLAccountDeletionError> {
        let url = format!("{}/v1/users/user", self.base_api);

        let authorization = format!("Bearer {}", gdl_token);
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            reqwest::header::HeaderValue::try_from(&authorization).map_err(|e| {
                RequestGDLAccountDeletionError::RequestFailed(anyhow::anyhow!(
                    "Invalid GDL authorization header: {e}"
                ))
            })?,
        );

        let resp = self
            .client
            .delete(url)
            .headers(headers)
            .send()
            .await
            .map_err(|err| RequestGDLAccountDeletionError::RequestFailed(err.into()))?;

        let status = resp.status();

        if status == StatusCode::TOO_MANY_REQUESTS {
            let retry_after = resp
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u32>().ok());

            return Err(RequestGDLAccountDeletionError::TooManyRequests(
                retry_after.unwrap_or(0),
            ));
        }

        if !status.is_success() {
            // Try to parse error message from response body
            let body = resp.bytes().await.ok();
            if let Some(bytes) = body {
                if let Ok(error_resp) = serde_json::from_slice::<EnderiumErrorResponse>(&bytes) {
                    return Err(RequestGDLAccountDeletionError::ServerError(
                        error_resp.error.message,
                    ));
                }
            }
            return Err(RequestGDLAccountDeletionError::RequestFailed(
                anyhow::anyhow!("Request failed with status: {}", status),
            ));
        }

        Ok(())
    }

    pub async fn cancel_deletion(
        &self,
        gdl_token: String,
    ) -> Result<(), CancelGDLAccountDeletionError> {
        let url = format!("{}/v1/users/cancel-account-deletion", self.base_api);

        let authorization = format!("Bearer {}", gdl_token);
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            reqwest::header::HeaderValue::try_from(&authorization).map_err(|e| {
                CancelGDLAccountDeletionError::RequestFailed(anyhow::anyhow!(
                    "Invalid GDL authorization header: {e}"
                ))
            })?,
        );

        let resp = self
            .client
            .post(url)
            .headers(headers)
            .send()
            .await
            .map_err(|err| CancelGDLAccountDeletionError::RequestFailed(err.into()))?;

        let status = resp.status();

        if status.is_success() {
            return Ok(());
        }

        // Parse the typed error code from the body so the frontend can
        // distinguish "nothing to cancel" (info, auto-resync) from a
        // real server error.
        let body = resp.bytes().await.ok();
        if let Some(bytes) = body {
            if let Ok(error_resp) = serde_json::from_slice::<EnderiumErrorResponse>(&bytes) {
                if error_resp.error.code == "NO_SCHEDULED_DELETION" {
                    return Err(CancelGDLAccountDeletionError::NoScheduledDeletion);
                }
                return Err(CancelGDLAccountDeletionError::ServerError(
                    error_resp.error.message,
                ));
            }
        }

        Err(CancelGDLAccountDeletionError::RequestFailed(
            anyhow::anyhow!("Request failed with status: {}", status),
        ))
    }

    pub async fn change_display_name(
        &self,
        gdl_token: String,
        display_name: String,
    ) -> Result<(), ChangeDisplayNameError> {
        let url = format!("{}/v1/users/user/nickname", self.base_api);

        let authorization = format!("Bearer {}", gdl_token);
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            authorization
                .parse()
                .map_err(|e: InvalidHeaderValue| ChangeDisplayNameError::RequestFailed(e.into()))?,
        );
        headers.insert(
            CONTENT_TYPE,
            "application/json"
                .parse()
                .expect("failed to parse content type"),
        );

        let body = serde_json::to_string(&serde_json::json!({ "new_display_name": display_name }))
            .map_err(|e| ChangeDisplayNameError::RequestFailed(e.into()))?;

        let resp = self
            .client
            .put(url)
            .headers(headers)
            .body(reqwest::Body::from(body))
            .send()
            .await
            .map_err(|e| ChangeDisplayNameError::RequestFailed(e.into()))?;

        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            let retry_after = resp
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u32>().ok());

            return Err(ChangeDisplayNameError::TooManyRequests(
                retry_after.unwrap_or(0),
            ));
        }

        resp.error_for_status()
            .map_err(|e| ChangeDisplayNameError::RequestFailed(e.into()))?;

        Ok(())
    }

    pub async fn get_display_name_history(
        &self,
        friend_code: String,
    ) -> anyhow::Result<Vec<DisplayNameHistoryEntry>> {
        let url = format!(
            "{}/v1/users/users/{}/nickname-history",
            self.base_api, friend_code
        );

        let resp = self.client.get(url).send().await?;

        let resp = resp.error_for_status()?;

        let history: Vec<DisplayNameHistoryEntry> = resp.json().await?;

        Ok(history)
    }

    pub async fn clear_display_name_history(&self, gdl_token: String) -> anyhow::Result<()> {
        let url = format!("{}/v1/users/user/nickname-history", self.base_api);

        let authorization = format!("Bearer {}", gdl_token);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization.parse()?);

        let resp = self.client.delete(url).headers(headers).send().await?;

        resp.error_for_status()?;

        Ok(())
    }

    pub async fn upload_profile_icon(
        &self,
        gdl_token: String,
        icon_path: String,
    ) -> Result<(), AvatarUploadError> {
        // reqwest-middleware does not support multipart form data
        // so we need to use reqwest directly
        let client = reqwest::Client::new();

        let url = format!("{}/v1/users/user/avatar", self.base_api);

        let form = Form::new().file("avatar", icon_path).await.map_err(|e| {
            AvatarUploadError::Unknown(format!("failed to build multipart form: {}", e))
        })?;

        let authorization = format!("Bearer {}", gdl_token);

        let resp = client
            .put(url)
            .header(AUTHORIZATION, authorization)
            // Don't set Content-Type manually - reqwest sets it with the correct boundary
            .multipart(form)
            .send()
            .await?;

        let status = resp.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(AvatarUploadError::from_response(status, &body))
        }
    }

    pub async fn delete_profile_icon(&self, gdl_token: String) -> anyhow::Result<()> {
        let url = format!("{}/v1/users/user/avatar", self.base_api);

        let authorization = format!("Bearer {}", gdl_token);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization.parse()?);

        let resp = self.client.delete(url).headers(headers).send().await?;

        resp.error_for_status()?;

        Ok(())
    }

    pub async fn get_subscription_status(&self) {}

    /// Get presigned download URL for a share (no auth required)
    pub async fn get_presigned_download_url(
        &self,
        share_code: String,
    ) -> Result<String, InstanceShareError> {
        let url = format!("{}/v1/instance-share/presigned-download-url", self.base_api);

        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            "application/json"
                .parse()
                .expect("failed to parse content type"),
        );

        let body = serde_json::to_string(&GetPresignedDownloadUrlBody { share_code })?;

        let resp = self
            .client
            .post(url)
            .headers(headers)
            .body(reqwest::Body::from(body))
            .send()
            .await?;

        handle_instance_share_response_text(resp).await
    }

    pub async fn get_presigned_upload_url(
        &self,
        gdl_token: String,
        content_length: u64,
        sha256_checksum: String,
        title: Option<String>,
        expiration_days: Option<i32>,
        max_downloads: Option<i32>,
        metadata: ShareMetadata,
    ) -> Result<GetPresignedUploadUrlResponse, InstanceShareError> {
        let url = format!("{}/v1/instance-share/presigned-upload-url", self.base_api);

        let authorization = format!("Bearer {}", gdl_token);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization.parse()?);
        headers.insert(
            CONTENT_TYPE,
            "application/json"
                .parse()
                .expect("failed to parse content type"),
        );

        let body = serde_json::to_string(&GetPresignedUploadUrlBody {
            content_length,
            sha256_checksum,
            title,
            expiration_days,
            max_downloads,
            minecraft_version: metadata.minecraft_version,
            modloader_type: metadata.modloader_type,
            modloader_version: metadata.modloader_version,
            mods: metadata.mods,
        })?;

        let resp = self
            .client
            .post(url)
            .headers(headers)
            .body(reqwest::Body::from(body))
            .send()
            .await?;

        handle_instance_share_response(resp).await
    }

    pub async fn upload_share_instance(
        &self,
        presigned_url: String,
        file: tokio::fs::File,
        file_size: u64,
        sha256_checksum: String,
        progress_tx: tokio::sync::mpsc::Sender<i32>,
        cancel_token: CancellationToken,
    ) -> anyhow::Result<()> {
        // reqwest-middleware's retry layer clones requests, which fails for
        // streaming bodies (wrap_stream is not cloneable). The upload goes to
        // a presigned S3 URL, so no GDL middleware is needed — use reqwest
        // directly. Retrying a streaming upload automatically would be wrong
        // regardless (stream is single-use and progress would reset).
        let client = reqwest::Client::new();

        let mut reader_stream = tokio_util::io::ReaderStream::new(file);
        let mut uploaded = 0u64;
        let tx_clone = progress_tx.clone();

        let async_stream = async_stream::stream! {
            while let Some(chunk) = reader_stream.next().await {
                if cancel_token.is_cancelled() {
                    tracing::info!("ShareInstance: cancelled during phase 4 (upload), {}% uploaded", ((uploaded as f64 / file_size as f64) * 100.0) as i32);
                    yield Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "Upload cancelled"));
                    return;
                }
                if let Ok(chunk) = &chunk {
                    uploaded += chunk.len() as u64;
                    let progress = ((uploaded as f64 / file_size as f64) * 100.0) as i32;
                    let _ = tx_clone.send(progress).await;
                }
                yield chunk;
            }
        };

        let resp = client
            .put(presigned_url)
            .header("Content-Type", "application/vnd.gdlauncher.gdlpack")
            .header("Content-Length", file_size)
            .header("x-amz-checksum-sha256", sha256_checksum)
            .header("x-amz-sdk-checksum-algorithm", "SHA256")
            .body(reqwest::Body::wrap_stream(async_stream))
            .send()
            .await?;

        resp.error_for_status()?;

        // Send 100% completion
        let _ = progress_tx.send(100).await;

        Ok(())
    }

    pub async fn wait_for_share_instance(
        &self,
        file_key: String,
        gdl_token: String,
    ) -> Result<WaitForShareInstanceResponse, InstanceShareError> {
        let url = format!(
            "{}/v1/instance-share/wait-for-upload-complete",
            self.base_api
        );

        let authorization = format!("Bearer {}", gdl_token);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization.parse()?);
        headers.insert(
            CONTENT_TYPE,
            "application/json"
                .parse()
                .expect("failed to parse content type"),
        );

        let body = serde_json::to_string(&RequestShareInstanceBody { file_key })?;

        let resp = self
            .client
            .post(url)
            .body(reqwest::Body::from(body))
            .headers(headers)
            .send()
            .await?;

        handle_instance_share_response(resp).await
    }

    /// Get paginated list of user's shares
    pub async fn get_user_shares(
        &self,
        gdl_token: String,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<PaginatedShares, InstanceShareError> {
        let mut url = format!("{}/v1/instance-share/my-shares", self.base_api);

        // Add query params
        let mut params = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let authorization = format!("Bearer {}", gdl_token);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization.parse()?);

        let resp = self.client.get(url).headers(headers).send().await?;

        handle_instance_share_response(resp).await
    }

    /// Delete a share by its code
    pub async fn delete_share(
        &self,
        gdl_token: String,
        share_code: String,
    ) -> Result<(), InstanceShareError> {
        let url = format!("{}/v1/instance-share/share/{}", self.base_api, share_code);

        let authorization = format!("Bearer {}", gdl_token);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization.parse()?);

        let resp = self.client.delete(url).headers(headers).send().await?;

        handle_instance_share_response_empty(resp).await
    }

    pub async fn get_quota(&self, gdl_token: String) -> Result<QuotaInfo, InstanceShareError> {
        let url = format!("{}/v1/instance-share/quota", self.base_api);

        let authorization = format!("Bearer {}", gdl_token);

        let resp = self
            .client
            .get(url)
            .header("Authorization", authorization)
            .send()
            .await?;

        handle_instance_share_response(resp).await
    }

    /// Update a share's metadata (title and/or max_downloads)
    pub async fn update_share(
        &self,
        gdl_token: String,
        share_code: String,
        body: UpdateShareBody,
    ) -> Result<(), InstanceShareError> {
        let url = format!("{}/v1/instance-share/share/{}", self.base_api, share_code);

        let authorization = format!("Bearer {}", gdl_token);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization.parse()?);
        headers.insert(
            CONTENT_TYPE,
            "application/json"
                .parse()
                .expect("failed to parse content type"),
        );

        let body_str = serde_json::to_string(&body)?;

        let resp = self
            .client
            .patch(url)
            .headers(headers)
            .body(reqwest::Body::from(body_str))
            .send()
            .await?;

        handle_instance_share_response_empty(resp).await
    }

    /// Regenerate a share code (invalidates the old code)
    pub async fn regenerate_share_code(
        &self,
        gdl_token: String,
        share_code: String,
    ) -> Result<RegenerateShareCodeResponse, InstanceShareError> {
        let url = format!(
            "{}/v1/instance-share/share/{}/regenerate",
            self.base_api, share_code
        );

        let authorization = format!("Bearer {}", gdl_token);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization.parse()?);

        let resp = self.client.post(url).headers(headers).send().await?;

        handle_instance_share_response(resp).await
    }

    /// Report a share. `report_type` is one of "share_background",
    /// "share_title", "share_content".
    pub async fn report_share(
        &self,
        gdl_token: String,
        share_code: String,
        body: ReportShareBody,
    ) -> Result<(), InstanceShareError> {
        let url = format!(
            "{}/v1/instance-share/share/{}/report",
            self.base_api, share_code
        );

        let authorization = format!("Bearer {}", gdl_token);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization.parse()?);
        headers.insert(
            CONTENT_TYPE,
            "application/json"
                .parse()
                .expect("failed to parse content type"),
        );

        let body_str = serde_json::to_string(&body)?;

        let resp = self
            .client
            .post(url)
            .headers(headers)
            .body(reqwest::Body::from(body_str))
            .send()
            .await?;

        handle_instance_share_response_empty(resp).await
    }

    /// Validate if a share code exists and is not expired (no auth required)
    pub async fn validate_share_code(
        &self,
        share_code: String,
    ) -> Result<bool, InstanceShareError> {
        let url = format!(
            "{}/v1/instance-share/share/{}/validate",
            self.base_api, share_code
        );

        let resp = self.client.get(url).send().await?;

        match resp.status() {
            StatusCode::NO_CONTENT => Ok(true),
            StatusCode::NOT_FOUND => Ok(false),
            status => {
                let body = resp.text().await.unwrap_or_default();
                Err(InstanceShareError::from_response(status, &body))
            }
        }
    }

    /// Get share preview (no auth required)
    pub async fn get_share_preview(
        &self,
        share_code: String,
    ) -> Result<SharePreview, InstanceShareError> {
        let url = format!(
            "{}/v1/instance-share/share/{}/preview",
            self.base_api, share_code
        );

        let resp = self.client.get(url).send().await?;

        handle_instance_share_response(resp).await
    }

    /// Upload a background image for a share
    /// Returns the background URL on success
    pub async fn upload_share_background(
        &self,
        gdl_token: String,
        share_code: String,
        image_data: Vec<u8>,
    ) -> Result<String, InstanceShareError> {
        // reqwest-middleware does not support multipart form data
        // so we need to use reqwest directly
        let client = reqwest::Client::new();

        let url = format!(
            "{}/v1/instance-share/share/{}/background",
            self.base_api, share_code
        );

        let part = reqwest::multipart::Part::bytes(image_data)
            .file_name("background")
            .mime_str("application/octet-stream")
            .map_err(|e| InstanceShareError::Unknown(e.to_string()))?;

        let form = Form::new().part("file", part);

        let authorization = format!("Bearer {}", gdl_token);

        let resp = client
            .post(url)
            .header(AUTHORIZATION, authorization)
            .multipart(form)
            .send()
            .await?;

        // Response contains { background_url: "https://..." }
        #[derive(Deserialize)]
        struct BackgroundResponse {
            background_url: String,
        }

        let result: BackgroundResponse = handle_instance_share_response(resp).await?;
        Ok(result.background_url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bodyless_408_maps_to_upload_timeout() {
        // Infrastructure timeouts (a proxy or tower TimeoutLayer cutting a
        // long poll) reply 408 with an empty body instead of enderium's error
        // JSON. They must map to the typed timeout so callers re-poll instead
        // of surfacing an unknown error.
        let err = InstanceShareError::from_response(StatusCode::REQUEST_TIMEOUT, "");
        assert!(
            matches!(err, InstanceShareError::UploadTimeout),
            "got {err:?}"
        );
    }

    #[test]
    fn typed_upload_timeout_body_maps_to_upload_timeout() {
        let body = r#"{"error":{"code":"UPLOAD_TIMEOUT","message":"Upload did not complete in time"}}"#;
        let err = InstanceShareError::from_response(StatusCode::REQUEST_TIMEOUT, body);
        assert!(
            matches!(err, InstanceShareError::UploadTimeout),
            "got {err:?}"
        );
    }

    #[test]
    fn unparseable_non_timeout_body_stays_unknown() {
        let err = InstanceShareError::from_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "<html>bad gateway</html>",
        );
        assert!(matches!(err, InstanceShareError::Unknown(_)), "got {err:?}");
    }
}
