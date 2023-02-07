use crate::db;
use crate::db::PrismaClient;
use crate::managers::ManagersInner;
use log::trace;
use prisma_client_rust::NewClientError;
use std::{
    path::Path,
    sync::{Arc, Weak},
};
use thiserror::Error;
use tokio::sync::RwLock;

use super::Managers;

mod database;

#[derive(Error, Debug)]
pub enum PersistenceManagerError {}

pub(crate) struct PersistenceManager {
    managers: Weak<ManagersInner>,
    db_client: Arc<PrismaClient>,
}

impl PersistenceManager {
    pub async fn make_for_app(app: &Managers) -> PersistenceManager {
        // TODO: don't unwrap. Managers should return a result
        let db_client = database::load_and_migrate().await.unwrap();

        PersistenceManager {
            managers: Arc::downgrade(app),
            db_client: Arc::new(db_client),
        }
    }

    pub async fn get_db_client(&self) -> Arc<PrismaClient> {
        trace!("retrieving db client");
        self.db_client.clone()
    }
}
