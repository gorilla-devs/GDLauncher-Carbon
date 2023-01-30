use std::sync::{Arc, Weak};
use thiserror::Error;
use tokio::sync::RwLock;
use crate::app::App;

#[derive(Error, Debug)]
pub enum JavaManagerError {
    #[error("app reference not found")]
    AppNotFoundError,

}

pub(crate) struct JavaInstanceManager {
    app: Weak<RwLock<App>>,
}

impl JavaInstanceManager {
    pub fn make_for_app(app: &Arc<RwLock<App>>) -> JavaInstanceManager {
        JavaInstanceManager {
            app: Arc::downgrade(app),
        }
    }



}
