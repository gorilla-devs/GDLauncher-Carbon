use crate::app::App;
use std::sync::{Arc, Weak};
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Error, Debug)]
pub enum JavaManagerError {
    #[error("app reference not found")]
    AppNotFoundError,
}

pub(crate) struct MinecraftModManager {
    app: Weak<RwLock<App>>,
}

impl MinecraftModManager {
    pub fn make_for_app(app: &Arc<RwLock<App>>) -> MinecraftModManager {
        MinecraftModManager {
            app: Arc::downgrade(app),
        }
    }
}
