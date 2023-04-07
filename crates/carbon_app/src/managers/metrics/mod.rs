use super::ManagerRef;

mod sender;

pub(crate) struct AnalyticsManager {}

impl AnalyticsManager {
    pub fn new() -> Self {
        Self {}
    }
}

impl ManagerRef<'_, AnalyticsManager> {
    pub async fn track_event(&self, event: &str) {}
}
