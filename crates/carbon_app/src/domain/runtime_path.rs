use std::{
    mem::ManuallyDrop,
    ops::Deref,
    path::{Path, PathBuf},
};

use anyhow::anyhow;

#[derive(Clone)]
pub struct RuntimePath(PathBuf);

pub struct RootPath(PathBuf);

impl RootPath {
    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
    }
}

pub struct LibrariesPath(PathBuf);

// TODO: Ideally maven_coordinate should be its own type that we can sanitise
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

    pub fn get_legacy_path(&self) -> PathBuf {
        self.0.join("virtual").join("legacy")
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
        Self(PathBuf::new())
    }

    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
    }

    pub fn get_instance_path(&self, instance_id: &str) -> InstancePath {
        InstancePath(self.0.join(instance_id))
    }
}

#[derive(Clone)]
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

    pub async fn maketmp(&self) -> anyhow::Result<Tempfolder> {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time is somehow pre-epoch")
            .as_millis();

        let mut path = self.to_path();

        for i in 0..1000 {
            if i == 0 {
                path.push(time.to_string());
            } else {
                path.push(format!("{time}{i}"));
            }

            if tokio::fs::create_dir_all(&path).await.is_ok() {
                return Ok(Tempfolder(path));
            }

            path.pop();
        }

        Err(anyhow!("Could not create tmpdir"))
    }
}

pub struct Tempfolder(PathBuf);

impl Tempfolder {
    /// Extract the contained path without deleting the tmpdir.
    pub fn into_path(self) -> PathBuf {
        let v = ManuallyDrop::new(self);

        // SAFETY: v is not dropped so v.0 can be extracted safely.
        let path = unsafe { std::ptr::read(&v.0 as *const PathBuf) };

        path
    }

    pub async fn rename(self, path: impl AsRef<Path>) -> std::io::Result<()> {
        tokio::fs::rename(self.into_path(), path).await
    }
}

impl Deref for Tempfolder {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl Drop for Tempfolder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
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
