use crate::coordinator::CacheCoordinator;
use crate::events::*;
use crate::storage::*;
use crate::ui_integration::UIIntegrationManager;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

/// Main cache system interface for GDLauncher Carbon
/// This provides a unified interface to the addon caching system
pub struct CacheManager {
    coordinator: Arc<RwLock<Option<CacheCoordinator>>>,
    ui_manager: Arc<RwLock<Option<UIIntegrationManager>>>,
    storage: Arc<dyn AddonStorage>,
    config: StorageConfig,
}

impl CacheManager {
    pub fn new(storage: Arc<dyn AddonStorage>, config: StorageConfig) -> Self {
        Self {
            coordinator: Arc::new(RwLock::new(None)),
            ui_manager: Arc::new(RwLock::new(None)),
            storage,
            config,
        }
    }

    /// Initialize the caching system
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing cache manager");

        // Create coordinator
        let mut coordinator = CacheCoordinator::new(self.storage.clone(), self.config.clone())?;
        coordinator.start().await?;

        // Store the coordinator
        *self.coordinator.write().await = Some(coordinator);

        // Only start UI manager in non-test environment
        if !cfg!(test) {
            let mut ui_manager = UIIntegrationManager::new();
            ui_manager.start().await?;
            *self.ui_manager.write().await = Some(ui_manager);
        }

        info!("Cache manager initialized successfully");
        Ok(())
    }

    /// Shutdown the caching system
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down cache manager");

        let shutdown_timeout = std::time::Duration::from_secs(10);

        if let Some(mut coordinator) = self.coordinator.write().await.take() {
            let result = tokio::time::timeout(shutdown_timeout, coordinator.shutdown()).await;
            match result {
                Ok(Ok(())) => {
                    debug!("Coordinator shutdown successfully");
                }
                Ok(Err(e)) => {
                    error!("Coordinator shutdown failed: {}", e);
                }
                Err(_) => {
                    error!("Coordinator shutdown timed out");
                }
            }
        }

        if let Some(mut ui_manager) = self.ui_manager.write().await.take() {
            let result = tokio::time::timeout(shutdown_timeout, ui_manager.shutdown()).await;
            match result {
                Ok(Ok(())) => {
                    debug!("UI manager shutdown successfully");
                }
                Ok(Err(e)) => {
                    error!("UI manager shutdown failed: {}", e);
                }
                Err(_) => {
                    error!("UI manager shutdown timed out");
                }
            }
        }

        info!("Cache manager shutdown complete");
        Ok(())
    }

    /// Add addon for caching
    pub async fn cache_addon(&self, file_path: PathBuf, instance_id: Option<String>) -> Result<()> {
        if let Some(coordinator) = self.coordinator.read().await.as_ref() {
            debug!("Caching addon: {:?}", file_path);
            let event = CacheEvent::AddAddon {
                path: file_path,
                instance_id,
            };
            coordinator.send_event(event)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Cache manager not initialized"))
        }
    }

    /// Prioritize instance caching
    pub async fn prioritize_instance(&self, instance_id: String, priority: Priority) -> Result<()> {
        if let Some(coordinator) = self.coordinator.read().await.as_ref() {
            debug!(
                "Prioritizing instance '{}' with priority {:?}",
                instance_id, priority
            );
            coordinator.prioritize_instance(instance_id, priority);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Cache manager not initialized"))
        }
    }

    /// Get addon metadata
    pub async fn get_addon_metadata(&self, addon_id: &str) -> Result<Option<LocalMetadata>> {
        self.storage.get_metadata(addon_id).await
    }

    /// Get addon checksums
    pub async fn get_addon_checksums(&self, addon_id: &str) -> Result<Option<Checksums>> {
        self.storage.get_checksums(addon_id).await
    }

    /// Find addon by checksum
    pub async fn find_by_checksum(
        &self,
        checksum_type: ChecksumType,
        checksum: &str,
    ) -> Result<Option<String>> {
        self.storage.find_by_checksum(checksum_type, checksum).await
    }

    /// Get available versions for addon
    pub async fn get_addon_versions(&self, addon_id: &str) -> Result<Vec<Version>> {
        self.storage.get_versions(addon_id).await
    }

    /// Check if an addon has updates available
    pub async fn has_addon_updates(&self, addon_id: &str) -> Result<bool> {
        // Get current metadata
        let current_metadata = match self.storage.get_metadata(addon_id).await? {
            Some(metadata) => metadata,
            None => return Ok(false), // No metadata means no updates can be checked
        };

        // Get available versions
        let versions = self.storage.get_versions(addon_id).await?;

        // Find latest release version
        if let Some(latest_version) = versions
            .iter()
            .filter(|v| v.version_type == VersionType::Release)
            .max_by(|a, b| a.version_number.cmp(&b.version_number))
        {
            // Simple version comparison - in production you'd want proper semver comparison
            Ok(latest_version.version_number != current_metadata.version)
        } else {
            Ok(false)
        }
    }

    /// Get the latest available version for an addon
    pub async fn get_latest_addon_version(&self, addon_id: &str) -> Result<Option<Version>> {
        let versions = self.storage.get_versions(addon_id).await?;

        Ok(versions
            .iter()
            .filter(|v| v.version_type == VersionType::Release)
            .max_by(|a, b| a.version_number.cmp(&b.version_number))
            .cloned())
    }

    /// Get all addons with available updates for an instance
    pub async fn get_instance_addon_updates(
        &self,
        instance_id: &str,
    ) -> Result<Vec<(String, Version)>> {
        let mut updates = Vec::new();

        let addon_ids = self.storage.get_instance_addons(instance_id).await?;

        for addon_id in addon_ids {
            if self.has_addon_updates(&addon_id).await? {
                if let Some(latest_version) = self.get_latest_addon_version(&addon_id).await? {
                    updates.push((addon_id, latest_version));
                }
            }
        }

        Ok(updates)
    }

    /// Get instance addons
    pub async fn get_instance_addons(&self, instance_id: &str) -> Result<Vec<String>> {
        self.storage.get_instance_addons(instance_id).await
    }

    /// Set online/offline status
    pub async fn set_online_status(&self, online: bool) -> Result<()> {
        if let Some(coordinator) = self.coordinator.read().await.as_ref() {
            coordinator.set_online_status(online);
        }
        Ok(())
    }

    /// Get UI event subscription for progress tracking
    pub async fn get_ui_events(
        &self,
    ) -> Option<tokio::sync::broadcast::Receiver<crate::ui_integration::UIEvent>> {
        if let Some(ui_manager) = self.ui_manager.read().await.as_ref() {
            Some(ui_manager.subscribe_to_ui_events())
        } else {
            None
        }
    }

    /// Get statistics about cached addons
    pub async fn get_cache_stats(&self) -> Result<CacheStats> {
        // This could be extended to provide detailed statistics
        Ok(CacheStats {
            total_addons: 0,
            cache_size: 0,
            instances: 0,
        })
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_addons: usize,
    pub cache_size: u64,
    pub instances: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::mock_storage::MockAddonStorage;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_cache_manager_initialization() {
        let storage = Arc::new(MockAddonStorage::new());
        let temp_dir = TempDir::new().unwrap();
        let config = StorageConfig {
            runtime_path: temp_dir.path().to_path_buf(),
            max_image_size: 1024 * 1024,
            max_cache_size: 100 * 1024 * 1024,
            cleanup_interval: 3600,
        };

        let mut cache_manager = CacheManager::new(storage, config);

        // Test initialization
        tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            cache_manager.initialize(),
        )
        .await
        .unwrap()
        .unwrap();

        // Test shutdown
        tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            cache_manager.shutdown(),
        )
        .await
        .unwrap()
        .unwrap();
    }

    #[tokio::test]
    async fn test_cache_manager_addon_caching() {
        let storage = Arc::new(MockAddonStorage::new());
        let temp_dir = TempDir::new().unwrap();
        let config = StorageConfig {
            runtime_path: temp_dir.path().to_path_buf(),
            max_image_size: 1024 * 1024,
            max_cache_size: 100 * 1024 * 1024,
            cleanup_interval: 3600,
        };

        let mut cache_manager = CacheManager::new(storage, config);
        tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            cache_manager.initialize(),
        )
        .await
        .unwrap()
        .unwrap();

        // Test addon caching
        let addon_path = temp_dir.path().join("test_addon.jar");
        std::fs::write(&addon_path, "test content").unwrap();

        tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            cache_manager.cache_addon(addon_path, Some("test_instance".to_string())),
        )
        .await
        .unwrap()
        .unwrap();

        tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            cache_manager.shutdown(),
        )
        .await
        .unwrap()
        .unwrap();
    }
}
