use std::sync::Arc;

use lazy_static::lazy_static;
use sqlx::SqliteConnection;
use tokio::sync::Mutex;

use self::{accounts::Accounts, settings::GDLSettings};


mod napi;
mod settings;
mod accounts;

lazy_static! {
    pub static ref CONN_REF: Arc<Mutex<Option<SqliteConnection>>> = Arc::new(Mutex::new(None));
    pub static ref GLOBAL_STORE: Arc<Mutex<GlobalStore>> = Arc::new(Mutex::new(GlobalStore::new()));
}

pub struct GlobalStore {
    pub settings: settings::GDLSettings,
    pub accounts: Accounts
}

impl GlobalStore {
    pub fn new() -> Self {
        Self {
            settings: GDLSettings::new(),
            accounts: Accounts::new()
        }
    }
}