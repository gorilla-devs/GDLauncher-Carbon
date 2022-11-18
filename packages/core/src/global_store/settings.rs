use serde::{Serialize, Deserialize};
use std::{collections::HashMap, sync::Arc};
use crate::java::JavaComponent;

pub type JavaMajorVersion = u8;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct GDLSettings {
    pub java: JavaSettings,
    pub discord_rpc: DiscordRPC,
    pub launcher: LauncherSettings,
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

#[derive(Serialize, Deserialize, PartialEq, Debug)]
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

    pub fn get_components(&self) -> &HashMap<JavaMajorVersion, Vec<JavaComponent>> {
        &self.components
    }

    pub fn get_args(&self) -> &Vec<ArgumentComponent> {
        &self.args
    }

}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ArgumentComponent {
    pub name: String,
    pub value: String,
}

impl ArgumentComponent {
    pub fn new(name: String, value: String) -> Self {
        Self { name, value }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct DiscordRPC {
    pub enabled: bool,
}

impl DiscordRPC {
    pub fn new() -> Self {
        Self { enabled: true }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LauncherSettings {
    pub quit_on_game_close: bool,
    pub quit_on_game_launch: Option<RecordPlaySession>,
}

impl LauncherSettings {
    pub fn new() -> Self {
        Self {
            quit_on_game_close: false,
            quit_on_game_launch: Some(RecordPlaySession::new()),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct RecordPlaySession {
    /// Keeps track of the time the game was launched
    pub record_playtime: bool,
    /// Records the general computer's usage of resources during the session
    pub record_resources_usage: bool,
    /// Records how much memory, CPU and general resources the game used during the session
    pub record_game_performance: bool,
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
