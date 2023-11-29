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

    pub async fn get_total_ram(&self) -> u64 {
        let lock = self.system.lock().await;
        lock.total_memory()
    }

    pub async fn get_used_ram(&self) -> u64 {
        let mut lock = self.system.lock().await;
        lock.refresh_memory();
        lock.used_memory()
    }

    pub async fn get_cpus(&self) -> u32 {
        let lock = self.system.lock().await;
        lock.cpus().len() as u32
    }

    pub async fn get_os_version(&self) -> Option<String> {
        let lock = self.system.lock().await;
        lock.os_version()
    }
}

impl ManagerRef<'_, SystemInfoManager> {}
