use std::sync::{Arc, Weak};
use prisma_client_rust::NewClientError;
use thiserror::Error;
use tokio::sync::RwLock;
use crate::api::app::App;
use crate::db::PrismaClient;

#[derive(Error, Debug)]
pub enum  PersistenceManagerError{
    #[error("error raised while trying to build the client for DB : {0}")]
    ClientError(#[from]NewClientError)
}

#[derive(Default, Clone)]
pub(crate) struct PersistenceManager{
    app: Weak<RwLock<App>>
}

impl PersistenceManager{

    pub fn make_for_app(app: &Arc<RwLock<App>>) -> PersistenceManager{
        PersistenceManager{
            app: Arc::downgrade(app),
        }
    }

    pub async fn get_db_client(&self) -> Result<PrismaClient, PersistenceManagerError>{
        Ok(crate::db::new_client().await?)
    }

}

