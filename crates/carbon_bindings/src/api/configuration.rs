use std::sync::{Arc, Weak};
use prisma_client_rust::QueryError;
use serde::{Deserialize, Serialize};
use crate::api::app::{App, AppError};
use thiserror::Error;
use tokio::sync::RwLock;
use crate::api::configuration::ConfigurationManagerError::{AppConfigurationNotFound, AppNotFoundError, ThemeNotFound};
use crate::api::persistence::PersistenceManagerError;
use crate::db::app_configuration::{SetParam, UniqueWhereParam};

#[derive(Error, Debug)]
pub enum ConfigurationManagerError {

    #[error("app reference not found")]
    AppNotFoundError,

    #[error("app raised an error : {0}")]
    AppError(#[from]AppError),

    #[error("error raised while executing query : ")]
    ThemeNotFound,

    #[error("error raised while executing query ")]
    AppConfigurationNotFound,

    #[error("error : {0}")]
    PersistenceManagerError(#[from] PersistenceManagerError),

    #[error("error raised while executing query : {0}")]
    QueryError(#[from] QueryError),

}

#[derive(Serialize, Deserialize, Ord, PartialOrd, PartialEq, Eq)]
pub struct AppConfiguration {
    pub default_db_url: String,
    pub app_theme: String,
}

pub(crate) struct ConfigurationManager {
    app: Weak<RwLock<App>>,
}

impl ConfigurationManager {

    pub fn make_for_app(app: &Arc<RwLock<App>>) -> ConfigurationManager {
        ConfigurationManager {
            app: Arc::downgrade(app),
        }
    }

    pub async fn get_theme(&self) -> Result<String, ConfigurationManagerError> {
        let app = self.app.upgrade()
            .ok_or(AppNotFoundError)?;
        let app = app.read().await;
        let persistence_manager = app.get_persistence_manager().await?;
        persistence_manager.get_db_client()
            .await?
            .app_configuration()
            .find_first(vec![])
            .exec()
            .await?
            .ok_or(AppConfigurationNotFound)?
            .theme
            .ok_or(ThemeNotFound)

    }

    pub async fn set_theme(&self, theme: String) -> Result<(), ConfigurationManagerError> {
        let app = self.app.upgrade()
            .ok_or(AppNotFoundError)?;
        let app = app.read().await;
        let persistence_manager = app.get_persistence_manager().await?;
        persistence_manager.get_db_client()
            .await?
            .app_configuration()
            .upsert(UniqueWhereParam::IdEquals(0), vec![],vec![SetParam::SetTheme(theme.into())])
            .exec()
            .await?;
        Ok(())
    }

}
