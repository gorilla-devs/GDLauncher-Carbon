use std::sync::Arc;

use carbon_bindings::api::{GlobalContext, GlobalContextInner};
use tokio::sync::RwLock;

pub(crate) fn generate_context() -> GlobalContext {
    Arc::new(RwLock::new(GlobalContextInner::new(
        std::env::current_dir().unwrap(),
    )))
}
