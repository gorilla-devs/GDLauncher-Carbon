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
use tracing::error;

use self::account::AccountManager;
use self::download::DownloadManager;
use self::instance::InstanceManager;
use self::minecraft::MinecraftManager;
use self::rich_presence::RichPresenceManager;
use self::vtask::VisualTaskManager;

pub mod account;
pub mod download;
pub mod instance;
pub mod java;
mod metadata;
mod metrics;
mod minecraft;
mod modplatforms;
mod prisma_client;
pub mod rich_presence;
mod settings;
pub mod system_info;
pub mod vtask;

pub type App = Arc<AppInner>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("manager {0} not found")]
    ManagerNotFound(String),
}

pub const GDL_API_BASE: &str = env!("BASE_API");

mod app {
    use tracing::error;

    use crate::{cache_middleware, domain};

    use super::{
        java::JavaManager, metadata::cache::MetaCacheManager, metrics::MetricsManager,
        modplatforms::ModplatformsManager, system_info::SystemInfoManager, *,
    };

    pub struct AppInner {
        settings_manager: SettingsManager,
        java_manager: JavaManager,
        pub(crate) minecraft_manager: MinecraftManager,
        account_manager: AccountManager,
        pub(crate) invalidation_channel: broadcast::Sender<InvalidationEvent>,
        download_manager: DownloadManager,
        pub(crate) instance_manager: InstanceManager,
        meta_cache_manager: MetaCacheManager,
        pub(crate) metrics_manager: MetricsManager,
        pub(crate) modplatforms_manager: ModplatformsManager,
        pub(crate) reqwest_client: reqwest_middleware::ClientWithMiddleware,
        pub(crate) prisma_client: Arc<PrismaClient>,
        task_manager: VisualTaskManager,
        system_info_manager: SystemInfoManager,
        rich_presence_manager: rich_presence::RichPresenceManager,
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
            let db_client = match prisma_client::load_and_migrate(runtime_path.clone()).await {
                Ok(client) => Arc::new(client),
                Err(prisma_client::DatabaseError::Migration(err)) => {
                    eprintln!(
                        "[_GDL_DB_MIGRATION_FAILED_]: Database migration failed: {}",
                        err
                    );
                    error!("Database migration failed: {}", err);
                    std::process::exit(1);
                }
                Err(err) => {
                    error!("Database connection failed: {}", err);
                    eprintln!("Database connection failed: {}", err);
                    std::process::exit(1);
                }
            };

            // eprintln!("[_GDL_DB_MIGRATION_FAILED_]: Database migration failed");
            // std::process::exit(1);

            let app = Arc::new(UnsafeCell::new(MaybeUninit::<AppInner>::uninit()));
            let unsaferef = UnsafeAppRef(Arc::downgrade(&app));

            // SAFETY: cannot be used until after the ref is initialized.
            let client = reqwest::Client::builder()
                .user_agent(format!(
                    "{} {}",
                    env!("USER_AGENT_PREFIX"),
                    env!("APP_VERSION")
                ))
                .build()
                .unwrap();

            let reqwest = cache_middleware::new_client(
                unsaferef.clone(),
                reqwest_middleware::ClientBuilder::new(client),
            );

            let app = unsafe {
                let inner = Arc::into_raw(app);

                (*inner).get().write(MaybeUninit::new(AppInner {
                    settings_manager: SettingsManager::new(runtime_path, reqwest.clone()),
                    java_manager: JavaManager::new(),
                    minecraft_manager: MinecraftManager::new(),
                    account_manager: AccountManager::new(),
                    modplatforms_manager: ModplatformsManager::new(unsaferef),
                    download_manager: DownloadManager::new(),
                    instance_manager: InstanceManager::new(),
                    meta_cache_manager: MetaCacheManager::new(),
                    metrics_manager: MetricsManager::new(Arc::clone(&db_client)),
                    invalidation_channel,
                    reqwest_client: reqwest,
                    prisma_client: Arc::clone(&db_client),
                    task_manager: VisualTaskManager::new(),
                    system_info_manager: SystemInfoManager::new(),
                    rich_presence_manager: rich_presence::RichPresenceManager::new(),
                }));

                // SAFETY: This pointer cast is safe because UnsafeCell and MaybeUninit do not
                // change the repr of their contained type.
                Arc::from_raw(inner.cast::<AppInner>())
            };

            account::AccountRefreshService::start(Arc::downgrade(&app));

            let _app = app.clone();
            tokio::spawn(async move {
                _app.meta_cache_manager().launch_background_tasks().await;
                _app.clone()
                    .instance_manager()
                    .launch_background_tasks()
                    .await;
            });

            let _app = app.clone();
            tokio::spawn(async move {
                let _ = _app.clone().rich_presence_manager().start_presence().await;
            });

            let _app = app.clone();
            tokio::spawn(async move {
                let _ = reqwest::get(format!("{}/v1/announcement", GDL_API_BASE)).await;
                _app.metrics_manager()
                    .track_event(domain::metrics::Event::LauncherStarted)
                    .await;
            });

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
        manager_getter!(instance_manager: InstanceManager);
        manager_getter!(meta_cache_manager: MetaCacheManager);
        manager_getter!(system_info_manager: SystemInfoManager);
        manager_getter!(rich_presence_manager: RichPresenceManager);

        pub fn invalidate(&self, key: Key, args: Option<serde_json::Value>) {
            match self
                .invalidation_channel
                .send(InvalidationEvent::new(key.full, args))
            {
                Ok(_) => {}
                Err(e) => {
                    error!("Error sending invalidation request: {e}");
                }
            }
        }

        pub async fn wait_for_invalidation(
            &self,
            key: Key,
        ) -> Result<InvalidationEvent, RecvError> {
            let mut recv = self.invalidation_channel.subscribe();
            loop {
                let event = recv.recv().await?;
                if event.key == key.full {
                    return Ok(event);
                }
            }
        }
    }
}

pub use app::AppInner;

pub struct ManagerRef<'a, T> {
    pub manager: &'a T,
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
#[derive(Clone)]
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
