use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use zip::{write::FileOptions, ZipWriter};

use crate::{
    domain::{
        instance::{ExportEntry, ExportTarget, InstanceId},
        vtask::VisualTaskId,
    },
    managers::ManagerRef,
};

mod curseforge_archive;

#[derive(Debug)]
pub struct InstanceExportManager {}

impl InstanceExportManager {
    pub fn new() -> Self {
        Self {}
    }
}

impl ManagerRef<'_, InstanceExportManager> {
    pub async fn export_instance(
        self,
        instance_id: InstanceId,
        target: ExportTarget,
        save_path: PathBuf,
        link_mods: bool,
        filter: ExportEntry,
    ) -> anyhow::Result<VisualTaskId> {
        match target {
            ExportTarget::Curseforge => {
                curseforge_archive::export_curseforge(
                    self.app.clone(),
                    instance_id,
                    save_path,
                    link_mods,
                    filter,
                )
                .await
            }
        }
    }
}

fn zip_excluding<W: io::Write + io::Seek>(
    zip: &mut ZipWriter<W>,
    options: FileOptions,
    base_path: &Path,
    filter: &ExportEntry,
) -> anyhow::Result<()> {
    fn walk_recursive<W: io::Write + io::Seek>(
        zip: &mut ZipWriter<W>,
        options: FileOptions,
        path: &Path,
        relpath: &[&str],
        filter: Option<&ExportEntry>,
    ) -> anyhow::Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();

            let Some(subfilter) = filter
                .as_ref()
                .map(|f| f.0.get(&*name))
                .unwrap_or(Some(&None))
            else {
                continue;
            };

            let path = PathBuf::from_iter(relpath.iter().chain([&*name].iter()));
            let pathstr = path.to_str().unwrap();

            if entry.metadata()?.is_dir() {
                let relpath = &[relpath, &[&*name][..]].concat()[..];
                walk_recursive(zip, options, &entry.path(), relpath, subfilter.as_ref())?;
            } else {
                zip.start_file(pathstr, options)?;
                io::copy(&mut File::open(entry.path())?, zip)?;
            }
        }

        Ok(())
    }

    walk_recursive(zip, options, base_path, &[], Some(filter))
}
