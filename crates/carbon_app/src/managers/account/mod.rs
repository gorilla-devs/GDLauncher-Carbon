use crate::{
    api::keys::account::*,
    db::{self, read_filters::StringFilter},
    error::define_single_error,
    managers::{account::enroll::InvalidateCtx, AppRef},
};
use async_trait::async_trait;
use carbon_domain::account::*;
use chrono::{FixedOffset, Utc};
use prisma_client_rust::{
    chrono::DateTime, prisma_errors::query_engine::RecordNotFound, QueryError,
};
use std::{collections::HashMap, mem, sync::Arc};

use thiserror::Error;
use tokio::sync::RwLock;

pub use self::enroll::EnrollmentError;
use self::{
    api::DeviceCode,
    enroll::{EnrollmentStatus, EnrollmentTask},
};

use super::{configuration::ConfigurationError, ManagerRef};

pub mod api;
mod enroll;

pub(crate) struct AccountManager {
    currently_refreshing: RwLock<HashMap<String, EnrollmentTask>>,
    active_enrollment: RwLock<Option<EnrollmentTask>>,
}

impl AccountManager {
    pub fn new() -> Self {
        Self {
            currently_refreshing: RwLock::new(HashMap::new()),
            active_enrollment: RwLock::new(None),
        }
    }
}

impl ManagerRef<'_, AccountManager> {
    pub async fn get_active_uuid(self) -> Result<Option<String>, GetActiveUuidError> {
        Ok(self
            .app
            .configuration_manager()
            .configuration()
            .get()
            .await?
            .active_account_uuid)
    }

    pub async fn set_active_uuid(self, uuid: Option<String>) -> Result<(), SetAccountError> {
        use db::account::WhereParam::Uuid;
        use db::app_configuration::SetParam::SetActiveAccountUuid;

        if let Some(uuid) = uuid.clone() {
            let account_entry = self
                .app
                .prisma_client
                .account()
                .find_first(vec![Uuid(StringFilter::Equals(uuid))])
                .exec()
                .await?;

            // Setting the active account to one not in the DB does not make sense.
            if account_entry.is_none() {
                return Err(SetAccountError::NoAccount);
            }
        }

        self.app
            .configuration_manager()
            .configuration()
            .set(SetActiveAccountUuid(uuid))
            .await?;

        self.app.invalidate(GET_ACTIVE_UUID, None);
        Ok(())
    }

    /// Get the active account's details.
    ///
    /// Not exposed to the frontend on purpose. Will NOT be invalidated.
    pub async fn get_active_account(&self) -> Result<Option<FullAccount>, GetActiveAccountError> {
        use db::account::WhereParam::Uuid;

        let Some(uuid) = self.get_active_uuid().await? else { return Ok(None) };

        let account = self
            .app
            .prisma_client
            .account()
            .find_first(vec![Uuid(StringFilter::Equals(uuid))])
            .exec()
            .await?
            .ok_or(GetActiveAccountError::AccountNotPresent)?;

        Ok(Some(account.try_into()?))
    }

    async fn get_account_entries(self) -> Result<Vec<db::account::Data>, QueryError> {
        self.app
            .prisma_client
            .account()
            .find_many(Vec::new())
            .exec()
            .await
    }

    pub async fn get_account_list(self) -> Result<Vec<Account>, GetAccountListError> {
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
                    type_,
                }
            })
            .collect())
    }

    pub async fn get_account_status(
        self,
        uuid: String,
    ) -> Result<Option<AccountStatus>, GetAccountStatusError> {
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
        let mut account = AccountWithStatus::from(account);

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
    async fn add_account(self, account: FullAccount) -> Result<(), AddAccountError> {
        use db::account::{SetParam, UniqueWhereParam};

        let db_account = self
            .app
            .prisma_client
            .account()
            .find_unique(UniqueWhereParam::UuidEquals(account.uuid.clone()))
            .exec()
            .await?;

        if db_account.is_some() {
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
                } => set_params.extend([
                    SetParam::SetAccessToken(Some(access_token)),
                    SetParam::SetMsRefreshToken(refresh_token),
                    SetParam::SetTokenExpires(Some(token_expires)),
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
                } => vec![
                    SetParam::SetAccessToken(Some(access_token)),
                    SetParam::SetMsRefreshToken(refresh_token),
                    SetParam::SetTokenExpires(Some(token_expires)),
                ],
            };

            self.app
                .prisma_client
                .account()
                .create(account.uuid, account.username, set_params)
                .exec()
                .await?;

            self.app.invalidate(GET_ACCOUNTS, None);
        }

        Ok(())
    }

    pub async fn refresh_account(self, uuid: String) -> Result<(), RefreshAccountError> {
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
            return Err(RefreshAccountError::NoRefreshToken)
        };

        // stays locked until we insert an enrollment task
        let mut refreshing = self.currently_refreshing.write().await;
        if refreshing.contains_key(&uuid) {
            return Err(RefreshAccountError::AlreadyRefreshing);
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
                let status = enrollment.status.read().await.clone();

                match status {
                    EnrollmentStatus::Complete(account) => {
                        account_manager
                            .add_account(account.clone().into())
                            .await
                            .expect(
                            "db error, this can't be handled in the account invalidator right now",
                        );
                        refreshing.remove(&self.account.uuid);
                    }
                    EnrollmentStatus::Failed(_) => {
                        let FullAccountType::Microsoft { access_token, token_expires, .. } = &self.account.type_ else {
                            panic!("account type was not microsoft during refresh");
                        };

                        account_manager.add_account(FullAccount {
                            username: self.account.username.clone(),
                            uuid: self.account.uuid.clone(),
                            type_: FullAccountType::Microsoft {
                                access_token: access_token.clone(),
                                refresh_token: None,
                                token_expires: token_expires.clone(),
                            }
                        }).await.expect("db error, this can't be handled in the account invalidator right now");
                    }
                    _ => {}
                }

                ()
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

    pub async fn delete_account(self, uuid: String) -> Result<(), DeleteAccountError> {
        use db::account::UniqueWhereParam;

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
                    Err(DeleteAccountError::NoAccount)
                } else {
                    Err(DeleteAccountError::Query(e))
                }
            }
        }
    }

    pub async fn begin_enrollment(self) -> Result<(), BeginEnrollmentStatusError> {
        match &mut *self.active_enrollment.write().await {
            Some(_) => Err(BeginEnrollmentStatusError::InProgress),
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
                Ok(())
            }
        }
    }

    pub async fn cancel_enrollment(self) -> Result<(), CancelEnrollmentStatusError> {
        let enrollment = self.active_enrollment.write().await.take();

        match enrollment {
            Some(_) => Ok(()),
            None => Err(CancelEnrollmentStatusError::NotActive),
        }
    }

    pub async fn get_enrollment_status(
        self,
    ) -> Result<FEEnrollmentStatus, GetEnrollmentStatusError> {
        match &*self.active_enrollment.read().await {
            None => Err(GetEnrollmentStatusError::NotActive),
            Some(enrollment) => Ok(FEEnrollmentStatus::from_enrollment_status(
                &*enrollment.status.read().await,
            )),
        }
    }

    pub async fn finalize_enrollment(self) -> Result<(), FinalizeEnrollmentError> {
        let enrollment = self.active_enrollment.write().await.take();

        match enrollment {
            None => Err(FinalizeEnrollmentError::NotActive),
            Some(enrollment) => {
                let mut status = EnrollmentStatus::RequestingCode;
                mem::swap(&mut *enrollment.status.write().await, &mut status);

                match status {
                    EnrollmentStatus::Complete(account) => {
                        let uuid = account.mc.profile.uuid.clone();
                        self.add_account(account.into()).await?;

                        self.set_active_uuid(Some(uuid)).await?;

                        Ok(())
                    }
                    _ => Err(FinalizeEnrollmentError::NotComplete),
                }
            }
        }
    }
}

define_single_error!(GetActiveUuidError::Query(ConfigurationError));
define_single_error!(GetAccountEntriesError::Query(QueryError));
define_single_error!(AddAccountError::Query(QueryError));
define_single_error!(GetAccountListError::Query(QueryError));

#[derive(Error, Debug)]
pub enum GetActiveAccountError {
    #[error("get active uuid error: {0}")]
    GetActiveUuid(#[from] GetActiveUuidError),

    #[error("query error: {0}")]
    Query(#[from] QueryError),

    #[error("account selected but not present")]
    AccountNotPresent,

    #[error("could not parse account from db: {0}")]
    Parse(#[from] FullAccountLoadError),
}

#[derive(Error, Debug)]
pub enum GetAccountStatusError {
    #[error("loading account from db: {0}")]
    DbLoad(#[from] FullAccountLoadError),

    #[error("query error: {0}")]
    Query(#[from] QueryError),
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
pub enum GetEnrollmentStatusError {
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

    #[error("account add error: {0}")]
    AddAccount(#[from] AddAccountError),

    #[error("set account error: {0}")]
    SetAccount(#[from] SetAccountError),
}

#[derive(Error, Debug)]
pub enum DeleteAccountError {
    #[error("account does not exist and cannot be deleted")]
    NoAccount,

    #[error("query error: {0}")]
    Query(#[from] QueryError),
}

#[derive(Error, Debug)]
pub enum SetAccountError {
    #[error("config error: {0}")]
    Configuration(#[from] ConfigurationError),

    #[error("query error: {0}")]
    Query(#[from] QueryError),

    #[error("account does not exist and cannot be set as the active account")]
    NoAccount,
}

pub struct FullAccount {
    pub username: String,
    pub uuid: String,
    pub type_: FullAccountType,
}

pub enum FullAccountType {
    Offline,
    Microsoft {
        access_token: String,
        refresh_token: Option<String>,
        token_expires: DateTime<FixedOffset>,
    },
}

impl From<FullAccount> for db::account::Data {
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
        }
    }
}

impl TryFrom<db::account::Data> for FullAccount {
    type Error = FullAccountLoadError;

    fn try_from(value: db::account::Data) -> Result<Self, Self::Error> {
        Ok(Self {
            uuid: value.uuid,
            username: value.username,
            type_: match value.access_token {
                Some(access_token) => FullAccountType::Microsoft {
                    access_token,
                    refresh_token: value.ms_refresh_token,
                    token_expires: value
                        .token_expires
                        .ok_or(FullAccountLoadError::ExpiryMissing)?,
                },
                None => FullAccountType::Offline,
            },
        })
    }
}

impl From<FullAccount> for AccountWithStatus {
    fn from(value: FullAccount) -> Self {
        Self {
            account: Account {
                username: value.username,
                uuid: value.uuid,
                type_: match value.type_ {
                    FullAccountType::Microsoft { .. } => AccountType::Microsoft,
                    FullAccountType::Offline => AccountType::Offline,
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
            },
        }
    }
}

#[derive(Error, Debug)]
pub enum FullAccountLoadError {
    #[error("missing token expiration time")]
    ExpiryMissing,
}

// Temporary until enroll errors are fixed
pub enum FEEnrollmentStatus {
    RequestingCode,
    PollingCode(DeviceCode),
    QueryAccount,
    Complete(Account),
    Failed(EnrollmentError),
}

impl FEEnrollmentStatus {
    fn from_enrollment_status(status: &EnrollmentStatus) -> FEEnrollmentStatus {
        match status {
            EnrollmentStatus::RequestingCode => Self::RequestingCode,
            EnrollmentStatus::PollingCode(code) => Self::PollingCode(code.clone()),
            EnrollmentStatus::McLogin | EnrollmentStatus::PopulateAccount => Self::QueryAccount,
            EnrollmentStatus::Complete(account) => FEEnrollmentStatus::Complete(Account {
                username: account.mc.profile.username.clone(),
                uuid: account.mc.profile.uuid.clone(),
                type_: AccountType::Microsoft,
            }),
            EnrollmentStatus::Failed(err) => FEEnrollmentStatus::Failed(err.clone()),
        }
    }
}
