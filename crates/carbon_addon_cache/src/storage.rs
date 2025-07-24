use crate::events::*;
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;

#[async_trait]
pub trait AddonStorage: Send + Sync {
    // Metadata operations
    async fn store_metadata(&self, addon_id: &str, metadata: &LocalMetadata) -> Result<()>;
    async fn get_metadata(&self, addon_id: &str) -> Result<Option<LocalMetadata>>;
    async fn list_metadata(&self) -> Result<Vec<String>>;

    // Checksum operations
    async fn store_checksums(&self, addon_id: &str, checksums: &Checksums) -> Result<()>;
    async fn get_checksums(&self, addon_id: &str) -> Result<Option<Checksums>>;
    async fn find_by_checksum(
        &self,
        checksum_type: ChecksumType,
        checksum: &str,
    ) -> Result<Option<String>>;

    // Image operations
    async fn store_image(&self, addon_id: &str, image_type: ImageType, data: &[u8]) -> Result<()>;
    async fn get_image(&self, addon_id: &str, image_type: ImageType) -> Result<Option<Vec<u8>>>;
    async fn list_images(&self, addon_id: &str) -> Result<Vec<ImageType>>;

    // Platform data operations
    async fn store_platform_data(&self, addon_id: &str, data: &ModplatformData) -> Result<()>;
    async fn get_platform_data(&self, addon_id: &str) -> Result<Option<ModplatformData>>;

    // Version operations
    async fn store_versions(&self, addon_id: &str, versions: &[Version]) -> Result<()>;
    async fn get_versions(&self, addon_id: &str) -> Result<Vec<Version>>;
    async fn get_latest_version(&self, addon_id: &str) -> Result<Option<Version>>;

    // Hard link tracking
    async fn store_hard_link_status(&self, addon_id: &str, status: HardLinkStatus) -> Result<()>;
    async fn get_hard_link_status(&self, addon_id: &str) -> Result<Option<HardLinkStatus>>;
    async fn list_orphaned_files(&self) -> Result<Vec<String>>;

    // Cache status operations
    async fn store_cache_status(&self, addon_id: &str, status: CacheStatus) -> Result<()>;
    async fn get_cache_status(&self, addon_id: &str) -> Result<Option<CacheStatus>>;
    async fn list_by_status(&self, status: CacheStatus) -> Result<Vec<String>>;

    // Instance tracking
    async fn link_addon_to_instance(
        &self,
        addon_id: &str,
        instance_id: &str,
        file_path: &PathBuf,
    ) -> Result<()>;
    async fn unlink_addon_from_instance(&self, addon_id: &str, instance_id: &str) -> Result<()>;
    async fn get_instance_addons(&self, instance_id: &str) -> Result<Vec<String>>;
    async fn get_addon_instances(&self, addon_id: &str) -> Result<Vec<String>>;

    // Cleanup operations
    async fn cleanup_orphaned_data(&self) -> Result<()>;
    async fn vacuum_database(&self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct HardLinkStatus {
    pub blake3_hash: String,
    pub central_path: String,
    pub instance_paths: String, // JSON string
    pub link_valid: bool,
    pub created_at: u64,
    pub last_verified: u64,
}

#[derive(Debug, Clone)]
pub struct CacheStatus {
    pub stage: CacheStage,
    pub completed: bool,
    pub error: Option<String>,
    pub last_updated: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumType {
    Blake3,
    Sha256,
    Md5,
    Murmur2,
}

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub runtime_path: PathBuf,
    pub max_image_size: u64,
    pub max_cache_size: u64,
    pub cleanup_interval: u64,
}
