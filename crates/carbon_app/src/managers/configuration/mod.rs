use std::path::PathBuf;

use super::AppRef;
use crate::db;
use crate::db::app_configuration::SetParam::SetTheme;
use crate::db::app_configuration::UniqueWhereParam;
use log::trace;
use prisma_client_rust::QueryError;
use thiserror::Error;

pub mod runtime_path;

#[derive(Error, Debug)]
pub enum ConfigurationManagerError {
    #[error("error raised while executing query: ")]
    ThemeNotFound,
    #[error("error raised while executing query: {0}")]
    QueryError(#[from] QueryError),
    #[error("App configuration does not exist on DB")]
    AppConfigurationNotFound,
}

pub(crate) struct ConfigurationManager {
    app: AppRef,
    runtime_path: runtime_path::RuntimePath,
}

impl ConfigurationManager {
    pub fn new(runtime_path: PathBuf) -> Self {
        Self {
            app: AppRef::uninit(),
            runtime_path: runtime_path::RuntimePath::new(runtime_path),
        }
    }

    pub fn get_appref(&self) -> &AppRef {
        &self.app
    }

    pub async fn get_theme(&self) -> Result<String, ConfigurationManagerError> {
        trace!("retrieving current theme from db");
        let app_config = self
            .app
            .upgrade()
            .prisma_client
            .app_configuration()
            .find_unique(db::app_configuration::id::equals(0))
            .exec()
            .await?
            .ok_or(ConfigurationManagerError::AppConfigurationNotFound)?;

        let theme = app_config.theme;
        trace!("retrieved current theme from db : {theme}");
        Ok(theme)
    }

    pub async fn set_theme(&self, theme: String) -> Result<(), ConfigurationManagerError> {
        trace!("writing theme in db : {theme}");
        self.app
            .upgrade()
            .prisma_client
            .app_configuration()
            .update(UniqueWhereParam::IdEquals(0), vec![SetTheme(theme)])
            .exec()
            .await?;
        trace!("wrote theme into db");
        Ok(())
    }
}
