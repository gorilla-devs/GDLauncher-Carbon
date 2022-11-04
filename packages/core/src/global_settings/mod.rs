// NOTE: As this is rust, it is relatively easy to move things around,
// and in the event that the configuration becomes too complex, we can just change settings.rs to
// settings/mod.rs, and move the structs around.

use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::ops::Deref;
use std::sync::Arc;
// As this is an Arc, cloning it is "free", so deriving clone is not a problem,
// and won't use a lot of memory.
#[derive(Clone)]
pub struct Settings {
    inner: Arc<RawSettings>,
}
/// Implementing Deref for Settings allows for the raw usage of the settings
/// as if the wrapper was never there, even if it is wrapped in an Arc for thread sharing.
impl Deref for Settings {
    type Target = RawSettings;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        return Ok(Settings {
            inner: Arc::new(RawSettings::new()?),
        });
    }
}

#[derive(Debug, Deserialize)]
pub struct RawSettings {
    pub curseforge: Curseforge,
    pub server: Server,
}

#[derive(Debug, Deserialize)]
pub struct Curseforge {
    pub api_key: String,
    pub endpoint: String,
}

#[derive(Debug, Deserialize)]
pub struct Server {
    pub ip: String,
    pub port: u16,
}

impl RawSettings {
    pub fn new() -> Result<Self, ConfigError> {
        let config = Config::builder()
            // Set all the default values
            .set_default("curseforge.endpoint", "https://api.curseforge.com/v1/")?
            .set_default("server.ip", "127.0.0.1")?
            .set_default("server.port", "8000")?
            .set_default("log.console_level", "info")?
            .set_default("log.file_level", "warn")?
            .set_default("log.directory", "logs")?
            .add_source(File::with_name("./settings.toml").required(true))
            .build()?;

        config.try_deserialize()
    }
}
