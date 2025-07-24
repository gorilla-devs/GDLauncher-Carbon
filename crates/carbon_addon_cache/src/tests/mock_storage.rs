use crate::events::*;
use crate::storage::*;
use anyhow::Result;
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct MockAddonStorage {
    metadata: Arc<RwLock<HashMap<String, LocalMetadata>>>,
    checksums: Arc<RwLock<HashMap<String, Checksums>>>,
    images: Arc<RwLock<HashMap<String, HashMap<ImageType, Vec<u8>>>>>,
    platform_data: Arc<RwLock<HashMap<String, ModplatformData>>>,
    versions: Arc<RwLock<HashMap<String, Vec<Version>>>>,
    hard_links: Arc<RwLock<HashMap<String, HardLinkStatus>>>,
    cache_status: Arc<RwLock<HashMap<String, CacheStatus>>>,
    instance_links: Arc<RwLock<HashMap<String, HashMap<String, PathBuf>>>>,
}

impl MockAddonStorage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&self) {
        self.metadata.write().clear();
        self.checksums.write().clear();
        self.images.write().clear();
        self.platform_data.write().clear();
        self.versions.write().clear();
        self.hard_links.write().clear();
        self.cache_status.write().clear();
        self.instance_links.write().clear();
    }

    pub fn get_stored_metadata_count(&self) -> usize {
        self.metadata.read().len()
    }

    pub fn get_stored_checksums_count(&self) -> usize {
        self.checksums.read().len()
    }

    pub fn has_image(&self, addon_id: &str, image_type: ImageType) -> bool {
        self.images
            .read()
            .get(addon_id)
            .map(|images| images.contains_key(&image_type))
            .unwrap_or(false)
    }
}

#[async_trait]
impl AddonStorage for MockAddonStorage {
    async fn store_metadata(&self, addon_id: &str, metadata: &LocalMetadata) -> Result<()> {
        self.metadata
            .write()
            .insert(addon_id.to_string(), metadata.clone());
        Ok(())
    }

    async fn get_metadata(&self, addon_id: &str) -> Result<Option<LocalMetadata>> {
        Ok(self.metadata.read().get(addon_id).cloned())
    }

    async fn list_metadata(&self) -> Result<Vec<String>> {
        Ok(self.metadata.read().keys().cloned().collect())
    }

    async fn store_checksums(&self, addon_id: &str, checksums: &Checksums) -> Result<()> {
        self.checksums
            .write()
            .insert(addon_id.to_string(), checksums.clone());
        Ok(())
    }

    async fn get_checksums(&self, addon_id: &str) -> Result<Option<Checksums>> {
        Ok(self.checksums.read().get(addon_id).cloned())
    }

    async fn find_by_checksum(
        &self,
        checksum_type: ChecksumType,
        checksum: &str,
    ) -> Result<Option<String>> {
        for (addon_id, checksums) in self.checksums.read().iter() {
            let matches = match checksum_type {
                ChecksumType::Blake3 => checksums.blake3 == checksum,
                ChecksumType::Sha256 => checksums.sha256 == checksum,
                ChecksumType::Md5 => checksums.md5 == checksum,
                ChecksumType::Murmur2 => checksums.murmur2.to_string() == checksum,
            };

            if matches {
                return Ok(Some(addon_id.clone()));
            }
        }
        Ok(None)
    }

    async fn store_image(&self, addon_id: &str, image_type: ImageType, data: &[u8]) -> Result<()> {
        self.images
            .write()
            .entry(addon_id.to_string())
            .or_insert_with(HashMap::new)
            .insert(image_type, data.to_vec());
        Ok(())
    }

    async fn get_image(&self, addon_id: &str, image_type: ImageType) -> Result<Option<Vec<u8>>> {
        Ok(self
            .images
            .read()
            .get(addon_id)
            .and_then(|images| images.get(&image_type))
            .map(|v| v.clone()))
    }

    async fn list_images(&self, addon_id: &str) -> Result<Vec<ImageType>> {
        Ok(self
            .images
            .read()
            .get(addon_id)
            .map(|images| images.keys().cloned().collect())
            .unwrap_or_default())
    }

    async fn store_platform_data(&self, addon_id: &str, data: &ModplatformData) -> Result<()> {
        self.platform_data
            .write()
            .insert(addon_id.to_string(), data.clone());
        Ok(())
    }

    async fn get_platform_data(&self, addon_id: &str) -> Result<Option<ModplatformData>> {
        Ok(self.platform_data.read().get(addon_id).cloned())
    }

    async fn store_versions(&self, addon_id: &str, versions: &[Version]) -> Result<()> {
        self.versions
            .write()
            .insert(addon_id.to_string(), versions.to_vec());
        Ok(())
    }

    async fn get_versions(&self, addon_id: &str) -> Result<Vec<Version>> {
        Ok(self
            .versions
            .read()
            .get(addon_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn get_latest_version(&self, addon_id: &str) -> Result<Option<Version>> {
        Ok(self
            .versions
            .read()
            .get(addon_id)
            .and_then(|versions| versions.first())
            .cloned())
    }

    async fn store_hard_link_status(&self, addon_id: &str, status: HardLinkStatus) -> Result<()> {
        self.hard_links.write().insert(addon_id.to_string(), status);
        Ok(())
    }

    async fn get_hard_link_status(&self, addon_id: &str) -> Result<Option<HardLinkStatus>> {
        Ok(self.hard_links.read().get(addon_id).cloned())
    }

    async fn list_orphaned_files(&self) -> Result<Vec<String>> {
        Ok(vec![]) // Mock implementation returns empty list
    }

    async fn store_cache_status(&self, addon_id: &str, status: CacheStatus) -> Result<()> {
        self.cache_status
            .write()
            .insert(addon_id.to_string(), status);
        Ok(())
    }

    async fn get_cache_status(&self, addon_id: &str) -> Result<Option<CacheStatus>> {
        Ok(self.cache_status.read().get(addon_id).cloned())
    }

    async fn list_by_status(&self, _status: CacheStatus) -> Result<Vec<String>> {
        Ok(vec![]) // Mock implementation
    }

    async fn link_addon_to_instance(
        &self,
        addon_id: &str,
        instance_id: &str,
        file_path: &PathBuf,
    ) -> Result<()> {
        self.instance_links
            .write()
            .entry(addon_id.to_string())
            .or_insert_with(HashMap::new)
            .insert(instance_id.to_string(), file_path.clone());
        Ok(())
    }

    async fn unlink_addon_from_instance(&self, addon_id: &str, instance_id: &str) -> Result<()> {
        if let Some(instances) = self.instance_links.write().get_mut(addon_id) {
            instances.remove(instance_id);
        }
        Ok(())
    }

    async fn get_instance_addons(&self, instance_id: &str) -> Result<Vec<String>> {
        let links = self.instance_links.read();
        let mut addons = Vec::new();

        for (addon_id, instances) in links.iter() {
            if instances.contains_key(instance_id) {
                addons.push(addon_id.clone());
            }
        }

        Ok(addons)
    }

    async fn get_addon_instances(&self, addon_id: &str) -> Result<Vec<String>> {
        Ok(self
            .instance_links
            .read()
            .get(addon_id)
            .map(|instances| instances.keys().cloned().collect())
            .unwrap_or_default())
    }

    async fn cleanup_orphaned_data(&self) -> Result<()> {
        Ok(()) // Mock implementation does nothing
    }

    async fn vacuum_database(&self) -> Result<()> {
        Ok(()) // Mock implementation does nothing
    }
}
