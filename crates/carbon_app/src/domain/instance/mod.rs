use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::vtask::VisualTaskId;

use super::modplatforms::ModSources;

pub mod info;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct GroupId(pub i32);

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct InstanceId(pub i32);

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct GameLogId(pub i32);

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct InstanceModId(pub Uuid);

#[derive(Clone, Debug)]
pub struct GameLogEntry {
    pub id: GameLogId,
    pub instance_id: InstanceId,
    pub active: bool,
}

pub struct InstanceDetails {
    pub favorite: bool,
    pub name: String,
    pub version: Option<String>,
    pub modpack: Option<info::ModpackInfo>,
    pub locked: bool,
    pub global_java_args: bool,
    pub extra_java_args: Option<String>,
    pub memory: Option<(u16, u16)>,
    pub game_resolution: Option<info::GameResolution>,
    pub last_played: Option<DateTime<Utc>>,
    pub pre_launch_hook: Option<String>,
    pub post_exit_hook: Option<String>,
    pub wrapper_command: Option<String>,
    pub seconds_played: u32,
    pub modloaders: Vec<info::ModLoader>,
    pub state: LaunchState,
    pub notes: String,
    pub icon_revision: Option<u32>,
    pub has_pack_update: bool,
}

#[derive(Debug)]
pub struct InstanceModpackInfo {
    pub name: String,
    pub version_name: String,
    pub url_slug: String,
    pub has_image: bool,
}

#[derive(Debug)]
pub struct InstanceSettingsUpdate {
    pub instance_id: InstanceId,
    pub name: Option<String>,
    pub use_loaded_icon: Option<bool>,
    pub notes: Option<String>,
    pub version: Option<String>,
    pub modloader: Option<Option<info::ModLoader>>,
    pub global_java_args: Option<bool>,
    pub extra_java_args: Option<Option<String>>,
    pub memory: Option<Option<(u16, u16)>>,
    pub pre_launch_hook: Option<Option<String>>,
    pub post_exit_hook: Option<Option<String>>,
    pub wrapper_command: Option<Option<String>>,
    pub game_resolution: Option<Option<info::GameResolution>>,
    pub mod_sources: Option<Option<ModSources>>,
    pub modpack_locked: Option<Option<bool>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LaunchState {
    Inactive {
        failed_task: Option<VisualTaskId>,
    },
    Preparing(VisualTaskId),
    Running {
        start_time: DateTime<Utc>,
        log_id: GameLogId,
    },
    Deleting,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mod {
    pub id: String,
    pub filename: String,
    pub enabled: bool,
    pub metadata: Option<ModFileMetadata>,
    pub curseforge: Option<CurseForgeModMetadata>,
    pub modrinth: Option<ModrinthModMetadata>,
    pub has_update: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModFileMetadata {
    pub id: String,
    pub modid: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<String>,
    pub modloaders: Vec<info::ModLoaderType>,
    pub sha_512: Vec<u8>,
    pub sha_1: Vec<u8>,
    pub murmur_2: i32,
    pub has_image: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurseForgeModMetadata {
    pub project_id: u32,
    pub file_id: u32,
    pub name: String,
    pub version: String,
    pub urlslug: String,
    pub summary: String,
    pub authors: String,
    pub has_image: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModrinthModMetadata {
    pub project_id: String,
    pub version_id: String,
    pub title: String,
    pub version: String,
    pub urlslug: String,
    pub description: String,
    pub authors: String,
    pub has_image: bool,
}

#[derive(Debug, Copy, Clone)]
pub enum InstanceFolder {
    Root,
    Data,
    Mods,
    Configs,
    Screenshots,
    Saves,
    Logs,
    CrashReports,
    ResourcePacks,
    TexturePacks,
    ShaderPacks,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExploreEntry {
    pub name: String,
    pub type_: ExploreEntryType,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ExploreEntryType {
    File { size: u32 },
    Directory,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ExportTarget {
    Curseforge,
    Modrinth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportEntry(pub HashMap<String, Option<ExportEntry>>);
