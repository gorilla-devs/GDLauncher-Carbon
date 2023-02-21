use log::trace;
use serde::{Deserialize, Serialize};

use crate::api::keys::app::*;

use super::{persistence::ConfigurationError, AppRef};

#[derive(Serialize, Deserialize, Ord, PartialOrd, PartialEq, Eq)]
pub struct AppConfiguration {
    pub default_db_url: String,
    pub app_theme: String,
}

pub(crate) struct ConfigurationManager {
    app: AppRef,
}

impl ConfigurationManager {
    pub fn new() -> Self {
        Self {
            app: AppRef::uninit(),
        }
    }

    pub fn get_appref(&self) -> &AppRef {
        &self.app
    }

    pub async fn get_theme(&self) -> Result<String, ConfigurationError> {
        trace!("retrieving current theme from db");

        Ok(self
            .app
            .upgrade()
            .persistence_manager
            .configuration()
            .get()
            .await?
            .theme)
    }

    pub async fn set_theme(&self, theme: String) -> Result<(), ConfigurationError> {
        use crate::db::app_configuration::SetParam::SetTheme;

        trace!("writing theme in db: {theme}");

        self.app
            .upgrade()
            .persistence_manager
            .configuration()
            .set(SetTheme(theme.clone()))
            .await?;

        self.app.upgrade().invalidate(GET_THEME, Some(theme.into()));

        Ok(())
    }
}
