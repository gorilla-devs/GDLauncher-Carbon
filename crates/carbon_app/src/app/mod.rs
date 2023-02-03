mod configuration;
mod persistence;

use crate::api::InvalidationEvent;
use crate::app::configuration::{ConfigurationManager, ConfigurationManagerError};
use crate::app::persistence::PersistenceManager;
use rspc::{ErrorCode, Router, RouterBuilderLike};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, RwLock, RwLockReadGuard};

pub type GlobalContext = Arc<RwLock<App>>;
type AppComponentContainer<M> = Option<RwLock<M>>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("manager {0} not found")]
    ManagerNotFound(String),
}

pub struct App {
    //instances: Instances,
    configuration_manager: AppComponentContainer<ConfigurationManager>,
    persistence_manager: AppComponentContainer<PersistenceManager>,
    invalidation_channel: broadcast::Sender<InvalidationEvent>,
}

impl App {
    pub async fn new_with_invalidation_channel(
        invalidation_channel: broadcast::Sender<InvalidationEvent>,
    ) -> GlobalContext {
        let app = Arc::new(RwLock::new(App {
            configuration_manager: None,
            persistence_manager: None,
            invalidation_channel,
        }));
        let configuration_manager = ConfigurationManager::make_for_app(&app);
        let persistence_manager = PersistenceManager::make_for_app(&app).await;
        app.write().await.persistence_manager = Some(RwLock::new(persistence_manager));
        app.write().await.configuration_manager = Some(RwLock::new(configuration_manager));
        app
    }

    pub(crate) async fn get_persistence_manager(
        &self,
    ) -> Result<RwLockReadGuard<PersistenceManager>, AppError> {
        Ok(self
            .persistence_manager
            .as_ref()
            .ok_or_else(|| AppError::ManagerNotFound("".to_string()))?
            .read()
            .await)
    }

    pub(crate) async fn get_configuration_manager(
        &self,
    ) -> Result<RwLockReadGuard<ConfigurationManager>, AppError> {
        Ok(self
            .configuration_manager
            .as_ref()
            .ok_or_else(|| AppError::ManagerNotFound("".to_string()))?
            .read()
            .await)
    }

    pub fn invalidate(&self, key: impl Into<String>, args: Option<serde_json::Value>) {
        match self
            .invalidation_channel
            .send(InvalidationEvent::new(key, args))
        {
            Ok(_) => (),
            Err(e) => {
                println!("Error sending invalidation request: {}", e);
            }
        }
    }

    pub async fn wait_for_invalidation(&self) -> Result<InvalidationEvent, RecvError> {
        self.invalidation_channel.subscribe().recv().await
    }
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("configuration error raised : ${0}")]
    ConfigurationManagerError(#[from] ConfigurationManagerError),

    #[error("app not found in ctx")]
    AppNotFound(),
}

impl Into<rspc::Error> for ApiError {
    fn into(self) -> rspc::Error {
        rspc::Error::new(ErrorCode::InternalServerError, format!("{:?}", self))
    }
}

impl Into<rspc::Error> for ConfigurationManagerError {
    fn into(self) -> rspc::Error {
        rspc::Error::new(ErrorCode::InternalServerError, format!("{:?}", self))
    }
}

pub(super) fn mount() -> impl RouterBuilderLike<GlobalContext> {
    Router::new()
        .query("getTheme", |t| {
            t(|app: GlobalContext, _args: ()| async move {
                let app = app.read().await;
                let configuration_manager =
                    app.get_configuration_manager().await.map_err(|error| {
                        rspc::Error::new(ErrorCode::InternalServerError, format!("{:?}", error))
                    })?;
                configuration_manager
                    .get_theme()
                    .await
                    .map_err(|error| error.into())
            })
        })
        .mutation("setTheme", |t| {
            t(|app: GlobalContext, new_theme: String| async move {
                let app = app.read().await;
                let configuration_manager =
                    app.get_configuration_manager().await.map_err(|error| {
                        rspc::Error::new(ErrorCode::InternalServerError, format!("{:?}", error))
                    })?;
                configuration_manager
                    .set_theme(new_theme.clone())
                    .await
                    .map_err(|error| {
                        rspc::Error::new(ErrorCode::InternalServerError, format!("{:?}", error))
                    })?;
                app.invalidate("app.getTheme", Some(new_theme.into()));
                Ok(())
            })
        })
}

// mod test {
//     use crate::app::App;
//     use env_logger::Builder;
//     use log::{trace, LevelFilter};
//     use std::time::Duration;

//     #[tokio::test]
//     #[tracing_test::traced_test]
//     async fn read_write_theme_ok() {
//         Builder::new().filter_level(LevelFilter::Trace).init();

//         let theme = "super good theme";
//         trace!("trying write theme {}", theme);

//         let (invalidation_channel, _) = tokio::sync::broadcast::channel(1);

//         let app = App::new_with_invalidation_channel(invalidation_channel).await;
//         let app = app.read().await;

//         let configuration_manager = app.configuration_manager.as_ref().expect("");
//         let configuration_manager = configuration_manager.read().await;

//         configuration_manager
//             .set_theme(theme.to_string())
//             .await
//             .expect("unable to write theme");

//         let read_theme = configuration_manager
//             .get_theme()
//             .await
//             .expect("unable to read theme");

//         assert_eq!(read_theme, theme);

//         trace!("read correctly theme from configuration");
//     }
// }
