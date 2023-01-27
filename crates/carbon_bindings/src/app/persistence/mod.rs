use std::sync::{Arc, Weak};
use log::trace;
use prisma_client_rust::NewClientError;
use thiserror::Error;
use tokio::sync::RwLock;
use crate::app::App;
use crate::db;
use crate::db::PrismaClient;

#[derive(Error, Debug)]
pub enum PersistenceManagerError {
    #[error("error raised while trying to build the client for DB : {0}")]
    ClientError(#[from]NewClientError)
}

pub(crate) struct PersistenceManager {
    app: Weak<RwLock<App>>,
    db_client: Arc<RwLock<PrismaClient>>,
}

impl PersistenceManager {
    pub async fn make_for_app(app: &Arc<RwLock<App>>) -> PersistenceManager {
        let db_client = db::new_client().await.expect("unable to connect to db!");
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

