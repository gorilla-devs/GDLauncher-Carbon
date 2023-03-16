use chrono::{DateTime, Utc};

pub mod schema;

pub struct InstanceDetails {
    pub name: String,
    pub version: String,
    pub last_played: DateTime<Utc>,
    pub seconds_played: u32,
    pub instance_start_time: Option<DateTime<Utc>>,
    pub modloaders: Vec<ModLoader>,
    pub notes: String,
}

pub struct ModLoader {
    pub type_: ModLoaderType,
    pub version: String,
}

pub enum ModLoaderType {
    Forge,
    Fabirc,
}
