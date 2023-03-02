use std::{ops::Deref, path::PathBuf};

use carbon_domain::maven::MavenCoordinates;

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
    pub fn get_library_path(&self, maven_coordinate: MavenCoordinates) -> PathBuf {
        self.0.join(maven_coordinate.into_path())
    }
}

pub struct AssetsPath(PathBuf);

impl AssetsPath {
    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
    }

    pub fn get_legacy_path(&self) -> PathBuf {
        self.0.join("virtual").join("legacy")
    }

    pub fn get_asset_path(&self, asset_hash: &str) -> PathBuf {
        self.0
            .join("objects")
            .join(&asset_hash[..2])
            .join(asset_hash)
    }
}

pub struct VersionsPath(PathBuf);

impl VersionsPath {
    pub fn get_client_version_path(&self, version: &str) -> PathBuf {
        self.0.join("client").join(version).with_extension("jar")
    }

    pub fn get_server_version_path(&self, version: &str) -> PathBuf {
        self.0.join("server").join(version).with_extension("jar")
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

// TODO: WIP
pub struct InstancesPath(PathBuf);

impl InstancesPath {
    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
    }

    pub fn get_instance_path(&self, instance_id: String) -> InstancePath {
        InstancePath(self.0.join(instance_id))
    }
}

pub struct InstancePath(PathBuf);

impl InstancePath {
    pub fn get_root(&self) -> PathBuf {
        self.0.clone()
    }

    pub fn get_mods_path(&self) -> PathBuf {
        self.0.join("mods")
    }

    pub fn get_config_path(&self) -> PathBuf {
        self.0.join("config")
    }

    pub fn get_resourcepacks_path(&self) -> PathBuf {
        self.0.join("resourcepacks")
    }

    pub fn get_texturepacks_path(&self) -> PathBuf {
        self.0.join("texturepacks")
    }

    pub fn get_shaderpacks_path(&self) -> PathBuf {
        self.0.join("shaderpacks")
    }

    pub fn get_saves_path(&self) -> PathBuf {
        self.0.join("saves")
    }

    pub fn get_logs_path(&self) -> PathBuf {
        self.0.join("logs")
    }

    pub fn get_crash_reports_path(&self) -> PathBuf {
        self.0.join("crash-reports")
    }

    pub fn get_screenshots_path(&self) -> PathBuf {
        self.0.join("screenshots")
    }

    pub fn get_options_file_path(&self) -> PathBuf {
        self.0.join("options.txt")
    }
}

// TODO: WIP
pub struct TempPath(PathBuf);

impl TempPath {
    pub fn to_path(&self) -> PathBuf {
        self.0.clone()
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

    pub fn get_versions(&self) -> VersionsPath {
        VersionsPath(self.0.join("versions"))
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
