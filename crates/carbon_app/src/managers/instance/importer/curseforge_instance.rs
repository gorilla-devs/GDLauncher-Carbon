//! This module handles importing Curseforge instance imports.

use super::{
    ImportScanStatus, ImporterState, InstanceImporter, InternalImportEntry, InvalidImportEntry,
};
use crate::{
    api::{keys, translation::Translation},
    domain::{instance::info::GameVersion, modplatforms::curseforge::manifest::Manifest},
    managers::instance::InstanceVersionSource,
};
use crate::{
    domain::vtask::VisualTaskId,
    managers::{App, AppInner},
};
use error_stack::{report, FutureExt as _, ResultExt};
use futures::TryFutureExt;
use std::{path, sync::Arc};
use tokio::{fs, io::AsyncWriteExt, sync::Mutex};

#[derive(Clone, Debug, thiserror::Error)]
enum CurseforgeInstanceImporterError {
    #[error("not an instance directory")]
    NotAnInstanceDir,
    #[error("not a minecraft modpack")]
    NotAModpack(Manifest),
}

/// Curseforge instance importer.
#[derive(Debug, Default)]
pub struct CurseforgeInstanceImporter {
    state: Mutex<ImporterState<ImportableInstance>>,
}

#[derive(Clone, Debug)]
struct ImportableInstance {
    name: String,
    path: path::PathBuf,
    manifest: Manifest,
}

impl From<ImportableInstance> for super::ImportableInstance {
    fn from(importable: ImportableInstance) -> Self {
        Self {
            filename: importable.path.to_string_lossy().into(),
            instance_name: importable.name,
        }
    }
}

#[async_trait]
impl InstanceImporter for CurseforgeInstanceImporter {
    #[instrument(name = "CurseforgeInstanceImporter::scan", skip(self, app), err)]
    async fn scan(&self, app: &App, scan_path: path::PathBuf) -> anyhow::Result<()> {
        info!("scanning dir");

        // We expect the scan path to be a directory
        if !scan_path.is_dir() {
            return Ok(());
        }

        // Handle the entry type. If we get an entry back, then
        // we store it and notify our status change.
        //
        // If this directory is not a curseforge instance, then
        // we recurse until we find one, otherwise, if it is, and the
        // manifest is not valid, then we mark the instance as invalid, and
        // notify the status change
        match Self::parse_instance_dir(&scan_path).await {
            Ok(entry) => {
                self.state
                    .lock()
                    .await
                    .push_multi(InternalImportEntry::Valid(entry));

                app.invalidate(keys::instance::GET_IMPORT_SCAN_STATUS, None);
            }
            Err(err) => match err.current_context() {
                CurseforgeInstanceImporterError::NotAModpack(manifest) => {
                    self.state
                        .lock()
                        .await
                        .push_multi(InternalImportEntry::Invalid(InvalidImportEntry {
                            name: manifest.name.to_owned(),
                            reason: Translation::InstanceImportCfZipNotMinecraftModpack,
                        }));

                    app.invalidate(keys::instance::GET_IMPORT_SCAN_STATUS, None);
                }
                CurseforgeInstanceImporterError::NotAnInstanceDir => {
                    let mut dirs = fs::read_dir(&scan_path).await?;

                    while let Some(dir) = dirs.next_entry().await? {
                        self.scan(app, dir.path()).await?;
                    }
                }
            },
        }

        Ok(())
    }

    #[instrument(name = "CurseforgeInstanceImporter::get_status", skip_all, ?ret)]
    async fn get_status(&self) -> ImportScanStatus {
        self.state.lock().await.clone().into()
    }

    #[instrument(name = "CurseforgeInstanceImporter::begin_import", skip(app), ret, ?err)]
    async fn begin_import(
        &self,
        app: &App,
        index: u32,
        name: Option<String>,
    ) -> anyhow::Result<VisualTaskId> {
        info!("starting import");

        let importable = self
            .state
            .lock()
            .await
            .get(index)
            .cloned()
            .ok_or_else(|| anyhow!("invalid importable instance index"))?;

        let version = GameVersion::Standard(importable.manifest.minecraft.clone().try_into()?);

        let id = app
            .instance_manager()
            .create_instance_ext(
                app.instance_manager().get_default_group().await?,
                name.unwrap_or_else(|| importable.manifest.name.clone()),
                false,
                InstanceVersionSource::Version(version),
                Default::default(),
                |instance_path| async {
                    let (instance_path,) = (instance_path,);

                    let setup_path = instance_path.join(".setup");

                    fs::create_dir_all(&setup_path).await?;

                    let curseforge_zip_path = setup_path.join("curseforge");

                    // We're using a vec here to avoid needing to call
                    // `tokio::task::block_in_place`, which is very expensive
                    let zip_file_buf = std::io::Cursor::new(vec![]);

                    let mut zip = zip::write::ZipWriter::new(zip_file_buf);

                    zip.start_file("manifest.json", Default::default());
                    serde_json::to_writer(&mut zip, &importable.manifest)?;
                    let zip_file_buf = zip.finish()?.into_inner();

                    let mut file = fs::File::create(setup_path.join("curseforge")).await?;

                    file.write_all(zip_file_buf.as_slice()).await?;

                    // let dest = instance_path.join(".setup").join("curseforge");

                    // copy_dir_recursively(importable.path, dest)
                    //     .await
                    //     .map_err(|report| anyhow!(report))

                    Ok(())
                },
            )
            .await?;

        app.instance_manager()
            .prepare_game(id, None, None)
            .await
            .map(|r| r.1)
    }
}

impl CurseforgeInstanceImporter {
    /// Creates a new instance of the curseforge instance importer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attempts to parse the directory as a curseforge instance by
    /// parsing the `manifest.json` file.
    async fn parse_instance_dir(
        path: &path::Path,
    ) -> error_stack::Result<ImportableInstance, CurseforgeInstanceImporterError> {
        let manifest = fs::read_to_string(path.join("manifest.json"))
            .await
            .change_context(CurseforgeInstanceImporterError::NotAnInstanceDir)
            .attach_printable("reading manifest file")?;

        let manifest = serde_json::from_str::<Manifest>(&manifest)
            .change_context(CurseforgeInstanceImporterError::NotAnInstanceDir)
            .attach_printable("could not parse as an instance manifest file")?;

        if manifest.manifest_type != "minecraftModpack" {
            return Err(report!(CurseforgeInstanceImporterError::NotAModpack(
                manifest
            )));
        }

        Ok(ImportableInstance {
            name: manifest.name.clone(),
            path: path.to_owned(),
            manifest,
        })
    }

    pub fn get_default_scan_path() -> anyhow::Result<path::PathBuf> {
        let dirs = directories::BaseDirs::new().ok_or(anyhow!("Cannot build basedirs"))?;

        Ok(dirs
            .home_dir()
            .join("curseforge")
            .join("minecraft")
            .join("Instances"))
    }
}
