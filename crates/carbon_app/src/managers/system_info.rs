use super::ManagerRef;
use std::sync::Arc;
use sysinfo::{System, SystemExt};
use tokio::sync::Mutex;

pub(crate) struct SystemInfoManager {
    system: Arc<Mutex<System>>,
}

impl SystemInfoManager {
    pub fn new() -> Self {
        Self {
            system: Arc::new(Mutex::new(System::new_all())),
        }
    }

    pub async fn get_total_ram(&self) -> u32 {
        let mut lock = self.system.lock().await;
        lock.refresh_memory();
        lock.total_memory() as u32
    }

    pub async fn get_used_ram(&self) -> u32 {
        let mut lock = self.system.lock().await;
        lock.refresh_memory();
        lock.used_memory() as u32
    }
}

impl ManagerRef<'_, SystemInfoManager> {}
