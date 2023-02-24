use crate::api::keys::{app::*, Key};
use crate::api::router::router;
use crate::api::InvalidationEvent;
use crate::db::PrismaClient;
use crate::error;
use crate::managers::configuration::ConfigurationManager;
use rspc::RouterBuilderLike;
use std::cell::UnsafeCell;
use std::path::PathBuf;
use std::sync::{Arc, Weak};
use thiserror::Error;
use tokio::sync::broadcast::{self, error::RecvError};

use self::account::AccountManager;
use self::minecraft::MinecraftManager;
use self::queue::TaskQueue;

pub mod account;
mod configuration;
pub mod download;
mod minecraft;
mod prisma_client;
pub mod queue;

pub type Managers = Arc<ManagersInner>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("manager {0} not found")]
    ManagerNotFound(String),
}

pub struct ManagersInner {
    //instances: Instances,
    pub(crate) configuration_manager: ConfigurationManager,
    pub(crate) minecraft_manager: MinecraftManager,
    pub(crate) account_manager: AccountManager,
    invalidation_channel: broadcast::Sender<InvalidationEvent>,
    pub(crate) reqwest_client: reqwest::Client,
    pub(crate) prisma_client: Arc<PrismaClient>,
    pub(crate) task_queue: TaskQueue,
}

pub struct AppRef(UnsafeCell<Option<Weak<ManagersInner>>>);

unsafe impl Send for AppRef {}
unsafe impl Sync for AppRef {}

impl AppRef {
    pub fn uninit() -> Self {
        Self(UnsafeCell::new(None))
    }

    /// # Safety
    /// This function is safe to call only during the manager init.
    /// No managers may spawn tasks that can depend on the app
    /// during `new`.
    pub unsafe fn init(&self, app: Weak<ManagersInner>) {
        *self.0.get() = Some(app);
    }

    fn ref_inner(&self) -> &Option<Weak<ManagersInner>> {
        // Safety is enforced by Self::init's invariants
        unsafe { &*self.0.get() }
    }

    pub fn upgrade(&self) -> Managers {
        self.ref_inner()
            .as_ref()
            .expect("App was used before initialization")
            .upgrade()
            .expect("App was dropped before its final usage")
    }
}

impl Clone for AppRef {
    fn clone(&self) -> Self {
        Self(UnsafeCell::new(self.ref_inner().clone()))
    }
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
            task_queue: TaskQueue::new(2 /* todo: download slots */),
        });

        let weak = Arc::downgrade(&app);

        // SAFETY: This is safe as long as `get_appref` only returns
        // the appref, without doing anything else, and `new` does
        // not spawn tasks that may access `app`.
        // Before this block, attempting to access the appref will
        // panic, and after this block it will be safe. The appref
        // CANNOT be safely accessed inside of this block.
        unsafe {
            app.configuration_manager.get_appref().init(weak.clone());
            app.minecraft_manager.get_appref().init(weak.clone());
            app.account_manager.get_appref().init(weak);
        }

        app
    }

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
            app.configuration_manager
                .get_theme()
                .await
                .map_err(error::into_rspc)
        }

        mutation SET_THEME[app, new_theme: String] {
            app.configuration_manager
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
