use super::ManagerRef;
use crate::{
    api::{keys::settings::*, settings::FESettingsUpdate},
    db::app_configuration,
    domain::runtime_path,
};
use anyhow::anyhow;
use std::{path::PathBuf, sync::Arc};
use sysinfo::{System, SystemExt};

pub(crate) struct SystemInfoManager {
    system: System,
}

impl SystemInfoManager {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
        }
    }

    pub fn get_total_ram(&self) -> u32 {
        self.system.refresh_memory();
        self.system.total_memory() as u32
    }

    pub fn get_used_ram(&self) -> u32 {
        self.system.refresh_memory();
        self.system.used_memory() as u32
    }
}

impl ManagerRef<'_, SystemInfoManager> {}
