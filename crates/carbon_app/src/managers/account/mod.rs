use crate::{
    api::keys::account::*,
    db::{self, read_filters::StringFilter},
    managers::account::enroll::InvalidateCtx,
};
use anyhow::ensure;
use async_trait::async_trait;
use carbon_domain::account::*;
use chrono::{FixedOffset, Utc};
use prisma_client_rust::{chrono::DateTime, prisma_errors::query_engine::RecordNotFound};
use std::mem;

use thiserror::Error;
use tokio::sync::RwLock;

use anyhow::{anyhow, bail};

pub use self::enroll::EnrollmentError;
use self::{
    api::DeviceCode,
    enroll::{EnrollmentStatus, EnrollmentTask},
};

use super::{AppRef, ManagerRef};

use std::sync::Arc;

pub mod api;
mod enroll;

pub(crate) struct AccountManager {
    currently_refreshing: RwLock<Vec<String>>,
    active_enrollment: RwLock<Option<EnrollmentTask>>,
}

impl AccountManager {
    pub fn new() -> Self {
        Self {
            currently_refreshing: RwLock::new(Vec::new()),
            active_enrollment: RwLock::new(None),
        }
    }
}

impl ManagerRef<'_, AccountManager> {
    pub async fn get_active_uuid(self) -> anyhow::Result<Option<String>> {
        Ok(self
            .app
            .configuration_manager()
            .configuration()
            .get()
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
            .ok_or_else(|| anyhow!("currenly active account could not be read from database"))?;

        Ok(Some(account.try_into()?))
    }

    async fn get_account_entries(self) -> anyhow::Result<Vec<db::account::Data>> {
        Ok(self
            .app
            .prisma_client
            .account()
            .find_many(Vec::new())
            .exec()
            .await?)
    }

    pub async fn get_account_list(self) -> anyhow::Result<Vec<Account>> {
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

    pub async fn get_account_status(self, uuid: String) -> anyhow::Result<Option<AccountStatus>> {
        use db::account::UniqueWhereParam;

        let account = self
            .app
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

    async fn add_account(self, account: FullAccount) -> anyhow::Result<()> {
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
            .prisma_client
            .account()
            .create(account.uuid, account.username, set_params)
            .exec()
            .await?;

        self.app.invalidate(GET_ACCOUNTS, None);
        Ok(())
    }

    pub async fn delete_account(self, uuid: String) -> anyhow::Result<()> {
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
                Ok(())
            }
        }
    }

    pub async fn cancel_enrollment(self) -> anyhow::Result<()> {
        let enrollment = self.active_enrollment.write().await.take();

        match enrollment {
            Some(_) => Ok(()),
            None => bail!(CancelEnrollmentStatusError::NotActive),
        }
    }

    pub async fn get_enrollment_status(self) -> anyhow::Result<FEEnrollmentStatus> {
        match &*self.active_enrollment.read().await {
            None => bail!(GetEnrollmentStatusError::NotActive),
            Some(enrollment) => Ok(FEEnrollmentStatus::from_enrollment_status(
                &*enrollment.status.read().await,
            )),
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
                    _ => bail!(FinalizeEnrollmentError::NotComplete),
                }
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

pub struct FullAccount {
    pub username: String,
    pub uuid: String,
    pub type_: FullAccountType,
}

pub enum FullAccountType {
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

impl TryFrom<db::account::Data> for FullAccount {
    type Error = FullAccountLoadError;

    fn try_from(value: db::account::Data) -> Result<Self, Self::Error> {
        Ok(Self {
            type_: match value.access_token {
                Some(access_token) => FullAccountType::Microsoft {
                    access_token,
                    refresh_token: value.ms_refresh_token.ok_or_else(|| {
                        FullAccountLoadError::MissingRefreshToken(value.uuid.clone())
                    })?,
                    token_expires: value.token_expires.ok_or_else(|| {
                        FullAccountLoadError::MissingExpiration(value.uuid.clone())
                    })?,
                },
                None => FullAccountType::Offline,
            },
            uuid: value.uuid,
            username: value.username,
        })
    }
}

#[derive(Error, Debug)]
pub enum FullAccountLoadError {
    #[error(
        "attempted to parse microsoft account DB entry(uuid {0}), but was missing refresh token"
    )]
    MissingRefreshToken(String),

    #[error("attempted to parse microsoft account DB entry(uuid {0}), but was missing refresh token expiration timestamp")]
    MissingExpiration(String),
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
