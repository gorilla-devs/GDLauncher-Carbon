use crate::api::keys::settings::GET_SETTINGS;
use crate::api::{CoreModuleStatus, update_core_module_status};
use crate::domain::account::*;
use crate::{
    api::keys::account::*,
    managers::account::{api::GetProfileError, enroll::InvalidateCtx},
};
use anyhow::{Context, ensure};
use anyhow::{anyhow, bail};
use async_trait::async_trait;
use axum::extract;
use carbon_repos::db_error::DbError;
use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::account as account_repo;
use chrono::{DateTime, FixedOffset, Utc};
use gdl_account::{
    CancelGDLAccountDeletionError, ChangeDisplayNameError, DisplayNameHistoryEntry,
    GDLAccountStatus, GDLAccountTask, GDLUser, GetAccountError, GetPresignedUploadUrlResponse,
    PaginatedShares, QuotaInfo, RegisterAccountBody, RequestGDLAccountDeletionError,
    RequestNewEmailChangeError, RequestNewVerificationTokenError, ShareInfo, ShareMetadata,
    SharePreview, WaitForShareInstanceResponse,
};
use jwt::{Header, Token};
use reqwest::Client;
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::{
    collections::{BTreeMap, HashMap},
    mem,
    sync::{Arc, Weak},
    time::{Duration, Instant},
};
use thiserror::Error;
use tokio::fs::File;
use tokio::select;
use tokio::sync::watch::Sender;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, trace, warn};

pub use self::enroll::{EnrollmentError, EnrollmentStatus};
use self::{enroll::EnrollmentTask, skin::SkinManager};

use super::{AppInner, AppRef, ManagerRef};

pub mod api;
mod enroll;
pub mod gdl_account;
mod oauth_server;
pub mod protocol_handler;
pub mod skin;

/// Initial backoff duration for GDL token exchange retries.
const GDL_TOKEN_INITIAL_BACKOFF: Duration = Duration::from_secs(30);
/// Maximum backoff duration for GDL token exchange retries.
const GDL_TOKEN_MAX_BACKOFF: Duration = Duration::from_secs(300);
/// How long before a JWT's `exp` to stop treating it as usable. Covers the
/// request's own flight time plus modest client clock skew.
const TOKEN_EXPIRY_SKEW: Duration = Duration::from_secs(60);
/// How long to wait on a refresh started elsewhere before giving up. Generous:
/// a full refresh is several sequential network round-trips (MS, Xbox, MC).
const REFRESH_WAIT_TIMEOUT: Duration = Duration::from_secs(60);
const REFRESH_WAIT_POLL_INTERVAL: Duration = Duration::from_millis(250);

pub(crate) struct AccountManager {
    currently_refreshing: RwLock<HashMap<String, EnrollmentTask>>,
    active_enrollment: RwLock<Option<EnrollmentTask>>,
    /// Account refreshing will be disabled until this time has passed
    refreshloop_sleep: Mutex<Option<Instant>>,
    skin_manager: SkinManager,

    gdl_account_task: GDLAccountTask,
    /// Protocol handler for gdlauncher:// OAuth callbacks
    protocol_handler: protocol_handler::ProtocolHandler,
    /// Per-account backoff state for GDL token exchange retries.
    /// Key: account UUID, Value: (next_retry_at, current_backoff_duration)
    gdl_token_exchange_backoff: Mutex<HashMap<String, (Instant, Duration)>>,
}

impl AccountManager {
    pub fn new(client: reqwest_middleware::ClientWithMiddleware, gdl_base_api: String) -> Self {
        Self {
            currently_refreshing: RwLock::new(HashMap::new()),
            active_enrollment: RwLock::new(None),
            refreshloop_sleep: Mutex::new(None),
            skin_manager: SkinManager {},

            gdl_account_task: GDLAccountTask::new(client, gdl_base_api),
            protocol_handler: protocol_handler::ProtocolHandler::new(),
            gdl_token_exchange_backoff: Mutex::new(HashMap::new()),
        }
    }
}

impl<'s> ManagerRef<'s, AccountManager> {
    pub async fn get_active_uuid(self) -> anyhow::Result<Option<String>> {
        Ok(self
            .app
            .settings_manager()
            .get_settings()
            .await?
            .active_account_uuid)
    }

    pub async fn set_active_uuid(self, uuid: Option<String>) -> anyhow::Result<()> {
        use carbon_repos::repos::app_configuration::AppConfigurationPatch;

        if let Some(uuid) = uuid.clone() {
            let account_entry = self
                .app
                .db
                .read({
                    let uuid = uuid.clone();
                    move |conn| Ok(account_repo::get_account(conn, &uuid)?)
                })
                .await?;

            // Setting the active account to one not in the DB does not make sense.
            ensure!(
                account_entry.is_some(),
                SetActiveUuidError::AccountDoesNotExist(uuid)
            );
        }

        info!("Set active account to {uuid:?}");

        self.app
            .settings_manager()
            .set(AppConfigurationPatch {
                active_account_uuid: Some(uuid),
                ..Default::default()
            })
            .await?;

        self.app.invalidate(GET_ACTIVE_UUID, None);
        Ok(())
    }

    /// Exchange a Microsoft id_token for a GDL JWT and store it in the database.
    ///
    /// This should be called after enrollment or refresh completes.
    /// If the exchange fails (e.g., network error), it logs a warning but doesn't fail.
    /// The GDL token can be exchanged again later.
    pub async fn exchange_gdl_token(self, uuid: &str) -> anyhow::Result<String> {
        // Get the account's MS id_token
        let account = self
            .app
            .db
            .read({
                let uuid = uuid.to_string();
                move |conn| Ok(account_repo::get_account(conn, &uuid)?)
            })
            .await?
            .ok_or_else(|| anyhow!("Account not found: {}", uuid))?;

        let Some(id_token) = account.id_token else {
            bail!(
                "No MS id_token for account {}, cannot exchange GDL token",
                uuid
            );
        };

        // Exchange for GDL token
        let response = self
            .gdl_account_task
            .exchange_token(&id_token)
            .await
            .with_context(|| format!("GDL token exchange failed for account {uuid}"))?;

        info!("Successfully exchanged GDL token for account {}", uuid);

        let token = response.access_token.clone();

        // Store the GDL token in the database
        self.app
            .db
            .write({
                let uuid = uuid.to_string();
                move |conn| {
                    Ok(account_repo::set_account_gdl_token(
                        conn,
                        &uuid,
                        Some(&response.access_token),
                    )?)
                }
            })
            .await?;

        self.app
            .invalidate(GET_ACCOUNTS, Some(uuid.to_string().into()));

        Ok(token)
    }

    /// Ensure we have a *usable* GDL auth token for the given account.
    ///
    /// The backend mints GDL tokens with the same `exp` as the Microsoft
    /// id_token they were exchanged from (~1h), so a stored token goes stale
    /// long before anything else about the account does. Presence is therefore
    /// not evidence of validity — every path here re-checks `exp`.
    ///
    /// Fast path: the stored token is still within its lifetime.
    /// Slow path: refresh the Microsoft tokens if the id_token is itself dead
    /// (the exchange endpoint would only answer 401 otherwise), then exchange.
    ///
    /// `force` skips the fast path, for when the backend has rejected a token
    /// that still looked locally valid.
    async fn ensure_gdl_auth_token(
        self,
        account: &account_repo::AccountRow,
        force: bool,
    ) -> anyhow::Result<String> {
        let uuid = &account.uuid;

        // Fast path — stored token still within its lifetime
        if !force {
            if let Some(token) = &account.gdl_token {
                if !is_jwt_expired(token) {
                    return Ok(token.clone());
                }
                debug!("Stored GDL token for {uuid} has expired, re-exchanging");
            }
        }

        // Check backoff
        {
            let backoff_map = self.manager.gdl_token_exchange_backoff.lock().await;
            if let Some((next_retry_at, _)) = backoff_map.get(uuid.as_str()) {
                let now = Instant::now();
                if now < *next_retry_at {
                    let remaining = next_retry_at.duration_since(now);
                    bail!(
                        "GDL token exchange rate-limited for account {uuid}, next retry in {}s",
                        remaining.as_secs()
                    );
                }
            }
        }

        // An expired id_token can only ever yield a 401 from the exchange
        // endpoint. Refreshing the Microsoft account re-mints the id_token and,
        // through the enrollment task, stores a freshly exchanged GDL token.
        let id_token_usable = account
            .id_token
            .as_deref()
            .is_some_and(|token| !is_jwt_expired(token));

        if !id_token_usable {
            debug!("MS id_token for {uuid} is missing or expired, refreshing Microsoft account");

            self.refresh_ms_tokens(uuid)
                .await
                .with_context(|| format!("cannot mint a GDL token for account {uuid}"))?;

            // The refresh stores a token exchanged from the new id_token; take
            // it rather than burning a second exchange on the same id_token.
            if let Some(token) = self
                .get_account_entries()
                .await?
                .into_iter()
                .find(|entry| &entry.uuid == uuid)
                .and_then(|entry| entry.gdl_token)
                .filter(|token| !is_jwt_expired(token))
            {
                self.manager
                    .gdl_token_exchange_backoff
                    .lock()
                    .await
                    .remove(uuid.as_str());

                return Ok(token);
            }
        }

        // Attempt exchange
        match self.exchange_gdl_token(uuid).await {
            Ok(token) => {
                // Clear backoff on success
                self.manager
                    .gdl_token_exchange_backoff
                    .lock()
                    .await
                    .remove(uuid.as_str());
                Ok(token)
            }
            Err(e) => {
                // Set/increase backoff
                let mut backoff_map = self.manager.gdl_token_exchange_backoff.lock().await;
                let new_backoff = match backoff_map.get(uuid.as_str()) {
                    Some((_, current)) => (*current * 2).min(GDL_TOKEN_MAX_BACKOFF),
                    None => GDL_TOKEN_INITIAL_BACKOFF,
                };
                backoff_map.insert(uuid.clone(), (Instant::now() + new_backoff, new_backoff));
                warn!(
                    "GDL token exchange failed for account {uuid}, backing off for {}s: {e}",
                    new_backoff.as_secs()
                );
                Err(e)
            }
        }
    }

    /// The GDL account email for `uuid`, or `None` if it can't be resolved.
    /// Best-effort: this only feeds ad personalization, so a failure here must
    /// never fail the caller.
    async fn gdl_account_email(self, uuid: &str) -> Option<String> {
        let account = self
            .get_account_entries()
            .await
            .ok()?
            .into_iter()
            .find(|account| account.uuid == uuid)?;

        match self.fetch_gdl_user(&account).await {
            Ok(user) => user.map(|user| user.email),
            Err(e) => {
                warn!("Could not resolve GDL account email for {uuid}: {e}");
                None
            }
        }
    }

    /// Fetch the GDL profile for an account, re-minting the token once if the
    /// backend rejects it.
    ///
    /// The local `exp` check in [`Self::ensure_gdl_auth_token`] cannot see
    /// server-side invalidation (a rotated signing key, a revoked token), so a
    /// 401 here is treated as "this token is dead" rather than surfaced to the
    /// user — otherwise the stored token wedges every future call and only
    /// deleting the database clears it.
    async fn fetch_gdl_user(
        self,
        account: &account_repo::AccountRow,
    ) -> anyhow::Result<Option<GDLUser>> {
        let token = self.ensure_gdl_auth_token(account, false).await?;

        match self.gdl_account_task.get_account(token).await {
            Err(GetAccountError::Unauthorized) => {
                warn!(
                    "Server rejected the GDL token for {}, re-exchanging and retrying",
                    account.uuid
                );

                // Re-read first: the call above may have refreshed the account,
                // and deciding off the stale snapshot would repeat that work.
                let account = self
                    .app
                    .db
                    .read({
                        let uuid = account.uuid.clone();
                        move |conn| Ok(account_repo::get_account(conn, &uuid)?)
                    })
                    .await?
                    .ok_or_else(|| anyhow!("Account not found: {}", account.uuid))?;

                let token = self.ensure_gdl_auth_token(&account, true).await?;

                Ok(self.gdl_account_task.get_account(token).await?)
            }
            other => Ok(other?),
        }
    }

    /// Drive a Microsoft token refresh to completion so the account ends up
    /// with a fresh `id_token` (and the `gdl_token` exchanged from it).
    ///
    /// The periodic refresh loop only fires within 12h of the *access* token
    /// expiring, which says nothing about the ~1h id_token, so GDL callers that
    /// need a live token have to ask for one directly.
    async fn refresh_ms_tokens(self, uuid: &str) -> anyhow::Result<()> {
        match self.refresh_account(uuid.to_string()).await {
            Ok(handler) => {
                handler.await??;
                Ok(())
            }
            Err(e) => {
                // Someone else is already refreshing this account; wait for
                // their result instead of racing a second enrollment.
                if let Some(RefreshAccountError::AlreadyRefreshing) = e.downcast_ref() {
                    return self.wait_for_refresh(uuid).await;
                }

                Err(e)
            }
        }
    }

    /// Wait for an in-flight refresh of `uuid` to drop out of the refresh map.
    async fn wait_for_refresh(self, uuid: &str) -> anyhow::Result<()> {
        let deadline = Instant::now() + REFRESH_WAIT_TIMEOUT;

        loop {
            if !self.currently_refreshing.read().await.contains_key(uuid) {
                return Ok(());
            }

            if Instant::now() >= deadline {
                bail!("timed out waiting for an in-flight refresh of account {uuid}");
            }

            tokio::time::sleep(REFRESH_WAIT_POLL_INTERVAL).await;
        }
    }

    /// Re-exchange GDL tokens for all Microsoft accounts on startup.
    /// This ensures tokens are always valid even if the backend target changed
    /// (e.g. switching between prod and dev where JWT signing keys differ).
    ///
    /// Accounts whose stored id_token has expired are skipped: the exchange
    /// endpoint would reject it, and the refresh loop re-mints both tokens
    /// anyway. GDL callers recover on demand via `ensure_gdl_auth_token`.
    pub async fn refresh_all_gdl_tokens(self) -> anyhow::Result<()> {
        let accounts = self.get_account_entries().await?;

        let is_eligible = |account: &account_repo::AccountRow| {
            account
                .id_token
                .as_deref()
                .is_some_and(|token| !is_jwt_expired(token))
        };

        let eligible = accounts.iter().filter(|a| is_eligible(a)).count();
        debug!(
            "[startup-timing] refresh_all_gdl_tokens: {} eligible account(s) of {} total",
            eligible,
            accounts.len()
        );

        for account in accounts {
            if is_eligible(&account) {
                let t = std::time::Instant::now();
                info!("Refreshing GDL token for account {}", account.uuid);
                let result = self.exchange_gdl_token(&account.uuid).await;
                let elapsed = t.elapsed().as_secs_f64();
                match result {
                    Ok(_) => debug!(
                        "[startup-timing] exchange_gdl_token({}) ok in {:.2}s",
                        account.uuid, elapsed
                    ),
                    Err(e) => warn!(
                        "[startup-timing] exchange_gdl_token({}) failed in {:.2}s: {}",
                        account.uuid, elapsed, e
                    ),
                }
            }
        }

        Ok(())
    }

    /// Get the active account's details.
    ///
    /// Not exposed to the frontend on purpose. Will NOT be invalidated.
    pub async fn get_active_account(&self) -> anyhow::Result<Option<FullAccount>> {
        let Some(uuid) = self.get_active_uuid().await? else {
            return Ok(None);
        };

        let account = self
            .app
            .db
            .read(move |conn| Ok(account_repo::get_account(conn, &uuid)?))
            .await?
            .ok_or_else(|| anyhow!("currenly active account could not be read from database"))?;

        Ok(Some(account.try_into()?))
    }

    async fn get_account_entries(self) -> anyhow::Result<Vec<account_repo::AccountRow>> {
        Ok(self
            .app
            .db
            .read(|conn| Ok(account_repo::get_accounts_by_last_used(conn)?))
            .await?)
    }

    pub async fn get_account_list(self) -> anyhow::Result<Vec<AccountWithStatus>> {
        let accounts = self.get_account_entries().await?;

        Ok(accounts
            .into_iter()
            .map(|account| {
                let account = FullAccount::try_from(account).unwrap();
                let account = AccountWithStatus::from(account);

                account
            })
            .collect())
    }

    async fn get_account(self, uuid: String) -> anyhow::Result<Option<AccountWithStatus>> {
        let account = self
            .app
            .db
            .read(move |conn| Ok(account_repo::get_account(conn, &uuid)?))
            .await?;

        let Some(account) = account else {
            return Ok(None);
        };
        let account = FullAccount::try_from(account)?;
        let account = AccountWithStatus::from(account);

        Ok(Some(account))
    }

    pub async fn get_account_status(self, uuid: String) -> anyhow::Result<Option<AccountStatus>> {
        let Some(mut account) = self.get_account(uuid).await? else {
            return Ok(None);
        };

        if let AccountType::Microsoft { .. } = &account.account.type_ {
            let refreshing = self
                .currently_refreshing
                .read()
                .await
                .contains_key(&account.account.uuid);

            if refreshing {
                account.status = AccountStatus::Refreshing;
            }
        }

        Ok(Some(account.status))
    }

    pub async fn wait_for_account_verification(self, uuid: String) -> anyhow::Result<()> {
        let account = self
            .get_account_entries()
            .await?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(anyhow::anyhow!(
                "attempted to get an account that does not exist"
            ))?;

        let auth_token = self.ensure_gdl_auth_token(&account, false).await?;

        info!("Waiting for account validation");

        self.gdl_account_task
            .wait_for_account_validation(auth_token)
            .await
    }

    pub async fn peek_gdl_account(self, uuid: String) -> anyhow::Result<Option<GDLUser>> {
        let account = self
            .get_account_entries()
            .await?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(anyhow::anyhow!(
                "attempted to get an account that does not exist"
            ))?;

        self.fetch_gdl_user(&account).await
    }

    pub async fn request_gdl_account_deletion(
        self,
        uuid: String,
    ) -> Result<(), RequestGDLAccountDeletionError> {
        let account = self
            .get_account_entries()
            .await
            .map_err(|e| RequestGDLAccountDeletionError::RequestFailed(e))?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(RequestGDLAccountDeletionError::RequestFailed(
                anyhow::anyhow!(
                    "attempted to request a gdl account deletion for an account that does not exist"
                ),
            ))?;

        let auth_token = self
            .ensure_gdl_auth_token(&account, false)
            .await
            .map_err(RequestGDLAccountDeletionError::RequestFailed)?;

        let deletion = self.gdl_account_task.request_deletion(auth_token).await;

        self.app
            .invalidate(PEEK_GDL_ACCOUNT, Some(uuid.clone().into()));
        self.app.invalidate(GET_GDL_ACCOUNT, None);

        deletion?;

        Ok(())
    }

    pub async fn cancel_gdl_account_deletion(
        self,
        uuid: String,
    ) -> Result<(), CancelGDLAccountDeletionError> {
        let account = self
            .get_account_entries()
            .await
            .map_err(CancelGDLAccountDeletionError::RequestFailed)?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(CancelGDLAccountDeletionError::RequestFailed(
                anyhow::anyhow!(
                    "attempted to cancel a gdl account deletion for an account that does not exist"
                ),
            ))?;

        let auth_token = self
            .ensure_gdl_auth_token(&account, false)
            .await
            .map_err(CancelGDLAccountDeletionError::RequestFailed)?;

        let result = self.gdl_account_task.cancel_deletion(auth_token).await;

        // Invalidate regardless of outcome: success = row cleared,
        // `NoScheduledDeletion` = our local copy is stale, server error
        // = a refetch can't hurt.
        self.app
            .invalidate(PEEK_GDL_ACCOUNT, Some(uuid.clone().into()));
        self.app.invalidate(GET_GDL_ACCOUNT, None);

        result?;

        Ok(())
    }

    pub async fn register_gdl_account(
        self,
        uuid: String,
        body: RegisterAccountBody,
    ) -> anyhow::Result<GDLUser> {
        let account = self
            .get_account_entries()
            .await?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(anyhow::anyhow!(
                "attempted to get an account that does not exist"
            ))?;

        let auth_token = self.ensure_gdl_auth_token(&account, false).await?;

        let user = self
            .gdl_account_task
            .register_account(body, auth_token)
            .await
            .with_context(|| format!("failed to register account: {uuid}"))?;

        self.app
            .invalidate(PEEK_GDL_ACCOUNT, Some(uuid.clone().into()));
        self.app.invalidate(GET_GDL_ACCOUNT, None);

        Ok(user)
    }

    pub async fn save_gdl_account(&self, uuid: Option<String>) -> anyhow::Result<()> {
        use carbon_repos::repos::app_configuration::AppConfigurationPatch;

        self.app
            .settings_manager()
            .set(AppConfigurationPatch {
                gdl_account_uuid: Some(uuid.clone()),
                ..Default::default()
            })
            .await?;

        self.app.invalidate(GET_GDL_ACCOUNT, None);
        self.app.invalidate(GET_SETTINGS, None);

        // Notify Electron main process of GDL account email change for Overwolf ad personalization
        let email = match uuid {
            Some(account_uuid) => self.gdl_account_email(&account_uuid).await,
            None => None,
        };

        println!("_GDL_ACCOUNT_EMAIL_:{}", email.unwrap_or_default());

        // TODO!: Should get status from the API

        Ok(())
    }

    pub async fn get_gdl_account(self) -> anyhow::Result<GDLAccountStatus> {
        let saved_gdl_account_uuid = self
            .app
            .settings_manager()
            .get_settings()
            .await?
            .gdl_account_uuid;

        let Some(saved_gdl_account_uuid) = saved_gdl_account_uuid else {
            return Ok(GDLAccountStatus::Unset);
        };

        if saved_gdl_account_uuid.is_empty() {
            return Ok(GDLAccountStatus::Skipped);
        }

        let account = self
            .get_account_entries()
            .await?
            .into_iter()
            .find(|account| account.uuid == saved_gdl_account_uuid)
            .ok_or(anyhow::anyhow!(
                "attempted to get a gdl account that does not exist"
            ))?;

        let Some(user) = self.fetch_gdl_user(&account).await? else {
            return Ok(GDLAccountStatus::Invalid);
        };

        Ok(GDLAccountStatus::Valid(user))
    }

    /// Get presigned download URL for a share (no auth required)
    pub async fn get_presigned_download_url(self, share_code: String) -> anyhow::Result<String> {
        self.gdl_account_task
            .get_presigned_download_url(share_code)
            .await
            .map_err(Into::into)
    }

    pub async fn get_presigned_upload_url(
        self,
        uuid: String,
        content_length: u64,
        sha256_checksum: String,
        title: Option<String>,
        expiration_days: Option<i32>,
        max_downloads: Option<i32>,
        metadata: ShareMetadata,
    ) -> anyhow::Result<GetPresignedUploadUrlResponse> {
        let account = self
            .get_account_entries()
            .await?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(anyhow::anyhow!(
                "attempted to get a presigned url for an account that does not exist"
            ))?;

        let auth_token = self.ensure_gdl_auth_token(&account, false).await?;

        self.gdl_account_task
            .get_presigned_upload_url(
                auth_token,
                content_length,
                sha256_checksum,
                title,
                expiration_days,
                max_downloads,
                metadata,
            )
            .await
            .map_err(Into::into)
    }

    pub async fn upload_share_instance(
        self,
        uuid: String,
        presigned_url: String,
        file: File,
        file_size: u64,
        sha256_checksum: String,
        progress_tx: tokio::sync::mpsc::Sender<i32>,
        cancel_token: tokio_util::sync::CancellationToken,
    ) -> anyhow::Result<()> {
        let account = self
            .get_account_entries()
            .await?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(anyhow::anyhow!(
                "attempted to upload a share instance for an account that does not exist"
            ))?;

        let _auth_token = self.ensure_gdl_auth_token(&account, false).await?;

        self.gdl_account_task
            .upload_share_instance(
                presigned_url,
                file,
                file_size,
                sha256_checksum,
                progress_tx,
                cancel_token,
            )
            .await
    }

    pub async fn wait_for_share_instance(
        self,
        uuid: String,
        file_key: String,
    ) -> anyhow::Result<WaitForShareInstanceResponse> {
        let account = self
            .get_account_entries()
            .await?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(anyhow::anyhow!(
                "attempted to upload a share instance for an account that does not exist"
            ))?;

        let auth_token = self.ensure_gdl_auth_token(&account, false).await?;

        self.gdl_account_task
            .wait_for_share_instance(file_key, auth_token)
            .await
            .map_err(Into::into)
    }

    /// Get paginated list of user's shares
    pub async fn get_user_shares(
        self,
        uuid: String,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> anyhow::Result<PaginatedShares> {
        let account = self
            .get_account_entries()
            .await?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(anyhow::anyhow!(
                "attempted to get shares for an account that does not exist"
            ))?;

        let auth_token = self.ensure_gdl_auth_token(&account, false).await?;

        self.gdl_account_task
            .get_user_shares(auth_token, limit, offset)
            .await
            .map_err(Into::into)
    }

    /// Delete a share by its code
    pub async fn delete_share(self, uuid: String, share_code: String) -> anyhow::Result<()> {
        let account = self
            .get_account_entries()
            .await?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(anyhow::anyhow!(
                "attempted to delete a share for an account that does not exist"
            ))?;

        let auth_token = self.ensure_gdl_auth_token(&account, false).await?;

        self.gdl_account_task
            .delete_share(auth_token, share_code)
            .await
            .map_err(Into::into)
    }

    /// Update a share's metadata (title and/or max_downloads)
    pub async fn update_share(
        self,
        uuid: String,
        share_code: String,
        title: Option<String>,
        max_downloads: Option<Option<i32>>,
    ) -> anyhow::Result<()> {
        let account = self
            .get_account_entries()
            .await?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(anyhow::anyhow!(
                "attempted to update a share for an account that does not exist"
            ))?;

        let auth_token = self.ensure_gdl_auth_token(&account, false).await?;

        self.gdl_account_task
            .update_share(
                auth_token,
                share_code,
                gdl_account::UpdateShareBody {
                    title,
                    max_downloads,
                },
            )
            .await
            .map_err(Into::into)
    }

    /// Report a share. Forwards to enderium's
    /// `POST /v1/instance-share/share/{code}/report`.
    pub async fn report_share(
        self,
        uuid: String,
        share_code: String,
        report_type: String,
        reason: Option<String>,
    ) -> anyhow::Result<()> {
        let account = self
            .get_account_entries()
            .await?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(anyhow::anyhow!(
                "attempted to report a share for an account that does not exist"
            ))?;

        let auth_token = self.ensure_gdl_auth_token(&account, false).await?;

        self.gdl_account_task
            .report_share(
                auth_token,
                share_code,
                gdl_account::ReportShareBody {
                    report_type,
                    reason,
                },
            )
            .await
            .map_err(Into::into)
    }

    /// Regenerate a share code (invalidates the old code)
    pub async fn regenerate_share_code(
        self,
        uuid: String,
        share_code: String,
    ) -> anyhow::Result<gdl_account::RegenerateShareCodeResponse> {
        let account = self
            .get_account_entries()
            .await?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(anyhow::anyhow!(
                "attempted to regenerate a share code for an account that does not exist"
            ))?;

        let auth_token = self.ensure_gdl_auth_token(&account, false).await?;

        self.gdl_account_task
            .regenerate_share_code(auth_token, share_code)
            .await
            .map_err(Into::into)
    }

    /// Get user's quota information for instance sharing
    pub async fn get_quota(self, uuid: String) -> anyhow::Result<QuotaInfo> {
        let account = self
            .get_account_entries()
            .await?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(anyhow::anyhow!(
                "attempted to get quota for an account that does not exist"
            ))?;

        let auth_token = self.ensure_gdl_auth_token(&account, false).await?;

        self.gdl_account_task
            .get_quota(auth_token)
            .await
            .map_err(Into::into)
    }

    /// Validate if a share code exists and is not expired (no auth required)
    pub async fn validate_share_code(self, share_code: String) -> anyhow::Result<bool> {
        self.gdl_account_task
            .validate_share_code(share_code)
            .await
            .map_err(Into::into)
    }

    /// Get share preview info (no auth required)
    pub async fn get_share_preview(self, share_code: String) -> anyhow::Result<SharePreview> {
        self.gdl_account_task
            .get_share_preview(share_code)
            .await
            .map_err(Into::into)
    }

    /// Upload a background image for a share
    pub async fn upload_share_background(
        self,
        uuid: String,
        share_code: String,
        image_data: Vec<u8>,
    ) -> anyhow::Result<String> {
        let account = self
            .get_account_entries()
            .await?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(anyhow::anyhow!(
                "attempted to upload background for an account that does not exist"
            ))?;

        let auth_token = self.ensure_gdl_auth_token(&account, false).await?;

        self.gdl_account_task
            .upload_share_background(auth_token, share_code, image_data)
            .await
            .map_err(Into::into)
    }

    pub async fn request_new_verification_token(
        self,
        uuid: String,
    ) -> Result<(), RequestNewVerificationTokenError> {
        let account = self
            .get_account_entries()
            .await
            .map_err(|e| RequestNewVerificationTokenError::RequestFailed(e))?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(RequestNewVerificationTokenError::RequestFailed(
                anyhow::anyhow!("attempted to get an account that does not exist"),
            ))?;

        let auth_token = self
            .ensure_gdl_auth_token(&account, false)
            .await
            .map_err(RequestNewVerificationTokenError::RequestFailed)?;

        let request = self
            .gdl_account_task
            .request_new_verification_token(auth_token)
            .await;

        self.app
            .invalidate(PEEK_GDL_ACCOUNT, Some(uuid.clone().into()));
        self.app.invalidate(GET_GDL_ACCOUNT, None);

        request?;

        Ok(())
    }

    pub async fn request_email_change(
        self,
        uuid: String,
        email: String,
    ) -> Result<(), RequestNewEmailChangeError> {
        let account = self
            .get_account_entries()
            .await
            .map_err(|e| RequestNewEmailChangeError::RequestFailed(e))?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(RequestNewEmailChangeError::RequestFailed(anyhow::anyhow!(
                "attempted to get an account that does not exist"
            )))?;

        let auth_token = self
            .ensure_gdl_auth_token(&account, false)
            .await
            .map_err(RequestNewEmailChangeError::RequestFailed)?;

        let request = self
            .gdl_account_task
            .request_email_change(auth_token, email)
            .await;

        self.app
            .invalidate(PEEK_GDL_ACCOUNT, Some(uuid.clone().into()));
        self.app.invalidate(GET_GDL_ACCOUNT, None);

        request?;

        Ok(())
    }

    pub async fn remove_gdl_account(self) -> anyhow::Result<()> {
        use carbon_repos::repos::app_configuration::AppConfigurationPatch;

        self.app
            .settings_manager()
            .set(AppConfigurationPatch {
                gdl_account_uuid: Some(None),
                ..Default::default()
            })
            .await?;

        self.app
            .settings_manager()
            .set(AppConfigurationPatch {
                gdl_account_status: Some(None),
                ..Default::default()
            })
            .await?;

        self.app.invalidate(GET_GDL_ACCOUNT, None);
        self.app.invalidate(GET_SETTINGS, None);

        // Notify Electron main process that GDL account was removed for Overwolf ad personalization
        info!("_GDL_ACCOUNT_EMAIL_:");
        println!("_GDL_ACCOUNT_EMAIL_:");

        Ok(())
    }

    pub async fn change_display_name(
        self,
        uuid: String,
        display_name: String,
    ) -> Result<(), ChangeDisplayNameError> {
        let account = self
            .get_account_entries()
            .await
            .map_err(|e| ChangeDisplayNameError::RequestFailed(e))?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(ChangeDisplayNameError::RequestFailed(anyhow::anyhow!(
                "attempted to get an account that does not exist"
            )))?;

        let auth_token = self
            .ensure_gdl_auth_token(&account, false)
            .await
            .map_err(ChangeDisplayNameError::RequestFailed)?;

        self.gdl_account_task
            .change_display_name(auth_token, display_name)
            .await?;

        self.app
            .invalidate(PEEK_GDL_ACCOUNT, Some(uuid.clone().into()));
        self.app.invalidate(GET_GDL_ACCOUNT, None);

        Ok(())
    }

    pub async fn get_display_name_history(
        self,
        friend_code: String,
    ) -> anyhow::Result<Vec<DisplayNameHistoryEntry>> {
        self.gdl_account_task
            .get_display_name_history(friend_code)
            .await
    }

    pub async fn clear_display_name_history(self, uuid: String) -> anyhow::Result<()> {
        let account = self
            .get_account_entries()
            .await?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(anyhow::anyhow!(
                "attempted to get an account that does not exist"
            ))?;

        let auth_token = self.ensure_gdl_auth_token(&account, false).await?;

        self.gdl_account_task
            .clear_display_name_history(auth_token)
            .await?;

        Ok(())
    }

    pub async fn upload_profile_icon(self, uuid: String, icon_path: String) -> anyhow::Result<()> {
        let account = self
            .get_account_entries()
            .await?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(anyhow::anyhow!(
                "attempted to get an account that does not exist"
            ))?;

        let auth_token = self.ensure_gdl_auth_token(&account, false).await?;

        let request = self
            .gdl_account_task
            .upload_profile_icon(auth_token, icon_path)
            .await;

        // Invalidate caches so UI refreshes with new avatar
        self.app
            .invalidate(PEEK_GDL_ACCOUNT, Some(uuid.clone().into()));
        self.app.invalidate(GET_GDL_ACCOUNT, None);

        request?;

        Ok(())
    }

    pub async fn delete_profile_icon(self, uuid: String) -> anyhow::Result<()> {
        let account = self
            .get_account_entries()
            .await?
            .into_iter()
            .find(|account| account.uuid == uuid)
            .ok_or(anyhow::anyhow!(
                "attempted to delete profile icon for an account that does not exist"
            ))?;

        let auth_token = self.ensure_gdl_auth_token(&account, false).await?;

        let request = self.gdl_account_task.delete_profile_icon(auth_token).await;

        // Invalidate caches so UI refreshes with generated avatar
        self.app
            .invalidate(PEEK_GDL_ACCOUNT, Some(uuid.clone().into()));
        self.app.invalidate(GET_GDL_ACCOUNT, None);

        request?;

        Ok(())
    }

    pub async fn check_username_available(
        self,
        access_token: String,
        username: String,
    ) -> anyhow::Result<api::UsernameAvailability> {
        api::check_username_available(&self.app.reqwest_client, &access_token, &username).await
    }

    pub async fn create_minecraft_profile(
        self,
        access_token: String,
        username: String,
    ) -> anyhow::Result<()> {
        let result =
            api::create_profile(&self.app.reqwest_client, &access_token, &username).await?;

        match result {
            Ok(_) => Ok(()),
            Err(api::CreateProfileError::InvalidUsername) => {
                bail!("InvalidUsername")
            }
            Err(api::CreateProfileError::NameNotAvailable) => {
                bail!("NameNotAvailable")
            }
        }
    }

    /// Add or update an account
    async fn add_account(self, account: FullAccount) -> anyhow::Result<()> {
        let uuid = account.uuid.clone();
        let db_account = self
            .app
            .db
            .read({
                let uuid = uuid.clone();
                move |conn| Ok(account_repo::get_account(conn, &uuid)?)
            })
            .await?;

        if db_account.is_some() {
            // don't change lastUsed
            info!("Updating account information for {:?}", &account.uuid);

            match account.type_ {
                FullAccountType::Offline => {
                    self.app
                        .db
                        .write(move |conn| {
                            Ok(account_repo::update_account_offline(
                                conn,
                                &account.uuid,
                                &account.username,
                            )?)
                        })
                        .await?;
                }
                FullAccountType::Microsoft {
                    access_token,
                    refresh_token,
                    token_expires,
                    id_token,
                    gdl_token,
                    email: _,
                    skin_id,
                } => {
                    let token_expires =
                        DbDateTime(token_expires.with_timezone(&FixedOffset::east(0)));
                    self.app
                        .db
                        .write(move |conn| {
                            Ok(account_repo::update_account_microsoft(
                                conn,
                                &account.uuid,
                                &account.username,
                                &access_token,
                                Some(token_expires),
                                refresh_token.as_deref(),
                                id_token.as_deref(),
                                gdl_token.as_deref(),
                                skin_id.as_deref(),
                            )?)
                        })
                        .await?;
                }
            }

            self.app.invalidate(GET_ACCOUNTS, Some(uuid.into()));
        } else {
            info!("Creating account {:?}", &account.uuid);

            let last_used = DbDateTime(Utc::now().into());
            match account.type_ {
                FullAccountType::Offline => {
                    self.app
                        .db
                        .write(move |conn| {
                            Ok(account_repo::insert_account_offline(
                                conn,
                                &account.uuid,
                                &account.username,
                                last_used,
                                None,
                            )?)
                        })
                        .await?;
                }
                FullAccountType::Microsoft {
                    access_token,
                    refresh_token,
                    token_expires,
                    id_token,
                    gdl_token,
                    email: _,
                    skin_id,
                } => {
                    let token_expires =
                        DbDateTime(token_expires.with_timezone(&FixedOffset::east(0)));
                    self.app
                        .db
                        .write(move |conn| {
                            Ok(account_repo::insert_account_microsoft(
                                conn,
                                &account.uuid,
                                &account.username,
                                last_used,
                                &access_token,
                                Some(token_expires),
                                refresh_token.as_deref(),
                                id_token.as_deref(),
                                gdl_token.as_deref(),
                                skin_id.as_deref(),
                            )?)
                        })
                        .await?;
                }
            }

            self.app.invalidate(GET_ACCOUNTS, None);
        }

        Ok(())
    }

    pub async fn refresh_account(
        self,
        uuid: String,
    ) -> anyhow::Result<tokio::task::JoinHandle<Result<(), futures::future::Aborted>>> {
        info!("Refreshing account {uuid}");

        let account = self
            .app
            .db
            .read({
                let uuid = uuid.clone();
                move |conn| Ok(account_repo::get_account(conn, &uuid)?)
            })
            .await?
            .ok_or(RefreshAccountError::NoAccount)?;

        let Some(refresh_token) = &account.ms_refresh_token else {
            warn!("No refresh token, aborting refresh for {uuid}");
            bail!(RefreshAccountError::NoRefreshToken)
        };

        // stays locked until we insert an enrollment task
        let mut refreshing = self.currently_refreshing.write().await;
        if refreshing.contains_key(&uuid) {
            warn!("{uuid} is already being refreshed");
            bail!(RefreshAccountError::AlreadyRefreshing);
        }

        struct Invalidator {
            app: AppRef,
            account: FullAccount,
        }

        #[async_trait]
        impl InvalidateCtx for Invalidator {
            async fn invalidate(&self) {
                let app = self.app.upgrade();
                let account_manager = app.account_manager();
                let mut refreshing = account_manager.currently_refreshing.write().await;
                // this should never happen
                let enrollment = refreshing.get(&self.account.uuid).expect("account refresh invalidator recieved an invalidation without an active enrollemt");
                let status = enrollment.status.read().await;

                match &*status {
                    EnrollmentStatus::Complete(account) => {
                        let r = account_manager.add_account(account.clone().into()).await;

                        match r {
                            Ok(_) => {
                                info!("Refreshed account {}", &self.account.uuid);
                                // Clear any GDL token exchange backoff on successful refresh
                                account_manager
                                    .manager
                                    .gdl_token_exchange_backoff
                                    .lock()
                                    .await
                                    .remove(&self.account.uuid);
                            }
                            Err(e) => {
                                error!({ error = ?e }, "Failed to update account information {}", &self.account.uuid)
                            }
                        }

                        drop(status);
                        refreshing.remove(&self.account.uuid);
                    }
                    EnrollmentStatus::Failed(e) => {
                        let FullAccountType::Microsoft {
                            access_token,
                            token_expires,
                            skin_id,
                            ..
                        } = &self.account.type_
                        else {
                            error!(
                                "account type was not microsoft during refresh for {}",
                                &self.account.uuid
                            );
                            return;
                        };

                        if let Ok(e) = e {
                            warn!(
                                "Failed to refresh account {} due to an account validity issue, marking the account as requiring relogin (Invalid)",
                                self.account.uuid,
                            );

                            account_manager.add_account(FullAccount {
                                username: self.account.username.clone(),
                                uuid: self.account.uuid.clone(),
                                type_: FullAccountType::Microsoft {
                                    access_token: access_token.clone(),
                                    refresh_token: None,
                                    id_token: None,
                                    gdl_token: None,
                                    email: None,
                                    token_expires: token_expires.clone(),
                                    skin_id: skin_id.clone(),
                                },
                                last_used: self.account.last_used.clone(),
                            }).await.expect("db error, this can't be handled in the account invalidator right now");
                        } else {
                            warn!("Failed to refresh account {}: {e:?}", self.account.uuid);
                        }

                        drop(status);
                        refreshing.remove(&self.account.uuid);
                    }
                    _ => {}
                }

                ()
            }
        }

        let (enrollment, handler) = EnrollmentTask::refresh(
            self.app.reqwest_client.clone(),
            refresh_token.clone(),
            Invalidator {
                app: AppRef(Arc::downgrade(self.app)),
                account: account.try_into()?,
            },
            self.manager.gdl_account_task.clone(),
        );

        refreshing.insert(uuid.clone(), enrollment);
        drop(refreshing);

        self.app.invalidate(GET_ACCOUNTS, Some(uuid.into()));

        Ok(handler)
    }

    pub async fn delete_account(self, uuid: String) -> anyhow::Result<()> {
        let settings = self.app.settings_manager().get_settings().await?;

        let active_account = settings.active_account_uuid;
        let active_gdl_account = settings.gdl_account_uuid;

        if let Some(active_account) = active_account {
            if active_account == uuid {
                let next_account = self
                    .app
                    .db
                    .read({
                        let uuid = uuid.clone();
                        move |conn| Ok(account_repo::get_next_active_account(conn, &uuid)?)
                    })
                    .await?
                    .map(|row| row.uuid);

                self.set_active_uuid(next_account).await?;
            }
        }

        let accounts = self.get_account_entries().await?;

        match (active_gdl_account, accounts.len()) {
            (Some(gdl_account), _) if gdl_account == uuid => {
                self.remove_gdl_account().await?;
            }
            (_, 1) => {
                self.remove_gdl_account().await?;
            }
            _ => {}
        }

        // A plain `DELETE` reports affected rows instead of erroring on a
        // missing row (PCR's `RecordNotFound`); `rows_affected == 0` is the
        // equivalent check.
        let rows_affected = self
            .app
            .db
            .write({
                let uuid = uuid.clone();
                move |conn| Ok(account_repo::delete_account(conn, &uuid)?)
            })
            .await?;

        if rows_affected == 0 {
            bail!(DeleteAccountError::AccountDoesNotExist(uuid))
        }

        info!("Deleted account {uuid}");

        // Clean up backoff state for deleted account
        self.manager
            .gdl_token_exchange_backoff
            .lock()
            .await
            .remove(&uuid);

        self.app.invalidate(GET_ACCOUNTS, None);

        Ok(())
    }

    pub async fn begin_enrollment(self) -> anyhow::Result<()> {
        let mut enrollment_lock = self.active_enrollment.write().await;

        // Automatically cancel any existing enrollment
        if enrollment_lock.is_some() {
            info!("Auto-canceling previous enrollment before starting new device code enrollment");
            enrollment_lock.take();
        }

        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(10);
        let reqwest_client = Client::builder().build()?;
        let client = ClientBuilder::new(reqwest_client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        struct Invalidator(AppRef);

        #[async_trait]
        impl InvalidateCtx for Invalidator {
            async fn invalidate(&self) {
                self.0.upgrade().invalidate(ENROLL_GET_STATUS, None);
            }
        }

        info!("Beginning account enrollment (device code)");

        let active_enrollment =
            EnrollmentTask::begin(client, Invalidator(AppRef(Arc::downgrade(self.app))));

        *enrollment_lock = Some(active_enrollment);
        self.app.invalidate(ENROLL_GET_STATUS, None);
        Ok(())
    }

    pub async fn begin_enrollment_browser(self, open_browser: bool) -> anyhow::Result<()> {
        let mut enrollment_lock = self.active_enrollment.write().await;

        // Automatically cancel any existing enrollment
        if enrollment_lock.is_some() {
            info!("Auto-canceling previous enrollment before starting new browser enrollment");
            enrollment_lock.take();
        }

        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(10);
        let reqwest_client = Client::builder().build()?;
        let client = ClientBuilder::new(reqwest_client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        struct Invalidator(AppRef);

        #[async_trait]
        impl InvalidateCtx for Invalidator {
            async fn invalidate(&self) {
                self.0.upgrade().invalidate(ENROLL_GET_STATUS, None);
            }
        }

        info!("Beginning account enrollment (browser)");

        let active_enrollment = EnrollmentTask::begin_browser(
            client,
            Invalidator(AppRef(Arc::downgrade(self.app))),
            open_browser,
        );

        *enrollment_lock = Some(active_enrollment);
        self.app.invalidate(ENROLL_GET_STATUS, None);
        Ok(())
    }

    pub async fn cancel_enrollment(self) -> anyhow::Result<()> {
        let enrollment = self.active_enrollment.write().await.take();
        self.app.invalidate(ENROLL_GET_STATUS, None);

        info!("Canceling account enrollment");

        match enrollment {
            Some(_) => Ok(()),
            None => bail!(CancelEnrollmentStatusError::NotActive),
        }
    }

    /// Handle an OAuth callback from the custom protocol (gdlauncher://oauth/callback)
    ///
    /// This stores the callback for consumption by an active enrollment task.
    /// The frontend should call this when Electron's 'open-url' event fires.
    pub async fn handle_protocol_callback(self, protocol_url: String) -> anyhow::Result<()> {
        info!("Received protocol callback");

        // Parse and store the callback
        self.protocol_handler.handle_callback(&protocol_url).await?;

        info!("Protocol callback stored successfully");
        Ok(())
    }

    /// Check if there's a pending protocol OAuth callback
    pub async fn has_protocol_callback(self) -> bool {
        self.protocol_handler.has_pending_code().await
    }

    /// Retrieve and consume a pending protocol OAuth callback
    ///
    /// Returns None if no callback is pending
    pub async fn take_protocol_callback(self) -> Option<protocol_handler::ProtocolOAuthCallback> {
        self.protocol_handler.take_pending_code().await
    }

    pub async fn get_enrollment_status<T>(
        self,
        f: impl FnOnce(&EnrollmentStatus) -> T,
    ) -> Option<T> {
        match &*self.active_enrollment.read().await {
            None => None,
            Some(enrollment) => Some(f(&*enrollment.status.read().await)),
        }
    }

    pub async fn finalize_enrollment(self) -> anyhow::Result<()> {
        let enrollment = self.active_enrollment.write().await.take();

        match enrollment {
            None => bail!(FinalizeEnrollmentError::NotActive),
            Some(enrollment) => {
                let mut status = EnrollmentStatus::RequestingCode;
                mem::swap(&mut *enrollment.status.write().await, &mut status);

                match status {
                    EnrollmentStatus::Complete(account) => {
                        let uuid = account.mc.profile.uuid.clone();
                        self.add_account(account.into()).await?;
                        self.set_active_uuid(Some(uuid.clone())).await?;

                        // Exchange MS token for GDL token (non-blocking)
                        match self.exchange_gdl_token(&uuid).await {
                            Ok(_) => {
                                self.manager
                                    .gdl_token_exchange_backoff
                                    .lock()
                                    .await
                                    .remove(&uuid);
                            }
                            Err(e) => {
                                warn!("Failed to exchange GDL token during enrollment: {}", e);
                            }
                        }

                        self.app.invalidate(ENROLL_GET_STATUS, None);

                        Ok(())
                    }
                    _ => bail!(FinalizeEnrollmentError::NotComplete),
                }
            }
        }
    }

    pub async fn resume_enrollment(self) -> anyhow::Result<()> {
        let enrollment_opt = self.active_enrollment.read().await;

        let enrollment = match &*enrollment_opt {
            None => bail!(ResumeEnrollmentError::NotActive),
            Some(enrollment) => enrollment,
        };

        let mut status_lock = enrollment.status.write().await;
        let status = mem::replace(&mut *status_lock, EnrollmentStatus::RequestingCode);

        match status {
            EnrollmentStatus::NeedsProfileCreation {
                access_token,
                ms_auth,
                mc_auth,
                entitlements,
            } => {
                // Update status to indicate we're fetching the profile
                *status_lock = EnrollmentStatus::McProfile;
                drop(status_lock);
                drop(enrollment_opt);

                // Fetch the profile again (should succeed now that user created it)
                let mc_profile = api::get_profile(&self.app.reqwest_client, &access_token)
                    .await?
                    .map_err(|e| {
                        anyhow::anyhow!("Failed to get profile after creation: {:?}", e)
                    })?;

                let account = api::McAccount {
                    entitlement: entitlements,
                    profile: mc_profile,
                    auth: mc_auth,
                };

                let full_account = api::FullAccount {
                    ms: ms_auth,
                    mc: account,
                    gdl_token: None, // Will be exchanged after enrollment
                };

                // Update status to Complete
                if let Some(enrollment) = &*self.active_enrollment.read().await {
                    *enrollment.status.write().await =
                        EnrollmentStatus::Complete(full_account.clone());
                }

                self.app.invalidate(ENROLL_GET_STATUS, None);

                // Finalize the enrollment
                let uuid = full_account.mc.profile.uuid.clone();
                self.add_account(full_account.into()).await?;
                self.set_active_uuid(Some(uuid.clone())).await?;

                // Exchange MS token for GDL token (non-blocking)
                match self.exchange_gdl_token(&uuid).await {
                    Ok(_) => {
                        self.manager
                            .gdl_token_exchange_backoff
                            .lock()
                            .await
                            .remove(&uuid);
                    }
                    Err(e) => {
                        warn!(
                            "Failed to exchange GDL token during resume_enrollment: {}",
                            e
                        );
                    }
                }

                Ok(())
            }
            other_status => {
                // Restore the original status
                if let Some(enrollment) = &*self.active_enrollment.read().await {
                    *enrollment.status.write().await = other_status;
                }
                bail!(ResumeEnrollmentError::NotInProfileCreationState)
            }
        }
    }

    /// Attempt to immediately update account information, expiring the account on failure.
    ///
    /// This function will reset the ongoing refresh countdown to avoid possible
    /// rate limiting.
    ///
    /// # Parameters
    /// lock_refresh - stop any new background account refreshes and wait 30 seconds
    ///                before performing more.
    #[tracing::instrument(skip(self, lock_refresh))]
    pub async fn refresh_account_status(
        self,
        uuid: String,
        lock_refresh: bool,
    ) -> anyhow::Result<()> {
        debug!("Checking account status");

        let mut refresh_lock = match lock_refresh {
            true => Some(self.refreshloop_sleep.lock().await),
            false => None,
        };

        let account = self
            .get_account(uuid.clone())
            .await?
            .ok_or_else(|| ValidateAccountError::AccountMissing(uuid.clone()))?;

        let access_token = match account.status {
            AccountStatus::Ok {
                access_token: Some(access_token),
                ..
            } => access_token,
            _ => {
                debug!(?account.status, "Account is not ok, ignoring");
                return Ok(());
            }
        };

        let profile = api::get_profile(&self.app.reqwest_client, &access_token).await;

        if let Some(refresh_lock) = &mut refresh_lock {
            **refresh_lock = Some(Instant::now() + Duration::from_secs(30));
        }

        drop(refresh_lock);

        let profile = match profile {
            Ok(Ok(x)) => x,
            Ok(Err(GetProfileError::AuthTokenInvalid)) => {
                info!("Auth token was invalid");
                // the account was expired prematurely
                let now = DbDateTime(Utc::now().into());
                self.app
                    .db
                    .write({
                        let uuid = uuid.clone();
                        move |conn| Ok(account_repo::expire_account_token_now(conn, &uuid, now)?)
                    })
                    .await?;

                self.app.invalidate(GET_ACCOUNTS, None);
                return Ok(());
            }
            Ok(Err(GetProfileError::GameProfileMissing)) => {
                info!("Game profile is missing");
                bail!(GetProfileError::GameProfileMissing)
            }
            Err(e) => bail!(e),
        };

        let skin_changed = account.account.skin_id.as_ref().map(|s| s as &str)
            != profile.skin.as_ref().map(|skin| &skin.id as &str);

        let new_skin_id = profile.skin.map(|skin| skin.id);
        self.app
            .db
            .write({
                let uuid = uuid.clone();
                let username = profile.username;
                let new_skin_id = new_skin_id;
                move |conn| {
                    Ok(account_repo::update_account_profile(
                        conn,
                        &uuid,
                        &username,
                        new_skin_id.as_deref(),
                    )?)
                }
            })
            .await?;

        if skin_changed {
            self.app.invalidate(GET_HEAD, Some(uuid.clone().into()));
        }

        debug!("Account is valid");

        Ok(())
    }

    pub fn skin_manager(self) -> ManagerRef<'s, SkinManager> {
        ManagerRef {
            app: self.app,
            manager: &self.manager.skin_manager,
        }
    }
}

pub struct AccountRefreshService;

impl AccountRefreshService {
    pub async fn start(app: Weak<AppInner>) {
        // account status check
        let app1 = app.clone();
        tokio::spawn(async move {
            let mut last_check_times = HashMap::<String, Instant>::new();

            while let Some(app) = app1.upgrade() {
                let account_manager = app.account_manager();

                // wait for all additional refreshing delays to complete to avoid rate limiting
                loop {
                    let mut sleep_until = account_manager.refreshloop_sleep.lock().await;

                    match &mut *sleep_until {
                        Some(time) => {
                            if *time < Instant::now() {
                                *sleep_until = None;
                                break;
                            }

                            tokio::time::sleep_until((*time).into()).await;
                        }
                        None => break,
                    }
                }

                // TODO: there's not really a way to handle an error in here
                if let Ok(accounts) = account_manager.get_account_entries().await {
                    // discard deleted accounts
                    last_check_times = last_check_times
                        .into_iter()
                        .filter(|(uuid, _)| {
                            accounts.iter().any(|account| {
                                &account.uuid == uuid
                                // any account that may have been removed and re-added as an offline account
                                // since last refresh
                                && account.access_token.is_some()
                            })
                        })
                        .collect();

                    // add any new accounts
                    for account in accounts {
                        if !last_check_times.contains_key(&account.uuid)
                            && account.access_token.is_some()
                        {
                            last_check_times.insert(account.uuid, Instant::now());
                        }
                    }

                    let least_recently_checked = last_check_times
                        .iter()
                        .min_by(|(_, a), (_, b)| a.cmp(b))
                        .map(|(uuid, _)| uuid);

                    if let Some(uuid) = least_recently_checked {
                        debug!("Checking least recently checked account {uuid} validity");

                        let r = account_manager
                            .refresh_account_status(uuid.clone(), false)
                            .await;

                        if let Err(e) = r {
                            error!({ error = ?e }, "Failed to check account status for {uuid}");
                        }

                        last_check_times.insert(uuid.clone(), Instant::now());
                    }
                }

                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });

        let notifier = Arc::new(tokio::sync::Notify::new());
        let notifier_clone = notifier.clone();

        tokio::spawn(async move {
            let mut first_check_done = false;

            while let Some(app) = app.upgrade() {
                let account_manager = app.account_manager();

                let current_account = account_manager.get_active_uuid().await.unwrap_or_default();
                let saved_gdl_account_uuid = app
                    .settings_manager()
                    .get_settings()
                    .await
                    .map(|settings| settings.gdl_account_uuid)
                    .unwrap_or_default();

                // TODO: there's not really a way to handle an error in here
                if let Ok(accounts) = account_manager.get_account_entries().await {
                    // Sort accounts by priority. Currently priority is
                    // 1. GDL account (if any)
                    // 2. Currently selected account (if any and different from GDL account)
                    // 3. All other accounts

                    let mut accounts = accounts.into_iter().collect::<Vec<_>>();
                    accounts.sort_by(|a, b| {
                        if let Some(gdl_account_uuid) = &saved_gdl_account_uuid {
                            if &a.uuid == gdl_account_uuid {
                                return Ordering::Less;
                            } else if &b.uuid == gdl_account_uuid {
                                return Ordering::Greater;
                            }
                        }

                        let Some(current_account) = &current_account else {
                            return Ordering::Equal;
                        };

                        if &a.uuid == current_account {
                            return Ordering::Less;
                        } else if &b.uuid == current_account {
                            return Ordering::Greater;
                        }

                        Ordering::Equal
                    });

                    for account in accounts {
                        trace!(
                            "Considering checking account: {} {}",
                            account.uuid, account.username
                        );

                        let uuid = account.uuid.clone();
                        // ignore badly formed account entries since we can't handle them
                        let Ok(account) = FullAccount::try_from(account) else {
                            tracing::error!(
                                "Badly formed account entry for uuid {uuid}. Cannot check refresh status."
                            );
                            continue;
                        };
                        let token_expires = match account.type_ {
                            FullAccountType::Microsoft { token_expires, .. } => Some(token_expires),
                            _ => None,
                        };

                        let now = Utc::now();
                        let token_expiration_threshold = token_expires
                            .map(|token_expires| token_expires - chrono::Duration::hours(12));

                        let should_refresh = match token_expiration_threshold {
                            Some(token_expiration_threshold) => token_expiration_threshold < now,
                            None => true,
                        };

                        if should_refresh {
                            debug!(
                                "Attempting to refresh access token for expired account {}",
                                &account.uuid
                            );
                            let Ok(handler) =
                                account_manager.refresh_account(account.uuid.clone()).await
                            else {
                                error!("Failed to refresh access token for {}", &account.uuid);
                                notifier_clone.notify_one();
                                first_check_done = true;
                                continue;
                            };

                            // This works because GDL account is guaranteed to be first in the list
                            // and if there is no gdl account, the currently selected account is guaranteed
                            // to be first
                            if !first_check_done {
                                let _ = handler.await;

                                notifier_clone.notify_one();
                                first_check_done = true;
                            }

                            break;
                        }
                    }
                }

                notifier_clone.notify_one();

                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });

        // Wait until The first refresh is complete, or 5 seconds have passed (for safety)
        select! {
            // tokio::sync::Notify is not cancellation safe, but in this case we don't care
            // because if it's cancelled, we'll just continue
            _ = notifier.notified() => {
                update_core_module_status(CoreModuleStatus::AccountRefreshComplete);

                info!("Initial refresh complete");
            }
            _ = tokio::time::sleep(Duration::from_secs(10)) => {
                error!("Failed to wait for initial refresh to complete");
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum GetActiveAccountError {
    #[error("account selected but not present")]
    AccountNotPresent,
}

#[derive(Error, Debug)]
pub enum GetAccountStatusError {
    #[error("getting account status: microsoft account token expiry date is unset (invalid state)")]
    TokenExpiryUnset,

    #[error("getting account status: microsoft account token is unset")]
    TokenUnset,
}

#[derive(Error, Debug)]
pub enum BeginEnrollmentStatusError {
    #[error("enrollment already active")]
    InProgress,
}

#[derive(Error, Debug)]
pub enum CancelEnrollmentStatusError {
    #[error("no active enrollment")]
    NotActive,
}

#[derive(Error, Debug)]
pub enum RefreshAccountError {
    #[error("already refreshing")]
    AlreadyRefreshing,

    #[error("account does not exist")]
    NoAccount,

    #[error("no refresh token")]
    NoRefreshToken,

    #[error("loading account from db: {0}")]
    DbLoad(#[from] FullAccountLoadError),

    #[error("query error")]
    Query(#[from] DbError),
}

#[derive(Error, Debug)]
pub enum FinalizeEnrollmentError {
    #[error("no active enrollment")]
    NotActive,

    #[error("enrollment is not complete")]
    NotComplete,
}

#[derive(Error, Debug)]
pub enum ResumeEnrollmentError {
    #[error("no active enrollment")]
    NotActive,

    #[error("enrollment is not in profile creation state")]
    NotInProfileCreationState,
}

#[derive(Error, Debug)]
pub enum DeleteAccountError {
    #[error("attempted to delete account that is not in the account list: {0}")]
    AccountDoesNotExist(String),
}

#[derive(Error, Debug)]
pub enum SetActiveUuidError {
    #[error(
        "attempted to set the active account to one that does not exist in the account list: {0}"
    )]
    AccountDoesNotExist(String),
}

#[derive(Error, Debug)]
pub enum ValidateAccountError {
    #[error("attempted to validate an account that was not present in the account list: {0}")]
    AccountMissing(String),
}

#[derive(Debug)]
pub struct FullAccount {
    pub username: String,
    pub uuid: String,
    pub type_: FullAccountType,
    pub last_used: DateTime<FixedOffset>,
}

#[derive(Debug, Clone)]
pub enum FullAccountType {
    Offline,
    Microsoft {
        access_token: String,
        refresh_token: Option<String>,
        id_token: Option<String>,
        gdl_token: Option<String>,
        email: Option<String>,
        token_expires: DateTime<Utc>,
        skin_id: Option<String>,
    },
}

#[derive(Clone, Serialize, Deserialize)]
struct Claims {
    email: Option<String>,
}

fn extract_email(token: Option<&String>) -> Option<String> {
    token.and_then(|token| {
        let claims: Result<Token<Header, Claims, _>, _> = jwt::Token::parse_unverified(&*token);
        claims.ok().and_then(|claims| claims.claims().email.clone())
    })
}

#[derive(Clone, Serialize, Deserialize)]
struct ExpiryClaims {
    exp: Option<u64>,
}

/// Whether `token` is past its `exp`, treating anything expiring within
/// [`TOKEN_EXPIRY_SKEW`] as already dead so a token can't lapse in flight
/// between this check and the request that uses it.
///
/// A token whose `exp` is unreadable counts as expired: a redundant exchange
/// is cheap, whereas trusting an unverifiable token risks a 401 that no
/// later call can recover from.
fn is_jwt_expired(token: &str) -> bool {
    let parsed: Result<Token<Header, ExpiryClaims, _>, _> = jwt::Token::parse_unverified(token);

    let Ok(parsed) = parsed else {
        return true;
    };

    let Some(exp) = parsed.claims().exp else {
        return true;
    };

    let now = Utc::now().timestamp();
    let deadline = now.saturating_add(TOKEN_EXPIRY_SKEW.as_secs() as i64);

    (exp as i64) <= deadline
}

// get_gdl_auth_token removed — replaced by ensure_gdl_auth_token on ManagerRef

/*impl From<FullAccount> for db::account::Data {
    fn from(value: FullAccount) -> Self {
        let (access_token, refresh_token, token_expires) = match value.type_ {
            FullAccountType::Offline => (None, None, None),
            FullAccountType::Microsoft {
                access_token,
                refresh_token,
                token_expires,
            } => (Some(access_token), refresh_token, Some(token_expires)),
        };

        Self {
            username: value.username,
            uuid: value.uuid,
            access_token,
            ms_refresh_token: refresh_token,
            token_expires,
            last_used: value.last_used,
        }
    }
}*/

impl TryFrom<account_repo::AccountRow> for FullAccount {
    type Error = FullAccountLoadError;

    fn try_from(value: account_repo::AccountRow) -> Result<Self, Self::Error> {
        Ok(Self {
            type_: match value.access_token {
                Some(access_token) => FullAccountType::Microsoft {
                    email: extract_email(value.id_token.as_ref()),
                    access_token,
                    refresh_token: value.ms_refresh_token,
                    id_token: value.id_token,
                    gdl_token: value.gdl_token,
                    token_expires: value
                        .token_expires
                        .map(|time| time.with_timezone(&Utc))
                        .ok_or_else(|| {
                            FullAccountLoadError::MissingExpiration(value.uuid.clone())
                        })?,
                    skin_id: value.skin_id,
                },
                None => FullAccountType::Offline,
            },
            last_used: value.last_used,
            uuid: value.uuid,
            username: value.username,
        })
    }
}

impl From<FullAccount> for AccountWithStatus {
    fn from(value: FullAccount) -> Self {
        Self {
            account: Account {
                username: value.username,
                uuid: value.uuid,
                last_used: value.last_used.into(),
                type_: match value.type_.clone() {
                    FullAccountType::Microsoft { email, .. } => AccountType::Microsoft { email },
                    FullAccountType::Offline => AccountType::Offline,
                },
                skin_id: match &value.type_ {
                    FullAccountType::Microsoft { skin_id, .. } => skin_id.clone(),
                    _ => None,
                },
            },
            status: match value.type_ {
                // An account is invalid only when it lacks the tokens needed to refresh
                // and launch. `email` is optional display metadata extracted from the
                // id_token (not all MSA id_tokens carry the claim) and never gates validity.
                FullAccountType::Microsoft {
                    refresh_token: None,
                    ..
                }
                | FullAccountType::Microsoft { id_token: None, .. } => AccountStatus::Invalid,
                FullAccountType::Microsoft {
                    access_token,
                    token_expires,
                    refresh_token: Some(_),
                    id_token: Some(_),
                    ..
                } => match Utc::now() > DateTime::<Utc>::from(token_expires) {
                    true => AccountStatus::Expired,
                    false => AccountStatus::Ok {
                        access_token: Some(access_token),
                    },
                },
                FullAccountType::Offline => AccountStatus::Ok { access_token: None },
            },
        }
    }
}

impl From<api::FullAccount> for FullAccount {
    fn from(value: api::FullAccount) -> Self {
        Self {
            username: value.mc.profile.username,
            uuid: value.mc.profile.uuid,
            type_: FullAccountType::Microsoft {
                access_token: value.mc.auth.access_token,
                refresh_token: Some(value.ms.refresh_token),
                id_token: Some(value.ms.id_token.clone()),
                gdl_token: value.gdl_token, // From parallel exchange during refresh
                email: extract_email(Some(&value.ms.id_token)),
                token_expires: DateTime::<Utc>::from(value.mc.auth.expires_at),
                skin_id: value.mc.profile.skin.map(|skin| skin.id),
            },
            last_used: Utc::now().into(),
        }
    }
}

#[derive(Error, Debug)]
pub enum FullAccountLoadError {
    #[error(
        "attempted to parse microsoft account DB entry(uuid {0}), but was missing refresh token expiration timestamp"
    )]
    MissingExpiration(String),
}

#[cfg(test)]
mod gdl_token_tests {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    /// Build an unsigned JWT with the given `exp` claim. `is_jwt_expired`
    /// parses unverified, so the signature only needs to be well-formed.
    fn jwt_with_exp(exp: i64) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","kid":"gdl-v1"}"#);
        let claims = URL_SAFE_NO_PAD.encode(format!(r#"{{"exp":{exp}}}"#));
        format!("{header}.{claims}.c2ln")
    }

    #[test]
    fn fresh_token_is_not_expired() {
        let exp = Utc::now().timestamp() + 3600;
        assert!(!is_jwt_expired(&jwt_with_exp(exp)));
    }

    /// The bug: a GDL token inherits the Microsoft id_token's `exp` (~1h), so
    /// one stored more than an hour ago is dead and can only produce a 401.
    #[test]
    fn hour_old_token_is_expired() {
        let exp = Utc::now().timestamp() - 1;
        assert!(is_jwt_expired(&jwt_with_exp(exp)));
    }

    #[test]
    fn token_expiring_within_skew_is_expired() {
        let exp = Utc::now().timestamp() + (TOKEN_EXPIRY_SKEW.as_secs() as i64) - 5;
        assert!(is_jwt_expired(&jwt_with_exp(exp)));
    }

    #[test]
    fn token_without_exp_claim_is_expired() {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256"}"#);
        let claims = URL_SAFE_NO_PAD.encode(r#"{"sub":"abc"}"#);
        assert!(is_jwt_expired(&format!("{header}.{claims}.c2ln")));
    }

    #[test]
    fn unparseable_token_is_expired() {
        assert!(is_jwt_expired("not-a-jwt"));
        assert!(is_jwt_expired(""));
    }

    /// The reported bug: an account whose stored GDL token has aged out (they
    /// only live ~1h, inheriting the MS id_token's `exp`) must never have that
    /// token handed back. Doing so sends it to `/v1/users/user`, which answers
    /// 401 — and since nothing re-minted it, every later call did the same,
    /// leaving "Failed to Check Account" until the database was deleted.
    ///
    /// Both tokens are expired here, so no exchange can succeed offline: the
    /// point is strictly that the dead token is not returned.
    #[tokio::test]
    async fn expired_stored_token_is_never_returned() {
        let app = crate::setup_managers_for_test().await;
        let expired = jwt_with_exp(Utc::now().timestamp() - 3600);

        app.db
            .write({
                let expired = expired.clone();
                move |conn| {
                    Ok(account_repo::insert_account_microsoft(
                        conn,
                        "test-uuid",
                        "test-user",
                        DbDateTime(Utc::now().into()),
                        "unused-access-token",
                        None,
                        None,
                        Some(&expired),
                        Some(&expired),
                        None,
                    )?)
                }
            })
            .await
            .unwrap();

        let account = app
            .db
            .read(|conn| Ok(account_repo::get_account(conn, "test-uuid")?))
            .await
            .unwrap()
            .unwrap();

        let result = app
            .account_manager()
            .ensure_gdl_auth_token(&account, false)
            .await;

        match result {
            Ok(token) => panic!("handed back a token for an expired account: {token}"),
            Err(_) => {}
        }
    }
}
