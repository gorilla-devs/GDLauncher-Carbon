use std::collections::HashMap;

use lazy_static::lazy_static;

use crate::java::JavaComponent;

pub type JavaMajorVersion = u8;

pub struct GDLSettings {
    java: JavaSettings,
    discord_rpc: DiscordRPC,
    launcher: LauncherSettings,
}

impl GDLSettings {
    pub fn new() -> Self {
        Self {
            java: JavaSettings::new(),
            discord_rpc: DiscordRPC::new(),
            launcher: LauncherSettings::new(),
        }
    }
}

pub struct JavaSettings {
    components: HashMap<JavaMajorVersion, Vec<JavaComponent>>,
    args: Vec<ArgumentComponent>,
}

impl JavaSettings {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            args: Vec::new(),
        }
    }
}

pub struct ArgumentComponent {
    pub name: String,
    pub value: String,
}

impl ArgumentComponent {
    pub fn new(name: String, value: String) -> Self {
        Self { name, value }
    }
}

pub struct DiscordRPC {
    pub enabled: bool,
}

impl DiscordRPC {
    pub fn new() -> Self {
        Self { enabled: true }
    }
}

pub struct LauncherSettings {
    quit_on_game_close: bool,
    quit_on_game_launch: Option<RecordPlaySession>,
}

impl LauncherSettings {
    pub fn new() -> Self {
        Self {
            quit_on_game_close: false,
            quit_on_game_launch: Some(RecordPlaySession::new()),
        }
    }
}

pub struct RecordPlaySession {
    /// Keeps track of the time the game was launched
    record_playtime: bool,
    /// Records the general computer's usage of resources during the session
    record_resources_usage: bool,
    /// Records how much memory, CPU and general resources the game used during the session
    record_game_performance: bool,
}

impl RecordPlaySession {
    pub fn new() -> Self {
        Self {
            record_playtime: true,
            record_resources_usage: true,
            record_game_performance: true,
        }
    }
}