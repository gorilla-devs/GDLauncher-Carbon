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
        // The share code becomes a filename component (`{share_code}.gdlpack`) below,
        // before it's ever checked against anything server-side. Reject anything but a
        // plain, bounded token up front so it can never be used to escape the temp
        // directory (e.g. a code containing `..` or a path separator).
        if !is_valid_share_code(&share_code) {
            return Err(anyhow!("Invalid share code"));
        }

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
        std::fs::write(
            good.join("instance.cfg"),
            "[General]\nConfigVersion=1.2\nInstanceType=OneSix\nname=Good Pack\niconKey=mypack\nnotes=hello\n\
             totalTimePlayed=120\nlastLaunchTime=1700000000000\n\
             ManagedPack=true\nManagedPackType=flame\nManagedPackID=12345\nManagedPackVersionID=67890\n\
             OverrideMemory=true\nMinMemAlloc=1024\nMaxMemAlloc=8192\n\
             OverrideJavaArgs=true\nJvmArgs=-XX:+UseG1GC\n\
             OverrideJavaLocation=true\nJavaPath=/usr/bin/java\n\
             OverrideCommands=true\nPreLaunchCommand=echo pre\nPostExitCommand=echo post\nWrapperCommand=gamemoderun\n\
             OverrideWindow=true\nMinecraftWinWidth=1920\nMinecraftWinHeight=1080\n",
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
            Some(JavaOverride::Path(Some(ref p))) if p == "/usr/bin/java"
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
