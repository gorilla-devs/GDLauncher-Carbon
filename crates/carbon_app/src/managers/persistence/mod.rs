use crate::db::{app_configuration, PrismaClient};
use log::trace;
use prisma_client_rust::QueryError;
use std::sync::Arc;
use thiserror::Error;

use super::AppRef;

mod database;

#[derive(Error, Debug)]
pub enum PersistenceManagerError {}

pub(crate) struct PersistenceManager {
    app: AppRef,
    db_client: Arc<PrismaClient>,
}

impl PersistenceManager {
    pub async fn new() -> Self {
        // TODO: don't unwrap. Managers should return a result
        let db_client = database::load_and_migrate().await.unwrap();

        Self {
            app: AppRef::uninit(),
            db_client: Arc::new(db_client),
        }
    }

    pub fn get_appref(&self) -> &AppRef {
        &self.app
    }

    pub async fn get_db_client(&self) -> Arc<PrismaClient> {
        trace!("retrieving db client");
        self.db_client.clone()
    }

    pub fn configuration(&self) -> Configuration {
        Configuration {
            persistance_manager: self,
        }
    }
}

pub struct Configuration<'a> {
    persistance_manager: &'a PersistenceManager,
}

impl Configuration<'_> {
    pub async fn get(self) -> Result<app_configuration::Data, ConfigurationError> {
        Ok(self
            .persistance_manager
            .get_db_client()
            .await
            .app_configuration()
            .find_unique(app_configuration::id::equals(0))
            .exec()
            .await?
            .ok_or(ConfigurationError::Missing)?)
    }

    pub async fn set(self, value: app_configuration::SetParam) -> Result<(), ConfigurationError> {
        self.persistance_manager
            .get_db_client()
            .await
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
    #[error("configuration row missing from DB")]
    Missing,

    #[error("query error: {0}")]
    Query(#[from] QueryError),
}
