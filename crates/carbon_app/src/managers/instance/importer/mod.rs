use std::{path::PathBuf, sync::Arc};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    sync::{RwLock, mpsc, watch},
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, trace};

use crate::{
    api::{keys::instance::*, translation::Translation},
    domain::vtask::VisualTaskId,
    managers::{AppInner, ManagerRef, modplatforms::curseforge::CurseForge, vtask::VisualTask},
};

use self::{
    curseforge::CurseforgeImporter, curseforge_archive::CurseforgeArchiveImporter,
    gdlpack::GdlpackImporter, legacy_gdlauncher::LegacyGDLauncherImporter,
    modrinth_archive::ModrinthArchiveImporter,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportShareCodeProgress {
    Progress(i32), // 0-100
}

use super::{InstanceManager, export::InstanceExportManager};

mod curseforge;
mod curseforge_archive;
mod gdlpack;
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
    pub async fn import_instance_share_code_with_progress(
        self,
        share_code: String,
        cancel_token: CancellationToken,
    ) -> anyhow::Result<(
        mpsc::Receiver<ImportShareCodeProgress>,
        tokio::task::JoinHandle<Result<(), anyhow::Error>>,
    )> {
        let (tx, rx) = mpsc::channel(10);
        let app = self.app.clone();

        let handle = tokio::spawn(async move {
            // 0-5%: Get presigned URL
            let _ = tx.send(ImportShareCodeProgress::Progress(0)).await;

            let presigned_url = app
                .account_manager()
                .get_presigned_download_url(share_code.clone())
                .await?;

            let _ = tx.send(ImportShareCodeProgress::Progress(5)).await;

            let tmpdir = app
                .settings_manager()
                .runtime_path
                .get_temp()
                .maketmpdir()
                .await?;

            let file_path = tmpdir.join(format!("{share_code}.gdlpack"));

            info!(
                "downloading instance share code {share_code} to {file_path:?} ({:?})",
                tmpdir.into_path()
            );

            let mut file = File::create(&file_path).await?;

            // 5-90%: Download file with progress
            let client = reqwest::Client::new();
            let response = client
                .get(&presigned_url)
                .send()
                .await?
                .error_for_status()?;

            let total_size = response.content_length();
            let mut downloaded: u64 = 0;
            let mut stream = response.bytes_stream();

            use futures::StreamExt;
            while let Some(chunk_result) = stream.next().await {
                if cancel_token.is_cancelled() {
                    return Err(anyhow!("Import cancelled"));
                }

                let chunk = chunk_result?;
                file.write_all(&chunk).await?;
                downloaded += chunk.len() as u64;

                if let Some(total) = total_size {
                    let progress = (downloaded as f64 / total as f64 * 85.0 + 5.0) as i32;
                    let progress = progress.min(90);
                    let _ = tx.send(ImportShareCodeProgress::Progress(progress)).await;
                }
            }

            file.flush().await?;

            // 90-95%: Scan archive
            let _ = tx.send(ImportShareCodeProgress::Progress(90)).await;

            let scanner = Entity::GDLPack.create_importer()?;
            scanner.scan(&app, file_path).await?;

            let _ = tx.send(ImportShareCodeProgress::Progress(95)).await;

            // 95-100%: Begin import
            let _ = scanner.begin_import(&app, 0, None).await?;

            let _ = tx.send(ImportShareCodeProgress::Progress(100)).await;

            Ok(())
        });

        Ok((rx, handle))
    }

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

                    let scanner = match entity.create_importer() {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::error!("Cannot create importer for {:?}: {}", entity, e);
                            *app.instance_manager()
                                .import_manager()
                                .scanner
                                .write()
                                .await = None;
                            continue;
                        }
                    };

                    *app.instance_manager()
                        .import_manager()
                        .scanner
                        .write()
                        .await = Some((true, scanner.clone()));

                    app.invalidate(GET_IMPORT_SCAN_STATUS, None);

                    let fut = async {
                        let r = scanner.scan(&app, path.clone()).await;

                        {
                            let import_manager = app.instance_manager().import_manager();

                            let mut scanner = import_manager.scanner.write().await;

                            if let Some((scanning, _)) = &mut *scanner {
                                *scanning = false;
                                app.invalidate(GET_IMPORT_SCAN_STATUS, None);
                            }
                        }

                        if let Err(e) = r {
                            tracing::error!({ error = ?e }, "instance scanning failed for path {path:?}");
                        }
                    };

                    app.invalidate(GET_IMPORT_SCAN_STATUS, None);

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
            None => Ok(FullImportScanStatus {
                status: ImportScanStatus::NoResults,
                scanning: false,
            }),
        }
    }

    pub async fn begin_import(
        self,
        index: u32,
        name: Option<String>,
    ) -> anyhow::Result<VisualTaskId> {
        trace!("Beginning import for option {index} with name {name:?}");

        match self.scanner.read().await.as_ref() {
            Some((_, scanner)) => Ok(scanner.begin_import(self.app, index, name).await?),
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

    pub fn export_manager(self) -> ManagerRef<'a, InstanceExportManager> {
        ManagerRef {
            manager: &self.app.instance_manager.export_manager,
            app: self.app,
        }
    }
}

pub enum SelectionType {
    File,
    Directory,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, EnumIter, Eq, PartialEq)]
pub enum Entity {
    LegacyGDLauncher,
    CurseForgeZip,
    CurseForge,
    MRPack,
    Modrinth,
    ATLauncher,
    Technic,
    FTB,
    MultiMC,
    PrismLauncher,
    GDLPack,
}

impl Entity {
    pub fn to_selection_type(self) -> SelectionType {
        match self {
            Self::LegacyGDLauncher => SelectionType::Directory,
            Self::CurseForgeZip => SelectionType::File,
            Self::CurseForge => SelectionType::Directory,
            Self::MRPack => SelectionType::File,
            Self::Modrinth => SelectionType::Directory,
            Self::ATLauncher => SelectionType::Directory,
            Self::Technic => SelectionType::Directory,
            Self::FTB => SelectionType::Directory,
            Self::MultiMC => SelectionType::Directory,
            Self::PrismLauncher => SelectionType::Directory,
            Self::GDLPack => SelectionType::File,
        }
    }

    pub fn list() -> Vec<(Self, bool, SelectionType)> {
        use strum::IntoEnumIterator;

        const SUPPORT: [Entity; 5] = [
            Entity::LegacyGDLauncher,
            Entity::CurseForgeZip,
            Entity::MRPack,
            Entity::CurseForge,
            Entity::GDLPack,
        ];

        Self::iter()
            .map(|v| (v, SUPPORT.contains(&v), v.to_selection_type()))
            .collect()
    }

    pub fn create_importer(self) -> anyhow::Result<Arc<dyn InstanceImporter>> {
        Ok(match self {
            Self::LegacyGDLauncher => Arc::new(LegacyGDLauncherImporter::new()),
            Self::CurseForgeZip => Arc::new(CurseforgeArchiveImporter::new()),
            Self::MRPack => Arc::new(ModrinthArchiveImporter::new()),
            Self::CurseForge => Arc::new(CurseforgeImporter::new()),
            Self::GDLPack => Arc::new(GdlpackImporter::new()),
            other => {
                return Err(anyhow::anyhow!(
                    "Importer for {:?} is not implemented",
                    other
                ));
            }
        })
    }

    pub async fn get_default_scan_path(self) -> anyhow::Result<Option<PathBuf>> {
        Ok(match self {
            Self::LegacyGDLauncher => {
                Some(LegacyGDLauncherImporter::get_default_scan_path().await?)
            }
            Self::CurseForge => Some(CurseforgeImporter::get_default_scan_path().await?),
            _ => None,
        })
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
    async fn get_status(&self) -> ImportScanStatus;
    async fn begin_import(
        &self,
        app: &Arc<AppInner>,
        index: u32,
        name: Option<String>,
    ) -> anyhow::Result<(VisualTaskId)>;
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
