use std::{collections::HashMap, path::PathBuf};

pub type JavaMajorVersion = u8;

pub struct GDLSettings {
    java: JavaSettings,
    discord_rpc: DiscordRPC,
    launcher: LauncherSettings,
}

pub struct JavaSettings {
    components: HashMap<JavaMajorVersion, Vec<JavaComponent>>,
    args: Vec<ArgumentComponent>,
}

pub struct ArgumentComponent {
    pub name: String,
    pub value: String,
}

pub struct JavaComponent {
    pub path: PathBuf,
    pub full_version: String,
    pub arch: String,
    /// Indicates whether the component has manually been added by the user
    pub is_custom: bool,
}

pub struct DiscordRPC {
    pub enabled: bool,
}

pub struct LauncherSettings {
    quit_on_game_close: bool,
    quit_on_game_launch: Option<RecordPlaySession>,
}

pub struct RecordPlaySession {
    /// Keeps track of the time the game was launched
    record_playtime: bool,
    /// Records the general computer's usage of resources during the session
    record_resources_usage: bool,
    /// Records how much memory, CPU and general resources the game used during the session
    record_game_performance: bool,
}
