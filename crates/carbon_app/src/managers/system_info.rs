use super::ManagerRef;
use std::sync::Arc;
use sysinfo::System;
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

    pub async fn get_available_ram(&self) -> u64 {
        let mut lock = self.system.lock().await;
        lock.refresh_memory();
        lock.available_memory()
    }

    pub async fn get_cpus(&self) -> u32 {
        let lock = self.system.lock().await;
        lock.cpus().len() as u32
    }

    pub async fn get_os_version(&self) -> Option<String> {
        let Some(os_version) = sysinfo::System::os_version() else {
            return None;
        };

        Some(os_version)
    }
}

impl SystemInfoManager {
    pub async fn get_process_metrics(
        &self,
        pid: u32,
    ) -> Option<crate::domain::server::ProcessMetrics> {
        use sysinfo::{Pid, ProcessesToUpdate};

        let mut lock = self.system.lock().await;
        let pid = Pid::from_u32(pid);

        // Use ProcessesToUpdate::All so that sysinfo's clear_procs runs,
        // which is required for compute_cpu_usage to execute.
        // ProcessesToUpdate::Some skips clear_procs entirely, leaving cpu_usage at 0.
        lock.refresh_processes(ProcessesToUpdate::All);

        let process = lock.process(pid)?;
        let num_cpus = lock.cpus().len().max(1) as f32;
        Some(crate::domain::server::ProcessMetrics {
            cpu_percent: process.cpu_usage() / num_cpus,
            memory_bytes: process.memory(),
        })
    }
}

impl ManagerRef<'_, SystemInfoManager> {}
