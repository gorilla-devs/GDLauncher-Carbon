use crate::domain::account::*;
use crate::{
    api::keys::account::*,
    db::{self, read_filters::StringFilter},
    managers::account::{api::GetProfileError, enroll::InvalidateCtx},
};
use anyhow::ensure;
use async_trait::async_trait;
use chrono::{FixedOffset, Utc};
use prisma_client_rust::{
    chrono::DateTime, prisma_errors::query_engine::RecordNotFound, Direction, QueryError,
};
use std::{
    collections::HashMap,
    mem,
    sync::{Arc, Weak},
    time::{Duration, Instant},
};

use thiserror::Error;
use tokio::sync::{Mutex, RwLock};

use anyhow::{anyhow, bail};

pub use self::enroll::{EnrollmentError, EnrollmentStatus};
use self::{enroll::EnrollmentTask, skin::SkinManager};

use super::{AppInner, AppRef, ManagerRef};

pub mod api;
mod enroll;
pub mod skin;

pub(crate) struct AccountManager {
    currently_refreshing: RwLock<HashMap<String, EnrollmentTask>>,
    active_enrollment: RwLock<Option<EnrollmentTask>>,
    /// Account refreshing will be disabled until this time has passed
    refreshloop_sleep: Mutex<Option<Instant>>,
    skin_manager: SkinManager,
}

impl AccountManager {
    pub fn new() -> Self {
        Self {
            currently_refreshing: RwLock::new(HashMap::new()),
            active_enrollment: RwLock::new(None),
            refreshloop_sleep: Mutex::new(None),
            skin_manager: SkinManager {},
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
        use db::account::WhereParam::Uuid;
        use db::app_configuration::SetParam::SetActiveAccountUuid;

        if let Some(uuid) = uuid.clone() {
            let account_entry = self
                .app
                .prisma_client
                .account()
                .find_first(vec![Uuid(StringFilter::Equals(uuid.clone()))])
                .exec()
                .await?;

            // Setting the active account to one not in the DB does not make sense.
            ensure!(
                account_entry.is_some(),
                SetActiveUuidError::AccountDoesNotExist(uuid)
            );
        }

        self.app
            .settings_manager()
            .set(SetActiveAccountUuid(uuid))
            .await?;

        self.app.invalidate(GET_ACTIVE_UUID, None);
        Ok(())
    }

    /// Get the active account's details.
    ///
    /// Not exposed to the frontend on purpose. Will NOT be invalidated.
    pub async fn get_active_account(&self) -> anyhow::Result<Option<FullAccount>> {
        use db::account::WhereParam::Uuid;

        let Some(uuid) = self.get_active_uuid().await? else { return Ok(None) };

        let account = self
            .app
            .prisma_client
            .account()
            .find_first(vec![Uuid(StringFilter::Equals(uuid))])
            .exec()
            .await?
            .ok_or_else(|| anyhow!("currently active account could not be read from database"))?;

        Ok(Some(account.try_into()?))
    }

    async fn get_account_entries(self) -> anyhow::Result<Vec<db::account::Data>> {
        use db::account::OrderByParam;

        Ok(self
            .app
            .prisma_client
            .account()
            .find_many(Vec::new())
            .order_by(OrderByParam::LastUsed(Direction::Desc))
            .exec()
            .await?)
    }

    pub async fn get_account_list(self) -> anyhow::Result<Vec<Account>> {
        let accounts = self.get_account_entries().await?;

        Ok(accounts
            .into_iter()
            .map(|account| {
                let type_ = match &account.access_token {
                    None => AccountType::Offline,
                    Some(_) => AccountType::Microsoft,
                };

                Account {
                    username: account.username,
                    uuid: account.uuid,
                    last_used: account.last_used.into(),
                    type_,
                    skin_id: account.skin_id,
                }
            })
            .collect())
    }

    async fn get_account(self, uuid: String) -> anyhow::Result<Option<AccountWithStatus>> {
        use db::account::UniqueWhereParam;

        let account = self
            .app
            .prisma_client
            .account()
            .find_unique(UniqueWhereParam::UuidEquals(uuid))
            .exec()
            .await?;

        let Some(account) = account else { return Ok(None) };
        let account = FullAccount::try_from(account)?;
        let account = AccountWithStatus::from(account);

        Ok(Some(account))
    }

    pub async fn get_account_status(self, uuid: String) -> anyhow::Result<Option<AccountStatus>> {
        let Some(mut account) = self.get_account(uuid).await? else { return Ok(None) };

        if let AccountType::Microsoft = &account.account.type_ {
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

    /// Add or update an account
    async fn add_account(self, account: FullAccount) -> anyhow::Result<()> {
        use db::account::{SetParam, UniqueWhereParam};

        let db_account = self
            .app
            .prisma_client
            .account()
            .find_unique(UniqueWhereParam::UuidEquals(account.uuid.clone()))
            .exec()
            .await?;

        if db_account.is_some() {
            // don't change lastUsed
            let mut set_params = vec![SetParam::SetUsername(account.username)];

            match account.type_ {
                FullAccountType::Offline => set_params.extend([
                    SetParam::SetAccessToken(None),
                    SetParam::SetMsRefreshToken(None),
                    SetParam::SetTokenExpires(None),
                ]),
                FullAccountType::Microsoft {
                    access_token,
                    refresh_token,
                    token_expires,
                    skin_id,
                } => set_params.extend([
                    SetParam::SetAccessToken(Some(access_token)),
                    SetParam::SetMsRefreshToken(refresh_token),
                    SetParam::SetTokenExpires(Some(token_expires)),
                    SetParam::SetSkinId(skin_id),
                ]),
            }

            self.app
                .prisma_client
                .account()
                .update(
                    UniqueWhereParam::UuidEquals(account.uuid.clone()),
                    set_params,
                )
                .exec()
                .await?;

            self.app
                .invalidate(GET_ACCOUNT_STATUS, Some(account.uuid.into()));
        } else {
            let set_params = match account.type_ {
                FullAccountType::Offline => Vec::new(),
                FullAccountType::Microsoft {
                    access_token,
                    refresh_token,
                    token_expires,
                    skin_id,
                } => vec![
                    SetParam::SetAccessToken(Some(access_token)),
                    SetParam::SetMsRefreshToken(refresh_token),
                    SetParam::SetTokenExpires(Some(token_expires)),
                    SetParam::SetSkinId(skin_id),
                ],
            };

            self.app
                .prisma_client
                .account()
                .create(
                    account.uuid,
                    account.username,
                    Utc::now().into(),
                    set_params,
                )
                .exec()
                .await?;

            self.app.invalidate(GET_ACCOUNTS, None);
        }

        Ok(())
    }

    pub async fn refresh_account(self, uuid: String) -> anyhow::Result<()> {
        use db::account::UniqueWhereParam;

        let account = self
            .app
            .prisma_client
            .account()
            .find_unique(UniqueWhereParam::UuidEquals(uuid.clone()))
            .exec()
            .await?
            .ok_or(RefreshAccountError::NoAccount)?;

        let Some(refresh_token) = &account.ms_refresh_token else {
            bail!(RefreshAccountError::NoRefreshToken)
        };

        // stays locked until we insert an enrollment task
        let mut refreshing = self.currently_refreshing.write().await;
        if refreshing.contains_key(&uuid) {
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
                let enrollment = refreshing.get(&self.account.uuid).expect("account refresh invalidator received an invalidation without an active enrollment");
                let status = enrollment.status.read().await;

                match &*status {
                    EnrollmentStatus::Complete(account) => {
                        account_manager
                            .add_account(account.clone().into())
                            .await
                            .expect(
                            "db error, this can't be handled in the account invalidator right now",
                        );
                        drop(status);
                        refreshing.remove(&self.account.uuid);
                    }
                    EnrollmentStatus::Failed(_) => {
                        let FullAccountType::Microsoft { access_token, token_expires, skin_id, .. } = &self.account.type_ else {
                            panic!("account type was not microsoft during refresh");
                        };

                        account_manager.add_account(FullAccount {
                            username: self.account.username.clone(),
                            uuid: self.account.uuid.clone(),
                            type_: FullAccountType::Microsoft {
                                access_token: access_token.clone(),
                                refresh_token: None,
                                token_expires: *token_expires,
                                skin_id: skin_id.clone(),
                            },
                            last_used: self.account.last_used,
                        }).await.expect("db error, this can't be handled in the account invalidator right now");
                    }
                    _ => {}
                }
            }
        }

        let enrollment = EnrollmentTask::refresh(
            self.app.reqwest_client.clone(),
            refresh_token.clone(),
            Invalidator {
                app: AppRef(Arc::downgrade(self.app)),
                account: account.try_into()?,
            },
        );

        refreshing.insert(uuid.clone(), enrollment);
        drop(refreshing);

        self.app.invalidate(GET_ACCOUNT_STATUS, Some(uuid.into()));

        Ok(())
    }

    pub async fn delete_account(self, uuid: String) -> anyhow::Result<()> {
        use db::account::{OrderByParam, UniqueWhereParam};

        let active_account = self
            .app
            .settings_manager()
            .get_settings()
            .await?
            .active_account_uuid;

        if let Some(active_account) = active_account {
            if active_account == uuid {
                let next_account = self
                    .app
                    .prisma_client
                    .account()
                    .find_first(Vec::new())
                    .order_by(OrderByParam::LastUsed(Direction::Desc))
                    .exec()
                    .await?
                    .map(|data| data.uuid);

                self.set_active_uuid(next_account).await?;
            }
        }

        let result = self
            .app
            .prisma_client
            .account()
            .delete(UniqueWhereParam::UuidEquals(uuid.clone()))
            .exec()
            .await;

        match result {
            Ok(_) => {
                self.app.invalidate(GET_ACCOUNTS, None);
                self.app.invalidate(GET_ACCOUNT_STATUS, Some(uuid.into()));

                Ok(())
            }
            Err(e) => {
                if e.is_prisma_error::<RecordNotFound>() {
                    bail!(DeleteAccountError::AccountDoesNotExist(uuid))
                } else {
                    bail!(e)
                }
            }
        }
    }

    pub async fn begin_enrollment(self) -> anyhow::Result<()> {
        match &mut *self.active_enrollment.write().await {
            Some(_) => bail!(BeginEnrollmentStatusError::InProgress),
            enrollment @ None => {
                let client = self.app.reqwest_client.clone();

                struct Invalidator(AppRef);

                #[async_trait]
                impl InvalidateCtx for Invalidator {
                    async fn invalidate(&self) {
                        self.0.upgrade().invalidate(ENROLL_GET_STATUS, None);
                    }
                }

                let active_enrollment =
                    EnrollmentTask::begin(client, Invalidator(AppRef(Arc::downgrade(self.app))));

                *enrollment = Some(active_enrollment);
                self.app.invalidate(ENROLL_GET_STATUS, None);
                Ok(())
            }
        }
    }

    pub async fn cancel_enrollment(self) -> anyhow::Result<()> {
        let enrollment = self.active_enrollment.write().await.take();
        self.app.invalidate(ENROLL_GET_STATUS, None);

        match enrollment {
            Some(_) => Ok(()),
            None => bail!(CancelEnrollmentStatusError::NotActive),
        }
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
                        self.set_active_uuid(Some(uuid)).await?;
                        self.app.invalidate(ENROLL_GET_STATUS, None);

                        Ok(())
                    }
                    _ => bail!(FinalizeEnrollmentError::NotComplete),
                }
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
    pub async fn refresh_account_status(
        self,
        uuid: String,
        lock_refresh: bool,
    ) -> anyhow::Result<()> {
        use db::account::{SetParam, UniqueWhereParam};

        let mut refresh_lock = match lock_refresh {
            true => Some(self.refreshloop_sleep.lock().await),
            false => None,
        };

        let account = self
            .get_account(uuid.clone())
            .await?
            .ok_or_else(|| ValidateAccountError::AccountMissing(uuid.clone()))?;

        let AccountStatus::Ok { access_token: Some(access_token) } = account.status else { return Ok(()) };
        let profile = api::get_profile(&self.app.reqwest_client, &access_token).await;

        if let Some(refresh_lock) = &mut refresh_lock {
            **refresh_lock = Some(Instant::now() + Duration::from_secs(30));
        }

        drop(refresh_lock);

        let profile = match profile {
            Ok(Ok(x)) => x,
            Ok(Err(GetProfileError::AuthTokenInvalid)) => {
                // the account was expired prematurely
                self.app
                    .prisma_client
                    .account()
                    .update(
                        UniqueWhereParam::UuidEquals(uuid.clone()),
                        vec![SetParam::SetTokenExpires(Some(Utc::now().into()))],
                    )
                    .exec()
                    .await?;

                self.app.invalidate(GET_ACCOUNT_STATUS, Some(uuid.into()));
                return Ok(());
            }
            Ok(Err(GetProfileError::GameProfileMissing)) => {
                bail!(GetProfileError::GameProfileMissing)
            }
            Err(e) => bail!(e),
        };

        let skin_changed = account.account.skin_id.as_ref().map(|s| s as &str)
            != profile.skin.as_ref().map(|skin| &skin.id as &str);

        self.app
            .prisma_client
            .account()
            .update(
                UniqueWhereParam::UuidEquals(uuid.clone()),
                vec![
                    SetParam::SetUsername(profile.username),
                    SetParam::SetSkinId(profile.skin.map(|skin| skin.id)),
                ],
            )
            .exec()
            .await?;

        if skin_changed {
            self.app.invalidate(GET_HEAD, Some(uuid.into()));
        }

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
    pub fn start(app: Weak<AppInner>) {
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
                        // ignore the result since we can't do anything if it failed.
                        let _ = account_manager
                            .refresh_account_status(uuid.clone(), false)
                            .await;
                        last_check_times.insert(uuid.clone(), Instant::now());
                    }
                }

                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });

        tokio::spawn(async move {
            while let Some(app) = app.upgrade() {
                let account_manager = app.account_manager();

                // TODO: there's not really a way to handle an error in here
                if let Ok(accounts) = account_manager.get_account_entries().await {
                    for account in accounts {
                        // ignore badly formed account entries since we can't handle them
                        let Ok(account) = FullAccount::try_from(account) else { continue };
                        let FullAccountType::Microsoft { token_expires, .. } = account.type_ else { continue };

                        if token_expires < Utc::now() - chrono::Duration::hours(1) {
                            // still can't handle errors
                            let _ = account_manager.refresh_account(account.uuid).await;
                            break;
                        }
                    }
                }

                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });
    }
}

#[derive(Error, Debug)]
pub enum GetActiveAccountError {
    #[error("account selected but not present")]
    AccountNotPresent,
}

#[derive(Error, Debug)]
pub enum GetAccountStatusError {
    #[error(
        "getting account status: microsoft account token expiry date is unset (invalid state)"
    )]
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
    Query(#[from] QueryError),
}

#[derive(Error, Debug)]
pub enum FinalizeEnrollmentError {
    #[error("no active enrollment")]
    NotActive,

    #[error("enrollment is not complete")]
    NotComplete,
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

#[derive(Debug)]
pub enum FullAccountType {
    Offline,
    Microsoft {
        access_token: String,
        refresh_token: Option<String>,
        token_expires: DateTime<FixedOffset>,
        skin_id: Option<String>,
    },
}

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

impl TryFrom<db::account::Data> for FullAccount {
    type Error = FullAccountLoadError;

    fn try_from(value: db::account::Data) -> Result<Self, Self::Error> {
        Ok(Self {
            type_: match value.access_token {
                Some(access_token) => FullAccountType::Microsoft {
                    access_token,
                    refresh_token: value.ms_refresh_token,
                    token_expires: value.token_expires.ok_or_else(|| {
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
                type_: match value.type_ {
                    FullAccountType::Microsoft { .. } => AccountType::Microsoft,
                    FullAccountType::Offline => AccountType::Offline,
                },
                skin_id: match &value.type_ {
                    FullAccountType::Microsoft { skin_id, .. } => skin_id.clone(),
                    _ => None,
                },
            },
            status: match value.type_ {
                FullAccountType::Microsoft {
                    refresh_token: None,
                    ..
                } => AccountStatus::Invalid,
                FullAccountType::Microsoft {
                    access_token,
                    token_expires,
                    refresh_token: Some(_),
                    skin_id: _,
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
                token_expires: DateTime::<FixedOffset>::from(value.mc.auth.expires_at),
                skin_id: value.mc.profile.skin.map(|skin| skin.id),
            },
            last_used: Utc::now().into(),
        }
    }
}

#[derive(Error, Debug)]
pub enum FullAccountLoadError {
    #[error("attempted to parse microsoft account DB entry(uuid {0}), but was missing refresh token expiration timestamp")]
    MissingExpiration(String),
}
