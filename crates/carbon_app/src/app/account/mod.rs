use crate::{app::App, db};
use carbon_domain::account::*;
use chrono::{FixedOffset, Utc};
use prisma_client_rust::{chrono::DateTime, QueryError};
use rspc::ErrorCode;
use std::sync::{Arc, Weak};

use thiserror::Error;
use tokio::sync::RwLock;

use super::AppError;

mod api;
mod enroll;

pub(crate) struct AccountManager {
    app: Weak<RwLock<App>>,
    currently_refreshing: RwLock<Vec<String>>,
}

impl AccountManager {
    pub fn make_for_app(app: &Arc<RwLock<App>>) -> Self {
        Self {
            app: Arc::downgrade(app),
            currently_refreshing: RwLock::new(Vec::new()),
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
