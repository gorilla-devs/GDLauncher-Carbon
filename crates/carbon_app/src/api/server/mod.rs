use super::keys::server::*;
use super::router::router;
use crate::domain::server::{
    self as domain, BannedIpEntry, BannedPlayerEntry, OpsEntry, ServerAddon, ServerGroupId,
    ServerGroupMoveTarget, ServerId, ServerMoveTarget, ServerSettingsUpdate, WhitelistEntry,
};
use crate::error::{AxumError, FeError};
use crate::managers::server::PlayerListFile;
use crate::managers::{App, AppInner};
use anyhow::anyhow;
use axum::extract::ws::Message;
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
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
    icon_revision: Option<i32>,
    modloader_type: Option<String>,
    modloader_version: Option<String>,
    modpack_info: Option<FEServerModpackInfo>,
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEServerModpackInfo {
    platform: String,
    project_id: String,
    file_id: String,
}

impl From<domain::ServerModpackInfo> for FEServerModpackInfo {
    fn from(info: domain::ServerModpackInfo) -> Self {
        Self {
            platform: info.platform,
            project_id: info.project_id,
            file_id: info.file_id,
        }
    }
}

#[derive(Type, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "status")]
pub enum FEServerState {
    Stopped {
        #[serde(skip_serializing_if = "Option::is_none")]
        failed_task: Option<i32>,
    },
    Installing {
        task_id: i32,
    },
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
    icon_revision: Option<i32>,
    modloader_type: Option<String>,
    modloader_version: Option<String>,
    modpack_info: Option<FEServerModpackInfo>,
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
            icon_revision: d.icon_revision.map(|v| v as i32),
            modloader_type: d.modloader_type,
            modloader_version: d.modloader_version,
            modpack_info: d.modpack_info.map(FEServerModpackInfo::from),
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
    modloader_type: Option<String>,
    modloader_version: Option<String>,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateServer {
    id: FEServerId,
    name: Option<String>,
    xmx: Option<i32>,
    xms: Option<i32>,
    extra_java_args: Option<Option<String>>,
    auto_restart: Option<bool>,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetServerFavorite {
    id: FEServerId,
    favorite: bool,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendCommand {
    id: FEServerId,
    command: String,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetServerIcon {
    id: FEServerId,
    base64_data: String,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveServer {
    server: FEServerId,
    target: FEMoveServerTarget,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum FEMoveServerTarget {
    BeforeServer(FEServerId),
    EndOfGroup(FEServerGroupId),
    BeforeGroup(FEServerGroupId),
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveServerGroup {
    group: FEServerGroupId,
    target: FEMoveServerGroupTarget,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum FEMoveServerGroupTarget {
    BeforeGroup(FEServerGroupId),
    BeforeServer(FEServerId),
    EndOfLibrary,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateFolderFromServers {
    servers: Vec<FEServerId>,
    #[specta(optional)]
    target_server_id: Option<FEServerId>,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameServerGroup {
    group: FEServerGroupId,
    name: String,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateServerProperties {
    id: FEServerId,
    properties: std::collections::HashMap<String, String>,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddPlayerRequest {
    server_id: FEServerId,
    username: String,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemovePlayerRequest {
    server_id: FEServerId,
    uuid: String,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddOpRequest {
    server_id: FEServerId,
    username: String,
    level: i32,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BanPlayerRequest {
    server_id: FEServerId,
    username: String,
    reason: Option<String>,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BanIpRequest {
    server_id: FEServerId,
    ip: String,
    reason: Option<String>,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnbanIpRequest {
    server_id: FEServerId,
    ip: String,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EnableServerAddonRequest {
    server_id: FEServerId,
    addon_id: String,
    enabled: bool,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteServerAddonRequest {
    server_id: FEServerId,
    addon_id: String,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallServerMod {
    server_id: FEServerId,
    mod_source: ServerModSource,
}

#[derive(Type, Debug, Deserialize)]
enum ServerModSource {
    Curseforge(ServerCurseforgeMod),
    Modrinth(ServerModrinthMod),
}

#[derive(Type, Debug, Deserialize)]
struct ServerCurseforgeMod {
    project_id: u32,
    file_id: u32,
}

#[derive(Type, Debug, Deserialize)]
struct ServerModrinthMod {
    project_id: String,
    version_id: String,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallLatestServerMod {
    server_id: FEServerId,
    mod_source: ServerLatestModSource,
}

#[derive(Type, Debug, Deserialize)]
enum ServerLatestModSource {
    Curseforge(u32),
    Modrinth(String),
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateServerFromModpack {
    name: String,
    port: Option<i32>,
    group: Option<FEServerGroupId>,
    modpack_source: FEServerModpackSource,
    icon_url: Option<String>,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum FEServerModpackSource {
    Curseforge {
        project_id: u32,
        file_id: u32,
        server_pack_file_id: u32,
    },
    Modrinth {
        project_id: String,
        version_id: String,
    },
}

fn convert_state(state: &domain::ServerState) -> FEServerState {
    match state {
        domain::ServerState::Stopped { failed_task } => FEServerState::Stopped {
            failed_task: failed_task.map(|t| t.0),
        },
        domain::ServerState::Installing(task_id) => {
            FEServerState::Installing { task_id: task_id.0 }
        }
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
                            .unwrap_or(FEServerState::Stopped { failed_task: None });

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
                            icon_revision: s.icon_revision.map(|v| v as i32),
                            modloader_type: s.modloader_type,
                            modloader_version: s.modloader_version,
                            modpack_info: s.modpack_info.map(FEServerModpackInfo::from),
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
                    details.modloader_type,
                    details.modloader_version,
                )
                .await
                .map(FEServerId::from)
        }

        mutation CREATE_SERVER_FROM_MODPACK[app, details: CreateServerFromModpack] {
            if details.name.is_empty() {
                return Err(anyhow::anyhow!("server name cannot be empty"));
            }

            let group: domain::ServerGroupId = match details.group {
                Some(group) => group.into(),
                None => app.server_manager()
                    .get_default_group()
                    .await?
            };

            let modpack_source = match details.modpack_source {
                FEServerModpackSource::Curseforge { project_id, file_id, server_pack_file_id } => {
                    crate::managers::server::modpack::ServerModpackSource::Curseforge {
                        project_id,
                        file_id,
                        server_pack_file_id,
                    }
                }
                FEServerModpackSource::Modrinth { project_id, version_id } => {
                    crate::managers::server::modpack::ServerModpackSource::Modrinth {
                        project_id,
                        version_id,
                    }
                }
            };

            app.server_manager()
                .create_server_from_modpack(
                    group,
                    details.name,
                    modpack_source,
                    details.port,
                    details.icon_url,
                )
                .await
                .map(FEServerId::from)
        }

        mutation DELETE_SERVER[app, id: FEServerId] {
            app.server_manager()
                .delete_server(id.into())
                .await
        }

        mutation REINSTALL_SERVER[app, id: FEServerId] {
            app.server_manager()
                .reinstall_server_from_modpack(id.into())
                .await
                .map(super::vtask::FETaskId::from)
        }

        mutation ACCEPT_EULA[app, id: FEServerId] {
            app.server_manager()
                .accept_eula(id.into())
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

        mutation SET_FAVORITE[app, args: SetServerFavorite] {
            app.server_manager()
                .set_favorite(args.id.into(), args.favorite)
                .await
        }

        mutation UPDATE_SERVER[app, update: UpdateServer] {
            app.server_manager()
                .update_server(ServerSettingsUpdate {
                    server_id: update.id.into(),
                    name: update.name,
                    xmx: update.xmx,
                    xms: update.xms,
                    extra_java_args: update.extra_java_args,
                    auto_restart: update.auto_restart,
                })
                .await
        }

        mutation SET_SERVER_ICON[app, args: SetServerIcon] {
            app.server_manager()
                .set_server_icon(args.id.into(), args.base64_data)
                .await
        }

        mutation MOVE_SERVER[app, move_server: MoveServer] {
            let target = match move_server.target {
                FEMoveServerTarget::BeforeServer(id) => ServerMoveTarget::BeforeServer(id.into()),
                FEMoveServerTarget::EndOfGroup(group) => ServerMoveTarget::EndOfGroup(group.into()),
                FEMoveServerTarget::BeforeGroup(group) => ServerMoveTarget::BeforeGroup(group.into()),
            };
            app.server_manager()
                .move_server(move_server.server.into(), target)
                .await
        }

        mutation MOVE_SERVER_GROUP[app, move_data: MoveServerGroup] {
            let target = match move_data.target {
                FEMoveServerGroupTarget::BeforeGroup(id) => ServerGroupMoveTarget::BeforeGroup(id.into()),
                FEMoveServerGroupTarget::BeforeServer(id) => ServerGroupMoveTarget::BeforeServer(id.into()),
                FEMoveServerGroupTarget::EndOfLibrary => ServerGroupMoveTarget::EndOfLibrary,
            };
            app.server_manager()
                .move_server_group(move_data.group.into(), target)
                .await
        }

        mutation CREATE_FOLDER_FROM_SERVERS[app, data: CreateFolderFromServers] {
            app.server_manager()
                .create_folder_from_servers(
                    data.servers.into_iter().map(|id| id.into()).collect(),
                    data.target_server_id.map(|id| id.into()),
                )
                .await
                .map(FEServerGroupId::from)
        }

        mutation ARRANGE_SERVER_LIBRARY[app, args: ()] {
            app.server_manager()
                .arrange_server_library()
                .await
        }

        mutation RENAME_SERVER_GROUP[app, rename: RenameServerGroup] {
            app.server_manager()
                .rename_server_group(rename.group.into(), rename.name)
                .await
        }

        mutation DELETE_SERVER_GROUP[app, id: FEServerGroupId] {
            app.server_manager()
                .delete_server_group(id.into())
                .await
        }

        // server.properties
        query GET_SERVER_PROPERTIES[app, id: FEServerId] {
            app.server_manager()
                .get_server_properties(id.into())
                .await
                .map(|props| props.into_iter().collect::<std::collections::HashMap<String, String>>())
        }

        mutation UPDATE_SERVER_PROPERTIES[app, req: UpdateServerProperties] {
            app.server_manager()
                .update_server_properties(req.id.into(), req.properties)
                .await
        }

        // Whitelist
        query GET_WHITELIST[app, id: FEServerId] {
            app.server_manager()
                .get_player_list::<WhitelistEntry>(id.into(), PlayerListFile::Whitelist)
                .await
        }

        mutation ADD_TO_WHITELIST[app, req: AddPlayerRequest] {
            let id: ServerId = req.server_id.into();
            let (uuid, name) = app.server_manager()
                .resolve_player_uuid(&req.username).await?;
            let entry = WhitelistEntry { uuid, name: name.clone() };
            let mut list = app.server_manager()
                .get_player_list::<WhitelistEntry>(id, PlayerListFile::Whitelist).await?;
            list.push(entry);
            app.server_manager()
                .write_player_list(id, PlayerListFile::Whitelist, &list).await?;
            app.server_manager()
                .send_console_if_running(id, format!("whitelist add {}", name)).await;
            Ok(())
        }

        mutation REMOVE_FROM_WHITELIST[app, req: RemovePlayerRequest] {
            let id: ServerId = req.server_id.into();
            let mut list = app.server_manager()
                .get_player_list::<WhitelistEntry>(id, PlayerListFile::Whitelist).await?;
            let removed_name = list.iter().find(|e| e.uuid == req.uuid).map(|e| e.name.clone());
            list.retain(|e| e.uuid != req.uuid);
            app.server_manager()
                .write_player_list(id, PlayerListFile::Whitelist, &list).await?;
            if let Some(name) = removed_name {
                app.server_manager()
                    .send_console_if_running(id, format!("whitelist remove {}", name)).await;
            }
            Ok(())
        }

        // Ops
        query GET_OPS[app, id: FEServerId] {
            app.server_manager()
                .get_player_list::<OpsEntry>(id.into(), PlayerListFile::Ops).await
        }

        mutation ADD_OP[app, req: AddOpRequest] {
            let id: ServerId = req.server_id.into();
            let (uuid, name) = app.server_manager()
                .resolve_player_uuid(&req.username).await?;
            let entry = OpsEntry { uuid, name: name.clone(), level: req.level, bypasses_player_limit: false };
            let mut list = app.server_manager()
                .get_player_list::<OpsEntry>(id, PlayerListFile::Ops).await?;
            list.push(entry);
            app.server_manager()
                .write_player_list(id, PlayerListFile::Ops, &list).await?;
            app.server_manager()
                .send_console_if_running(id, format!("op {}", name)).await;
            Ok(())
        }

        mutation REMOVE_OP[app, req: RemovePlayerRequest] {
            let id: ServerId = req.server_id.into();
            let mut list = app.server_manager()
                .get_player_list::<OpsEntry>(id, PlayerListFile::Ops).await?;
            let removed_name = list.iter().find(|e| e.uuid == req.uuid).map(|e| e.name.clone());
            list.retain(|e| e.uuid != req.uuid);
            app.server_manager()
                .write_player_list(id, PlayerListFile::Ops, &list).await?;
            if let Some(name) = removed_name {
                app.server_manager()
                    .send_console_if_running(id, format!("deop {}", name)).await;
            }
            Ok(())
        }

        // Banned players
        query GET_BANNED_PLAYERS[app, id: FEServerId] {
            app.server_manager()
                .get_player_list::<BannedPlayerEntry>(id.into(), PlayerListFile::BannedPlayers).await
        }

        mutation BAN_PLAYER[app, req: BanPlayerRequest] {
            let id: ServerId = req.server_id.into();
            let (uuid, name) = app.server_manager()
                .resolve_player_uuid(&req.username).await?;
            let reason = req.reason.unwrap_or_else(|| "Banned by operator".to_string());
            let entry = BannedPlayerEntry {
                uuid, name: name.clone(),
                created: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S %z").to_string(),
                source: "GDLauncher".to_string(),
                expires: "forever".to_string(),
                reason: reason.clone(),
            };
            let mut list = app.server_manager()
                .get_player_list::<BannedPlayerEntry>(id, PlayerListFile::BannedPlayers).await?;
            list.push(entry);
            app.server_manager()
                .write_player_list(id, PlayerListFile::BannedPlayers, &list).await?;
            app.server_manager()
                .send_console_if_running(id, format!("ban {} {}", name, reason)).await;
            Ok(())
        }

        mutation UNBAN_PLAYER[app, req: RemovePlayerRequest] {
            let id: ServerId = req.server_id.into();
            let mut list = app.server_manager()
                .get_player_list::<BannedPlayerEntry>(id, PlayerListFile::BannedPlayers).await?;
            let removed_name = list.iter().find(|e| e.uuid == req.uuid).map(|e| e.name.clone());
            list.retain(|e| e.uuid != req.uuid);
            app.server_manager()
                .write_player_list(id, PlayerListFile::BannedPlayers, &list).await?;
            if let Some(name) = removed_name {
                app.server_manager()
                    .send_console_if_running(id, format!("pardon {}", name)).await;
            }
            Ok(())
        }

        // Banned IPs
        query GET_BANNED_IPS[app, id: FEServerId] {
            app.server_manager()
                .get_player_list::<BannedIpEntry>(id.into(), PlayerListFile::BannedIps).await
        }

        mutation BAN_IP[app, req: BanIpRequest] {
            let id: ServerId = req.server_id.into();
            let reason = req.reason.unwrap_or_else(|| "Banned by operator".to_string());
            let entry = BannedIpEntry {
                ip: req.ip.clone(),
                created: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S %z").to_string(),
                source: "GDLauncher".to_string(),
                expires: "forever".to_string(),
                reason: reason.clone(),
            };
            let mut list = app.server_manager()
                .get_player_list::<BannedIpEntry>(id, PlayerListFile::BannedIps).await?;
            list.push(entry);
            app.server_manager()
                .write_player_list(id, PlayerListFile::BannedIps, &list).await?;
            app.server_manager()
                .send_console_if_running(id, format!("ban-ip {} {}", req.ip, reason)).await;
            Ok(())
        }

        mutation UNBAN_IP[app, req: UnbanIpRequest] {
            let id: ServerId = req.server_id.into();
            let mut list = app.server_manager()
                .get_player_list::<BannedIpEntry>(id, PlayerListFile::BannedIps).await?;
            list.retain(|e| e.ip != req.ip);
            app.server_manager()
                .write_player_list(id, PlayerListFile::BannedIps, &list).await?;
            app.server_manager()
                .send_console_if_running(id, format!("pardon-ip {}", req.ip)).await;
            Ok(())
        }

        // Server Addons
        query GET_SERVER_ADDONS[app, id: FEServerId] {
            app.server_manager()
                .list_server_addons(id.into())
                .await
        }

        mutation ENABLE_SERVER_ADDON[app, req: EnableServerAddonRequest] {
            app.server_manager()
                .enable_server_addon(req.server_id.into(), req.addon_id, req.enabled)
                .await
        }

        mutation DELETE_SERVER_ADDON[app, req: DeleteServerAddonRequest] {
            app.server_manager()
                .delete_server_addon(req.server_id.into(), req.addon_id)
                .await
        }

        mutation INSTALL_SERVER_MOD[app, req: InstallServerMod] {
            let task = match req.mod_source {
                ServerModSource::Curseforge(cf_mod) => {
                    app.server_manager()
                        .install_curseforge_mod(
                            req.server_id.into(),
                            cf_mod.project_id,
                            cf_mod.file_id,
                        )
                        .await?
                }
                ServerModSource::Modrinth(mdr_mod) => {
                    app.server_manager()
                        .install_modrinth_mod(
                            req.server_id.into(),
                            mdr_mod.project_id,
                            mdr_mod.version_id,
                        )
                        .await?
                }
            };
            Ok(super::vtask::FETaskId::from(task))
        }

        mutation INSTALL_LATEST_SERVER_MOD[app, req: InstallLatestServerMod] {
            let task = match req.mod_source {
                ServerLatestModSource::Curseforge(project_id) => {
                    app.server_manager()
                        .install_latest_curseforge_mod(
                            req.server_id.into(),
                            project_id,
                        )
                        .await?
                }
                ServerLatestModSource::Modrinth(project_id) => {
                    app.server_manager()
                        .install_latest_modrinth_mod(
                            req.server_id.into(),
                            project_id,
                        )
                        .await?
                }
            };
            Ok(super::vtask::FETaskId::from(task))
        }

        mutation OPEN_SERVER_FOLDER[app, id: FEServerId] {
            app.server_manager()
                .open_folder(id.into())
                .await
        }

        mutation PRIORITIZE_SERVER_CACHE[app, server_id: Option<FEServerId>] {
            use crate::managers::metadata::cache::CacheEntityId;
            app.meta_cache_manager()
                .watch_and_prioritize(server_id.map(|id| CacheEntityId::Server(id.0)))
                .await;

            Ok(())
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

#[derive(Deserialize)]
struct ServerIconQuery {
    id: i32,
    rev: Option<i32>,
}

async fn server_icon(
    State(app): State<Arc<AppInner>>,
    Query(query): Query<ServerIconQuery>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let icon = app
        .server_manager()
        .server_icon(ServerId(query.id))
        .await
        .map_err(|e| FeError::from_anyhow(&e).make_axum())?;

    let res = match icon {
        Some((name, icon)) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                "filename",
                name.parse::<HeaderValue>()
                    .map_err(|e| FeError::from_anyhow(&anyhow!(e)).make_axum())?,
            );

            (StatusCode::OK, headers, icon)
        }
        None => (StatusCode::NO_CONTENT, HeaderMap::new(), Vec::new()),
    };

    Ok::<_, AxumError>(res)
}

pub fn mount_axum_router() -> axum::Router<Arc<AppInner>> {
    axum::Router::new()
        .route("/log", axum::routing::get(server_log_ws_handler))
        .route("/metrics", axum::routing::get(server_metrics_ws_handler))
        .route("/serverIcon", axum::routing::get(server_icon))
        .route(
            "/serverModIcon",
            axum::routing::get(
                |State(app): State<Arc<AppInner>>,
                 Query(query): Query<ServerModIconQuery>| async move {
                    use carbon_repos::db::{
                        server_mod_file_cache as sfcdb,
                        mod_metadata as metadb,
                        curse_forge_mod_cache as cfdb,
                        modrinth_mod_cache as mrdb,
                    };

                    let entry = app
                        .prisma_client
                        .server_mod_file_cache()
                        .find_unique(sfcdb::UniqueWhereParam::IdEquals(query.mod_id.clone()))
                        .with(
                            sfcdb::metadata::fetch()
                                .with(metadb::logo_image::fetch())
                                .with(metadb::curseforge::fetch().with(cfdb::logo_image::fetch()))
                                .with(metadb::modrinth::fetch().with(mrdb::logo_image::fetch())),
                        )
                        .exec()
                        .await
                        .map_err(|e| FeError::from_anyhow(&e.into()).make_axum())?
                        .ok_or_else(|| FeError::from_anyhow(
                            &anyhow::anyhow!("Server mod not found: {}", query.mod_id)
                        ).make_axum())?;

                    let metadata = entry.metadata
                        .ok_or_else(|| FeError::from_anyhow(
                            &anyhow::anyhow!("broken db state")
                        ).make_axum())?;

                    // Try all platforms in priority order (curseforge → modrinth → metadata)
                    // since ServerAddon only has a single has_image flag
                    let cf_icon = metadata.curseforge.flatten()
                        .and_then(|cf| cf.logo_image.flatten())
                        .and_then(|img| img.data);
                    let mr_icon = metadata.modrinth.flatten()
                        .and_then(|mr| mr.logo_image.flatten())
                        .and_then(|img| img.data);
                    let meta_icon = metadata.logo_image.flatten().map(|m| m.data);

                    let icon = cf_icon.or(mr_icon).or(meta_icon);

                    let res = match icon {
                        Some(data) => (StatusCode::OK, data),
                        None => (StatusCode::NO_CONTENT, Vec::new()),
                    };

                    Ok::<_, AxumError>(res)
                },
            ),
        )
}

#[derive(Deserialize)]
struct ServerModIconQuery {
    #[allow(dead_code)]
    server_id: i32,
    mod_id: String,
    #[allow(dead_code)]
    platform: String,
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

        // Send existing log history immediately on connect — even when
        // empty. The frontend uses this initial array to replace its local
        // log state, so a new (empty) session correctly clears stale logs
        // from a previous crash that the user was still viewing.
        {
            let logs = log_rx.borrow_and_update().clone();
            let msg = serde_json::to_string(&logs).unwrap_or_default();
            if socket.send(Message::Text(msg)).await.is_err() {
                return;
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

            if socket.send(Message::Text(msg.to_string())).await.is_err() {
                break;
            }
        }
    })
}
