use crate::db;
use crate::db::app_configuration::SetParam::SetTheme;
use crate::db::app_configuration::UniqueWhereParam;
use crate::managers::persistence::PersistenceManagerError;
use crate::managers::settings::ConfigurationManagerError::{
    AppConfigurationNotFound, AppNotFoundError,
};
use crate::managers::{AppError, ManagersInner};
use log::trace;
use prisma_client_rust::QueryError;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Weak};
use thiserror::Error;

use super::Managers;

#[derive(Error, Debug)]
pub enum ConfigurationManagerError {
    #[error("app reference not found")]
    AppNotFoundError,

    #[error("app raised an error : {0}")]
    AppError(#[from] AppError),

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
    app: Weak<ManagersInner>,
}

impl ConfigurationManager {
    pub fn make_for_app(app: &Managers) -> ConfigurationManager {
        ConfigurationManager {
            app: Arc::downgrade(app),
        }
    }

    fn get_app(&self) -> Result<Managers, ConfigurationManagerError> {
        self.app.upgrade().ok_or(AppNotFoundError)
    }

    pub async fn get_theme(&self) -> Result<String, ConfigurationManagerError> {
        trace!("retrieving current theme from db");
        let app = self.app.upgrade().ok_or(AppNotFoundError)?;
        let persistence_manager = app.get_persistence_manager().await?;
        let app_config = persistence_manager
            .get_db_client()
            .await
            .app_configuration()
            .find_unique(db::app_configuration::id::equals(0))
            .exec()
            .await?
            .ok_or(AppConfigurationNotFound)?;
        let theme = app_config.theme;
        trace!("retrieved current theme from db : {theme}");
        Ok(theme)
    }

    pub async fn set_theme(&self, theme: String) -> Result<(), ConfigurationManagerError> {
        trace!("writing theme in db : {theme}");
        let app = self.app.upgrade().ok_or(AppNotFoundError)?;
        let persistence_manager = app.get_persistence_manager().await?;
        persistence_manager
            .get_db_client()
            .await
            .app_configuration()
            .update(UniqueWhereParam::IdEquals(0), vec![SetTheme(theme)])
            .exec()
            .await?;
        trace!("wrote theme into db");
        Ok(())
    }
}
