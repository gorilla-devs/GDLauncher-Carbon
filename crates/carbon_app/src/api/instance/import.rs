use std::{path::PathBuf, sync::Arc};

use rspc::Type;
use serde::{Deserialize, Serialize};

use crate::{
    api::vtask::FETaskId,
    managers::{
        instance::importer::{self, InstanceImporter},
        AppInner,
    },
};

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FEEntity {
    LegacyGDLauncher,
    MRPack,
    CurseForgeZip,
    Modrinth,
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
    pub entity: FEEntity,
    pub name: String,
    pub icon: Option<String>,
    pub import_once: bool,
}

impl From<importer::ImportableInstance> for FEImportableInstance {
    fn from(instance: importer::ImportableInstance) -> Self {
        Self {
            entity: instance.entity.into(),
            name: instance.name,
            icon: instance.icon,
            import_once: instance.import_once,
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FEScanEntity {
    pub entity: FEEntity,
    pub scan_paths: Vec<String>,
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FEImportInstance {
    pub entity: FEEntity,
    pub index: u32,
    pub name: String,
}

pub async fn scan_importable_instances(
    app: Arc<AppInner>,
    entity: FEEntity,
    scan_paths: Vec<PathBuf>,
) -> anyhow::Result<()> {
    let instance_manager = app.instance_manager();
    let mut importer = instance_manager.importer.lock().await;

    match entity {
        FEEntity::LegacyGDLauncher => {
            importer
                .legacy_gdlauncher
                .scan(app.clone(), scan_paths)
                .await
        }
        FEEntity::CurseForgeZip => importer.curseforge_zip.scan(app.clone(), scan_paths).await,
        FEEntity::MRPack => importer.mrpack.scan(app.clone(), scan_paths).await,
        _ => anyhow::bail!("Unsupported entity"),
    }
}

pub async fn get_default_scan_path(
    app: Arc<AppInner>,
    entity: FEEntity,
) -> anyhow::Result<Option<PathBuf>> {
    let instance_manager = app.instance_manager();
    let importer = instance_manager.importer.lock().await;

    match entity {
        FEEntity::LegacyGDLauncher => {
            importer
                .legacy_gdlauncher
                .get_default_scan_path(app.clone())
                .await
        }
        FEEntity::CurseForgeZip => {
            importer
                .curseforge_zip
                .get_default_scan_path(app.clone())
                .await
        }
        FEEntity::MRPack => importer.mrpack.get_default_scan_path(app.clone()).await,
        _ => anyhow::bail!("Unsupported entity"),
    }
}

pub async fn get_importable_instances(
    app: Arc<AppInner>,
    entity: FEEntity,
) -> anyhow::Result<Vec<FEImportableInstance>> {
    let instance_manager = app.instance_manager();
    let importer = instance_manager.importer.lock().await;

    match entity {
        FEEntity::LegacyGDLauncher => importer
            .legacy_gdlauncher
            .get_available()
            .await
            .map(|instances| instances.into_iter().map(Into::into).collect()),
        FEEntity::CurseForgeZip => importer
            .curseforge_zip
            .get_available()
            .await
            .map(|instances| instances.into_iter().map(Into::into).collect()),
        FEEntity::MRPack => importer
            .mrpack
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
    let instance_manager = app.instance_manager();
    let importer = instance_manager.importer.lock().await;

    match args.entity {
        FEEntity::LegacyGDLauncher => importer
            .legacy_gdlauncher
            .import(app.clone(), args.index, &args.name)
            .await
            .map(Into::into),
        FEEntity::CurseForgeZip => importer
            .curseforge_zip
            .import(app.clone(), args.index, &args.name)
            .await
            .map(Into::into),
        FEEntity::MRPack => importer
            .mrpack
            .import(app.clone(), args.index, &args.name)
            .await
            .map(Into::into),

        _ => anyhow::bail!("Unsupported entity"),
    }
}
