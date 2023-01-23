use std::sync::Arc;

use carbon_bindings::api::{GlobalContext, GlobalContextInner, InvalidationEvent};
use tokio::sync::RwLock;

pub(crate) fn generate_context(
    invalidation_sender: tokio::sync::broadcast::Sender<InvalidationEvent>,
) -> GlobalContext {
    Arc::new(RwLock::new(GlobalContextInner::new(
        std::env::current_dir().unwrap(),
        invalidation_sender,
    )))
}
