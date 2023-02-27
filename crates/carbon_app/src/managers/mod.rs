use crate::api::keys::{app::*, Key};
use crate::api::router::router;
use crate::api::InvalidationEvent;
use crate::db::PrismaClient;
use crate::error;
use crate::managers::configuration::ConfigurationManager;
use rspc::RouterBuilderLike;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Weak};
use thiserror::Error;
use tokio::sync::broadcast::{self, error::RecvError};

use self::account::AccountManager;
use self::minecraft::MinecraftManager;

pub mod account;
mod configuration;
mod minecraft;
mod prisma_client;

pub type Managers = Arc<ManagersInner>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("manager {0} not found")]
    ManagerNotFound(String),
}

pub struct ManagersInner {
    //instances: Instances,
    configuration_manager: ConfigurationManager,
    minecraft_manager: MinecraftManager,
    account_manager: AccountManager,
    invalidation_channel: broadcast::Sender<InvalidationEvent>,
    pub(crate) reqwest_client: reqwest::Client,
    pub(crate) prisma_client: Arc<PrismaClient>,
}

pub struct ManagerRef<'a, T> {
    manager: &'a T,
    pub app: &'a Arc<ManagersInner>,
}

impl<T> Copy for ManagerRef<'_, T> {}
impl<T> Clone for ManagerRef<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Deref for ManagerRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.manager
    }
}

pub struct AppRef(pub Weak<ManagersInner>);

impl AppRef {
    pub fn upgrade(&self) -> Managers {
        self.0
            .upgrade()
            .expect("App was dropped before its final usage")
    }
}

macro_rules! manager_getter {
    ($manager:ident: $type:path) => {
        pub(crate) fn $manager<'a>(self: &'a Arc<Self>) -> ManagerRef<'a, $type> {
            ManagerRef {
                manager: &self.$manager,
                app: &self,
            }
        }
    };
}

impl ManagersInner {
    pub async fn new(
        invalidation_channel: broadcast::Sender<InvalidationEvent>,
        runtime_path: PathBuf,
    ) -> Managers {
        let db_client = prisma_client::load_and_migrate().await.unwrap();

        let app = Arc::new(ManagersInner {
            configuration_manager: ConfigurationManager::new(runtime_path),
            minecraft_manager: MinecraftManager::new(),
            account_manager: AccountManager::new(),
            invalidation_channel,
            reqwest_client: reqwest::Client::new(),
            prisma_client: Arc::new(db_client),
        });

        app
    }

    manager_getter!(configuration_manager: ConfigurationManager);
    manager_getter!(minecraft_manager: MinecraftManager);
    manager_getter!(account_manager: AccountManager);

    pub fn invalidate(&self, key: Key, args: Option<serde_json::Value>) {
        match self
            .invalidation_channel
            .send(InvalidationEvent::new(key.full, args))
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

pub(super) fn mount() -> impl RouterBuilderLike<Managers> {
    router! {
        query GET_THEME[app, _args: ()] {
            app.configuration_manager()
                .get_theme()
                .await
                .map_err(error::into_rspc)
        }

        mutation SET_THEME[app, new_theme: String] {
            app.configuration_manager()
                .set_theme(new_theme.clone())
                .await
                .map_err(error::into_rspc)?;
            Ok(())
        }
    }
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
