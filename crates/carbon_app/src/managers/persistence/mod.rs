use crate::db::PrismaClient;
use log::trace;
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
}
