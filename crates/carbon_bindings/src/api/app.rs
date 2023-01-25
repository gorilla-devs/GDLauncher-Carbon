use std::sync::Arc;
use rspc::{Router, RouterBuilderLike};
use thiserror::Error;
use tokio::sync::{broadcast, RwLock, RwLockReadGuard};
use tokio::sync::broadcast::error::RecvError;
use crate::api::configuration::ConfigurationManager;
use crate::api::InvalidationEvent;
use crate::api::persistence::PersistenceManager;

pub type AppContainer = Arc<RwLock<App>>;
type AppComponentContainer<M> = Option<RwLock<M>>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("manager {0} not found")]
    ManagerNotFound(String)
}

pub struct App {
    //instances: Instances,
    configuration_manager: AppComponentContainer<ConfigurationManager>,
    persistence_manager: AppComponentContainer<PersistenceManager>,
    invalidation_channel: broadcast::Sender<InvalidationEvent>,
}



impl App {

    pub async fn new_with_invalidation_channel(invalidation_channel: broadcast::Sender<InvalidationEvent>) -> AppContainer
    {
        let app = Arc::new(RwLock::new(App{
            configuration_manager: None,
            persistence_manager: None,
            invalidation_channel,
        }));
        let persistence_manager = PersistenceManager::make_for_app(&app);
        let configuration_manager = ConfigurationManager::make_for_app(&app);
        app.write().await.persistence_manager = Some(RwLock::new(persistence_manager));
        app.write().await.configuration_manager = Some(RwLock::new(configuration_manager));
        app
    }

    pub(crate) async fn get_persistence_manager(&self) -> Result<RwLockReadGuard<PersistenceManager>, AppError>
    {
        Ok(
            self.persistence_manager.as_ref()
                .ok_or(AppError::ManagerNotFound("".to_string()))?
                .read().await
        )
    }

    pub fn invalidate(&self, key: impl Into<String>, args: Option<serde_json::Value>) {
        match self.invalidation_channel.send(InvalidationEvent::new(key, args))
        {
            Ok(_) => (),
            Err(e) => {
                println!("Error sending invalidation request: {}", e);
            }
        }
    }

    pub async fn wait_for_invalidation(&self)->Result<InvalidationEvent, RecvError> {
        self.invalidation_channel.subscribe().recv().await
    }

}


pub(super) fn mount() -> impl RouterBuilderLike<AppContainer> {
    Router::new()
        .query("getTheme", |t| {
            t(|_ctx: AppContainer, _args: ()| async move { Ok("main") })
        })
        .mutation("setTheme", |t| {
            t(|ctx: AppContainer, v: String| async move {
                ctx.read().await.invalidate("app.getTheme", None);
            })
        })
}
