//! Zips exported by MultiMC / Prism; metadata is read in place and the archive is only extracted on import.

use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::anyhow;
use tokio::sync::RwLock;
use tracing::trace;

use super::{
    ImportScanStatus, ImportableInstance, ImporterState, InstanceImporter, InternalImportEntry,
    InvalidImportEntry,
    common::{apply_imported_settings, copy_game_dir, group_for_import, resolve_standard_version},
    mmc::{MmcMeta, game_dir, parse_mmc_meta},
};
use crate::{
    api::translation::Translation,
    domain::{instance::info::GameVersion, vtask::VisualTaskId},
    managers::{AppInner, instance::InstanceVersionSource},
};

const ICON_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "webp", "ico"];

#[derive(Debug, Clone)]
struct Importable {
    filename: String,
    path: PathBuf,
    prefix: String,
    meta: MmcMeta,
    icon_entry: Option<String>,
}

impl From<Importable> for ImportableInstance {
    fn from(value: Importable) -> Self {
        Self {
            filename: value.filename.clone(),
            instance_name: value.meta.name.unwrap_or(value.filename),
        }
    }
}

#[derive(Debug)]
pub struct MmcArchiveImporter {
    state: RwLock<ImporterState<Importable>>,
}

impl MmcArchiveImporter {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(ImporterState::NoResults),
        }
    }

    async fn scan_archive(&self, path: PathBuf) -> Option<InternalImportEntry<Importable>> {
        if !path.is_file() {
            return None;
        }

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let path2 = path.clone();
        let scanned = tokio::task::spawn_blocking(move || read_archive_meta(&path2))
            .await
            .ok()?;

        Some(match scanned {
            Ok(ArchiveMeta {
                prefix,
                meta,
                icon_entry,
            }) => InternalImportEntry::Valid(Importable {
                filename: name,
                path,
                prefix,
                meta,
                icon_entry,
            }),
            Err(reason) => InternalImportEntry::Invalid(InvalidImportEntry { name, reason }),
        })
    }
}

#[async_trait::async_trait]
impl InstanceImporter for MmcArchiveImporter {
    async fn scan(&self, _app: &Arc<AppInner>, scan_path: PathBuf) -> anyhow::Result<()> {
        if scan_path.is_file() {
            if let Some(entry) = self.scan_archive(scan_path).await {
                self.state.write().await.set_single(entry).await;
            }
        } else if scan_path.is_dir() {
            let Ok(mut dir) = tokio::fs::read_dir(&scan_path).await else {
                return Ok(());
            };

            while let Some(entry) = dir.next_entry().await? {
                let path = entry.path();
                let is_zip = path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("zip"));
                if !path.is_file() || !is_zip {
                    continue;
                }
                if let Some(entry) = self.scan_archive(path).await {
                    self.state.write().await.push_multi(entry).await;
                }
            }
        }

        Ok(())
    }

    async fn get_status(&self) -> ImportScanStatus {
        self.state.read().await.clone().into()
    }

    async fn begin_import(
        &self,
        app: &Arc<AppInner>,
        index: u32,
        name: Option<String>,
    ) -> anyhow::Result<VisualTaskId> {
        trace!(?index, ?name, "Beginning MultiMC/Prism zip import");

        let instance = self
            .state
            .read()
            .await
            .get(index)
            .await
            .cloned()
            .ok_or_else(|| anyhow!("invalid importable instance index {index}"))?;

        let icon = match &instance.icon_entry {
            Some(entry) => {
                let path = instance.path.clone();
                let entry = entry.clone();
                tokio::task::spawn_blocking(move || read_icon_entry(&path, &entry))
                    .await
                    .ok()
                    .flatten()
            }
            None => None,
        };

        let standard_version = resolve_standard_version(
            app,
            &instance.meta.minecraft_version,
            &instance.meta.loaders,
        )
        .await;
        let version = GameVersion::Standard(standard_version);

        let instance_version_source = match &instance.meta.managed {
            Some(modpack) => {
                InstanceVersionSource::ModpackWithKnownVersion(version, modpack.clone(), true)
            }
            None => InstanceVersionSource::Version(version),
        };

        let extract_root = app
            .settings_manager()
            .runtime_path
            .get_temp()
            .maketmpdir()
            .await?;

        let initializer = |instance_path: PathBuf| {
            let instance = &instance;
            let extract_root = &extract_root;
            async move {
                let extract_dir = extract_root.to_path_buf();
                let archive = instance.path.clone();
                tokio::task::spawn_blocking(move || extract_archive(&archive, &extract_dir))
                    .await??;

                let instance_root = extract_root.join(&instance.prefix);
                copy_game_dir(&game_dir(&instance_root), &instance_path, |_| true).await?;
                Ok(())
            }
        };

        let group = group_for_import(app, instance.meta.settings.group.as_deref()).await?;

        let id = app
            .instance_manager()
            .create_instance_ext(
                group,
                name.unwrap_or_else(|| {
                    instance
                        .meta
                        .name
                        .clone()
                        .unwrap_or_else(|| instance.filename.clone())
                }),
                icon,
                instance.meta.seconds_played,
                instance.meta.last_played,
                instance_version_source,
                instance.meta.settings.notes.clone().unwrap_or_default(),
                initializer,
            )
            .await?;

        drop(extract_root);

        apply_imported_settings(app, id, &instance.meta.settings).await;

        app.instance_manager()
            .prepare_game(id, None, None, true)
            .await
            .map(|r| r.1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArchiveMeta {
    prefix: String,
    meta: MmcMeta,
    icon_entry: Option<String>,
}

fn read_archive_meta(path: &Path) -> Result<ArchiveMeta, Translation> {
    let file = std::fs::File::open(path).map_err(|_| Translation::InstanceImportMmcZipMalformed)?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|_| Translation::InstanceImportMmcZipMalformed)?;

    let prefix = find_instance_prefix(zip.file_names())
        .ok_or(Translation::InstanceImportMmcZipMissingConfig)?;

    let mut read_text = |name: &str| -> Option<String> {
        let mut entry = zip.by_name(name).ok()?;
        let mut text = String::new();
        entry.read_to_string(&mut text).ok()?;
        Some(text)
    };

    let cfg_text = read_text(&format!("{prefix}instance.cfg"))
        .ok_or(Translation::InstanceImportMmcZipMissingConfig)?;
    let pack_text = read_text(&format!("{prefix}mmc-pack.json"));

    let meta = parse_mmc_meta(&cfg_text, pack_text.as_deref())
        .ok_or(Translation::InstanceImportMmcZipMalformed)?;

    let icon_entry = meta
        .icon_key
        .as_deref()
        .filter(|key| !key.is_empty() && *key != "default")
        .and_then(|key| {
            let names: Vec<String> = zip.file_names().map(str::to_string).collect();
            find_icon_entry(&names, &prefix, key)
        });

    Ok(ArchiveMeta {
        prefix,
        meta,
        icon_entry,
    })
}

/// `""` at the root or `Name/` when the export wrapped the instance; the shallowest wins.
fn find_instance_prefix<'a>(names: impl Iterator<Item = &'a str>) -> Option<String> {
    names
        .filter_map(|name| {
            let name = name.replace('\\', "/");
            let prefix = name.strip_suffix("instance.cfg")?.to_string();
            let depth = prefix.matches('/').count();
            let well_formed = prefix.is_empty() || prefix.ends_with('/');
            (well_formed && depth <= 1).then_some((depth, prefix))
        })
        .min()
        .map(|(_, prefix)| prefix)
}

fn find_icon_entry(names: &[String], prefix: &str, key: &str) -> Option<String> {
    let key = key.replace('\\', "/");
    let exact = format!("{prefix}{key}");
    if names.iter().any(|n| *n == exact)
        && key
            .rsplit('.')
            .next()
            .is_some_and(|ext| ICON_EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)))
    {
        return Some(exact);
    }

    ICON_EXTENSIONS
        .iter()
        .map(|ext| format!("{prefix}{key}.{ext}"))
        .find(|candidate| names.iter().any(|n| n == candidate))
}

fn read_icon_entry(path: &Path, entry_name: &str) -> Option<(String, Vec<u8>)> {
    let file = std::fs::File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    let mut entry = zip.by_name(entry_name).ok()?;
    let mut data = Vec::new();
    entry.read_to_end(&mut data).ok()?;
    if data.is_empty() {
        return None;
    }
    let ext = entry_name.rsplit('.').next().unwrap_or("png");
    Some((format!("icon.{ext}"), data))
}

/// `ZipArchive::extract` refuses entries that would escape `dest`.
fn extract_archive(path: &Path, dest: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file)?;
    zip.extract(dest)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::domain::instance::info::ModLoaderType;
    use std::io::Write;

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (name, data) in entries {
            if name.ends_with('/') {
                zip.add_directory(*name, options).unwrap();
            } else {
                zip.start_file(*name, options).unwrap();
                zip.write_all(data).unwrap();
            }
        }
        zip.finish().unwrap();
    }

    #[test]
    fn instance_prefix_is_found_at_root_or_one_level_down() {
        assert_eq!(
            find_instance_prefix(["instance.cfg", "mmc-pack.json"].into_iter()),
            Some(String::new())
        );
        assert_eq!(
            find_instance_prefix(
                [
                    "My Pack/",
                    "My Pack/instance.cfg",
                    "My Pack/.minecraft/mods/"
                ]
                .into_iter()
            ),
            Some("My Pack/".to_string())
        );
        assert_eq!(
            find_instance_prefix(["a/b/instance.cfg", "notinstance.cfg"].into_iter()),
            None
        );
        assert_eq!(
            find_instance_prefix(["Pack\\instance.cfg", "instance.cfg"].into_iter()),
            Some(String::new())
        );
    }

    #[test]
    fn icon_entries_are_located() {
        let names: Vec<String> = ["Pack/instance.cfg", "Pack/bee.png", "Pack/photo.jpeg"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            find_icon_entry(&names, "Pack/", "bee"),
            Some("Pack/bee.png".to_string())
        );
        assert_eq!(
            find_icon_entry(&names, "Pack/", "photo.jpeg"),
            Some("Pack/photo.jpeg".to_string())
        );
        assert_eq!(find_icon_entry(&names, "Pack/", "missing"), None);
    }

    #[tokio::test]
    async fn exported_zip_is_scanned_and_extracted() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("My Pack.zip");
        write_zip(
            &zip_path,
            &[
                ("My Pack/", b""),
                (
                    "My Pack/instance.cfg",
                    b"[General]\nConfigVersion=1.2\nInstanceType=OneSix\nname=My Pack\niconKey=mypack\ntotalTimePlayed=30\n",
                ),
                (
                    "My Pack/mmc-pack.json",
                    br#"{"components":[{"uid":"net.minecraft","version":"1.20.1"},{"uid":"net.fabricmc.fabric-loader","version":"0.15.0"}]}"#,
                ),
                ("My Pack/mypack.png", b"\x89PNG"),
                ("My Pack/.minecraft/mods/sodium.jar", b"jar"),
            ],
        );

        let importer = MmcArchiveImporter::new();
        let entry = importer.scan_archive(zip_path.clone()).await.unwrap();
        let InternalImportEntry::Valid(importable) = entry else {
            panic!("expected valid entry");
        };
        assert_eq!(importable.filename, "My Pack.zip");
        assert_eq!(importable.prefix, "My Pack/");
        assert_eq!(importable.meta.name.as_deref(), Some("My Pack"));
        assert_eq!(importable.meta.minecraft_version, "1.20.1");
        assert_eq!(
            importable.meta.loaders,
            vec![DeclaredModLoader::new(ModLoaderType::Fabric, "0.15.0")]
        );
        assert_eq!(importable.meta.seconds_played, Some(30));
        assert_eq!(importable.icon_entry.as_deref(), Some("My Pack/mypack.png"));
        assert_eq!(
            read_icon_entry(&zip_path, "My Pack/mypack.png"),
            Some(("icon.png".to_string(), b"\x89PNG".to_vec()))
        );

        let dest = tmp.path().join("extract");
        extract_archive(&zip_path, &dest).unwrap();
        assert!(dest.join("My Pack/.minecraft/mods/sodium.jar").is_file());
        assert_eq!(
            game_dir(&dest.join("My Pack")),
            dest.join("My Pack/.minecraft")
        );

        std::fs::write(tmp.path().join("junk.zip"), b"nope").unwrap();
        let entry = importer
            .scan_archive(tmp.path().join("junk.zip"))
            .await
            .unwrap();
        assert!(matches!(
            entry,
            InternalImportEntry::Invalid(InvalidImportEntry {
                reason: Translation::InstanceImportMmcZipMalformed,
                ..
            })
        ));

        let other = tmp.path().join("other.zip");
        write_zip(&other, &[("manifest.json", b"{}")]);
        let entry = importer.scan_archive(other).await.unwrap();
        assert!(matches!(
            entry,
            InternalImportEntry::Invalid(InvalidImportEntry {
                reason: Translation::InstanceImportMmcZipMissingConfig,
                ..
            })
        ));
    }

    use super::super::common::DeclaredModLoader;
}
