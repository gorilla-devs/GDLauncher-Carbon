use std::sync::Arc;

use lazy_static::lazy_static;
use sqlx::SqliteConnection;
use tokio::sync::Mutex;

mod napi;
mod settings;

lazy_static! {
    pub static ref GLOBAL_STORE: Arc<Mutex<Option<SqliteConnection>>> = Arc::new(Mutex::new(None));
}
