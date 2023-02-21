use crate::{
    api::keys::account::*,
    db::{self, read_filters::StringFilter},
    error::define_single_error,
    managers::account::enroll::InvalidateCtx,
};
use async_trait::async_trait;
use carbon_domain::account::*;
use chrono::{FixedOffset, Utc};
use prisma_client_rust::{
    chrono::DateTime, prisma_errors::query_engine::RecordNotFound, QueryError,
};
use std::mem;

use thiserror::Error;
use tokio::sync::RwLock;

use self::{
    api::DeviceCode,
    enroll::{EnrollmentStatus, EnrollmentTask},
};

use super::{configuration::ConfigurationError, AppRef};

pub mod api;
mod enroll;

pub(crate) struct AccountManager {
    app: AppRef,
    currently_refreshing: RwLock<Vec<String>>,
    active_enrollment: RwLock<Option<EnrollmentTask>>,
}

impl AccountManager {
    pub fn new() -> Self {
        Self {
            app: AppRef::uninit(),
            currently_refreshing: RwLock::new(Vec::new()),
            active_enrollment: RwLock::new(None),
        }
    }

    pub fn get_appref(&self) -> &AppRef {
        &self.app
    }

    pub async fn get_active_uuid(&self) -> Result<Option<String>, GetActiveUuidError> {
        Ok(self
            .app
            .upgrade()
            .configuration_manager
            .configuration()
            .get()
            .await?
            .active_account_uuid)
    }

    pub async fn set_active_uuid(&self, uuid: Option<String>) -> Result<(), SetAccountError> {
        use db::account::WhereParam::Uuid;
        use db::app_configuration::SetParam::SetActiveAccountUuid;

        if let Some(uuid) = uuid.clone() {
            let account_entry = self
                .app
                .upgrade()
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
            .upgrade()
            .configuration_manager
            .configuration()
            .set(SetActiveAccountUuid(uuid))
            .await?;

        self.app.upgrade().invalidate(GET_ACTIVE_UUID, None);
        Ok(())
    }

    async fn get_account_entries(&self) -> Result<Vec<db::account::Data>, QueryError> {
        Ok(self
            .app
            .upgrade()
            .prisma_client
            .account()
            .find_many(Vec::new())
            .exec()
            .await?)
    }

    pub async fn get_account_list(&self) -> Result<Vec<Account>, GetAccountListError> {
        let accounts = self.get_account_entries().await?;

        Ok(accounts
            .into_iter()
            .map(|account| {
                let type_ = match &account.ms_refresh_token {
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
        &self,
        uuid: String,
    ) -> Result<Option<AccountStatus>, GetAccountStatusError> {
        use db::account::UniqueWhereParam;

        let account = self
            .app
            .upgrade()
            .prisma_client
            .account()
            .find_unique(UniqueWhereParam::UuidEquals(uuid))
            .exec()
            .await?;

        let status = match account {
            Some(account) => Some(match account.ms_refresh_token {
                None => AccountStatus::Ok { access_token: None },
                Some(_) => {
                    let token_expires = account
                        .token_expires
                        .ok_or(GetAccountStatusError::TokenExpiryUnset)?;

                    let refreshing = self
                        .currently_refreshing
                        .read()
                        .await
                        .contains(&account.uuid);

                    if refreshing {
                        AccountStatus::Refreshing
                    } else if token_expires < Utc::now() {
                        let access_token = account
                            .access_token
                            .ok_or(GetAccountStatusError::TokenUnset)?;

                        AccountStatus::Ok {
                            access_token: Some(access_token),
                        }
                    } else {
                        AccountStatus::Expired
                    }
                }
            }),
            None => None,
        };

        Ok(status)
    }

    async fn add_account(&self, account: FullAccount) -> Result<(), AddAccountError> {
        use db::account::SetParam;

        let set_params = match account.type_ {
            FullAccountType::Offline => Vec::new(),
            FullAccountType::Microsoft {
                access_token,
                refresh_token,
                token_expires,
            } => vec![
                SetParam::SetAccessToken(Some(access_token)),
                SetParam::SetMsRefreshToken(Some(refresh_token)),
                SetParam::SetTokenExpires(Some(token_expires)),
            ],
        };

        self.app
            .upgrade()
            .prisma_client
            .account()
            .create(account.uuid, account.username, set_params)
            .exec()
            .await?;

        self.app.upgrade().invalidate(GET_ACCOUNTS, None);
        Ok(())
    }

    pub async fn delete_account(&self, uuid: String) -> Result<(), DeleteAccountError> {
        use db::account::UniqueWhereParam;

        let result = self
            .app
            .upgrade()
            .prisma_client
            .account()
            .delete(UniqueWhereParam::UuidEquals(uuid.clone()))
            .exec()
            .await;

        match result {
            Ok(_) => {
                self.app.upgrade().invalidate(GET_ACCOUNTS, None);
                self.app
                    .upgrade()
                    .invalidate(GET_ACCOUNT_STATUS, Some(uuid.into()));

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

    pub async fn begin_enrollment(&self) -> Result<(), BeginEnrollmentStatusError> {
        match &mut *self.active_enrollment.write().await {
            Some(_) => Err(BeginEnrollmentStatusError::InProgress),
            enrollment @ None => {
                let client = self.app.upgrade().reqwest_client.clone();

                struct Invalidator(AppRef);

                #[async_trait]
                impl InvalidateCtx for Invalidator {
                    async fn invalidate(&self) {
                        self.0.upgrade().invalidate(ENROLL_GET_STATUS, None);
                    }
                }

                let active_enrollment =
                    EnrollmentTask::begin(client, Invalidator(self.app.clone()));

                *enrollment = Some(active_enrollment);
                Ok(())
            }
        }
    }

    pub async fn cancel_enrollment(&self) -> Result<(), CancelEnrollmentStatusError> {
        let enrollment = self.active_enrollment.write().await.take();

        match enrollment {
            Some(_) => Ok(()),
            None => Err(CancelEnrollmentStatusError::NotActive),
        }
    }

    pub async fn get_enrollment_status(
        &self,
    ) -> Result<FEEnrollmentStatus, GetEnrollmentStatusError> {
        match &*self.active_enrollment.read().await {
            None => Err(GetEnrollmentStatusError::NotActive),
            Some(enrollment) => Ok(FEEnrollmentStatus::from_enrollment_status(
                &*enrollment.status.read().await,
            )),
        }
    }

    pub async fn finalize_enrollment(&self) -> Result<(), FinalizeEnrollmentError> {
        let enrollment = self.active_enrollment.write().await.take();

        match enrollment {
            None => Err(FinalizeEnrollmentError::NotActive),
            Some(enrollment) => {
                let mut status = EnrollmentStatus::RequestingCode;
                mem::swap(&mut *enrollment.status.write().await, &mut status);

                match status {
                    EnrollmentStatus::Complete(account) => {
                        self.add_account(FullAccount {
                            username: account.mc.profile.username,
                            uuid: account.mc.profile.uuid.clone(),
                            type_: FullAccountType::Microsoft {
                                access_token: account.mc.auth.access_token,
                                token_expires: DateTime::<FixedOffset>::from(
                                    account.mc.auth.expires_at,
                                ),
                                refresh_token: account.ms.refresh_token,
                            },
                        })
                        .await?;

                        self.set_active_uuid(Some(account.mc.profile.uuid)).await?;

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
pub enum GetAccountStatusError {
    #[error("account token expiry unset")]
    TokenExpiryUnset,

    #[error("account token unset")]
    TokenUnset,

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

struct FullAccount {
    username: String,
    uuid: String,
    type_: FullAccountType,
}

enum FullAccountType {
    Offline,
    Microsoft {
        access_token: String,
        refresh_token: String,
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
            } => (Some(access_token), Some(refresh_token), Some(token_expires)),
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

// Temporary until enroll errors are fixed
pub enum FEEnrollmentStatus {
    RequestingCode,
    PollingCode(DeviceCode),
    QueryAccount,
    Complete(Account),
    Failed(String),
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
            EnrollmentStatus::Failed(err) => FEEnrollmentStatus::Failed(format!("{:#?}", err)),
        }
    }
}
