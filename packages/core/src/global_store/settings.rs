use std::collections::HashMap;

use crate::java::JavaComponent;

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
