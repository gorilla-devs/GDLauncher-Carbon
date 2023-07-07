use std::sync::Arc;

use rspc::Type;
use serde::{Deserialize, Serialize};

use crate::managers::{
    instance::importer::{self, InstanceImporter},
    AppInner,
};

#[derive(Type, Debug, Serialize, Deserialize)]
pub enum FEEntity {
    LegacyGDLauncher,
}

impl From<FEEntity> for importer::Entity {
    fn from(entity: FEEntity) -> Self {
        match entity {
            FEEntity::LegacyGDLauncher => Self::LegacyGDLauncher,
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
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
pub struct FEImportInstance {
    pub entity: FEEntity,
    pub index: u32,
}

pub async fn scan_importable_instances(app: Arc<AppInner>, entity: FEEntity) -> anyhow::Result<()> {
    let locker = app.instance_manager();
    let mut locker = locker.importer.lock().await;

    match entity {
        FEEntity::LegacyGDLauncher => locker.legacy_gdlauncher.scan(app.clone()).await,
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
    }
}

pub async fn import_instance(app: Arc<AppInner>, args: FEImportInstance) -> anyhow::Result<()> {
    let locker = app.instance_manager();
    let locker = locker.importer.lock().await;

    match args.entity {
        FEEntity::LegacyGDLauncher => {
            locker
                .legacy_gdlauncher
                .import(app.clone(), args.index)
                .await
        }
    }
}
