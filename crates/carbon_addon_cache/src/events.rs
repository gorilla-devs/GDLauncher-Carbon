use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheEvent {
    // Input events
    AddAddon {
        path: PathBuf,
        instance_id: Option<String>,
    },
    PrioritizeInstance {
        instance_id: String,
    },

    // Pipeline events
    FilesCached {
        addon_id: String,
        metadata: BasicMetadata,
    },
    MetadataExtracted {
        addon_id: String,
        metadata: LocalMetadata,
    },
    ImagesProcessed {
        addon_id: String,
        images: Vec<ImageInfo>,
    },
    ModplatformDataFetched {
        addon_id: String,
        data: ModplatformData,
    },
    UpdatesChecked {
        addon_id: String,
        updates: Vec<Version>,
    },

    // Control events
    GoOnline,
    GoOffline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicMetadata {
    pub addon_id: String,
    pub file_path: PathBuf,
    pub file_size: u64,
    pub modified_time: u64,
    pub addon_type: AddonType,
    pub instance_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalMetadata {
    pub addon_id: String,
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub dependencies: Vec<Dependency>,
    pub checksums: Checksums,
    pub mod_format: ModFormat,
    pub minecraft_versions: Vec<String>,
    pub mod_loaders: Vec<String>,
    pub instance_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checksums {
    pub blake3: String,
    pub sha256: String,
    pub md5: String,
    pub murmur2: u32,
}

impl Default for Checksums {
    fn default() -> Self {
        Self {
            blake3: String::new(),
            sha256: String::new(),
            md5: String::new(),
            murmur2: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub image_type: ImageType,
    pub url: Option<String>,
    pub data: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModplatformData {
    pub platform: Platform,
    pub project_id: String,
    pub file_id: String,
    pub download_url: Option<String>,
    pub project_name: String,
    pub project_description: Option<String>,
    pub categories: Vec<String>,
    pub license: Option<String>,
    pub website_url: Option<String>,
    pub source_url: Option<String>,
    pub issues_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    pub version_number: String,
    pub version_type: VersionType,
    pub minecraft_versions: Vec<String>,
    pub mod_loaders: Vec<String>,
    pub release_date: String,
    pub download_url: String,
    pub changelog: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub mod_id: String,
    pub version_requirement: String,
    pub dependency_type: DependencyType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AddonType {
    Mod,
    ResourcePack,
    DataPack,
    ShaderPack,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModFormat {
    Fabric,
    Forge,
    Quilt,
    NeoForge,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    CurseForge,
    Modrinth,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImageType {
    Icon,
    Gallery,
    Featured,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VersionType {
    Release,
    Beta,
    Alpha,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    Required,
    Optional,
    Incompatible,
    Embedded,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheStage {
    FileCache,
    MetadataExtraction,
    ImageCache,
    ModplatformData,
    Updates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstanceCacheStatus {
    Idle,
    Caching {
        stage: CacheStage,
        current: usize,
        total: usize,
    },
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}
