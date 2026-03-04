use super::keys::server::*;
use super::router::router;
use crate::domain::server::{self as domain, ServerGroupId, ServerId, ServerSettingsUpdate};
use crate::managers::{App, AppInner};
use axum::extract::ws::Message;
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use rspc::RouterBuilder;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Arc;
use tracing::error;

// FE types for Specta auto-gen

#[derive(Type, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct FEServerGroupId(i32);

impl From<domain::ServerGroupId> for FEServerGroupId {
    fn from(value: domain::ServerGroupId) -> Self {
        Self(value.0)
    }
}

impl From<FEServerGroupId> for domain::ServerGroupId {
    fn from(value: FEServerGroupId) -> Self {
        Self(value.0)
    }
}

#[derive(Type, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct FEServerId(i32);

impl From<domain::ServerId> for FEServerId {
    fn from(value: domain::ServerId) -> Self {
        Self(value.0)
    }
}

impl From<FEServerId> for domain::ServerId {
    fn from(value: FEServerId) -> Self {
        Self(value.0)
    }
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListServerGroup {
    id: FEServerGroupId,
    name: String,
    library_position: Option<i32>,
}

impl From<domain::ServerGroup> for ListServerGroup {
    fn from(group: domain::ServerGroup) -> Self {
        Self {
            id: FEServerGroupId(group.id.0),
            name: group.name,
            library_position: group.library_position,
        }
    }
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListServer {
    id: FEServerId,
    group_id: FEServerGroupId,
    index: i32,
    library_position: Option<i32>,
    name: String,
    favorite: bool,
    server_type: domain::ServerType,
    game_version: String,
    port: i32,
    date_created: DateTime<Utc>,
    last_started: Option<DateTime<Utc>>,
    state: FEServerState,
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "status")]
pub enum FEServerState {
    Stopped,
    Starting,
    Running {
        uptime_seconds: i32,
        process_id: u32,
    },
    Stopping,
    Deleting,
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEServerDetails {
    id: FEServerId,
    name: String,
    favorite: bool,
    server_type: domain::ServerType,
    game_version: String,
    port: i32,
    motd: String,
    max_players: i32,
    online_mode: bool,
    xmx: i32,
    xms: i32,
    extra_java_args: String,
    auto_restart: bool,
    date_created: DateTime<Utc>,
    last_started: Option<DateTime<Utc>>,
    state: FEServerState,
}

impl From<domain::ServerDetails> for FEServerDetails {
    fn from(d: domain::ServerDetails) -> Self {
        Self {
            id: FEServerId(d.id.0),
            name: d.name,
            favorite: d.favorite,
            server_type: d.server_type,
            game_version: d.game_version,
            port: d.port,
            motd: d.motd,
            max_players: d.max_players,
            online_mode: d.online_mode,
            xmx: d.xmx,
            xms: d.xms,
            extra_java_args: d.extra_java_args,
            auto_restart: d.auto_restart,
            date_created: d.date_created,
            last_started: d.last_started,
            state: convert_state(&d.state),
        }
    }
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEServerMetrics {
    cpu_percent: f32,
    memory_mb: i32,
    uptime_seconds: i32,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateServer {
    name: String,
    game_version: String,
    port: Option<i32>,
    group: Option<FEServerGroupId>,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateServer {
    id: FEServerId,
    name: Option<String>,
    port: Option<i32>,
    motd: Option<String>,
    max_players: Option<i32>,
    online_mode: Option<bool>,
    xmx: Option<i32>,
    xms: Option<i32>,
    extra_java_args: Option<Option<String>>,
    auto_restart: Option<bool>,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetFavorite {
    id: FEServerId,
    favorite: bool,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendCommand {
    id: FEServerId,
    command: String,
}

fn convert_state(state: &domain::ServerState) -> FEServerState {
    match state {
        domain::ServerState::Stopped { .. } => FEServerState::Stopped,
        domain::ServerState::Starting(_) => FEServerState::Starting,
        domain::ServerState::Running {
            start_time,
            process_id,
            ..
        } => FEServerState::Running {
            uptime_seconds: (Utc::now() - *start_time).num_seconds() as i32,
            process_id: *process_id,
        },
        domain::ServerState::Stopping => FEServerState::Stopping,
        domain::ServerState::Deleting => FEServerState::Deleting,
    }
}

pub(super) fn mount() -> RouterBuilder<App> {
    router! {
        query GET_DEFAULT_GROUP[app, args: ()] {
            Ok(*app.server_manager()
                .get_default_group()
                .await?)
        }

        query GET_GROUPS[app, args: ()] {
            Ok(app.server_manager()
                .list_groups()
                .await?
                .into_iter()
                .map(ListServerGroup::from)
                .collect::<Vec<_>>())
        }

        query GET_ALL_SERVERS[app, args: ()] {
            let groups = app.server_manager()
                .list_groups()
                .await?;

            let active_servers = app.server_manager.servers.read().await;

            let servers: Vec<ListServer> = groups
                .into_iter()
                .flat_map(|group| {
                    group.servers.into_iter().map(|s| {
                        let state = active_servers
                            .get(&s.id)
                            .map(|sd| convert_state(&sd.state))
                            .unwrap_or(FEServerState::Stopped);

                        ListServer {
                            id: FEServerId(s.id.0),
                            group_id: FEServerGroupId(s.group_id.0),
                            index: s.index,
                            library_position: s.library_position,
                            name: s.name,
                            favorite: s.favorite,
                            server_type: s.server_type,
                            game_version: s.game_version,
                            port: s.port,
                            date_created: s.date_created,
                            last_started: s.last_started,
                            state,
                        }
                    })
                })
                .collect();

            Ok(servers)
        }

        mutation CREATE_SERVER[app, details: CreateServer] {
            if details.name.is_empty() {
                return Err(anyhow::anyhow!("server name cannot be empty"));
            }

            let group: domain::ServerGroupId = match details.group {
                Some(group) => group.into(),
                None => app.server_manager()
                    .get_default_group()
                    .await?
            };

            app.server_manager()
                .create_server(
                    group,
                    details.name,
                    details.game_version,
                    details.port,
                )
                .await
                .map(FEServerId::from)
        }

        mutation DELETE_SERVER[app, id: FEServerId] {
            app.server_manager()
                .delete_server(id.into())
                .await
        }

        mutation START_SERVER[app, id: FEServerId] {
            app.server_manager()
                .start_server(id.into())
                .await
        }

        mutation STOP_SERVER[app, id: FEServerId] {
            app.server_manager()
                .stop_server(id.into())
                .await
        }

        mutation KILL_SERVER[app, id: FEServerId] {
            app.server_manager()
                .kill_server(id.into())
                .await
        }

        mutation SEND_CONSOLE_COMMAND[app, cmd: SendCommand] {
            app.server_manager()
                .send_console_command(cmd.id.into(), cmd.command)
                .await
        }

        query GET_SERVER_DETAILS[app, id: FEServerId] {
            app.server_manager()
                .server_details(id.into())
                .await
                .map(FEServerDetails::from)
        }

        query GET_SERVER_METRICS[app, id: FEServerId] {
            let metrics = app.server_manager()
                .get_server_metrics(id.into())
                .await?;

            let state = app.server_manager()
                .get_server_state(id.into())
                .await?;

            let uptime = match &state {
                domain::ServerState::Running { start_time, .. } => {
                    (Utc::now() - *start_time).num_seconds()
                }
                _ => 0,
            };

            Ok(metrics.map(|m| FEServerMetrics {
                cpu_percent: m.cpu_percent,
                memory_mb: (m.memory_bytes / (1024 * 1024)) as i32,
                uptime_seconds: uptime as i32,
            }))
        }

        mutation SET_FAVORITE[app, args: SetFavorite] {
            app.server_manager()
                .set_favorite(args.id.into(), args.favorite)
                .await
        }

        mutation UPDATE_SERVER[app, update: UpdateServer] {
            app.server_manager()
                .update_server(ServerSettingsUpdate {
                    server_id: update.id.into(),
                    name: update.name,
                    port: update.port,
                    motd: update.motd,
                    max_players: update.max_players,
                    online_mode: update.online_mode,
                    xmx: update.xmx,
                    xms: update.xms,
                    extra_java_args: update.extra_java_args,
                    auto_restart: update.auto_restart,
                })
                .await
        }
    }
}

#[derive(Deserialize)]
struct ServerLogQuery {
    id: i32,
}

#[derive(Deserialize)]
struct ServerMetricsQuery {
    id: i32,
}

pub fn mount_axum_router() -> axum::Router<Arc<AppInner>> {
    axum::Router::new()
        .route("/log", axum::routing::get(server_log_ws_handler))
        .route("/metrics", axum::routing::get(server_metrics_ws_handler))
}

async fn server_log_ws_handler(
    req: WebSocketUpgrade,
    Query(query): Query<ServerLogQuery>,
    State(app): State<Arc<AppInner>>,
) -> impl IntoResponse {
    req.on_upgrade(move |mut socket| async move {
        let server_id = ServerId(query.id);
        let log_rx = match app.server_manager().get_server_log(server_id).await {
            Ok(Some(rx)) => rx,
            Ok(None) => {
                let _ = socket
                    .send(Message::Text(
                        serde_json::json!({"error": "Server not running"}).to_string(),
                    ))
                    .await;
                return;
            }
            Err(e) => {
                let _ = socket
                    .send(Message::Text(
                        serde_json::json!({"error": e.to_string()}).to_string(),
                    ))
                    .await;
                return;
            }
        };

        let mut log_rx = log_rx;

        // Send existing log history immediately on connect
        {
            let logs = log_rx.borrow_and_update().clone();
            if !logs.is_empty() {
                let msg = serde_json::to_string(&logs).unwrap_or_default();
                if socket.send(Message::Text(msg)).await.is_err() {
                    return;
                }
            }
        }

        // Then stream new changes
        loop {
            if log_rx.changed().await.is_err() {
                break;
            }
            let logs = log_rx.borrow_and_update().clone();
            let msg = serde_json::to_string(&logs).unwrap_or_default();
            if socket.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    })
}

async fn server_metrics_ws_handler(
    req: WebSocketUpgrade,
    Query(query): Query<ServerMetricsQuery>,
    State(app): State<Arc<AppInner>>,
) -> impl IntoResponse {
    req.on_upgrade(move |mut socket| async move {
        let server_id = ServerId(query.id);
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            let metrics = match app.server_manager().get_server_metrics(server_id).await {
                Ok(Some(m)) => m,
                Ok(None) => break,
                Err(_) => break,
            };

            let state = match app.server_manager().get_server_state(server_id).await {
                Ok(s) => s,
                Err(_) => break,
            };

            let uptime = match &state {
                domain::ServerState::Running { start_time, .. } => {
                    (Utc::now() - *start_time).num_seconds()
                }
                _ => 0,
            };

            let msg = serde_json::json!({
                "cpuPercent": metrics.cpu_percent,
                "memoryMb": metrics.memory_bytes / (1024 * 1024),
                "uptimeSeconds": uptime,
            });

            if socket
                .send(Message::Text(msg.to_string()))
                .await
                .is_err()
            {
                break;
            }
        }
    })
}
