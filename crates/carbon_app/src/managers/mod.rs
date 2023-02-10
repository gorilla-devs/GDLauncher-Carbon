use crate::api::keys::{app::*, Key};
use crate::api::router::router;
use crate::api::InvalidationEvent;
use crate::managers::instance::InstanceManager;
use crate::managers::persistence::PersistenceManager;
use crate::managers::settings::{ConfigurationManager, ConfigurationManagerError};
use rspc::{ErrorCode, RouterBuilderLike};
use std::cell::UnsafeCell;
use std::sync::{Arc, Weak};
use thiserror::Error;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

use self::minecraft::MinecraftManager;

mod instance;
mod minecraft;
mod persistence;
mod settings;

pub type Managers = Arc<ManagersInner>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("manager {0} not found")]
    ManagerNotFound(String),
}

pub struct ManagersInner {
    instance_manager: InstanceManager,
    configuration_manager: ConfigurationManager,
    persistence_manager: PersistenceManager,
    minecraft_manager: MinecraftManager,
    invalidation_channel: broadcast::Sender<InvalidationEvent>,
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

impl ManagersInner {
    pub async fn new_with_invalidation_channel(
        invalidation_channel: broadcast::Sender<InvalidationEvent>,
    ) -> Managers {
        let app = Arc::new(ManagersInner {
            instance_manager: InstanceManager::new(),
            configuration_manager: ConfigurationManager::new(),
            persistence_manager: PersistenceManager::new().await,
            minecraft_manager: MinecraftManager::new(),
            invalidation_channel,
        });

        let weak = Arc::downgrade(&app);

        // SAFETY: This is safe as long as `get_appref` only returns
        // the appref, without doing anything else, and `new` does
        // not spawn tasks that may access `app`.
        // Before this block, attempting to access the appref will
        // panic, and after this block it will be safe. The appref
        // CANNOT be safely accessed inside of this block.
        unsafe {
            app.instance_manager.get_appref().init(weak.clone());
            app.configuration_manager.get_appref().init(weak.clone());
            app.persistence_manager.get_appref().init(weak.clone());
            app.minecraft_manager.get_appref().init(weak.clone());
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

pub(super) fn mount() -> impl RouterBuilderLike<Managers> {
    router! {
        query GET_THEME[app, _args: ()] {
            app.configuration_manager
                .get_theme()
                .await
                .map_err(|error| error.into())
        }

        mutation SET_THEME[app, new_theme: String] {
            app.configuration_manager
                .set_theme(new_theme.clone())
                .await
                .map_err(|error| {
                    rspc::Error::new(ErrorCode::InternalServerError, format!("{:?}", error))
                })?;
            app.invalidate(GET_THEME, Some(new_theme.into()));
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
