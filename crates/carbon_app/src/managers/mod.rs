use crate::api::keys::Key;
use crate::api::InvalidationEvent;
use crate::db::PrismaClient;

use crate::managers::settings::SettingsManager;
use std::cell::UnsafeCell;

use std::mem::MaybeUninit;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Weak};
use thiserror::Error;

use tokio::sync::broadcast::{self, error::RecvError};

use self::account::AccountManager;
use self::download::DownloadManager;
use self::minecraft::MinecraftManager;
use self::vtask::VisualTaskManager;

pub mod account;
mod cache_manager;
pub mod download;
pub mod java;
mod metrics;
mod minecraft;
mod modplatforms;
mod prisma_client;
mod settings;
pub mod vtask;

pub type App = Arc<AppInner>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("manager {0} not found")]
    ManagerNotFound(String),
}

pub const GDL_API_BASE: &str = "https://api.gdlauncher.com";

mod app {
    use super::{java::JavaManager, metrics::MetricsManager, modplatforms::ModplatformsManager, *};

    pub struct AppInner {
        settings_manager: SettingsManager,
        java_manager: JavaManager,
        pub(crate) minecraft_manager: MinecraftManager,
        account_manager: AccountManager,
        invalidation_channel: broadcast::Sender<InvalidationEvent>,
        download_manager: DownloadManager,
        pub(crate) metrics_manager: MetricsManager,
        pub(crate) modplatforms_manager: ModplatformsManager,
        pub(crate) reqwest_client: reqwest_middleware::ClientWithMiddleware,
        pub(crate) prisma_client: Arc<PrismaClient>,
        pub(crate) task_manager: VisualTaskManager,
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

    impl AppInner {
        pub async fn new(
            invalidation_channel: broadcast::Sender<InvalidationEvent>,
            runtime_path: PathBuf,
        ) -> App {
            let db_client = prisma_client::load_and_migrate(runtime_path.clone())
                .await
                .unwrap();

            let app = Arc::new(UnsafeCell::new(MaybeUninit::<AppInner>::uninit()));
            let unsaferef = UnsafeAppRef(Arc::downgrade(&app));

            // SAFETY: cannot be used until after the ref is initialized.
            let reqwest = cache_manager::new_client(unsaferef);

            let app = unsafe {
                let inner = Arc::into_raw(app);

                (*inner).get().write(MaybeUninit::new(AppInner {
                    settings_manager: SettingsManager::new(runtime_path),
                    java_manager: JavaManager::new(),
                    minecraft_manager: MinecraftManager::new(),
                    account_manager: AccountManager::new(),
                    modplatforms_manager: ModplatformsManager::new(),
                    download_manager: DownloadManager::new(),
                    metrics_manager: MetricsManager::new(),
                    invalidation_channel,
                    reqwest_client: reqwest,
                    prisma_client: Arc::new(db_client),
                    task_manager: VisualTaskManager::new(),
                }));

                // SAFETY: This pointer cast is safe because UnsafeCell and MaybeUninit do not
                // change the repr of their contained type.
                Arc::from_raw(inner.cast::<AppInner>())
            };

            account::AccountRefreshService::start(Arc::downgrade(&app));

            app
        }

        manager_getter!(metrics_manager: MetricsManager);
        manager_getter!(modplatforms_manager: ModplatformsManager);
        manager_getter!(settings_manager: SettingsManager);
        manager_getter!(java_manager: JavaManager);
        manager_getter!(minecraft_manager: MinecraftManager);
        manager_getter!(account_manager: AccountManager);
        manager_getter!(download_manager: DownloadManager);
        manager_getter!(task_manager: VisualTaskManager);

        pub fn invalidate(&self, key: Key, args: Option<serde_json::Value>) {
            match self
                .invalidation_channel
                .send(InvalidationEvent::new(key.full, args))
            {
                Ok(_) => (),
                Err(e) => {
                    println!("Error sending invalidation request: {e}");
                }
            }
        }

        pub async fn wait_for_invalidation(&self) -> Result<InvalidationEvent, RecvError> {
            self.invalidation_channel.subscribe().recv().await
        }
    }
}

impl Drop for AppInner {
    fn drop(&mut self) {
        #[cfg(feature = "production")]
        #[cfg(not(test))]
        {
            use crate::domain::metrics::{Event, EventName};
            use crate::iridium_client::get_client;
            use std::collections::HashMap;

            let close_event = Event {
                name: EventName::AppClosed,
                properties: HashMap::new(),
            };

            let client = get_client();

            tokio::runtime::Handle::current().block_on(async move {
                println!("Collecting metric for app close");
                let res = self.metrics_manager.track_event(client, close_event).await;
                match res {
                    Ok(_) => println!("Successfully collected metric for app close"),
                    Err(e) => println!("Error collecting metric for app close: {e}"),
                }
            });
        }
    }
}

pub use app::AppInner;

pub struct ManagerRef<'a, T> {
    manager: &'a T,
    pub app: &'a Arc<AppInner>,
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

pub struct AppRef(pub Weak<AppInner>);

impl AppRef {
    pub fn upgrade(&self) -> App {
        self.0
            .upgrade()
            .expect("App was dropped before its final usage")
    }
}

// Unsafe, possibly uninitialized weak ref to AppInner
//
// SAFETY:
// This type (both MaybeUninits) must be initialized before it is used or dropped.
pub struct UnsafeAppRef(Weak<UnsafeCell<MaybeUninit<AppInner>>>);

unsafe impl Send for UnsafeAppRef {}
unsafe impl Sync for UnsafeAppRef {}

impl UnsafeAppRef {
    // SAFETY:
    // This type must me initialized before it is used.
    pub unsafe fn upgrade(&self) -> App {
        let arc = self
            .0
            .upgrade()
            .expect("App was dropped before its final usage");

        let inner = Arc::into_raw(arc);
        // SAFETY: This pointer cast is safe because UnsafeCell and MaybeUninit do not
        // change the repr of their contained type.
        Arc::from_raw(inner.cast::<AppInner>())
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
