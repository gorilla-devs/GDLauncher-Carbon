use crate::storage::{HardLinkStatus, StorageConfig};
use anyhow::Result;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

pub struct HardLinkManager {
    config: StorageConfig,
    link_status_cache: Arc<RwLock<HashMap<String, HardLinkStatus>>>,
    verification_handle: Option<JoinHandle<()>>,
}

impl HardLinkManager {
    pub fn new(config: StorageConfig) -> Self {
        Self {
            config,
            link_status_cache: Arc::new(RwLock::new(HashMap::new())),
            verification_handle: None,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting hard link manager");

        // Create centralized storage directory
        let addon_dir = self.config.runtime_path.join("addons");
        fs::create_dir_all(&addon_dir)?;

        // Start verification task
        let verification_handle = self.start_verification_task().await?;
        self.verification_handle = Some(verification_handle);

        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down hard link manager");

        if let Some(handle) = self.verification_handle.take() {
            handle.abort();
            let _ = handle.await;
        }

        Ok(())
    }

    /// Create a hard link from instance to centralized storage
    pub async fn create_hard_link(
        &self,
        addon_id: &str,
        blake3_hash: &str,
        instance_path: &Path,
    ) -> Result<PathBuf> {
        let central_path = self
            .config
            .runtime_path
            .join("addons")
            .join(format!("{}.jar", blake3_hash));

        // Create parent directory
        if let Some(parent) = central_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Skip if central file already exists with correct hash
        if central_path.exists() {
            if let Ok(existing_hash) = self.calculate_blake3_hash(&central_path).await {
                if existing_hash == blake3_hash {
                    debug!(
                        "Central file already exists with correct hash: {:?}",
                        central_path
                    );
                    self.update_link_status(
                        addon_id,
                        &central_path,
                        &[instance_path.to_path_buf()],
                        true,
                    )
                    .await?;
                    return Ok(central_path);
                }
            }
        }

        // Try to create hard link
        match fs::hard_link(instance_path, &central_path) {
            Ok(_) => {
                info!(
                    "Created hard link: {:?} -> {:?}",
                    instance_path, central_path
                );
                self.update_link_status(
                    addon_id,
                    &central_path,
                    &[instance_path.to_path_buf()],
                    true,
                )
                .await?;
                Ok(central_path)
            }
            Err(e) => {
                warn!("Failed to create hard link, falling back to copy: {}", e);

                // Fall back to copying the file
                fs::copy(instance_path, &central_path)?;
                info!("Copied file: {:?} -> {:?}", instance_path, central_path);
                self.update_link_status(
                    addon_id,
                    &central_path,
                    &[instance_path.to_path_buf()],
                    false,
                )
                .await?;
                Ok(central_path)
            }
        }
    }

    /// Verify that a hard link is still valid
    pub async fn verify_hard_link(&self, addon_id: &str, status: &HardLinkStatus) -> Result<bool> {
        let central_path = Path::new(&status.central_path);

        // Check if central file exists
        if !central_path.exists() {
            warn!("Central file missing: {:?}", central_path);
            return Ok(false);
        }

        // Verify Blake3 hash
        let current_hash = self.calculate_blake3_hash(central_path).await?;
        if current_hash != status.blake3_hash {
            warn!("Blake3 hash mismatch for: {:?}", central_path);
            return Ok(false);
        }

        // Check instance paths if it's a real hard link
        if status.link_valid {
            let instance_paths: Vec<PathBuf> =
                serde_json::from_str(&status.instance_paths).unwrap_or_default();

            for instance_path in &instance_paths {
                if !instance_path.exists() {
                    warn!("Instance file missing: {:?}", instance_path);
                    continue;
                }

                // Verify they point to the same file (same inode on Unix)
                if !self.are_hard_linked(central_path, instance_path)? {
                    warn!(
                        "Hard link broken: {:?} <-> {:?}",
                        central_path, instance_path
                    );
                    return Ok(false);
                }
            }
        }

        debug!("Hard link verified: {}", addon_id);
        Ok(true)
    }

    /// Add an instance path to an existing hard link
    pub async fn add_instance_link(&self, addon_id: &str, instance_path: &Path) -> Result<()> {
        let mut cache = self.link_status_cache.write();
        if let Some(status) = cache.get_mut(addon_id) {
            let mut instance_paths: Vec<PathBuf> =
                serde_json::from_str(&status.instance_paths).unwrap_or_default();

            if !instance_paths.contains(&instance_path.to_path_buf()) {
                let central_path = Path::new(&status.central_path);

                // Try to create hard link
                match fs::hard_link(central_path, instance_path) {
                    Ok(_) => {
                        info!(
                            "Created additional hard link: {:?} -> {:?}",
                            central_path, instance_path
                        );
                        instance_paths.push(instance_path.to_path_buf());
                        status.instance_paths = serde_json::to_string(&instance_paths)?;
                    }
                    Err(e) => {
                        warn!(
                            "Failed to create additional hard link, copying instead: {}",
                            e
                        );
                        fs::copy(central_path, instance_path)?;
                        instance_paths.push(instance_path.to_path_buf());
                        status.instance_paths = serde_json::to_string(&instance_paths)?;
                        status.link_valid = false;
                    }
                }
            }
        }

        Ok(())
    }

    /// Remove an instance path from a hard link
    pub async fn remove_instance_link(&self, addon_id: &str, instance_path: &Path) -> Result<()> {
        let mut cache = self.link_status_cache.write();
        if let Some(status) = cache.get_mut(addon_id) {
            let mut instance_paths: Vec<PathBuf> =
                serde_json::from_str(&status.instance_paths).unwrap_or_default();

            instance_paths.retain(|p| p != instance_path);
            status.instance_paths = serde_json::to_string(&instance_paths)?;

            // Remove the instance file if it exists
            if instance_path.exists() {
                if let Err(e) = fs::remove_file(instance_path) {
                    warn!("Failed to remove instance file {:?}: {}", instance_path, e);
                }
            }

            // If no more instance paths, we can remove the central file too
            if instance_paths.is_empty() {
                let central_path = Path::new(&status.central_path);
                if central_path.exists() {
                    if let Err(e) = fs::remove_file(central_path) {
                        warn!("Failed to remove central file {:?}: {}", central_path, e);
                    }
                }
                cache.remove(addon_id);
            }
        }

        Ok(())
    }

    /// Get hard link status for an addon
    pub async fn get_link_status(&self, addon_id: &str) -> Option<HardLinkStatus> {
        self.link_status_cache.read().get(addon_id).cloned()
    }

    /// List all orphaned files (files in central storage with no instance links)
    pub async fn list_orphaned_files(&self) -> Result<Vec<String>> {
        let addon_dir = self.config.runtime_path.join("addons");
        let mut orphaned = Vec::new();

        if addon_dir.exists() {
            for entry in fs::read_dir(&addon_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    let blake3_hash = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    // Check if any addon references this hash
                    let cache = self.link_status_cache.read();
                    let has_reference = cache
                        .values()
                        .any(|status| status.blake3_hash == blake3_hash);

                    if !has_reference {
                        orphaned.push(blake3_hash);
                    }
                }
            }
        }

        Ok(orphaned)
    }

    /// Clean up orphaned files
    pub async fn cleanup_orphaned_files(&self) -> Result<()> {
        let orphaned = self.list_orphaned_files().await?;
        let addon_dir = self.config.runtime_path.join("addons");

        for blake3_hash in orphaned {
            let dir_path = addon_dir.join(&blake3_hash);
            if dir_path.exists() {
                if let Err(e) = fs::remove_dir_all(&dir_path) {
                    warn!("Failed to remove orphaned directory {:?}: {}", dir_path, e);
                } else {
                    info!("Cleaned up orphaned directory: {:?}", dir_path);
                }
            }
        }

        Ok(())
    }

    async fn start_verification_task(&self) -> Result<JoinHandle<()>> {
        let link_status_cache = self.link_status_cache.clone();
        let config = self.config.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // Every hour

            loop {
                interval.tick().await;

                debug!("Starting hard link verification");

                let addon_ids: Vec<String> = {
                    let cache = link_status_cache.read();
                    cache.keys().cloned().collect()
                };

                for addon_id in addon_ids {
                    let status = {
                        let cache = link_status_cache.read();
                        cache.get(&addon_id).cloned()
                    };

                    if let Some(status) = status {
                        let manager = HardLinkManager::new(config.clone());
                        match manager.verify_hard_link(&addon_id, &status).await {
                            Ok(is_valid) => {
                                if !is_valid {
                                    warn!("Hard link verification failed for addon: {}", addon_id);
                                    // Mark as invalid in cache
                                    let mut cache = link_status_cache.write();
                                    if let Some(cached_status) = cache.get_mut(&addon_id) {
                                        cached_status.link_valid = false;
                                        cached_status.last_verified = SystemTime::now()
                                            .duration_since(UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_secs();
                                    }
                                } else {
                                    // Update last verified time
                                    let mut cache = link_status_cache.write();
                                    if let Some(cached_status) = cache.get_mut(&addon_id) {
                                        cached_status.last_verified = SystemTime::now()
                                            .duration_since(UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_secs();
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error verifying hard link for addon {}: {}", addon_id, e);
                            }
                        }
                    }
                }

                debug!("Hard link verification complete");
            }
        });

        Ok(handle)
    }

    async fn update_link_status(
        &self,
        addon_id: &str,
        central_path: &Path,
        instance_paths: &[PathBuf],
        link_valid: bool,
    ) -> Result<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        let status = HardLinkStatus {
            blake3_hash: self.calculate_blake3_hash(central_path).await?,
            central_path: central_path.to_string_lossy().to_string(),
            instance_paths: serde_json::to_string(instance_paths)?,
            link_valid,
            created_at: now,
            last_verified: now,
        };

        self.link_status_cache
            .write()
            .insert(addon_id.to_string(), status);
        Ok(())
    }

    async fn calculate_blake3_hash(&self, file_path: &Path) -> Result<String> {
        let contents = fs::read(file_path)?;
        let hash = blake3::hash(&contents);
        Ok(hash.to_hex().to_string())
    }

    fn are_hard_linked(&self, path1: &Path, path2: &Path) -> Result<bool> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            let metadata1 = fs::metadata(path1)?;
            let metadata2 = fs::metadata(path2)?;

            // Same inode and device means they're hard linked
            Ok(metadata1.ino() == metadata2.ino() && metadata1.dev() == metadata2.dev())
        }

        #[cfg(not(unix))]
        {
            // On non-Unix systems, we can't easily check for hard links
            // So we'll just compare file sizes and modification times as a heuristic
            let metadata1 = fs::metadata(path1)?;
            let metadata2 = fs::metadata(path2)?;

            Ok(metadata1.len() == metadata2.len()
                && metadata1.modified()? == metadata2.modified()?)
        }
    }
}

impl Drop for HardLinkManager {
    fn drop(&mut self) {
        if let Some(handle) = self.verification_handle.take() {
            handle.abort();
        }
    }
}
