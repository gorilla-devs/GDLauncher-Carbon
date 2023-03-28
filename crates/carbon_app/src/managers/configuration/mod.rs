use std::{path::PathBuf, sync::Arc};

use log::trace;
use thiserror::Error;

use crate::{
    api::keys::app::*,
    db::{app_configuration, PrismaClient},
};

use super::ManagerRef;

pub mod runtime_path;

pub(crate) struct ConfigurationManager {
    pub runtime_path: runtime_path::RuntimePath,
}

impl ConfigurationManager {
    pub fn new(runtime_path: PathBuf) -> Self {
        Self {
            runtime_path: runtime_path::RuntimePath::new(runtime_path),
        }
    }
}

impl ManagerRef<'_, ConfigurationManager> {
    pub async fn get_theme(self) -> anyhow::Result<String> {
        trace!("retrieving current theme from db");

        Ok(self.configuration().get().await?.theme)
    }

    pub async fn set_theme(self, theme: String) -> anyhow::Result<()> {
        use crate::db::app_configuration::SetParam::SetTheme;

        trace!("writing theme in db: {theme}");

        self.configuration().set(SetTheme(theme.clone())).await?;

        self.app.invalidate(GET_THEME, Some(theme.into()));

        Ok(())
    }

    pub fn configuration(self) -> Configuration {
        Configuration {
            client: self.app.prisma_client.clone(),
        }
    }
}

pub struct Configuration {
    client: Arc<PrismaClient>,
}

impl Configuration {
    pub async fn get(self) -> anyhow::Result<app_configuration::Data> {
        Ok(self
            .client
            .app_configuration()
            .find_unique(app_configuration::id::equals(0))
            .exec()
            .await?
            .ok_or(ConfigurationError::Missing)?)
    }

    pub async fn set(self, value: app_configuration::SetParam) -> anyhow::Result<()> {
        self.client
            .app_configuration()
            .update(
                app_configuration::UniqueWhereParam::IdEquals(0),
                vec![value],
            )
            .exec()
            .await?;

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum ConfigurationError {
    #[error("configuration row missing from gdl db")]
    Missing,
}
