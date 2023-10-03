use std::{path::PathBuf, sync::Arc};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;
use tokio::sync::{watch, RwLock};
use tracing::debug;

use crate::{
    api::translation::Translation,
    domain::vtask::VisualTaskId,
    managers::{AppInner, ManagerRef},
};

use self::{
    curseforge_archive::CurseforgeArchiveImporter, legacy_gdlauncher::LegacyGDLauncherImporter,
    modrinth_archive::ModrinthArchiveImporter,
};

use super::InstanceManager;

mod curseforge_archive;
mod legacy_gdlauncher;
mod modrinth_archive;

#[derive(Debug)]
pub struct InstanceImportManager {
    scan_path: watch::Sender<Option<(Entity, PathBuf)>>,
    scanner: RwLock<Option<(bool, Arc<dyn InstanceImporter>)>>,
}

impl InstanceImportManager {
    pub fn new() -> Self {
        Self {
            scan_path: watch::channel(None).0,
            scanner: RwLock::new(None),
        }
    }
}

impl ManagerRef<'_, InstanceImportManager> {
    pub fn set_scan_target(self, path: Option<(Entity, PathBuf)>) -> anyhow::Result<()> {
        self.scan_path
            .send(path)
            .map_err(|_| anyhow!("import scanning background task has died"))?;

        Ok(())
    }

    pub fn launch_background_tasks(self) {
        let mut rx = self.scan_path.subscribe();
        let app = self.app.clone();

        tokio::task::spawn(async move {
            while rx.changed().await.is_ok() {
                loop {
                    let target = rx.borrow().clone();
                    debug!({ target = ?target }, "import scanning target updated");
                    let Some((entity, path)) = target else {
                        *app.instance_manager()
                            .import_manager()
                            .scanner
                            .write()
                            .await = None;

                        break;
                    };

                    let scanner = entity.create_importer();

                    *app.instance_manager()
                        .import_manager()
                        .scanner
                        .write()
                        .await = Some((true, scanner.clone()));

                    let fut = async {
                        let r = scanner.scan(&app, path.clone()).await;

                        if let Err(e) = r {
                            tracing::error!({ error = ?e }, "instance scanning failed for path {path:?}");
                        }
                    };

                    let target_changed = async {
                        while rx.changed().await.is_ok() {
                            if matches!(&*rx.borrow(), Some((e, p)) if e == &entity && p == &path) {
                                break;
                            }
                        }
                    };

                    tokio::select! {
                        _ = target_changed => continue,
                        _ = fut => break,
                    }
                }
            }
        });
    }

    pub async fn scan_status(self) -> anyhow::Result<FullImportScanStatus> {
        match self.scanner.read().await.as_ref() {
            Some((scanning, scanner)) => Ok(FullImportScanStatus {
                status: scanner.get_status().await,
                scanning: *scanning,
            }),
            None => Err(anyhow!("scan target is not set")),
        }
    }

    pub async fn begin_import(self, index: u32) -> anyhow::Result<VisualTaskId> {
        match self.scanner.read().await.as_ref() {
            Some((_, scanner)) => Ok(scanner.begin_import(self.app, index).await?),
            None => Err(anyhow!("scan target is not set")),
        }
    }
}

impl<'a> ManagerRef<'a, InstanceManager> {
    pub fn import_manager(self) -> ManagerRef<'a, InstanceImportManager> {
        ManagerRef {
            manager: &self.app.instance_manager.import_manager,
            app: self.app,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, EnumIter, Eq, PartialEq)]
pub enum Entity {
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

impl Entity {
    pub fn get_available() -> Vec<Self> {
        use strum::IntoEnumIterator;
        Self::iter().collect()
    }

    pub fn create_importer(self) -> Arc<dyn InstanceImporter> {
        match self {
            Self::LegacyGDLauncher => Arc::new(LegacyGDLauncherImporter::new()),
            Self::CurseForgeZip => Arc::new(CurseforgeArchiveImporter::new()),
            Self::MRPack => Arc::new(ModrinthArchiveImporter::new()),
            _ => todo!(),
        }
    }
}

#[derive(Debug)]
pub struct ImportableInstance {
    pub filename: String,
    pub instance_name: String,
}

#[derive(Debug, Clone)]
pub struct InvalidImportEntry {
    pub name: String,
    pub reason: Translation,
}

#[derive(Debug)]
pub enum ImportEntry {
    Valid(ImportableInstance),
    Invalid(InvalidImportEntry),
}

pub enum ImportScanStatus {
    NoResults,
    SingleResult(ImportEntry),
    MultiResult(Vec<ImportEntry>),
}

pub struct FullImportScanStatus {
    pub scanning: bool,
    pub status: ImportScanStatus,
}

#[async_trait::async_trait]
pub trait InstanceImporter: std::fmt::Debug + Send + Sync {
    async fn scan(&self, app: &Arc<AppInner>, scan_path: PathBuf) -> anyhow::Result<()>;
    async fn get_default_scan_path(&self, app: &Arc<AppInner>) -> anyhow::Result<Option<PathBuf>>;
    async fn get_status(&self) -> ImportScanStatus;
    async fn begin_import(&self, app: &Arc<AppInner>, index: u32) -> anyhow::Result<VisualTaskId>;
}

#[derive(Debug, Clone)]
enum ImporterState<T: Clone + Into<ImportableInstance>> {
    NoResults,
    SingleResult(InternalImportEntry<T>),
    MultiResult(Vec<InternalImportEntry<T>>),
}

#[derive(Debug, Clone)]
enum InternalImportEntry<T: Clone + Into<ImportableInstance>> {
    Valid(T),
    Invalid(InvalidImportEntry),
}

impl<T: Clone + Into<ImportableInstance>> ImporterState<T> {
    async fn set_single(&mut self, entry: InternalImportEntry<T>) {
        *self = Self::SingleResult(entry);
    }

    async fn push_multi(&mut self, entry: InternalImportEntry<T>) {
        match self {
            Self::NoResults | Self::SingleResult(_) => {
                *self = Self::MultiResult(vec![entry]);
            }
            Self::MultiResult(entries) => {
                entries.push(entry);
            }
        }
    }

    async fn get(&self, index: u32) -> Option<&T> {
        match self {
            Self::SingleResult(InternalImportEntry::Valid(entry)) => Some(entry),
            Self::MultiResult(entries) => entries
                .get(index as usize)
                .map(|r| match r {
                    InternalImportEntry::Valid(entry) => Some(entry),
                    _ => None,
                })
                .flatten(),
            _ => None,
        }
    }
}

impl<T: Clone + Into<ImportableInstance>> From<ImporterState<T>> for ImportScanStatus {
    fn from(value: ImporterState<T>) -> Self {
        match value {
            ImporterState::NoResults => ImportScanStatus::NoResults,
            ImporterState::SingleResult(r) => ImportScanStatus::SingleResult(r.into()),
            ImporterState::MultiResult(r) => {
                ImportScanStatus::MultiResult(r.into_iter().map(Into::into).collect())
            }
        }
    }
}

impl<T: Clone + Into<ImportableInstance>> From<InternalImportEntry<T>> for ImportEntry {
    fn from(value: InternalImportEntry<T>) -> Self {
        match value {
            InternalImportEntry::Valid(t) => ImportEntry::Valid(t.into()),
            InternalImportEntry::Invalid(e) => ImportEntry::Invalid(e),
        }
    }
}
