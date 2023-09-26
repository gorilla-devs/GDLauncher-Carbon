use std::{sync::Arc, path::PathBuf};

use rspc::Type;
use serde::{Deserialize, Serialize};

use crate::{
    api::vtask::FETaskId,
    managers::{
        instance::importer::{self, Entity, ImportScanStatus, ImportEntry},
        AppInner,
    },
};

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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

pub async fn scan_importable_instances(app: Arc<AppInner>, _entity: FEEntity) -> anyhow::Result<()> {
    app.instance_manager()
        .import_manager()
        .set_scan_target(Some((Entity::LegacyGDLauncher, PathBuf::from("/home/admin/lpwinsync/gdlauncher_next"))))
}

pub async fn get_importable_instances(
    app: Arc<AppInner>,
    _entity: FEEntity,
) -> anyhow::Result<Vec<FEImportableInstance>> {
    let status = app.instance_manager()
        .import_manager()
        .scan_status()
        .await?;

    let v = match status.status {
        ImportScanStatus::SingleResult(ImportEntry::Valid(r)) => vec![r.into()],
        ImportScanStatus::MultiResult(r) => r.into_iter()
            .filter_map(|r| match r {
                ImportEntry::Valid(r) => Some(r),
                _ => None,
            }).map(Into::into).collect(),
        _ => vec![],
    };

    Ok(v)
}

pub async fn import_instance(
    app: Arc<AppInner>,
    args: FEImportInstance,
) -> anyhow::Result<FETaskId> {
    Ok(app.instance_manager()
        .import_manager()
        .begin_import(args.index)
        .await?
        .into())
}
