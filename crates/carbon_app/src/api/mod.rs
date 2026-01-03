use crate::base_api_override::get_base_api_override;
use crate::managers::{App, AppInner};
use crate::{app_version, managers};
use anyhow::Error;
use async_stream::stream;
use axum::extract::ws::Message;
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use rspc::RouterBuilder;
use semver::VersionReq;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Arc;
use tracing::{error, info, warn};

mod account;
pub mod instance;
mod java;
pub mod keys;
mod mc;
mod metrics;
mod modplatforms;
pub mod router;
pub mod settings;
mod system_info;
pub mod translation;
mod vtask;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct InvalidationEvent {
    pub key: &'static str,
    pub args: Option<serde_json::Value>,
}

impl InvalidationEvent {
    pub fn new(key: &'static str, args: Option<serde_json::Value>) -> Self {
        Self { key, args }
    }
}

#[derive(Serialize, Type)]
pub struct FEOperatingSystem {
    pub os: String,
    pub os_version: String,
}

#[derive(Type, Debug, Serialize, Clone)]
pub enum CoreModuleStatus {
    VerifyingTermsAndPrivacy,
    LoadAndMigrate,
    LaunchBackgroundTasks,
    RefreshMSAuth,
    RequestingCode,
    PollingCode,
    McLogin,
    XboxAuth,
    MCEntitlements,
    McProfile,
    AccountRefreshComplete,
}

impl std::fmt::Display for CoreModuleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self).unwrap().trim_matches('"')
        )
    }
}

pub fn update_core_module_status(status: CoreModuleStatus) {
    println!("_STATUS_:{status}");
}

#[derive(Deserialize, Serialize, Type)]
struct Announcement {
    title: String,
    content: String,
    #[serde(rename = "type")]
    _type: AnnouncementType,
    version_req: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
}

#[derive(Deserialize, Serialize, Type)]
#[serde(rename_all = "lowercase")]
enum AnnouncementType {
    Info,
    Warning,
    Error,
}

pub fn build_rspc_router(gdl_base_api: String) -> RouterBuilder<App> {
    let mut counter = Arc::new(0);

    rspc::Router::<App>::new()
        .query("echo", |t| t(|_ctx, args: String| async move { Ok(args) }))
        .query("getAppVersion", |t| {
            t(|_ctx, _: ()| async move { Ok(app_version::APP_VERSION) })
        })
        .mutation("longRunning", |t| {
            t(move |ctx, _: ()| async move {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                Ok(true)
            })
        })
        .query("fake", |t| {
            t(|_ctx, _: ()| async move { Ok(CoreModuleStatus::AccountRefreshComplete) })
        })
        .query("getAnnouncements", |t| {
            t(|app, _: ()| async move {
                let api_url = format!("{}/v1/announcements", get_base_api_override());
                let res = match app.reqwest_client.get(&api_url).send().await {
                    Ok(res) => res,
                    Err(e) => {
                        warn!("Failed to get announcements: {e}");
                        return vec![];
                    }
                };

                let announcements: Vec<Announcement> = match res.json().await {
                    Ok(announcements) => announcements,
                    Err(e) => {
                        warn!("Failed to parse announcements: {e}");
                        return vec![];
                    }
                };

                // Filter announcements based on version constraints and dates
                let filtered_announcements = announcements
                    .into_iter()
                    .filter(|announcement| {
                        if let Some(version_req) = &announcement.version_req {
                            if let Ok(version_req) = VersionReq::parse(&*version_req) {
                                if let Ok(app_version) =
                                    semver::Version::parse(&app_version::APP_VERSION)
                                {
                                    if !version_req.matches(&app_version) {
                                        return false;
                                    }
                                }
                            }
                        }

                        // Check start date
                        if let Some(start_date) = &announcement.start_date {
                            let now = chrono::Utc::now();
                            if let Ok(start) = chrono::DateTime::parse_from_rfc3339(start_date) {
                                if now < start {
                                    return false;
                                }
                            }
                        }

                        // Check end date
                        if let Some(end_date) = &announcement.end_date {
                            let now = chrono::Utc::now();
                            if let Ok(end) = chrono::DateTime::parse_from_rfc3339(end_date) {
                                if now > end {
                                    return false;
                                }
                            }
                        }

                        true
                    })
                    .collect::<Vec<_>>();

                filtered_announcements
            })
        })
        .merge(keys::account::GROUP_PREFIX, account::mount())
        .merge(keys::java::GROUP_PREFIX, java::mount())
        .merge(keys::mc::GROUP_PREFIX, mc::mount())
        .merge(keys::vtask::GROUP_PREFIX, vtask::mount())
        .merge(keys::instance::GROUP_PREFIX, instance::mount())
        .merge(keys::modplatforms::GROUP_PREFIX, modplatforms::mount())
        .merge(keys::settings::GROUP_PREFIX, settings::mount())
        .merge(keys::metrics::GROUP_PREFIX, metrics::mount())
        .merge(keys::systeminfo::GROUP_PREFIX, system_info::mount())
}

/// Query params for loading a local image file
#[derive(Deserialize)]
struct LoadImageQuery {
    path: String,
}

pub fn build_axum_vanilla_router() -> axum::Router<Arc<AppInner>> {
    axum::Router::new()
        .route("/", axum::routing::get(|| async { "Hello 'rspc'!" }))
        .route(
            "/invalidations",
            axum::routing::get(invalidation_ws_handler),
        )
        .route("/health", axum::routing::get(|| async { "OK" }))
        .route(
            "/loadImage",
            axum::routing::get(
                |axum::extract::Query(query): axum::extract::Query<LoadImageQuery>| async move {
                    let data = tokio::fs::read(&query.path)
                        .await
                        .map_err(|e| crate::error::FeError::from_anyhow(&e.into()).make_axum())?;
                    Ok::<_, crate::error::AxumError>(data)
                },
            ),
        )
        .nest("/account", account::mount_axum_router())
        .nest("/instance", instance::mount_axum_router())
}

async fn invalidation_ws_handler(
    req: WebSocketUpgrade,
    State(app): State<Arc<AppInner>>,
) -> impl IntoResponse {
    req.on_upgrade(|mut socket| async move {
        let mut channel = app.invalidation_channel.subscribe();
        info!("Invalidation channel connected");
        while let Ok(event) = channel.recv().await {
            let Ok(message) = serde_json::to_string(&event) else {
                error!("Failed to serialize invalidation event: {:?}", event);
                continue;
            };
            match socket.send(Message::Text(message)).await {
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to send invalidation event: {:?}", e);
                }
            }
        }

        info!("Invalidation channel disconnected");
    })
}

#[derive(Type, Debug, Deserialize, Clone)]
pub enum Set<T> {
    Set(T),
}

impl<T> Set<T> {
    pub fn inner(self) -> T {
        match self {
            Self::Set(t) => t,
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    #[ignore]
    #[allow(clippy::assertions_on_constants)]
    fn verify_iridium_feature() {
        #[cfg(feature = "production")]
        {
            assert!(true);
        }
        #[cfg(not(feature = "production"))]
        {
            assert!(false);
        }
    }
}
