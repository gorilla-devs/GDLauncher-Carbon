use std::{path::PathBuf, sync::Arc};

use anyhow::anyhow;
use futures::FutureExt;
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    sync::{RwLock, mpsc, watch},
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, instrument::WithSubscriber, trace};

use crate::{
    api::{keys::instance::*, translation::Translation},
    domain::vtask::VisualTaskId,
    managers::{AppInner, ManagerRef, modplatforms::curseforge::CurseForge, vtask::VisualTask},
};

use self::{
    atlauncher::AtLauncherImporter,
    curseforge::CurseforgeImporter,
    curseforge_archive::CurseforgeArchiveImporter,
    ftb::FtbImporter,
    gdlpack::GdlpackImporter,
    legacy_gdlauncher::LegacyGDLauncherImporter,
    mmc::{MmcFlavor, MmcImporter},
    mmc_archive::MmcArchiveImporter,
    modrinth::ModrinthAppImporter,
    modrinth_archive::ModrinthArchiveImporter,
    technic::TechnicImporter,
    vanilla::{VanillaFlavor, VanillaImporter},
    version_folder::{VersionFolderFlavor, VersionFolderImporter},
    xmcl::XmclImporter,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportShareCodeProgress {
    Progress(i32), // 0-100
}

use super::{
    InstanceManager,
    export::{InstanceExportManager, MAX_SHARE_SIZE_BYTES},
};

mod atlauncher;
mod common;
mod curseforge;
mod curseforge_archive;
mod ftb;
mod gdlpack;
mod legacy_gdlauncher;
mod mmc;
mod mmc_archive;
mod modrinth;
mod modrinth_archive;
mod technic;
mod vanilla;
mod version_folder;
mod xmcl;

/// A share code is a server-issued opaque token, but it becomes a filename
/// component (`{share_code}.gdlpack`) before it's ever checked against anything
/// server-side, so it's validated as such rather than trusted to already be
/// safe: a plain, bounded, separator-free token.
fn is_valid_share_code(share_code: &str) -> bool {
    !share_code.is_empty()
        && share_code.len() <= 64
        && share_code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[derive(Debug)]
pub struct InstanceImportManager {
    scan_path: watch::Sender<Option<(String, Entity, PathBuf)>>,
    scanner: RwLock<Option<ScanSession>>,
}

#[derive(Debug)]
struct ScanSession {
    id: String,
    scanning: bool,
    error: Option<String>,
    scanner: Arc<dyn InstanceImporter>,
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
        // The share code becomes a filename component (`{share_code}.gdlpack`) below,
        // before it's ever checked against anything server-side. Reject anything but a
        // plain, bounded token up front so it can never be used to escape the temp
        // directory (e.g. a code containing `..` or a path separator).
        if !is_valid_share_code(&share_code) {
            return Err(anyhow!("Invalid share code"));
        }

        let (tx, rx) = mpsc::channel(10);
        let app = self.app.clone();

        let activity = app.runtime_lifecycle.admit_owned(
            "share import operation",
            crate::managers::runtime_lifecycle::ActivityPolicy::CoreWork,
        )?;
        // The public handle only awaits this already retained producer.
        let worker = crate::managers::runtime_lifecycle::spawn_retained(activity, move |slot| {
            Box::pin(async move {
                let runtime_cancel = slot.as_ref().unwrap().cancellation();
                // 0-5%: Get presigned URL
                let _ = tx.send(ImportShareCodeProgress::Progress(0)).await;

                let presigned_url = app
                    .account_manager()
                    .get_presigned_download_url(share_code.clone())
                    .await?;

                let _ = tx.send(ImportShareCodeProgress::Progress(5)).await;

                anyhow::ensure!(!cancel_token.is_cancelled(), "Import cancelled");
                anyhow::ensure!(
                    !runtime_cancel.is_cancelled(),
                    "Import cancelled by runtime shutdown"
                );
                let tmpdir = app
                    .settings_manager()
                    .runtime_path
                    .get_temp()
                    .maketmpdir()
                    .await?;

                with_share_archive(slot.take().unwrap(), tmpdir, share_code, move |file_path| async move {
            let mut file = File::create(&file_path).await?;

            let downloaded = async {
            // 5-90%: Download file with progress
            let client = reqwest::Client::new();
            let response = tokio::select! {
                biased;
                _ = cancel_token.cancelled() => return Err(anyhow!("Import cancelled")),
                _ = runtime_cancel.cancelled() => return Err(anyhow!("Import cancelled by runtime shutdown")),
                response = client.get(&presigned_url).send() => response?.error_for_status()?,
            };

            let total_size = response.content_length();
            let mut downloaded: u64 = 0;
            let mut stream = response.bytes_stream();

            use futures::StreamExt;
            loop {
                let chunk_result = tokio::select! {
                    biased;
                    _ = cancel_token.cancelled() => return Err(anyhow!("Import cancelled")),
                _ = runtime_cancel.cancelled() => return Err(anyhow!("Import cancelled by runtime shutdown")),
                    chunk = stream.next() => match chunk { Some(chunk) => chunk, None => break },
                };
                if cancel_token.is_cancelled() || runtime_cancel.is_cancelled() {
                    return Err(anyhow!("Import cancelled"));
                }

                let chunk = chunk_result?;
                downloaded += chunk.len() as u64;

                // Bound the download independent of what the server claims via
                // Content-Length (total_size, used only for the progress percentage
                // below): a malicious or misbehaving presigned URL response could
                // otherwise stream an unbounded amount of data onto disk. Genuine
                // shares are already capped at this same size on export, so nothing
                // legitimate is ever rejected here.
                if downloaded > MAX_SHARE_SIZE_BYTES {
                    return Err(anyhow!(
                        "Share code archive exceeds the {MAX_SHARE_SIZE_BYTES} byte limit"
                    ));
                }

                file.write_all(&chunk).await?;

                if let Some(total) = total_size {
                    let progress = (downloaded as f64 / total as f64 * 85.0 + 5.0) as i32;
                    let progress = progress.min(90);
                    tokio::select! {
                        _ = cancel_token.cancelled() => return Err(anyhow!("Import cancelled")),
                _ = runtime_cancel.cancelled() => return Err(anyhow!("Import cancelled by runtime shutdown")),
                        _ = tx.send(ImportShareCodeProgress::Progress(progress)) => {},
                    }
                }
            }

            file.flush().await?;
            Ok::<_, anyhow::Error>(())
            }.await;
            drop(file.into_std().await);
            downloaded?;
            anyhow::ensure!(!cancel_token.is_cancelled() && !runtime_cancel.is_cancelled(), "Import cancelled");

            // 90-95%: Scan archive
            let _ = tx.send(ImportShareCodeProgress::Progress(90)).await;

            let scanner = Entity::GDLPack.create_importer()?;
            scanner.scan(&app, file_path).await?;

            let _ = tx.send(ImportShareCodeProgress::Progress(95)).await;

            // 95-100%: Begin import
            let _ = scanner.begin_import(&app, 0, None).await?;

            let _ = tx.send(ImportShareCodeProgress::Progress(100)).await;

            Ok(())
            }).await
            })
        });
        let handle = tokio::spawn(async move { worker.await? });

        Ok((rx, handle))
    }

    pub fn set_scan_target(self, path: Option<(Entity, PathBuf)>) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.scan_path
            .send(path.map(|(entity, path)| (id.clone(), entity, path)))
            .map_err(|_| anyhow!("import scanning background task has died"))?;
        self.app.invalidate(GET_IMPORT_SCAN_STATUS, None);
        Ok(id)
    }

    pub fn launch_background_tasks(self) {
        let rx = self.scan_path.subscribe();
        let app = self.app.clone();
        tokio::spawn(async move {
            let manager = app.instance_manager().import_manager();
            manager
                .manager
                .scan_loop(
                    rx,
                    |scanner, path| {
                        let app = app.clone();
                        async move { scanner.scan(&app, path).await }
                    },
                    || app.invalidate(GET_IMPORT_SCAN_STATUS, None),
                )
                .await;
        });
    }

    pub async fn scan_status(self) -> anyhow::Result<FullImportScanStatus> {
        self.manager.status().await
    }

    pub async fn begin_import(
        self,
        session: String,
        index: u32,
        name: Option<String>,
    ) -> anyhow::Result<VisualTaskId> {
        let runtime_activity = self.app.runtime_lifecycle.admit_owned(
            "accepted instance import",
            crate::managers::runtime_lifecycle::ActivityPolicy::CoreWork,
        )?;
        let scanner = match self.manager.selected_scanner(&session).await {
            Ok(scanner) => scanner,
            Err(error) => {
                runtime_activity.finish();
                return Err(error);
            }
        };
        let app = self.app.clone();
        // Admission captures an immutable, completed scanner. Closing the modal
        // revokes new requests, but cannot drop an accepted import's private IO.
        crate::managers::runtime_lifecycle::spawn_retained(runtime_activity, move |_| {
            Box::pin(async move { scanner.begin_import(&app, index, name).await })
        })
        .await?
    }
}

impl InstanceImportManager {
    async fn scan_loop<Run, Fut, Notify>(
        &self,
        mut rx: watch::Receiver<Option<(String, Entity, PathBuf)>>,
        run: Run,
        notify: Notify,
    ) where
        Run: Fn(Arc<dyn InstanceImporter>, PathBuf) -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<()>>,
        Notify: Fn(),
    {
        while rx.changed().await.is_ok() {
            loop {
                let target = rx.borrow_and_update().clone();
                let Some((id, entity, path)) = target else {
                    *self.scanner.write().await = None;
                    notify();
                    break;
                };
                let scanner = match entity.create_importer() {
                    Ok(scanner) => scanner,
                    Err(error) => {
                        tracing::error!(%error, "Cannot create importer");
                        break;
                    }
                };
                *self.scanner.write().await = Some(ScanSession {
                    id: id.clone(),
                    scanning: true,
                    error: None,
                    scanner: scanner.clone(),
                });
                notify();
                // A panicking importer must not take the scan task, and every
                // later scan for every launcher, down with it.
                let scan = std::panic::AssertUnwindSafe(run(scanner, path.clone())).catch_unwind();
                tokio::select! {
                    biased;
                    _ = wait_for_target_change(&mut rx, &id) => continue,
                    result = scan => {
                        let result = result.unwrap_or_else(|panic| {
                            let message = panic
                                .downcast_ref::<&str>()
                                .map(|m| m.to_string())
                                .or_else(|| panic.downcast_ref::<String>().cloned())
                                .unwrap_or_default();
                            tracing::error!(%message, "Import scan panicked");
                            Err(anyhow!(
                                "The scan stopped unexpectedly. Try another folder, or send your logs to support."
                            ))
                        });
                        let mut state = self.scanner.write().await;
                        if let Some(state) = state.as_mut().filter(|s| s.id == id) {
                            state.scanning = false;
                            state.error = result.err().map(|e| format!("{}: {e:#}", path.display()));
                        }
                        notify();
                        break;
                    }
                }
            }
        }
    }

    async fn selected_scanner(&self, id: &str) -> anyhow::Result<Arc<dyn InstanceImporter>> {
        let state = self.scanner.read().await;
        let current = self.scan_path.borrow();
        let state = state
            .as_ref()
            .filter(|s| s.id == id && !s.scanning && current.as_ref().is_some_and(|t| t.0 == id))
            .ok_or_else(|| {
                anyhow!(
                    "Import scan changed or is still running; wait for completion or retry the scan"
                )
            })?;
        Ok(state.scanner.clone())
    }

    async fn status(&self) -> anyhow::Result<FullImportScanStatus> {
        let state = self.scanner.read().await;
        let current = self.scan_path.borrow().clone();
        let Some((id, _, _)) = current else {
            return Ok(FullImportScanStatus {
                scanning: false,
                status: ImportScanStatus::NoResults,
                session: None,
                error: None,
            });
        };
        match state.as_ref().filter(|s| s.id == id) {
            Some(state) => Ok(FullImportScanStatus {
                scanning: state.scanning,
                status: state.scanner.get_status().await,
                session: Some(id),
                error: state.error.clone(),
            }),
            None => Ok(FullImportScanStatus {
                scanning: true,
                status: ImportScanStatus::NoResults,
                session: Some(id),
                error: None,
            }),
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
    MultiMCZip,
    Minecraft,
    XMCL,
    LunarClient,
    BadlionClient,
    FeatherClient,
    HMCL,
    PCL2,
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
            Self::MultiMCZip => SelectionType::File,
            Self::Minecraft => SelectionType::Directory,
            Self::XMCL => SelectionType::Directory,
            Self::LunarClient => SelectionType::Directory,
            Self::BadlionClient => SelectionType::Directory,
            Self::FeatherClient => SelectionType::Directory,
            Self::HMCL => SelectionType::Directory,
            Self::PCL2 => SelectionType::Directory,
        }
    }

    pub fn list() -> Vec<(Self, bool, SelectionType)> {
        use strum::IntoEnumIterator;

        Self::iter()
            .map(|v| (v, v.create_importer().is_ok(), v.to_selection_type()))
            .collect()
    }

    pub fn create_importer(self) -> anyhow::Result<Arc<dyn InstanceImporter>> {
        Ok(match self {
            Self::LegacyGDLauncher => Arc::new(LegacyGDLauncherImporter::new()),
            Self::CurseForgeZip => Arc::new(CurseforgeArchiveImporter::new()),
            Self::MRPack => Arc::new(ModrinthArchiveImporter::new()),
            Self::CurseForge => Arc::new(CurseforgeImporter::new()),
            Self::GDLPack => Arc::new(GdlpackImporter::new()),
            Self::Modrinth => Arc::new(ModrinthAppImporter::new()),
            Self::ATLauncher => Arc::new(AtLauncherImporter::new()),
            Self::Technic => Arc::new(TechnicImporter::new()),
            Self::FTB => Arc::new(FtbImporter::new()),
            Self::MultiMC => Arc::new(MmcImporter::new(MmcFlavor::MultiMC)),
            Self::PrismLauncher => Arc::new(MmcImporter::new(MmcFlavor::Prism)),
            Self::MultiMCZip => Arc::new(MmcArchiveImporter::new()),
            Self::Minecraft => Arc::new(VanillaImporter::new(VanillaFlavor::Minecraft)),
            Self::XMCL => Arc::new(XmclImporter::new()),
            Self::LunarClient => Arc::new(VanillaImporter::new(VanillaFlavor::Lunar)),
            Self::BadlionClient => Arc::new(VanillaImporter::new(VanillaFlavor::Badlion)),
            Self::FeatherClient => Arc::new(VanillaImporter::new(VanillaFlavor::Feather)),
            Self::HMCL => Arc::new(VersionFolderImporter::new(VersionFolderFlavor::Hmcl)),
            Self::PCL2 => Arc::new(VersionFolderImporter::new(VersionFolderFlavor::Pcl2)),
        })
    }

    pub async fn get_default_scan_path(self) -> anyhow::Result<Option<PathBuf>> {
        Ok(match self {
            Self::LegacyGDLauncher => {
                Some(LegacyGDLauncherImporter::get_default_scan_path().await?)
            }
            Self::CurseForge => Some(CurseforgeImporter::get_default_scan_path().await?),
            Self::Modrinth => ModrinthAppImporter::get_default_scan_path().await?,
            Self::ATLauncher => AtLauncherImporter::get_default_scan_path().await?,
            Self::Technic => TechnicImporter::get_default_scan_path().await?,
            Self::FTB => FtbImporter::get_default_scan_path().await?,
            Self::MultiMC => MmcImporter::get_default_scan_path(MmcFlavor::MultiMC).await?,
            Self::PrismLauncher => MmcImporter::get_default_scan_path(MmcFlavor::Prism).await?,
            Self::Minecraft => {
                VanillaImporter::get_default_scan_path(VanillaFlavor::Minecraft).await?
            }
            Self::XMCL => XmclImporter::get_default_scan_path().await?,
            Self::LunarClient => {
                VanillaImporter::get_default_scan_path(VanillaFlavor::Lunar).await?
            }
            Self::BadlionClient => {
                VanillaImporter::get_default_scan_path(VanillaFlavor::Badlion).await?
            }
            Self::FeatherClient => {
                VanillaImporter::get_default_scan_path(VanillaFlavor::Feather).await?
            }
            Self::HMCL => {
                VersionFolderImporter::get_default_scan_path(VersionFolderFlavor::Hmcl).await?
            }
            Self::PCL2 => {
                VersionFolderImporter::get_default_scan_path(VersionFolderFlavor::Pcl2).await?
            }
            Self::CurseForgeZip | Self::MRPack | Self::GDLPack | Self::MultiMCZip => None,
        })
    }
}

#[derive(Debug)]
pub struct ImportableInstance {
    pub filename: String,
    pub instance_name: String,
    /// Parts of the source left out of the import, shown before importing.
    pub warnings: Vec<Translation>,
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
    pub session: Option<String>,
    pub error: Option<String>,
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
            Self::SingleResult(InternalImportEntry::Valid(entry)) if index == 0 => Some(entry),
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn valid_share_codes_are_accepted() {
        assert!(is_valid_share_code("abcDEF123"));
        assert!(is_valid_share_code("a-code_with-mixed_separators-123"));
        assert!(is_valid_share_code(&"a".repeat(64)));
    }

    /// Regression test: the share code becomes a filename component
    /// (`{share_code}.gdlpack`) with no other validation, so anything that could
    /// escape the intended temp directory must be rejected up front.
    #[test]
    fn share_codes_that_could_escape_the_temp_directory_are_rejected() {
        assert!(!is_valid_share_code("../../etc/passwd"));
        assert!(!is_valid_share_code(".."));
        assert!(!is_valid_share_code("foo/bar"));
        assert!(!is_valid_share_code("foo\\bar"));
        assert!(!is_valid_share_code("/etc/passwd"));
    }

    #[test]
    fn empty_or_overlong_share_codes_are_rejected() {
        assert!(!is_valid_share_code(""));
        assert!(!is_valid_share_code(&"a".repeat(65)));
    }

    #[test]
    fn every_import_entity_is_supported() {
        for (entity, supported, _) in Entity::list() {
            assert!(supported, "{entity:?} has no importer");
            assert!(entity.create_importer().is_ok(), "{entity:?}");
        }
    }

    #[tokio::test]
    async fn launcher_scan_reports_valid_and_invalid_instances() {
        let env = crate::setup_managers_for_test().await;
        let app = &env.app;

        let tmp = tempfile::tempdir().unwrap();
        let instances = tmp.path().join("instances");

        let good = instances.join("Good");
        std::fs::create_dir_all(good.join(".minecraft")).unwrap();
        std::fs::write(
            good.join("instance.cfg"),
            "[General]\nConfigVersion=1.2\nInstanceType=OneSix\nname=Good Pack\n",
        )
        .unwrap();
        std::fs::write(
            good.join("mmc-pack.json"),
            r#"{"components":[{"uid":"net.minecraft","version":"1.20.1"}]}"#,
        )
        .unwrap();

        let broken = instances.join("Broken");
        std::fs::create_dir_all(&broken).unwrap();
        std::fs::write(broken.join("instance.cfg"), "name=No Version\n").unwrap();

        std::fs::create_dir_all(instances.join("not-an-instance")).unwrap();

        let scanner = Entity::PrismLauncher.create_importer().unwrap();
        scanner.scan(app, instances.clone()).await.unwrap();

        let ImportScanStatus::MultiResult(entries) = scanner.get_status().await else {
            panic!("expected a multi result");
        };
        assert_eq!(entries.len(), 2);
        let valid: Vec<_> = entries
            .iter()
            .filter_map(|e| match e {
                ImportEntry::Valid(i) => Some(i.instance_name.as_str()),
                ImportEntry::Invalid(_) => None,
            })
            .collect();
        assert_eq!(valid, vec!["Good Pack"]);
        assert!(entries.iter().any(|e| matches!(
            e,
            ImportEntry::Invalid(InvalidImportEntry { name, .. }) if name == "Broken"
        )));

        let scanner = Entity::PrismLauncher.create_importer().unwrap();
        scanner.scan(app, good).await.unwrap();
        assert!(matches!(
            scanner.get_status().await,
            ImportScanStatus::SingleResult(ImportEntry::Valid(ImportableInstance {
                instance_name,
                ..
            })) if instance_name == "Good Pack"
        ));

        let scanner = Entity::PrismLauncher.create_importer().unwrap();
        scanner.scan(app, tmp.path().join("missing")).await.unwrap();
        assert!(matches!(
            scanner.get_status().await,
            ImportScanStatus::NoResults
        ));
    }

    #[tokio::test]
    async fn begin_import_creates_a_configured_instance() {
        use crate::domain::instance::info::{
            CurseforgeModpack, GameResolution, JavaOverride, ModLoaderType, Modpack,
        };

        let env = crate::setup_managers_for_test().await;
        let app = &env.app;

        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path();
        let (_, png) = common::icon_from_data_url(
            "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==",
        )
        .unwrap();
        std::fs::create_dir_all(data.join("icons")).unwrap();
        std::fs::write(data.join("icons").join("mypack.png"), png).unwrap();
        let good = data.join("instances").join("Good");
        std::fs::create_dir_all(&good).unwrap();
        std::fs::write(
            data.join("instances").join("instgroups.json"),
            r#"{"formatVersion":"1","groups":{"Modded":{"hidden":false,"instances":["Good"]}}}"#,
        )
        .unwrap();
        let game = good.join(".minecraft");
        std::fs::create_dir_all(game.join("mods")).unwrap();
        std::fs::create_dir_all(game.join("saves").join("World")).unwrap();
        std::fs::write(game.join("mods").join("foo.jar"), b"jar").unwrap();
        std::fs::write(game.join("saves").join("World").join("level.dat"), b"dat").unwrap();
        std::fs::write(game.join("options.txt"), "lang:en_us\n").unwrap();
        // The override survives the scan only if its binary is there.
        let java = good.join("java-binary");
        std::fs::write(&java, b"").unwrap();
        let java = java.to_str().unwrap().to_string();
        let java_in_cfg = java.replace('\\', "\\\\");
        std::fs::write(
            good.join("instance.cfg"),
            format!("[General]\nConfigVersion=1.2\nInstanceType=OneSix\nname=Good Pack\niconKey=mypack\nnotes=hello\n\
             totalTimePlayed=120\nlastLaunchTime=1700000000000\n\
             ManagedPack=true\nManagedPackType=flame\nManagedPackID=12345\nManagedPackVersionID=67890\n\
             OverrideMemory=true\nMinMemAlloc=1024\nMaxMemAlloc=8192\n\
             OverrideJavaArgs=true\nJvmArgs=-XX:+UseG1GC\n\
             OverrideJavaLocation=true\nJavaPath={java_in_cfg}\n\
             OverrideCommands=true\nPreLaunchCommand=echo pre\nPostExitCommand=echo post\nWrapperCommand=gamemoderun\n\
             OverrideWindow=true\nMinecraftWinWidth=1920\nMinecraftWinHeight=1080\n"),
        )
        .unwrap();
        std::fs::write(
            good.join("mmc-pack.json"),
            r#"{"components":[{"uid":"net.minecraft","version":"1.20.1"},{"uid":"net.minecraftforge","version":"47.2.0"}]}"#,
        )
        .unwrap();

        let importer = Entity::PrismLauncher.create_importer().unwrap();
        importer.scan(app, good).await.unwrap();
        importer
            .begin_import(app, 0, Some("Imported Pack".to_string()))
            .await
            .unwrap();

        let groups = app.instance_manager().list_groups().await.unwrap();
        let group = groups
            .iter()
            .find(|g| g.name == "Modded")
            .expect("the launcher's group is created");
        assert_eq!(group.instances.len(), 1);
        let id = group.instances[0].id;

        let details = app.instance_manager().instance_details(id).await.unwrap();
        assert_eq!(details.name, "Imported Pack");
        assert_eq!(details.version.as_deref(), Some("1.20.1"));
        assert_eq!(details.modloaders.len(), 1);
        assert_eq!(details.modloaders[0].type_, ModLoaderType::Forge);
        assert!(details.modloaders[0].version.contains("47.2.0"));
        assert_eq!(
            details.modpack.map(|m| m.modpack),
            Some(Modpack::Curseforge(CurseforgeModpack {
                project_id: 12345,
                file_id: 67890,
            }))
        );
        assert_eq!(details.notes, "hello");
        assert_eq!(details.seconds_played, 120);
        assert_eq!(
            details.last_played.map(|d| d.timestamp()),
            Some(1_700_000_000)
        );
        assert_eq!(details.icon_revision, Some(1));
        assert_eq!(details.memory, Some((1024, 8192)));
        assert_eq!(details.extra_java_args.as_deref(), Some("-XX:+UseG1GC"));
        assert!(matches!(
            details.java_override,
            Some(JavaOverride::Path(Some(ref p))) if *p == java
        ));
        assert_eq!(details.pre_launch_hook.as_deref(), Some("echo pre"));
        assert_eq!(details.post_exit_hook.as_deref(), Some("echo post"));
        assert_eq!(details.wrapper_command.as_deref(), Some("gamemoderun"));
        assert_eq!(
            details.game_resolution,
            Some(GameResolution::Custom(1920, 1080))
        );

        let shortpath = app
            .instance_manager()
            .instances
            .read()
            .await
            .get(&id)
            .unwrap()
            .shortpath
            .clone();
        let root = app
            .settings_manager()
            .runtime_path
            .get_instances()
            .to_path()
            .join(shortpath);
        assert!(root.join(".setup").join("modpack-complete").is_dir());
        assert_eq!(
            std::fs::read(root.join("instance").join("mods").join("foo.jar")).unwrap(),
            b"jar"
        );
        assert!(
            root.join("instance")
                .join("saves")
                .join("World")
                .join("level.dat")
                .is_file()
        );
        assert_eq!(
            std::fs::read_to_string(root.join("instance").join("options.txt")).unwrap(),
            "lang:en_us\n"
        );
        assert!(!root.join("instance").join("instance.cfg").exists());
    }
}

async fn wait_for_target_change(
    rx: &mut watch::Receiver<Option<(String, Entity, PathBuf)>>,
    id: &str,
) {
    while rx.changed().await.is_ok() {
        if !rx.borrow().as_ref().is_some_and(|target| target.0 == id) {
            break;
        }
    }
}
#[cfg(test)]
mod task13_watch_tests {
    use super::*;
    #[tokio::test]
    async fn task13_replacement_and_cancel_interrupt_active_scan() {
        for target in [
            Some(("B".into(), Entity::CurseForge, PathBuf::from("synthetic-B"))),
            None,
        ] {
            let (tx, mut rx) = watch::channel(Some((
                "A".into(),
                Entity::CurseForge,
                PathBuf::from("synthetic-A"),
            )));
            tx.send(target).unwrap();
            assert!(
                tokio::time::timeout(
                    std::time::Duration::from_millis(50),
                    wait_for_target_change(&mut rx, "A")
                )
                .await
                .is_ok()
            );
        }
    }
}

/// Retain temporary storage through outstanding IO even if the request is dropped.
async fn with_share_archive<T, F, Fut>(
    activity: crate::managers::runtime_lifecycle::RuntimeActivity,
    tmpdir: carbon_rt_path::TempEntry<carbon_rt_path::tempentry::Folder>,
    share_code: String,
    work: F,
) -> anyhow::Result<T>
where
    T: Send + 'static,
    F: FnOnce(PathBuf) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = anyhow::Result<T>> + Send + 'static,
{
    let dispatcher = tracing::dispatcher::get_default(Clone::clone);
    crate::managers::runtime_lifecycle::spawn_retained(activity, move |_| {
        Box::pin(
            async move {
                let file_path = tmpdir.join(format!("{share_code}.gdlpack"));
                info!(
                    "downloading instance share code {share_code} to {file_path:?} ({:?})",
                    &*tmpdir
                );
                let result = work(file_path).await;
                drop(tmpdir); // synchronous archive cleanup precedes the operation ACK
                result
            }
            .with_subscriber(dispatcher),
        )
    })
    .await?
}

#[cfg(test)]
fn test_import_activity() -> crate::managers::runtime_lifecycle::RuntimeActivity {
    let lifecycle = Arc::new(crate::managers::runtime_lifecycle::RuntimeLifecycle::default());
    lifecycle
        .admit_owned(
            "fixture import",
            crate::managers::runtime_lifecycle::ActivityPolicy::CoreWork,
        )
        .unwrap()
}

#[cfg(test)]
mod task13_temp_tests {
    use super::*;
    use tracing::Instrument;
    #[tokio::test]
    async fn task13_info_logging_retains_temporary_owner() {
        let t = tempfile::tempdir().unwrap();
        let tmp = carbon_rt_path::TempPath::new(t.path().to_path_buf())
            .maketmpdir()
            .await
            .unwrap();
        let directory = tmp.to_path_buf();
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_writer(std::io::sink)
            .finish();
        let task = with_share_archive(
            test_import_activity(),
            tmp,
            "synthetic".into(),
            |path| async move {
                tokio::fs::write(path, b"synthetic archive").await?;
                Ok(())
            },
        )
        .with_subscriber(subscriber);
        task.await.unwrap();
        assert!(
            !directory.exists(),
            "completed import must release its temporary archive"
        );
    }
}

#[cfg(test)]
mod task13_session_tests {
    use super::*;
    #[tokio::test]
    async fn task13_latest_target_runs_cancellation_revokes_and_errors_retry() {
        let manager = Arc::new(InstanceImportManager::new());
        let rx = manager.scan_path.subscribe();
        let (started, mut starts) = mpsc::unbounded_channel();
        let (notice, mut notices) = mpsc::unbounded_channel();
        let copy = manager.clone();
        let worker = tokio::spawn(async move {
            copy.scan_loop(
                rx,
                move |_, path| {
                    let started = started.clone();
                    async move {
                        started.send(path.clone()).unwrap();
                        if path == PathBuf::from("A") {
                            std::future::pending::<()>().await;
                        }
                        if path == PathBuf::from("B") {
                            anyhow::bail!("synthetic late directory failure");
                        }
                        Ok(())
                    }
                },
                || {
                    let _ = notice.send(());
                },
            )
            .await;
        });
        manager
            .scan_path
            .send(Some(("one".into(), Entity::CurseForge, "A".into())))
            .unwrap();
        assert_eq!(starts.recv().await.unwrap(), PathBuf::from("A"));
        assert!(manager.selected_scanner("one").await.is_err());
        manager
            .scan_path
            .send(Some(("two".into(), Entity::CurseForge, "B".into())))
            .unwrap();
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(1), starts.recv())
                .await
                .unwrap()
                .unwrap(),
            PathBuf::from("B")
        );
        loop {
            let status = manager.status().await.unwrap();
            if !status.scanning {
                assert!(status.error.unwrap().contains("synthetic late"));
                break;
            }
            notices.recv().await.unwrap();
        }
        assert!(manager.selected_scanner("one").await.is_err());
        let accepted = manager.selected_scanner("two").await.unwrap();
        manager.scan_path.send(None).unwrap();
        assert!(manager.selected_scanner("two").await.is_err());
        assert!(manager.status().await.unwrap().session.is_none());
        assert!(matches!(
            accepted.get_status().await,
            ImportScanStatus::NoResults
        ));
        manager
            .scan_path
            .send(Some(("retry".into(), Entity::CurseForge, "C".into())))
            .unwrap();
        assert_eq!(starts.recv().await.unwrap(), PathBuf::from("C"));
        loop {
            let status = manager.status().await.unwrap();
            if !status.scanning {
                assert!(status.error.is_none());
                break;
            }
            notices.recv().await.unwrap();
        }
        worker.abort();
    }
    #[tokio::test]
    async fn task13_temp_survives_dropped_request_until_owned_io_settles() {
        let t = tempfile::tempdir().unwrap();
        let tmp = carbon_rt_path::TempPath::new(t.path().to_path_buf())
            .maketmpdir()
            .await
            .unwrap();
        let path = tmp.to_path_buf();
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, pending) = tokio::sync::oneshot::channel();
        let (finished, done) = tokio::sync::oneshot::channel();
        let request = tokio::spawn(with_share_archive(
            test_import_activity(),
            tmp,
            "synthetic".into(),
            move |file| async move {
                tokio::fs::write(&file, b"owned IO").await?;
                started.send(()).unwrap();
                pending.await.unwrap();
                tokio::fs::write(&file, b"settled IO").await?;
                let _ = finished.send(());
                Err::<(), _>(anyhow!("synthetic completed error"))
            },
        ));
        ready.await.unwrap();
        request.abort();
        assert!(path.exists());
        release.send(()).unwrap();
        done.await.unwrap();
        for _ in 0..100 {
            if !path.exists() {
                return;
            }
            tokio::task::yield_now().await;
        }
        panic!("owned temporary storage remained after IO settlement");
    }
}

#[cfg(test)]
mod scan_panic_tests {
    use super::*;

    async fn finished(
        manager: &InstanceImportManager,
        session: &str,
        notices: &mut mpsc::UnboundedReceiver<()>,
    ) -> FullImportScanStatus {
        loop {
            let status = manager.status().await.unwrap();
            if status.session.as_deref() == Some(session) && !status.scanning {
                return status;
            }
            notices.recv().await.unwrap();
        }
    }

    /// A drive root holding a legacy config.json used to panic the scanner and
    /// leave every importer unusable until GDLauncher restarted.
    #[tokio::test]
    async fn a_panicking_scan_is_reported_and_later_scans_still_run() {
        let manager = Arc::new(InstanceImportManager::new());
        let rx = manager.scan_path.subscribe();
        let (notice, mut notices) = mpsc::unbounded_channel();
        let copy = manager.clone();
        let worker = tokio::spawn(async move {
            copy.scan_loop(
                rx,
                |_, path| async move {
                    if path == PathBuf::from("panics") {
                        panic!("synthetic importer bug");
                    }
                    Ok::<(), anyhow::Error>(())
                },
                || {
                    let _ = notice.send(());
                },
            )
            .await;
        });
        manager
            .scan_path
            .send(Some((
                "one".into(),
                Entity::LegacyGDLauncher,
                "panics".into(),
            )))
            .unwrap();
        let status = finished(&manager, "one", &mut notices).await;
        let error = status.error.unwrap();
        assert!(error.contains("The scan stopped unexpectedly"), "{error}");
        // The bug's own text is for the log, not for the banner.
        assert!(!error.contains("synthetic importer bug"), "{error}");

        manager
            .scan_path
            .send(Some(("two".into(), Entity::CurseForge, "fine".into())))
            .unwrap();
        let status = finished(&manager, "two", &mut notices).await;
        assert!(status.error.is_none());
        assert!(!worker.is_finished());
        worker.abort();
    }
}
