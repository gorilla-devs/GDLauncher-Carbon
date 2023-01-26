use std::sync::{Arc, Weak};
use log::trace;
use prisma_client_rust::NewClientError;
use thiserror::Error;
use tokio::sync::RwLock;
use crate::app::App;
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
        trace!("retrieving db client");
        let client = crate::db::new_client().await?;
        trace!("db client correctly retrieved");
        Ok(client)
    }

}

