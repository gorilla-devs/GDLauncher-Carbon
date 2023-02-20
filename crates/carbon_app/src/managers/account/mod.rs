use crate::{
    api::keys::account::*,
    db::{self, read_filters::StringFilter},
    error::{HandlingActions, UError, UResult, UnexpectedError},
    managers::account::enroll::InvalidateCtx,
};
use async_trait::async_trait;
use carbon_domain::account::*;
use chrono::{FixedOffset, Utc};
use prisma_client_rust::{chrono::DateTime, prisma_errors::query_engine::RecordNotFound};
use rspc::ErrorCode;
use std::mem;

use thiserror::Error;
use tokio::sync::RwLock;

use self::{
    api::DeviceCode,
    enroll::{EnrollmentStatus, EnrollmentTask},
};

use super::AppRef;

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

    pub async fn get_active_uuid(&self) -> Result<Option<String>, UnexpectedError> {
        Ok(self
            .app
            .upgrade()
            .persistence_manager
            .configuration()
            .get()
            .await?
            .active_account_uuid)
    }

    pub async fn set_active_uuid(&self, uuid: Option<String>) -> UResult<(), SetAccountError> {
        use db::account::WhereParam::Uuid;
        use db::app_configuration::SetParam::SetActiveAccountUuid;

        if let Some(uuid) = uuid.clone() {
            let account_entry = self
                .app
                .upgrade()
                .persistence_manager
                .get_db_client()
                .await
                .account()
                .find_first(vec![Uuid(StringFilter::Equals(uuid))])
                .exec()
                .await
                .map_err(UnexpectedError::map(HandlingActions::None))
                .map_err(UError::Unexpected)?;

            // Setting the active account to one not in the DB does not make sense.
            if account_entry.is_none() {
                return Err(UError::Expected(SetAccountError::NoAccount));
            }
        }

        self.app
            .upgrade()
            .persistence_manager
            .configuration()
            .set(SetActiveAccountUuid(uuid))
            .await
            .map_err(UError::Unexpected)?;

        self.app.upgrade().invalidate(GET_ACTIVE_UUID, None);
        Ok(())
    }

    async fn get_account_entries(&self) -> Result<Vec<db::account::Data>, UnexpectedError> {
        Ok(self
            .app
            .upgrade()
            .persistence_manager
            .get_db_client()
            .await
            .account()
            .find_many(Vec::new())
            .exec()
            .await
            .map_err(UnexpectedError::map(HandlingActions::None))?)
    }

    pub async fn get_account_list(&self) -> Result<Vec<Account>, UnexpectedError> {
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
    ) -> Result<Option<AccountStatus>, UnexpectedError> {
        use db::account::UniqueWhereParam;

        let account = self
            .app
            .upgrade()
            .persistence_manager
            .get_db_client()
            .await
            .account()
            .find_unique(UniqueWhereParam::UuidEquals(uuid))
            .exec()
            .await
            .map_err(UnexpectedError::map(HandlingActions::None))?;

        let status = match account {
            Some(account) => Some(match account.ms_refresh_token {
                None => AccountStatus::Ok { access_token: None },
                Some(_) => {
                    let token_expires = account.token_expires.ok_or_else(|| {
                        UnexpectedError::direct("account token expiry unset", HandlingActions::None)
                    })?;

                    let refreshing = self
                        .currently_refreshing
                        .read()
                        .await
                        .contains(&account.uuid);

                    if refreshing {
                        AccountStatus::Refreshing
                    } else if token_expires < Utc::now() {
                        let access_token = account.access_token.ok_or_else(|| {
                            UnexpectedError::direct(
                                "account access token unset",
                                HandlingActions::None,
                            )
                        })?;

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

    async fn add_account(&self, account: FullAccount) -> Result<(), UnexpectedError> {
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
            .persistence_manager
            .get_db_client()
            .await
            .account()
            .create(account.uuid, account.username, set_params)
            .exec()
            .await
            .map_err(UnexpectedError::map(HandlingActions::None))?;

        self.app.upgrade().invalidate(GET_ACCOUNTS, None);
        Ok(())
    }

    pub async fn delete_account(&self, uuid: String) -> UResult<(), DeleteAccountError> {
        use db::account::UniqueWhereParam;

        let result = self
            .app
            .upgrade()
            .persistence_manager
            .get_db_client()
            .await
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
                    Err(UError::Expected(DeleteAccountError::NoAccount))
                } else {
                    Err(UError::Unexpected(UnexpectedError::new(
                        e,
                        HandlingActions::None,
                    )))
                }
            }
        }
    }

    pub async fn begin_enrollment(&self) -> Result<(), EnrollmentError> {
        match &mut *self.active_enrollment.write().await {
            Some(_) => Err(EnrollmentError::InProgress),
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

    pub async fn cancel_enrollment(&self) -> Result<(), EnrollmentError> {
        let enrollment = self.active_enrollment.write().await.take();

        match enrollment {
            Some(_) => Ok(()),
            None => Err(EnrollmentError::NotActive),
        }
    }

    pub async fn get_enrollment_status(&self) -> Result<FEEnrollmentStatus, EnrollmentError> {
        match &*self.active_enrollment.read().await {
            None => Err(EnrollmentError::NotActive),
            Some(enrollment) => Ok(FEEnrollmentStatus::from_enrollment_status(
                &*enrollment.status.read().await,
            )),
        }
    }

    pub async fn finalize_enrollment(&self) -> UResult<(), EnrollmentError> {
        let enrollment = self.active_enrollment.write().await.take();

        match enrollment {
            None => Err(EnrollmentError::NotActive)?,
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
                        .await
                        .map_err(UError::Unexpected)?;

                        self.set_active_uuid(Some(account.mc.profile.uuid))
                            .await
                            // it makes no sense for the active account to be missing from the account list
                            // immediately after adding it.
                            .map_err(UError::map_unexpected(HandlingActions::None))
                            .map_err(UError::Unexpected)?;

                        Ok(())
                    }
                    _ => Err(EnrollmentError::NotComplete)?,
                }
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum EnrollmentError {
    #[error("enrollment already in progress")]
    InProgress,

    #[error("no active enrollment")]
    NotActive,

    #[error("enrollment not complete")]
    NotComplete,
}

#[derive(Error, Debug)]
pub enum DeleteAccountError {
    #[error("account does not exist and cannot be deleted")]
    NoAccount,
}

#[derive(Error, Debug)]
pub enum SetAccountError {
    #[error("account does not exist and cannot be set as the active account")]
    NoAccount,
}

impl From<EnrollmentError> for rspc::Error {
    fn from(value: EnrollmentError) -> Self {
        rspc::Error::new(
            ErrorCode::InternalServerError,
            format!("Account Query Error: {}", value),
        )
    }
}

impl From<DeleteAccountError> for rspc::Error {
    fn from(value: DeleteAccountError) -> Self {
        rspc::Error::new(
            ErrorCode::InternalServerError,
            format!("Account Delete Error: {}", value),
        )
    }
}

impl From<SetAccountError> for rspc::Error {
    fn from(value: SetAccountError) -> Self {
        rspc::Error::new(
            ErrorCode::InternalServerError,
            format!("Account Set Error: {}", value),
        )
    }
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
