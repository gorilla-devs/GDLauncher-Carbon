use std::{
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::Deref,
    path::{Path, PathBuf},
    sync::atomic::{self, AtomicUsize},
};

use anyhow::Context;
use tokio::io::AsyncWriteExt;

#[derive(Clone)]
pub struct RuntimePath(PathBuf);

pub struct RootPath(PathBuf);

impl RootPath {
    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
    }
}

#[derive(Clone)]
pub struct LibrariesPath(PathBuf);

impl LibrariesPath {
    pub fn get_mc_client(&self, id: &str) -> PathBuf {
        self.0
            .join("net/minecraft/client")
            .join(id)
            .join(format!("{}.jar", id))
    }
    pub fn get_library_path(&self, library_path: String) -> PathBuf {
        self.0.join(library_path)
    }

    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
    }
}

pub struct AssetsPath(PathBuf);

impl AssetsPath {
    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
    }

    pub fn get_indexes_path(&self) -> PathBuf {
        self.0.join("indexes")
    }

    pub fn get_virtual_path(&self) -> PathBuf {
        self.0.join("virtual")
    }

    pub fn get_objects_path(&self) -> PathBuf {
        self.0.join("objects")
    }
}

pub struct NativesPath(PathBuf);

impl NativesPath {
    pub fn get_versioned(&self, version: &str) -> PathBuf {
        self.0.clone().join(version)
    }
}

// TODO: WIP
pub struct ManagedJavasPath(PathBuf);

impl ManagedJavasPath {
    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
    }
}

pub struct LoggingConfigsPath(PathBuf);

impl LoggingConfigsPath {
    pub fn get_client_path(&self, id: &str) -> PathBuf {
        self.0.clone().join(&id)
    }
}

// TODO: WIP
pub struct InstancesPath(PathBuf);

impl InstancesPath {
    pub fn subpath() -> InstancesPath {
        Self(PathBuf::from("instances"))
    }

    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
    }

    pub fn get_instance_path(&self, instance_id: &str) -> InstancePath {
        InstancePath(self.0.join(instance_id))
    }
}

#[derive(Debug, Clone)]
pub struct InstancePath(PathBuf);

impl InstancePath {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn get_root(&self) -> PathBuf {
        self.0.clone()
    }

    pub fn get_data_path(&self) -> PathBuf {
        self.0.join("instance")
    }

    pub fn get_resources_path(&self) -> PathBuf {
        self.get_data_path().join("resources")
    }

    pub fn get_mods_path(&self) -> PathBuf {
        self.get_data_path().join("mods")
    }

    pub fn get_config_path(&self) -> PathBuf {
        self.get_data_path().join("config")
    }

    pub fn get_resourcepacks_path(&self) -> PathBuf {
        self.get_data_path().join("resourcepacks")
    }

    pub fn get_texturepacks_path(&self) -> PathBuf {
        self.get_data_path().join("texturepacks")
    }

    pub fn get_shaderpacks_path(&self) -> PathBuf {
        self.get_data_path().join("shaderpacks")
    }

    pub fn get_saves_path(&self) -> PathBuf {
        self.get_data_path().join("saves")
    }

    pub fn get_logs_path(&self) -> PathBuf {
        self.get_data_path().join("logs")
    }
    pub fn get_plugins_path(&self) -> PathBuf {
        self.get_data_path().join("plugins")
    }

    pub fn get_crash_reports_path(&self) -> PathBuf {
        self.get_data_path().join("crash-reports")
    }

    pub fn get_screenshots_path(&self) -> PathBuf {
        self.get_data_path().join("screenshots")
    }

    pub fn get_options_file_path(&self) -> PathBuf {
        self.get_data_path().join("options.txt")
    }
}

pub struct TempPath(PathBuf);

impl TempPath {
    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
    }

    pub async fn maketmp<T: tempentry::TempEntryType>(&self) -> anyhow::Result<TempEntry<T>> {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time is somehow pre-epoch")
            .as_millis() as usize;

        let mut path = self.to_path();

        loop {
            static LAST_COUNT: AtomicUsize = AtomicUsize::new(0);
            let i = LAST_COUNT.fetch_add(1, atomic::Ordering::Relaxed);

            if i == 0 {
                path.push(time.to_string());
            } else {
                path.push(format!("{time}{i}"));
            }

            if path.exists() {
                continue;
            }

            let path_copy = path.clone();

            if tokio::task::spawn_blocking(move || T::create(&path_copy))
                .await
                .is_ok()
            {
                return Ok(TempEntry(path, PhantomData));
            }

            path.pop();
        }
    }

    pub async fn maketmpdir(&self) -> anyhow::Result<TempEntry<tempentry::Folder>> {
        self.maketmp().await
    }

    pub async fn maketmpfile(&self) -> anyhow::Result<TempEntry<tempentry::File>> {
        self.maketmp().await
    }

    pub async fn write_file_atomic(
        &self,
        path: impl AsRef<Path>,
        data: impl AsRef<[u8]>,
    ) -> anyhow::Result<()> {
        let tmp = self.maketmpfile().await?;

        let mut fd = tokio::fs::File::create(&*tmp).await?;
        fd.write_all(data.as_ref()).await?;
        fd.sync_all().await?;

        tmp.try_rename_or_move(path).await?;

        Ok(())
    }
}

pub struct TempEntry<T: tempentry::TempEntryType>(PathBuf, PhantomData<T>);

pub mod tempentry {
    use std::{io, path::Path};

    pub trait TempEntryType {
        // tokio::fs is just implemented as a spawn_blocking wrapper for the most part.
        fn create(path: &Path) -> io::Result<()>;
        fn drop_for(path: &Path);
    }

    pub struct Folder;
    pub struct File;

    impl TempEntryType for Folder {
        fn create(path: &Path) -> io::Result<()> {
            std::fs::create_dir_all(path)
        }

        fn drop_for(path: &Path) {
            let _ = std::fs::remove_dir_all(path);
        }
    }

    impl TempEntryType for File {
        fn create(path: &Path) -> io::Result<()> {
            path.parent()
                .map(|path| std::fs::create_dir_all(path))
                .transpose()?;

            // files will be created on write
            Ok(())
        }

        fn drop_for(path: &Path) {
            let _ = std::fs::remove_file(path);
        }
    }
}

impl<T: tempentry::TempEntryType> TempEntry<T> {
    /// Extract the contained path without deleting the tmpdir.
    pub fn into_path(self) -> PathBuf {
        let v = ManuallyDrop::new(self);

        // SAFETY: v is not dropped so v.0 can be extracted safely.
        let path = unsafe { std::ptr::read(&v.0 as *const PathBuf) };

        path
    }

    pub async fn try_rename_or_move(self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let res = tokio::fs::rename(&*self, &path).await;

        if let Err(err) = &res {
            tokio::fs::copy(&path, &*self).await.with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    path.as_ref().display(),
                    (&*self).display()
                )
            })?;
        }

        Ok(())
    }
}

impl<T: tempentry::TempEntryType> Deref for TempEntry<T> {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<T: tempentry::TempEntryType> Drop for TempEntry<T> {
    fn drop(&mut self) {
        T::drop_for(&*self)
    }
}

pub struct DownloadPath(PathBuf);

impl DownloadPath {
    pub fn to_pathbuf(&self) -> PathBuf {
        self.0.clone()
    }
}

impl RuntimePath {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn get_root(&self) -> RootPath {
        RootPath(self.0.clone())
    }

    pub fn get_libraries(&self) -> LibrariesPath {
        LibrariesPath(self.0.join("libraries"))
    }

    pub fn get_assets(&self) -> AssetsPath {
        AssetsPath(self.0.join("assets"))
    }

    pub fn get_natives(&self) -> NativesPath {
        NativesPath(self.0.join("natives"))
    }

    pub fn get_managed_javas(&self) -> ManagedJavasPath {
        ManagedJavasPath(self.0.join("managed_javas"))
    }

    pub fn get_instances(&self) -> InstancesPath {
        InstancesPath(self.0.join("instances"))
    }

    pub fn get_logging_configs(&self) -> LoggingConfigsPath {
        LoggingConfigsPath(self.0.join("logging_configs"))
    }

    pub fn get_temp(&self) -> TempPath {
        TempPath(self.0.join("temp"))
    }

    pub fn get_download(&self) -> DownloadPath {
        DownloadPath(self.0.join("download"))
    }
}

impl Deref for RuntimePath {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Recursivley copy from `from` to `to` except when excluded by `filter`.
/// Overwrites existing files. May fail if a parent directory is filtered but children are not.
pub async fn copy_dir_filter<F>(from: &Path, to: &Path, filter: F) -> anyhow::Result<()>
where
    F: for<'a> Fn(&'a Path) -> bool,
{
    let entries = walkdir::WalkDir::new(from).into_iter().filter_map(|entry| {
        let Ok(entry) = entry else { return None };

        let srcpath = entry.path().to_path_buf();
        let relpath = srcpath.strip_prefix(from).unwrap();

        if !filter(&relpath) {
            return None;
        }

        let destpath = to.join(relpath);

        Some(async move {
            if entry.metadata()?.is_dir() {
                tokio::fs::create_dir_all(destpath).await?;
            } else {
                tokio::fs::create_dir_all(destpath.parent().unwrap()).await?;
                tokio::fs::copy(srcpath, destpath).await?;
            }

            Ok::<_, anyhow::Error>(())
        })
    });

    futures::future::join_all(entries)
        .await
        .into_iter()
        .collect::<Result<_, _>>()?;

    Ok(())
}
