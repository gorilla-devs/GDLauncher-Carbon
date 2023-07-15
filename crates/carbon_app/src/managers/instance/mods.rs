use std::{
    ffi::{OsStr, OsString},
    io::Cursor,
    sync::Arc,
    time::Duration,
};

use anyhow::bail;
use carbon_net::Downloadable;
use md5::{Digest, Md5};
use thiserror::Error;

use crate::domain::instance as domain;
use crate::{
    api::keys::instance::*,
    api::translation::Translation,
    domain::{modplatforms::curseforge::filters::ModFileParameters, vtask::VisualTaskId},
    managers::{metadata, vtask::VisualTask, ManagerRef},
};

use super::{
    installer::{CurseforgeModInstaller, IntoInstaller, InstallResource},
    Instance, InstanceId, InstanceManager, InstanceType, InvalidInstanceIdError, Mod,
};

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
            .ok_or_else(|| InvalidModIdError(instance_id, id.clone()))?;

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

            if !enabled_path.exists() {
                bail!("mod does not exist on disk");
            }

            tokio::fs::rename(enabled_path, disabled_path).await?;
        }

        m.enabled = !m.enabled;
        self.app
            .invalidate(INSTANCE_DETAILS, Some(instance_id.0.into()));
        Ok(())
    }

    pub async fn delete_mod(self, instance_id: InstanceId, id: String) -> anyhow::Result<()> {
        let mut instances = self.instances.write().await;
        let mut instance = instances
            .get_mut(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let Instance { type_: InstanceType::Valid(data), shortpath, .. } = &mut instance else {
            bail!("enable_mod called on invalid instance");
        };

        let (i, m) = data
            .mods
            .iter_mut()
            .enumerate()
            .find(|(_, m)| m.id == id)
            .ok_or_else(|| InvalidModIdError(instance_id, id.clone()))?;

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

        if enabled_path.is_file() {
            tokio::fs::remove_file(enabled_path).await?;
        } else if disabled_path.is_file() {
            tokio::fs::remove_file(disabled_path).await?;
        }

        data.mods.remove(i);
        self.app
            .invalidate(INSTANCE_DETAILS, Some(instance_id.0.into()));
        Ok(())
    }

    pub async fn install_curseforge_mod(
        self,
        instance_id: InstanceId,
        project_id: u32,
        file_id: u32,
    ) -> anyhow::Result<VisualTaskId> {
        let installer = CurseforgeModInstaller::create(self.app, project_id, file_id)
            .await?
            .into_installer();

        let install_result = installer.install(self.app, &instance_id).await?;

        Ok(install_result.task)
    }
}

#[derive(Error, Debug)]
#[error("invalid mod id '{1}' given for instance '{0}'")]
pub struct InvalidModIdError(InstanceId, String);
