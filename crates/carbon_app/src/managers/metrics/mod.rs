use crate::domain::metrics::{Event, Pageview};

use super::{ManagerRef, GDL_API_BASE};

mod sender;

pub(crate) struct MetricsManager;

impl MetricsManager {
    pub fn new() -> Self {
        Self
    }
}

impl ManagerRef<'_, MetricsManager> {
    pub async fn track_pageview(&self, page: Pageview) {
        let endpoint = format!("{}/v1/metrics/pageview", GDL_API_BASE);
    }
    pub async fn track_event(&self, event: Event) {
        let endpoint = format!("{}/v1/metrics/event", GDL_API_BASE);
    }
}
