use std::{collections::HashSet, path::PathBuf, sync::Arc};
use tokio::{
    fs::create_dir_all,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};
use crate::{
    api::{instance::import::FEEntity, keys},
    domain::{
        instance::info::{
            CurseforgeModpack, GameVersion, ModLoader, ModLoaderType, Modpack, StandardVersion,
        },
        vtask::VisualTaskId,
    },
    managers::{instance::InstanceVersionSource, AppInner},
};
use super::InstanceArchiveImporter;

#[derive(Debug, Default)]
pub struct CurseforgeInstanceArchiveImporter {

}

#[async_trait::async_trait]
impl InstanceArchiveImporter for CurseforgeInstanceArchiveImporter {
    async fn import(&self, app: Arc<AppInner>, path: PathBuf) -> anyhow::Result<VisualTaskId> {


    }
}
