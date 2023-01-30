use crate::app::App;
use crate::db;
use crate::db::PrismaClient;
use log::trace;
use prisma_client_rust::NewClientError;
use std::{
    path::Path,
    sync::{Arc, Weak},
};
use thiserror::Error;
use tokio::sync::RwLock;

mod database;

#[derive(Error, Debug)]
pub enum PersistenceManagerError {}

pub(crate) struct PersistenceManager {
    app: Weak<RwLock<App>>,
    db_client: Arc<RwLock<PrismaClient>>,
}

impl PersistenceManager {
    pub async fn make_for_app(app: &Arc<RwLock<App>>) -> PersistenceManager {
        // TODO: don't unwrap. Managers should return a result
        let db_client = database::load_and_migrate().await.unwrap();

        PersistenceManager {
            app: Arc::downgrade(app),
            db_client: Arc::new(RwLock::new(db_client)),
        }
    }

    pub async fn get_db_client(&self) -> Arc<RwLock<PrismaClient>> {
        trace!("retrieving db client");
        self.db_client.clone()
    }
}
