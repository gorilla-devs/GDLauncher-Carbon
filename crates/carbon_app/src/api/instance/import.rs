use std::sync::Arc;

use rspc::Type;
use serde::{Deserialize, Serialize};

use crate::{
    api::vtask::FETaskId,
    managers::{
        instance::importer::{self, InstanceImporter},
        AppInner,
    },
};

use super::FEInstanceId;

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FEEntity {
    LegacyGDLauncher,
    MRPack,
    Modrinth,
    CurseForgeZip,
    CurseForge,
    ATLauncher,
    Technic,
    FTB,
    MultiMC,
    PrismLauncher,
}

impl From<FEEntity> for importer::Entity {
    fn from(entity: FEEntity) -> Self {
        match entity {
            FEEntity::LegacyGDLauncher => Self::LegacyGDLauncher,
            FEEntity::MRPack => Self::MRPack,
            FEEntity::Modrinth => Self::Modrinth,
            FEEntity::CurseForgeZip => Self::CurseForgeZip,
            FEEntity::CurseForge => Self::CurseForge,
            FEEntity::ATLauncher => Self::ATLauncher,
            FEEntity::Technic => Self::Technic,
            FEEntity::FTB => Self::FTB,
            FEEntity::MultiMC => Self::MultiMC,
            FEEntity::PrismLauncher => Self::PrismLauncher,
        }
    }
}

impl From<importer::Entity> for FEEntity {
    fn from(entity: importer::Entity) -> Self {
        match entity {
            importer::Entity::LegacyGDLauncher => Self::LegacyGDLauncher,
            importer::Entity::MRPack => Self::MRPack,
            importer::Entity::Modrinth => Self::Modrinth,
            importer::Entity::CurseForgeZip => Self::CurseForgeZip,
            importer::Entity::CurseForge => Self::CurseForge,
            importer::Entity::ATLauncher => Self::ATLauncher,
            importer::Entity::Technic => Self::Technic,
            importer::Entity::FTB => Self::FTB,
            importer::Entity::MultiMC => Self::MultiMC,
            importer::Entity::PrismLauncher => Self::PrismLauncher,
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FEImportableInstance {
    pub name: String,
}

impl From<importer::ImportableInstance> for FEImportableInstance {
    fn from(instance: importer::ImportableInstance) -> Self {
        Self {
            name: instance.name,
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FEImportInstance {
    pub entity: FEEntity,
    pub index: u32,
}

pub async fn scan_importable_instances(app: Arc<AppInner>, entity: FEEntity) -> anyhow::Result<()> {
    let locker = app.instance_manager();
    let mut locker = locker.importer.lock().await;

    match entity {
        FEEntity::LegacyGDLauncher => locker.legacy_gdlauncher.scan(app.clone()).await,
        _ => anyhow::bail!("Unsupported entity"),
    }
}

pub async fn get_importable_instances(
    app: Arc<AppInner>,
    entity: FEEntity,
) -> anyhow::Result<Vec<FEImportableInstance>> {
    let locker = app.instance_manager();
    let locker = locker.importer.lock().await;

    match entity {
        FEEntity::LegacyGDLauncher => locker
            .legacy_gdlauncher
            .get_available()
            .await
            .map(|instances| instances.into_iter().map(Into::into).collect()),
        _ => anyhow::bail!("Unsupported entity"),
    }
}

pub async fn import_instance(
    app: Arc<AppInner>,
    args: FEImportInstance,
) -> anyhow::Result<FETaskId> {
    let locker = app.instance_manager();
    let locker = locker.importer.lock().await;

    match args.entity {
        FEEntity::LegacyGDLauncher => locker
            .legacy_gdlauncher
            .import(app.clone(), args.index)
            .await
            .map(|task_id| task_id.into()),
        _ => anyhow::bail!("Unsupported entity"),
    }
}
