use std::ffi::OsStr;

use anyhow::{anyhow, bail};

use crate::managers::ManagerRef;

use super::{Instance, InstanceId, InstanceManager, InstanceType, InvalidInstanceIdError};

pub mod meta;

impl ManagerRef<'_, InstanceManager> {
    pub async fn enable_mod(
        self,
        instance_id: InstanceId,
        id: String,
        enabled: bool,
    ) -> anyhow::Result<()> {
        let mut instances = self.instances.write().await;
        let mut instance = instances
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let Instance { type_: InstanceType::Valid(data), shortpath, .. } = &mut instance else {
            bail!("enable_mod called on invalid instance");
        };

        let m = data
            .mods
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or_else(|| anyhow!("unable to find mod for {id}"))?;

        let mut disabled_path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath)
            .get_mods_path();

        let enabled_path = disabled_path.join(&m.filename);

        let mut disabled = m.filename.clone();
        disabled.push(OsStr::new(".disabled"));
        disabled_path.push(disabled);

        if enabled {
            if enabled_path.exists() {
                bail!("mod is already enabled");
            }

            if !disabled_path.exists() {
                bail!("mod does not exist on disk");
            }

            tokio::fs::rename(disabled_path, enabled_path).await?;
        } else {
            if disabled_path.exists() {
                bail!("mod is already disabled");
            }

            if enabled_path.exists() {
                bail!("mod does not exist on disk");
            }

            tokio::fs::rename(enabled_path, disabled_path).await?;
        }

        m.enabled = !m.enabled;
        Ok(())
    }
}
