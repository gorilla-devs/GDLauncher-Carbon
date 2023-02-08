use crate::app::App;
use serde::{Deserialize, Serialize};
use std::rc::Weak;
use std::sync::{Arc, RwLock};

#[derive(Serialize, Deserialize, Ord, PartialOrd, PartialEq, Eq)]
pub struct AppConfiguration {
    pub _id: i32,
    pub default_db_url: String,
    pub app_theme: String,
}

pub struct ConfigurationManager {
    app: RwLock<Arc<Weak<App>>>,
}
