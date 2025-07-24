use crate::events::*;
use crate::storage::*;
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

pub struct PrismaAddonStorage {
    db_client: Arc<dyn DatabaseClient + Send + Sync>,
}

// Abstract database client to avoid tight coupling with Prisma
#[async_trait]
pub trait DatabaseClient {
    async fn create_addon_metadata(&self, metadata: &LocalMetadata) -> Result<()>;
    async fn get_addon_metadata(&self, addon_id: &str) -> Result<Option<LocalMetadata>>;
    async fn list_addon_metadata(&self) -> Result<Vec<String>>;

    async fn create_addon_checksums(&self, addon_id: &str, checksums: &Checksums) -> Result<()>;
    async fn get_addon_checksums(&self, addon_id: &str) -> Result<Option<Checksums>>;
    async fn find_addon_by_checksum(
        &self,
        checksum_type: ChecksumType,
        checksum: &str,
    ) -> Result<Option<String>>;

    async fn create_addon_image(
        &self,
        addon_id: &str,
        image_type: ImageType,
        data: &[u8],
    ) -> Result<()>;
    async fn get_addon_image(
        &self,
        addon_id: &str,
        image_type: ImageType,
    ) -> Result<Option<Vec<u8>>>;
    async fn list_addon_images(&self, addon_id: &str) -> Result<Vec<ImageType>>;

    async fn create_addon_platform_data(
        &self,
        addon_id: &str,
        data: &ModplatformData,
    ) -> Result<()>;
    async fn get_addon_platform_data(&self, addon_id: &str) -> Result<Option<ModplatformData>>;

    async fn create_addon_versions(&self, addon_id: &str, versions: &[Version]) -> Result<()>;
    async fn get_addon_versions(&self, addon_id: &str) -> Result<Vec<Version>>;
    async fn get_latest_addon_version(&self, addon_id: &str) -> Result<Option<Version>>;

    async fn create_addon_hard_link(&self, addon_id: &str, status: HardLinkStatus) -> Result<()>;
    async fn get_addon_hard_link(&self, addon_id: &str) -> Result<Option<HardLinkStatus>>;
    async fn list_orphaned_hard_links(&self) -> Result<Vec<String>>;

    async fn create_addon_cache_status(&self, addon_id: &str, status: CacheStatus) -> Result<()>;
    async fn get_addon_cache_status(&self, addon_id: &str) -> Result<Option<CacheStatus>>;
    async fn list_addons_by_cache_status(&self, status: CacheStatus) -> Result<Vec<String>>;

    async fn create_addon_instance_link(
        &self,
        addon_id: &str,
        instance_id: &str,
        file_path: &PathBuf,
    ) -> Result<()>;
    async fn delete_addon_instance_link(&self, addon_id: &str, instance_id: &str) -> Result<()>;
    async fn get_instance_addons(&self, instance_id: &str) -> Result<Vec<String>>;
    async fn get_addon_instances(&self, addon_id: &str) -> Result<Vec<String>>;

    async fn cleanup_orphaned_data(&self) -> Result<()>;
    async fn vacuum_database(&self) -> Result<()>;
}

impl PrismaAddonStorage {
    pub fn new(db_client: Arc<dyn DatabaseClient + Send + Sync>) -> Self {
        Self { db_client }
    }
}

#[async_trait]
impl AddonStorage for PrismaAddonStorage {
    async fn store_metadata(&self, addon_id: &str, metadata: &LocalMetadata) -> Result<()> {
        self.db_client.create_addon_metadata(metadata).await
    }

    async fn get_metadata(&self, addon_id: &str) -> Result<Option<LocalMetadata>> {
        self.db_client.get_addon_metadata(addon_id).await
    }

    async fn list_metadata(&self) -> Result<Vec<String>> {
        self.db_client.list_addon_metadata().await
    }

    async fn store_checksums(&self, addon_id: &str, checksums: &Checksums) -> Result<()> {
        self.db_client
            .create_addon_checksums(addon_id, checksums)
            .await
    }

    async fn get_checksums(&self, addon_id: &str) -> Result<Option<Checksums>> {
        self.db_client.get_addon_checksums(addon_id).await
    }

    async fn find_by_checksum(
        &self,
        checksum_type: ChecksumType,
        checksum: &str,
    ) -> Result<Option<String>> {
        self.db_client
            .find_addon_by_checksum(checksum_type, checksum)
            .await
    }

    async fn store_image(&self, addon_id: &str, image_type: ImageType, data: &[u8]) -> Result<()> {
        self.db_client
            .create_addon_image(addon_id, image_type, data)
            .await
    }

    async fn get_image(&self, addon_id: &str, image_type: ImageType) -> Result<Option<Vec<u8>>> {
        self.db_client.get_addon_image(addon_id, image_type).await
    }

    async fn list_images(&self, addon_id: &str) -> Result<Vec<ImageType>> {
        self.db_client.list_addon_images(addon_id).await
    }

    async fn store_platform_data(&self, addon_id: &str, data: &ModplatformData) -> Result<()> {
        self.db_client
            .create_addon_platform_data(addon_id, data)
            .await
    }

    async fn get_platform_data(&self, addon_id: &str) -> Result<Option<ModplatformData>> {
        self.db_client.get_addon_platform_data(addon_id).await
    }

    async fn store_versions(&self, addon_id: &str, versions: &[Version]) -> Result<()> {
        self.db_client
            .create_addon_versions(addon_id, versions)
            .await
    }

    async fn get_versions(&self, addon_id: &str) -> Result<Vec<Version>> {
        self.db_client.get_addon_versions(addon_id).await
    }

    async fn get_latest_version(&self, addon_id: &str) -> Result<Option<Version>> {
        self.db_client.get_latest_addon_version(addon_id).await
    }

    async fn store_hard_link_status(&self, addon_id: &str, status: HardLinkStatus) -> Result<()> {
        self.db_client
            .create_addon_hard_link(addon_id, status)
            .await
    }

    async fn get_hard_link_status(&self, addon_id: &str) -> Result<Option<HardLinkStatus>> {
        self.db_client.get_addon_hard_link(addon_id).await
    }

    async fn list_orphaned_files(&self) -> Result<Vec<String>> {
        self.db_client.list_orphaned_hard_links().await
    }

    async fn store_cache_status(&self, addon_id: &str, status: CacheStatus) -> Result<()> {
        self.db_client
            .create_addon_cache_status(addon_id, status)
            .await
    }

    async fn get_cache_status(&self, addon_id: &str) -> Result<Option<CacheStatus>> {
        self.db_client.get_addon_cache_status(addon_id).await
    }

    async fn list_by_status(&self, status: CacheStatus) -> Result<Vec<String>> {
        self.db_client.list_addons_by_cache_status(status).await
    }

    async fn link_addon_to_instance(
        &self,
        addon_id: &str,
        instance_id: &str,
        file_path: &PathBuf,
    ) -> Result<()> {
        self.db_client
            .create_addon_instance_link(addon_id, instance_id, file_path)
            .await
    }

    async fn unlink_addon_from_instance(&self, addon_id: &str, instance_id: &str) -> Result<()> {
        self.db_client
            .delete_addon_instance_link(addon_id, instance_id)
            .await
    }

    async fn get_instance_addons(&self, instance_id: &str) -> Result<Vec<String>> {
        self.db_client.get_instance_addons(instance_id).await
    }

    async fn get_addon_instances(&self, addon_id: &str) -> Result<Vec<String>> {
        self.db_client.get_addon_instances(addon_id).await
    }

    async fn cleanup_orphaned_data(&self) -> Result<()> {
        self.db_client.cleanup_orphaned_data().await
    }

    async fn vacuum_database(&self) -> Result<()> {
        self.db_client.vacuum_database().await
    }
}
