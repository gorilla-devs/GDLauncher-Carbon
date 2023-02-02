pub(crate) mod configuration;
pub(crate) mod instance;
pub(crate) mod java;
pub(crate) mod minecraft_mod;
pub(crate) mod representation;

use crate::api::InvalidationEvent;
use crate::app::configuration::{ConfigurationManager, ConfigurationManagerError};
use crate::app::instance::InstanceManager;
use crate::app::java::JavaInstanceManager;
use crate::db;
use crate::db::PrismaClient;
use carbon_macro::gd_launcher_app;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, RwLock, RwLockReadGuard};

pub type AppContainer = Arc<RwLock<App>>;
type AppComponentContainer<M> = Option<RwLock<M>>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("app component named {0} not found")]
    ComponentIsMissing(String),
}

#[gd_launcher_app]
pub struct App {
    instance_manager: AppComponentContainer<InstanceManager>,
    configuration_manager: AppComponentContainer<ConfigurationManager>,
    java_manager: AppComponentContainer<JavaInstanceManager>,
    db_client: AppComponentContainer<PrismaClient>,
    invalidation_channel: broadcast::Sender<InvalidationEvent>,
}

impl App {
    pub async fn new_with_invalidation_channel(
        invalidation_channel: broadcast::Sender<InvalidationEvent>,
    ) -> AppContainer {
        let app = Arc::new(RwLock::new(App {
            instance_manager: None,
            configuration_manager: None,
            java_manager: None,
            db_client: None,
            invalidation_channel,
        }));
        let java_manager = JavaInstanceManager::make_for_app(&app);
        let configuration_manager = ConfigurationManager::make_for_app(&app);
        let instance_manager = InstanceManager::make_for_app(&app);
        let db_client = db::new_client().await.expect("unable to connect to db!");
        app.write().await.java_manager = Some(RwLock::new(java_manager));
        app.write().await.configuration_manager = Some(RwLock::new(configuration_manager));
        app.write().await.instance_manager = Some(RwLock::new(instance_manager));
        app.write().await.db_client = Some(RwLock::new(db_client));
        app
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

mod test {
    use crate::app::App;
    use env_logger::Builder;
    use log::{trace, LevelFilter};
    use std::time::Duration;

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn read_write_theme_ok() {
        Builder::new().filter_level(LevelFilter::Trace).init();

        let theme = "super good theme";
        trace!("trying write theme {}", theme);

        let (invalidation_channel, _) = tokio::sync::broadcast::channel(1);
        let app = App::new_with_invalidation_channel(invalidation_channel).await;
        let app = app.read().await;

        let configuration_manager = app.configuration_manager.as_ref().expect("");
        let configuration_manager = configuration_manager.read().await;

        configuration_manager
            .set_theme(theme.to_string())
            .await
            .expect("unable to write theme");

        let read_theme = configuration_manager
            .get_theme()
            .await
            .expect("unable to read theme");

        assert_eq!(read_theme, theme);

        trace!("read correctly theme from configuration");
    }
}
