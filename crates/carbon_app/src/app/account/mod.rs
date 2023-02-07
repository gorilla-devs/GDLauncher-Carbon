use crate::{
    app::{account::enroll::InvalidateCtx, App},
    db,
};
use async_trait::async_trait;
use carbon_domain::account::*;
use chrono::{FixedOffset, Utc};
use prisma_client_rust::{chrono::DateTime, QueryError};
use rspc::ErrorCode;
use std::{
    mem,
    sync::{Arc, Weak},
};

use thiserror::Error;
use tokio::sync::RwLock;

use self::{
    api::DeviceCode,
    enroll::{EnrollmentStatus, EnrollmentTask},
};

use super::AppError;

mod api;
mod enroll;

pub(crate) struct AccountManager {
    app: Weak<RwLock<App>>,
    currently_refreshing: RwLock<Vec<String>>,
    active_enrollment: RwLock<Option<EnrollmentTask>>,
}

impl AccountManager {
    pub fn make_for_app(app: &Arc<RwLock<App>>) -> Self {
        Self {
            app: Arc::downgrade(app),
            currently_refreshing: RwLock::new(Vec::new()),
            active_enrollment: RwLock::new(None),
        }
    }

    pub async fn get_active_uuid(&self) -> Result<Option<String>, AccountError> {
        let app = self.app.upgrade().ok_or(AccountError::AppNotFound)?;
        let app = app.read().await;
        let persistence_manager = app.get_persistence_manager().await?;

        Ok(persistence_manager
            .get_db_client()
            .await
            .read()
            .await
            .app_configuration()
            .find_unique(db::app_configuration::id::equals(0))
            .exec()
            .await?
            .ok_or(AccountError::AppConfigurationNotFound)?
            .active_account_uuid)
    }

    pub async fn set_active_uuid(&self, uuid: Option<String>) -> Result<(), AccountError> {
        use db::app_configuration::{SetParam::SetActiveAccountUuid, UniqueWhereParam};

        let app = self.app.upgrade().ok_or(AccountError::AppNotFound)?;
        let app = app.read().await;
        let persistence_manager = app.get_persistence_manager().await?;

        persistence_manager
            .get_db_client()
            .await
            .read()
            .await
            .app_configuration()
            .update(
                UniqueWhereParam::IdEquals(0),
                vec![SetActiveAccountUuid(uuid)],
            )
            .exec()
            .await?;

        // TODO: invalidate get_active_uuid
        Ok(())
    }

    async fn get_account_entries(&self) -> Result<Vec<db::account::Data>, AccountError> {
        let app = self.app.upgrade().ok_or(AccountError::AppNotFound)?;
        let app = app.read().await;
        let persistence_manager = app.get_persistence_manager().await?;

        Ok(persistence_manager
            .get_db_client()
            .await
            .read()
            .await
            .account()
            .find_many(Vec::new())
            .exec()
            .await?)
    }

    pub async fn get_account_list(&self) -> Result<Vec<Account>, AccountError> {
        let accounts = self.get_account_entries().await?;

        accounts
            .into_iter()
            .map(|account| {
                let type_ = match &account.ms_refresh_token {
                    None => AccountType::Offline,
                    Some(_) => AccountType::Microsoft,
                };

                Ok(Account {
                    username: account.username,
                    uuid: account.uuid,
                    type_,
                })
            })
            .collect::<Result<_, _>>()
    }

    pub async fn get_account_status(
        &self,
        uuid: String,
    ) -> Result<Option<AccountStatus>, AccountError> {
        use db::account::UniqueWhereParam;

        let app = self.app.upgrade().ok_or(AccountError::AppNotFound)?;
        let app = app.read().await;
        let persistence_manager = app.get_persistence_manager().await?;

        let account = persistence_manager
            .get_db_client()
            .await
            .read()
            .await
            .account()
            .find_unique(UniqueWhereParam::UuidEquals(uuid))
            .exec()
            .await?;

        let status = match account {
            Some(account) => Some(match account.ms_refresh_token {
                None => AccountStatus::Launchable { access_token: None },
                Some(_) => {
                    let token_expires = account
                        .token_expires
                        .ok_or(AccountError::DbError(AccountDbError::ExpiryUnset))?;

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
                            .ok_or(AccountError::DbError(AccountDbError::TokenUnset))?;

                        AccountStatus::Launchable {
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

    async fn add_account(&self, account: FullAccount) -> Result<(), AccountError> {
        use db::account::SetParam;

        let app = self.app.upgrade().ok_or(AccountError::AppNotFound)?;
        let app = app.read().await;
        let persistence_manager = app.get_persistence_manager().await?;

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

        persistence_manager
            .get_db_client()
            .await
            .read()
            .await
            .account()
            .create(account.uuid, account.username, set_params)
            .exec()
            .await?;

        // TODO: invalidate get_account_list
        Ok(())
    }

    pub async fn delete_account(&self, uuid: String) -> Result<(), AccountError> {
        use db::account::UniqueWhereParam;

        let app = self.app.upgrade().ok_or(AccountError::AppNotFound)?;
        let app = app.read().await;
        let persistence_manager = app.get_persistence_manager().await?;

        persistence_manager
            .get_db_client()
            .await
            .read()
            .await
            .account()
            .delete(UniqueWhereParam::UuidEquals(uuid))
            .exec()
            .await?;

        // TODO: invalidate get_account_list, get_account_status
        Ok(())
    }

    pub async fn begin_enrollment(&self) -> Result<(), EnrollmentError> {
        match &*self.active_enrollment.read().await {
            Some(_) => Err(EnrollmentError::InProgress),
            None => {
                let client = self
                    .app
                    .upgrade()
                    .unwrap() // waiting on rwlock/weak removal PR
                    .read()
                    .await
                    .reqwest_client
                    .clone();

                struct Invalidator(Weak<RwLock<App>>);

                #[async_trait]
                impl InvalidateCtx for Invalidator {
                    async fn invalidate(&self) {
                        // TODO: invalidate status endpoint
                    }
                }

                let enrollment = EnrollmentTask::begin(client, Invalidator(self.app.clone()));

                *self.active_enrollment.write().await = Some(enrollment);

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

    pub async fn finalize_enrollment(&self) -> Result<(), EnrollmentError> {
        let enrollment = self.active_enrollment.write().await.take();

        match enrollment {
            None => Err(EnrollmentError::NotActive),
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
                    _ => Err(EnrollmentError::NotComplete),
                }
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum AccountError {
    #[error("app reference not found")]
    AppNotFound,

    #[error("app configuration not found")]
    AppConfigurationNotFound,

    #[error("app raised an error : {0}")]
    AppError(#[from] AppError),

    #[error("executed invalid query: {0}")]
    QueryError(#[from] QueryError),

    #[error("database error: {0}")]
    DbError(#[from] AccountDbError),
}

#[derive(Error, Debug)]
pub enum AccountDbError {
    #[error("ms account access token unset")]
    TokenUnset,

    #[error("ms account access token exiry date unset")]
    ExpiryUnset,
}

impl From<AccountError> for rspc::Error {
    fn from(value: AccountError) -> Self {
        rspc::Error::new(
            ErrorCode::InternalServerError,
            format!("Account Query Error: {}", value),
        )
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

    #[error("account error: {0}")]
    AccountError(#[from] AccountError),
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
