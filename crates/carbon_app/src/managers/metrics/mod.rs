use crate::domain::metrics::{Event, Pageview};

use super::ManagerRef;

mod sender;

pub(super) struct MetricsManager;

impl MetricsManager {
    pub fn new() -> Self {
        Self
    }
}

impl ManagerRef<'_, MetricsManager> {
    pub async fn track_pageview(&self, page: Pageview) {}
    pub async fn track_event(&self, event: Event) {}
    async fn send_keepalive(&self) {}
}
