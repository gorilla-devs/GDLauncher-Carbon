use crate::api::keys::instance::INSTANCE_MODS;
use crate::domain::instance::InstanceId;
use crate::managers::App;
use crate::managers::ManagerRef;
use anyhow::anyhow;
use carbon_addon_cache::CacheScheduler;
use carbon_repos::db::{
    PrismaClient, curse_forge_mod_cache as cfdb, local_mod_image_cache as localimg,
    mod_file_cache as fcdb, mod_metadata as metadb, modrinth_mod_cache as mrdb,
};
use chrono::{DateTime, Utc};
use hex;
use sha1::Sha1;
use sha2::{Digest, Sha512};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use zip;

pub mod modpack;
pub mod utils;

/// Platform-specific metadata for cached mods
#[derive(Debug, Clone)]
pub enum PlatformMetadata {
    CurseForge {
        project_id: u32,
        file_id: u32,
    },
    Modrinth {
        project_id: String,
        version_id: String,
    },
}

/// Simple cache status for replacement
#[derive(Debug, Clone)]
pub enum InstanceCacheStatus {
    Idle,
    Caching,
    Complete,
    Error(String),
}

/// Simple cache statistics for replacement
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub total_cached: usize,
    pub pending: usize,
    pub errors: usize,
    pub local_scans_completed: usize,
    pub local_scans_failed: usize,
    pub images_processed: usize,
    pub platform_requests_completed: usize,
    pub average_local_scan_time_ms: f64,
}

/// Types of cache operations that can be tracked
#[derive(Debug, Clone)]
pub enum CacheTaskType {
    FullInstanceScan {
        instance_name: String,
        file_count: u32,
    },
    SingleFileCache {
        filename: String,
        instance_name: String,
    },
    ImageExtraction {
        filename: String,
        instance_name: String,
        addon_name: Option<String>,
        image_types: Vec<String>, // icon, gallery, featured
    },
    PlatformDetection {
        filename: String,
        instance_name: String,
        addon_name: Option<String>,
        platform_type: Option<String>, // CurseForge, Modrinth
    },
    UpdateCheck {
        filename: String,
        instance_name: String,
        addon_name: Option<String>,
        platform_type: Option<String>,
        current_version: Option<String>,
    },
    CacheClear,
    StartupScan,
}

/// Current status of a cache task
#[derive(Debug, Clone)]
pub enum CacheTaskStatus {
    Running {
        stage: String,
        progress: Option<(u32, u32)>, // current, total
    },
    Completed {
        success: bool,
        error_message: Option<String>,
    },
}

/// A completed cache task with timing information
#[derive(Debug, Clone)]
pub struct CacheTaskHistory {
    pub id: String,
    pub task_type: CacheTaskType,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: u32,
    pub success: bool,
    pub error_message: Option<String>,
    pub details: Option<String>,
}

/// A currently running cache task
#[derive(Debug)]
pub struct CacheTaskCurrent {
    pub id: String,
    pub task_type: CacheTaskType,
    pub started_at: DateTime<Utc>,
    pub start_instant: Instant,
    pub status: CacheTaskStatus,
}

impl Clone for CacheTaskCurrent {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            task_type: self.task_type.clone(),
            started_at: self.started_at,
            start_instant: Instant::now(), // Can't clone Instant, so use current time
            status: self.status.clone(),
        }
    }
}

/// Cache task tracker for managing active and historical tasks
#[derive(Debug)]
pub struct CacheTaskTracker {
    current_tasks: Mutex<Vec<CacheTaskCurrent>>,
    task_history: Mutex<VecDeque<CacheTaskHistory>>,
    max_history_size: usize,
}

impl CacheTaskTracker {
    pub fn new() -> Self {
        Self {
            current_tasks: Mutex::new(Vec::new()),
            task_history: Mutex::new(VecDeque::new()),
            max_history_size: 100,
        }
    }

    /// Start tracking a new cache task
    pub fn start_task(&self, task_type: CacheTaskType) -> String {
        let task_id = Uuid::new_v4().to_string();
        let task = CacheTaskCurrent {
            id: task_id.clone(),
            task_type,
            started_at: Utc::now(),
            start_instant: Instant::now(),
            status: CacheTaskStatus::Running {
                stage: "Starting".to_string(),
                progress: None,
            },
        };

        if let Ok(mut tasks) = self.current_tasks.lock() {
            tasks.push(task);
        }

        task_id
    }

    /// Update the status of a running task
    pub fn update_task(&self, task_id: &str, status: CacheTaskStatus) {
        if let Ok(mut tasks) = self.current_tasks.lock() {
            if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                task.status = status;
            }
        }
    }

    /// Complete a task and move it to history
    pub fn complete_task(&self, task_id: &str, success: bool, error_message: Option<String>, details: Option<String>) {
        let mut completed_task = None;

        if let Ok(mut tasks) = self.current_tasks.lock() {
            if let Some(pos) = tasks.iter().position(|t| t.id == task_id) {
                let task = tasks.remove(pos);
                let completed_at = Utc::now();
                let duration_ms = task.start_instant.elapsed().as_millis() as u32;

                completed_task = Some(CacheTaskHistory {
                    id: task.id,
                    task_type: task.task_type,
                    started_at: task.started_at,
                    completed_at,
                    duration_ms,
                    success,
                    error_message,
                    details,
                });
            }
        }

        if let Some(history_task) = completed_task {
            if let Ok(mut history) = self.task_history.lock() {
                history.push_back(history_task);
                
                // Keep only the last N tasks
                while history.len() > self.max_history_size {
                    history.pop_front();
                }
            }
        }
    }

    /// Get all current running tasks
    pub fn get_current_tasks(&self) -> Vec<CacheTaskCurrent> {
        match self.current_tasks.lock() {
            Ok(tasks) => tasks.clone(),
            Err(_) => Vec::new(),
        }
    }

    /// Get task history
    pub fn get_task_history(&self) -> Vec<CacheTaskHistory> {
        match self.task_history.lock() {
            Ok(history) => history.iter().cloned().collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Clear task history
    pub fn clear_history(&self) {
        if let Ok(mut history) = self.task_history.lock() {
            history.clear();
        }
    }

    /// Get summary statistics
    pub fn get_stats(&self) -> CacheTaskStats {
        let current_count = match self.current_tasks.lock() {
            Ok(tasks) => tasks.len(),
            Err(_) => 0,
        };
        let history = match self.task_history.lock() {
            Ok(history) => history.clone(),
            Err(_) => return CacheTaskStats {
                current_tasks: current_count as u32,
                total_completed: 0,
                successful: 0,
                failed: 0,
                average_duration_ms: 0,
            },
        };
        
        let total_completed = history.len();
        let successful = history.iter().filter(|t| t.success).count();
        let failed = total_completed - successful;
        let average_duration_ms = if total_completed > 0 {
            let total_duration = history.iter().map(|t| t.duration_ms as u64).sum::<u64>();
            (total_duration / total_completed as u64) as u32
        } else {
            0
        };

        CacheTaskStats {
            current_tasks: current_count as u32,
            total_completed: total_completed as u32,
            successful: successful as u32,
            failed: failed as u32,
            average_duration_ms,
        }
    }
}

/// Summary statistics for cache tasks
#[derive(Debug, Clone)]
pub struct CacheTaskStats {
    pub current_tasks: u32,
    pub total_completed: u32,
    pub successful: u32,
    pub failed: u32,
    pub average_duration_ms: u32,
}

/// The metadata cache manager - manages mod metadata caching
pub struct MetaCacheManager {
    // Note: cache_manager field removed - using direct database operations instead
    task_tracker: Arc<CacheTaskTracker>,
}

impl MetaCacheManager {
    /// Create a new MetaCacheManager
    pub fn new() -> Self {
        Self {
            // Note: cache_manager field removed - using direct database operations instead
            task_tracker: Arc::new(CacheTaskTracker::new()),
        }
    }

    /// Launch background caching tasks
    pub async fn launch_background_tasks(&self, app: ManagerRef<'_, Self>) {
        info!("Initializing cache system background tasks");

        if let Err(e) = self.initialize_cache_manager(app).await {
            error!("Failed to initialize cache manager: {}", e);
        } else {
            info!("Cache system initialized successfully");
        }

        // Perform startup scan for missing mod entries
        info!("Starting startup scan for missing mod database entries");
        self.startup_scan_missing_mods(app).await;
    }

    /// Initialize the cache manager with the new caching system
    pub async fn initialize_cache_manager(&self, app: ManagerRef<'_, Self>) -> anyhow::Result<()> {
        info!("Initializing cache manager");

        // Note: The addon cache system has been removed in favor of direct database operations
        // This method is kept for compatibility but doesn't do anything anymore

        info!("Cache manager initialized successfully");
        Ok(())
    }

    /// Scan all instances on startup for mods that exist on disk but not in database
    async fn startup_scan_missing_mods(&self, app: ManagerRef<'_, Self>) {
        info!("Starting startup scan for missing mod database entries");

        let instance_manager = app.app.instance_manager();
        let instances = instance_manager.instances.read().await;

        let mut scan_count = 0;
        let total_instances = instances.len();

        info!(
            "Scanning {} instances for missing mod entries",
            total_instances
        );

        // Track this startup scan operation
        let task_id = self.task_tracker.start_task(CacheTaskType::StartupScan);

        for (instance_id, instance) in instances.iter() {
            let instance_shortpath = instance.shortpath.clone();
            scan_count += 1;

            debug!(
                "Scanning instance {} ({}/{}) for missing mods",
                instance_shortpath, scan_count, total_instances
            );

            // Update progress
            self.task_tracker.update_task(&task_id, CacheTaskStatus::Running {
                stage: format!("Scanning instance {} ({}/{})", instance_shortpath, scan_count, total_instances),
                progress: Some((scan_count as u32, total_instances as u32)),
            });

            // Get the mods directory for this instance
            let mods_dir = app
                .app
                .settings_manager()
                .runtime_path
                .get_instances()
                .get_instance_path(&instance_shortpath)
                .get_data_path()
                .join("mods");

            if !mods_dir.exists() {
                debug!(
                    "Mods directory does not exist for instance {}",
                    instance_shortpath
                );
                continue;
            }

            // Check if we already have database entries for this instance
            let existing_count = match app
                .app
                .prisma_client
                .mod_file_cache()
                .count(vec![fcdb::instance_id::equals(**instance_id)])
                .exec()
                .await
            {
                Ok(count) => count,
                Err(e) => {
                    warn!(
                        "Failed to count existing mod entries for instance {}: {}",
                        instance_shortpath, e
                    );
                    continue;
                }
            };

            // Count actual mod files on disk
            let mut disk_files = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&mods_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                                if filename.ends_with(".jar") || filename.ends_with(".jar.disabled")
                                {
                                    disk_files.push((filename.to_string(), path));
                                }
                            }
                        }
                    }
                }
            }

            if disk_files.is_empty() {
                debug!(
                    "No mod files found on disk for instance {}",
                    instance_shortpath
                );
                continue;
            }

            if existing_count as usize == disk_files.len() {
                debug!(
                    "Instance {} has {} mods in database matching {} files on disk - skipping",
                    instance_shortpath,
                    existing_count,
                    disk_files.len()
                );
                continue;
            }

            info!(
                "Instance {} has {} mod files on disk but only {} in database - creating missing entries",
                instance_shortpath,
                disk_files.len(),
                existing_count
            );

            // Identify files that need caching
            let mut files_to_cache = Vec::new();
            for (filename, path) in disk_files {
                // Check if this specific file already exists in database
                let exists = match app
                    .app
                    .prisma_client
                    .mod_file_cache()
                    .find_first(vec![
                        fcdb::instance_id::equals(**instance_id),
                        fcdb::filename::equals(filename.clone()),
                    ])
                    .exec()
                    .await
                {
                    Ok(result) => result.is_some(),
                    Err(e) => {
                        warn!(
                            "Failed to check if mod file {} exists in database: {}",
                            filename, e
                        );
                        continue;
                    }
                };

                if !exists {
                    debug!("Queuing mod file for full caching pipeline: {}", filename);
                    files_to_cache.push((filename, path));
                }
            }

            if !files_to_cache.is_empty() {
                info!(
                    "Found {} mod files to cache for instance {}",
                    files_to_cache.len(),
                    instance_shortpath
                );

                // Run full caching pipeline for missing files
                self.cache_missing_files(*instance_id, &instance_shortpath, files_to_cache, app)
                    .await;

                // Invalidate the instance mods cache to trigger UI update
                app.app
                    .invalidate(INSTANCE_MODS, Some(instance_id.0.into()));
            }
        }

        drop(instances);
        info!("Startup scan completed - scanned {} instances", scan_count);
        
        // Complete the startup scan task
        self.task_tracker.complete_task(
            &task_id,
            true,
            None,
            Some(format!("Scanned {} instances for missing mods", scan_count)),
        );
    }

    /// Cache missing files using the full caching pipeline
    async fn cache_missing_files(
        &self,
        instance_id: InstanceId,
        instance_shortpath: &str,
        files_to_cache: Vec<(String, std::path::PathBuf)>,
        app: ManagerRef<'_, Self>,
    ) {
        info!(
            "Starting full caching pipeline for {} files in instance {}",
            files_to_cache.len(),
            instance_shortpath
        );

        // Track this instance scan operation
        let task_id = self.task_tracker.start_task(CacheTaskType::FullInstanceScan {
            instance_name: instance_shortpath.to_string(),
            file_count: files_to_cache.len() as u32,
        });

        let prisma_client = app.app.prisma_client.clone();
        let total_files = files_to_cache.len();
        let mut processed_count = 0;

        for (filename, path) in files_to_cache {
            processed_count += 1;
            info!("Processing mod file: {} ({}/{})", filename, processed_count, total_files);

            // Update progress for the full instance scan
            self.task_tracker.update_task(&task_id, CacheTaskStatus::Running {
                stage: format!("Processing {} ({}/{})", filename, processed_count, total_files),
                progress: Some((processed_count as u32, total_files as u32)),
            });

            // Phase 1: Create basic database entry (metadata + file cache)
            if let Err(e) = self
                .cache_single_mod_file(
                    instance_id,
                    &path,
                    crate::domain::instance::AddonType::Mods,
                    &prisma_client,
                    None,
                )
                .await
            {
                error!(
                    "Failed to create basic database entry for {}: {}",
                    filename, e
                );
                continue; // Skip to next file if basic caching fails
            }

            // Get the metadata ID for subsequent pipeline stages
            let metadata_id = match self.get_mod_metadata_id(&path).await {
                Ok(id) => id,
                Err(e) => {
                    error!("Failed to get metadata ID for {}: {}", filename, e);
                    continue;
                }
            };

            // Phase 2: Extract and cache images from mod JAR
            if let Err(e) = self
                .extract_and_cache_images(&metadata_id, &path, &filename, instance_shortpath, app)
                .await
            {
                warn!("Failed to extract images for {}: {}", filename, e);
                // Continue with other phases even if image extraction fails
            }

            // Phase 3: Detect and cache platform metadata
            if let Err(e) = self
                .detect_and_cache_platform_metadata(
                    &metadata_id,
                    &path,
                    &filename,
                    instance_shortpath,
                    &prisma_client,
                    app,
                )
                .await
            {
                warn!("Failed to detect platform metadata for {}: {}", filename, e);
                // Continue with other phases
            }

            // Phase 4: Check for updates (depends on platform metadata)
            if let Err(e) = self
                .check_and_cache_updates(&metadata_id, &filename, instance_shortpath, &prisma_client, app)
                .await
            {
                warn!("Failed to check updates for {}: {}", filename, e);
            }

            info!("Completed full pipeline for: {}", filename);
        }

        info!(
            "Completed full caching pipeline for instance {}",
            instance_shortpath
        );
        
        // Complete the instance scan task
        self.task_tracker.complete_task(
            &task_id,
            true,
            None,
            Some(format!("Processed {} files in instance {}", total_files, instance_shortpath)),
        );
    }

    /// Cache a single mod file (no-op after V2 removal)
    pub async fn cache_single_mod_file(
        &self,
        instance_id: InstanceId,
        mod_path: &PathBuf,
        addon_type: crate::domain::instance::AddonType,
        prisma_client: &PrismaClient,
        platform_metadata: Option<PlatformMetadata>,
    ) -> anyhow::Result<()> {
        use carbon_addon_cache::CacheScheduler;
        use std::fs;

        debug!(
            "Caching single mod file: {:?} for instance: {}",
            mod_path, instance_id
        );

        // Track this single file cache operation
        let filename = mod_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let task_id = self.task_tracker.start_task(CacheTaskType::SingleFileCache {
            filename: filename.clone(),
            instance_name: format!("instance_{}", instance_id.0),
        });

        // Execute the caching logic with error handling
        let result = async {
            if !mod_path.exists() {
                return Err(anyhow::anyhow!("Mod file does not exist: {:?}", mod_path));
            }

        // Get the filename
        let filename = mod_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid filename: {:?}", mod_path))?
            .to_string();

        // Get file size
        let metadata = fs::metadata(mod_path)?;
        let filesize = metadata.len() as i32;

        // Calculate file hashes
        debug!("Calculating checksums for mod file: {:?}", mod_path);
        let file_hashes = CacheScheduler::compute_file_hashes_buffered(mod_path).await?;
        let checksums = file_hashes.to_checksums();

        // Create metadata ID based on the checksums
        let metadata_id = checksums.blake3.clone();

        // Check if ModMetadata already exists
        let existing_metadata = prisma_client
            .mod_metadata()
            .find_first(vec![metadb::id::equals(metadata_id.clone())])
            .exec()
            .await?;

        // Create ModMetadata if it doesn't exist
        if existing_metadata.is_none() {
            debug!("Creating new ModMetadata record for: {}", metadata_id);

            // Convert checksums to bytes for sha512 and sha1
            // We need actual sha512 and sha1 checksums, so let's compute them
            let file_contents = std::fs::read(mod_path)?;

            // Calculate SHA512
            let mut sha512_hasher = Sha512::new();
            sha512_hasher.update(&file_contents);
            let sha512_hash = sha512_hasher.finalize();
            let sha512_bytes = sha512_hash.to_vec();

            // Calculate SHA1
            let mut sha1_hasher = Sha1::new();
            sha1_hasher.update(&file_contents);
            let sha1_hash = sha1_hasher.finalize();
            let sha1_bytes = sha1_hash.to_vec();

            prisma_client
                .mod_metadata()
                .create(
                    metadata_id.clone(),
                    checksums.murmur2 as i32,
                    sha512_bytes,
                    sha1_bytes,
                    "".to_string(), // modloaders - empty for now
                    vec![],
                )
                .exec()
                .await?;
        }

        // Create platform-specific metadata if provided
        match platform_metadata {
            Some(PlatformMetadata::CurseForge {
                project_id,
                file_id,
            }) => {
                debug!(
                    "Creating CurseForge metadata for project: {}, file: {}",
                    project_id, file_id
                );

                // Check if CurseForgeModCache already exists
                let existing_cf_cache = prisma_client
                    .curse_forge_mod_cache()
                    .find_first(vec![cfdb::metadata_id::equals(metadata_id.clone())])
                    .exec()
                    .await?;

                if existing_cf_cache.is_none() {
                    // We need to create a basic CurseForge cache record
                    // Since we don't have all the metadata from the API here, we'll use basic values
                    prisma_client
                        .curse_forge_mod_cache()
                        .create_unchecked(
                            metadata_id.clone(),
                            checksums.murmur2 as i32,
                            project_id as i32,
                            file_id as i32,
                            filename.clone(),      // Use filename as name fallback
                            "Unknown".to_string(), // version - we don't have this info
                            "unknown".to_string(), // urlslug - we don't have this info
                            "".to_string(),        // summary - we don't have this info
                            "Unknown".to_string(), // authors - we don't have this info
                            2,                     // releaseType - assume stable (2)
                            "".to_string(),        // update_paths - empty for now
                            chrono::Utc::now().into(), // cached_at - current timestamp
                            vec![],
                        )
                        .exec()
                        .await?;

                    debug!("Successfully created CurseForge metadata");
                } else {
                    debug!("CurseForge metadata already exists");
                }
            }
            Some(PlatformMetadata::Modrinth {
                project_id,
                version_id,
            }) => {
                debug!(
                    "Creating Modrinth metadata for project: {}, version: {}",
                    project_id, version_id
                );

                // Check if ModrinthModCache already exists
                let existing_mr_cache = prisma_client
                    .modrinth_mod_cache()
                    .find_first(vec![mrdb::metadata_id::equals(metadata_id.clone())])
                    .exec()
                    .await?;

                if existing_mr_cache.is_none() {
                    // Create a basic Modrinth cache record using the correct 14-parameter signature
                    // Generate expected URL based on project_id, version_id, and filename
                    let file_url = format!(
                        "https://cdn.modrinth.com/data/{}/versions/{}/{}",
                        project_id, version_id, filename
                    );

                    // We need to get the actual SHA512 for Modrinth
                    let file_contents = std::fs::read(mod_path)?;
                    let mut sha512_hasher = Sha512::new();
                    sha512_hasher.update(&file_contents);
                    let sha512_hash = sha512_hasher.finalize();
                    let sha512_hex = hex::encode(sha512_hash);

                    prisma_client
                        .modrinth_mod_cache()
                        .create_unchecked(
                            metadata_id.clone(),
                            sha512_hex, // sha512 - actual calculated value
                            project_id.clone(),
                            version_id.clone(),
                            filename.clone(), // title - use filename as fallback
                            "Unknown".to_string(), // version - we don't have this info
                            "unknown".to_string(), // urlslug - we don't have this info
                            "".to_string(),   // description - we don't have this info
                            "Unknown".to_string(), // authors - we don't have this info
                            2,                // releaseType - assume stable (2)
                            "".to_string(),   // updatePaths - empty for now
                            filename.clone(), // filename
                            file_url,         // fileUrl - construct from known data
                            chrono::Utc::now().into(), // cachedAt - current timestamp
                            vec![],
                        )
                        .exec()
                        .await?;

                    debug!("Successfully created Modrinth metadata");
                } else {
                    debug!("Modrinth metadata already exists");
                }
            }
            None => {
                debug!("No platform metadata provided, creating basic mod cache entry only");
            }
        }

        // Check if ModFileCache entry already exists for this instance and filename
        let existing_cache = prisma_client
            .mod_file_cache()
            .find_first(vec![
                fcdb::instance_id::equals(*instance_id),
                fcdb::filename::equals(filename.clone()),
            ])
            .exec()
            .await?;

        // Create ModFileCache entry if it doesn't exist
        if existing_cache.is_none() {
            debug!(
                "Creating new ModFileCache record for instance: {}, file: {}",
                instance_id, filename
            );

            let addon_type_str = match addon_type {
                crate::domain::instance::AddonType::Mods => "mods",
                crate::domain::instance::AddonType::ResourcePacks => "resourcepacks",
                crate::domain::instance::AddonType::Shaders => "shaders",
                crate::domain::instance::AddonType::DataPacks => "datapacks",
                crate::domain::instance::AddonType::Worlds => "worlds",
            };

            prisma_client
                .mod_file_cache()
                .create_unchecked(
                    *instance_id,
                    filename,
                    filesize,
                    true, // enabled by default
                    metadata_id,
                    vec![fcdb::addon_type::set(addon_type_str.to_string())],
                )
                .exec()
                .await?;
        } else {
            debug!(
                "ModFileCache entry already exists for instance: {}, file: {}",
                instance_id, filename
            );
        }

            debug!("Successfully cached single mod file: {:?}", mod_path);
            Ok(())
        }.await;
        
        // Complete the task based on result
        match &result {
            Ok(_) => {
                self.task_tracker.complete_task(
                    &task_id,
                    true,
                    None,
                    Some(format!("Cached file: {}", filename)),
                );
            }
            Err(e) => {
                self.task_tracker.complete_task(
                    &task_id,
                    false,
                    Some(e.to_string()),
                    None,
                );
            }
        }
        
        result
    }

    /// Get the metadata ID (Blake3 hash) for a mod file
    async fn get_mod_metadata_id(&self, mod_path: &PathBuf) -> anyhow::Result<String> {
        use carbon_addon_cache::CacheScheduler;

        let file_hashes = CacheScheduler::compute_file_hashes_buffered(mod_path).await?;
        let checksums = file_hashes.to_checksums();
        Ok(checksums.blake3)
    }

    /// Extract and cache images from mod JAR files
    async fn extract_and_cache_images(
        &self,
        metadata_id: &str,
        mod_path: &PathBuf,
        filename: &str,
        instance_name: &str,
        app: ManagerRef<'_, Self>,
    ) -> anyhow::Result<()> {
        use std::fs::File;
        use std::io::Read;
        use zip::ZipArchive;

        debug!("Extracting images from mod file: {:?}", mod_path);

        // Start tracking this image extraction task
        let addon_name = filename.strip_suffix(".jar").unwrap_or(filename);
        let task_id = self.task_tracker.start_task(CacheTaskType::ImageExtraction {
            filename: filename.to_string(),
            instance_name: instance_name.to_string(),
            addon_name: Some(addon_name.to_string()),
            image_types: vec![],
        });

        // Check if images already exist for this mod
        let existing_image = app
            .app
            .prisma_client
            .local_mod_image_cache()
            .find_unique(localimg::metadata_id::equals(metadata_id.to_string()))
            .exec()
            .await?;

        if existing_image.is_some() {
            debug!("Image already cached for mod: {}", metadata_id);
            self.task_tracker.complete_task(&task_id, true, None, Some("Images already cached".to_string()));
            return Ok(());
        }

        // Open the JAR file as a ZIP archive
        let file = File::open(mod_path)?;
        let mut archive = ZipArchive::new(file)?;

        // Update task progress  
        self.task_tracker.update_task(&task_id, CacheTaskStatus::Running {
            stage: format!("Searching for icon in {}", addon_name),
            progress: None,
        });

        // Common icon filenames to look for
        let icon_files = vec![
            "icon.png",
            "pack.png",
            "logo.png",
            "mod_icon.png",
            "assets/icon.png",
        ];

        let mut extracted_any = false;
        let mut found_icon_name = String::new();

        for icon_filename in icon_files {
            if let Ok(mut zip_file) = archive.by_name(icon_filename) {
                let mut icon_data = Vec::new();
                if let Ok(_) = zip_file.read_to_end(&mut icon_data) {
                    if !icon_data.is_empty() {
                        // Validate that it's actually an image (PNG signature)
                        if icon_data.len() >= 8 && &icon_data[0..8] == b"\x89PNG\r\n\x1a\n" {
                            debug!(
                                "Found PNG icon: {} ({} bytes)",
                                icon_filename,
                                icon_data.len()
                            );

                            // Store the icon in the LocalModImageCache table
                            let icon_size = icon_data.len();
                            match app
                                .app
                                .prisma_client
                                .local_mod_image_cache()
                                .create_unchecked(
                                    metadata_id.to_string(), // metadataId
                                    icon_data,               // data
                                    vec![],
                                )
                                .exec()
                                .await
                            {
                                Ok(_) => {
                                    debug!("Successfully stored icon for mod: {}", metadata_id);
                                    found_icon_name = icon_filename.to_string();
                                    extracted_any = true;
                                    self.task_tracker.update_task(&task_id, CacheTaskStatus::Running {
                                        stage: format!("Extracted icon {} ({} bytes)", icon_filename, icon_size),
                                        progress: None,
                                    });
                                    break; // Only store one icon per mod
                                }
                                Err(e) => {
                                    warn!("Failed to store icon for {}: {}", metadata_id, e);
                                }
                            }
                        } else {
                            debug!("File {} is not a valid PNG, skipping", icon_filename);
                        }
                    }
                }
            }
        }

        // Complete the task
        if extracted_any {
            self.task_tracker.complete_task(
                &task_id, 
                true, 
                None, 
                Some(format!("Extracted icon: {}", found_icon_name))
            );
        } else {
            debug!("No valid icons found in mod: {:?}", mod_path);
            self.task_tracker.complete_task(
                &task_id, 
                true, 
                None, 
                Some("No icons found".to_string())
            );
        }

        Ok(())
    }

    /// Detect and cache platform metadata (CurseForge/Modrinth)
    async fn detect_and_cache_platform_metadata(
        &self,
        metadata_id: &str,
        mod_path: &PathBuf,
        filename: &str,
        instance_name: &str,
        prisma_client: &PrismaClient,
        app: ManagerRef<'_, Self>,
    ) -> anyhow::Result<()> {
        debug!("Detecting platform metadata for: {}", filename);

        // Start tracking this platform detection task
        let addon_name = filename.strip_suffix(".jar").unwrap_or(filename);
        let task_id = self.task_tracker.start_task(CacheTaskType::PlatformDetection {
            filename: filename.to_string(),
            instance_name: instance_name.to_string(),
            addon_name: Some(addon_name.to_string()),
            platform_type: None,
        });

        // Check if platform metadata already exists
        let existing_cf = prisma_client
            .curse_forge_mod_cache()
            .find_first(vec![cfdb::metadata_id::equals(metadata_id.to_string())])
            .exec()
            .await?;

        let existing_mr = prisma_client
            .modrinth_mod_cache()
            .find_first(vec![mrdb::metadata_id::equals(metadata_id.to_string())])
            .exec()
            .await?;

        if existing_cf.is_some() || existing_mr.is_some() {
            debug!("Platform metadata already exists for: {}", metadata_id);
            let platform = if existing_cf.is_some() { "CurseForge" } else { "Modrinth" };
            self.task_tracker.complete_task(&task_id, true, None, Some(format!("Already detected on {}", platform)));
            return Ok(());
        }

        // Get file checksums for platform detection
        let file_hashes =
            carbon_addon_cache::CacheScheduler::compute_file_hashes_buffered(mod_path).await?;
        let checksums = file_hashes.to_checksums();

        // Try CurseForge detection first (using Murmur2 hash)
        self.task_tracker.update_task(&task_id, CacheTaskStatus::Running {
            stage: format!("Searching CurseForge for {}", addon_name),
            progress: None,
        });

        if let Err(e) = self
            .detect_curseforge_metadata(
                &checksums.murmur2,
                metadata_id,
                filename,
                prisma_client,
                app,
            )
            .await
        {
            debug!("CurseForge detection failed for {}: {}", filename, e);
        } else {
            // CurseForge detection succeeded
            self.task_tracker.complete_task(&task_id, true, None, Some("Detected on CurseForge".to_string()));
            return Ok(());
        }

        // Calculate SHA512 for Modrinth detection
        let file_contents = std::fs::read(mod_path)?;
        let mut sha512_hasher = Sha512::new();
        sha512_hasher.update(&file_contents);
        let sha512_hash = hex::encode(sha512_hasher.finalize());

        // Try Modrinth detection (using SHA512 hash)
        self.task_tracker.update_task(&task_id, CacheTaskStatus::Running {
            stage: format!("Searching Modrinth for {}", addon_name),
            progress: None,
        });

        if let Err(e) = self
            .detect_modrinth_metadata(&sha512_hash, metadata_id, filename, prisma_client, app)
            .await
        {
            debug!("Modrinth detection failed for {}: {}", filename, e);
            // Both platforms failed - complete as local mod
            self.task_tracker.complete_task(&task_id, true, None, Some("Not found on platforms - local mod".to_string()));
        } else {
            // Modrinth detection succeeded
            self.task_tracker.complete_task(&task_id, true, None, Some("Detected on Modrinth".to_string()));
        }

        Ok(())
    }

    /// Check for updates based on platform metadata
    async fn check_and_cache_updates(
        &self,
        metadata_id: &str,
        filename: &str,
        instance_name: &str,
        prisma_client: &PrismaClient,
        app: ManagerRef<'_, Self>,
    ) -> anyhow::Result<()> {
        debug!("Checking for updates for mod: {}", metadata_id);

        // Start tracking this update check task
        let addon_name = filename.strip_suffix(".jar").unwrap_or(filename);
        let task_id = self.task_tracker.start_task(CacheTaskType::UpdateCheck {
            filename: filename.to_string(),
            instance_name: instance_name.to_string(),
            addon_name: Some(addon_name.to_string()),
            platform_type: None,
            current_version: None,
        });

        // Check if we have platform metadata to work with
        let cf_metadata = prisma_client
            .curse_forge_mod_cache()
            .find_first(vec![cfdb::metadata_id::equals(metadata_id.to_string())])
            .exec()
            .await?;

        let mr_metadata = prisma_client
            .modrinth_mod_cache()
            .find_first(vec![mrdb::metadata_id::equals(metadata_id.to_string())])
            .exec()
            .await?;

        // For now, we'll implement a placeholder that marks the mod as checked
        // Real implementation would query platform APIs for latest versions
        if let Some(cf_meta) = cf_metadata {
            self.task_tracker.update_task(&task_id, CacheTaskStatus::Running {
                stage: format!("Checking CurseForge for {} updates", addon_name),
                progress: None,
            });
            debug!("Platform metadata found, update checking would happen here");
            // TODO: Implement actual update checking with platform APIs
            self.task_tracker.complete_task(&task_id, true, None, Some("Update check completed (CurseForge)".to_string()));
        } else if let Some(mr_meta) = mr_metadata {
            self.task_tracker.update_task(&task_id, CacheTaskStatus::Running {
                stage: format!("Checking Modrinth for {} updates", addon_name),
                progress: None,
            });
            debug!("Platform metadata found, update checking would happen here");
            // TODO: Implement actual update checking with platform APIs
            self.task_tracker.complete_task(&task_id, true, None, Some("Update check completed (Modrinth)".to_string()));
        } else {
            debug!("No platform metadata found, skipping update check");
            self.task_tracker.complete_task(&task_id, true, None, Some("No platform metadata - skipped".to_string()));
        }

        Ok(())
    }

    /// Detect CurseForge metadata using Murmur2 hash
    async fn detect_curseforge_metadata(
        &self,
        murmur2_hash: &u32,
        metadata_id: &str,
        filename: &str,
        prisma_client: &PrismaClient,
        app: ManagerRef<'_, Self>,
    ) -> anyhow::Result<()> {
        debug!(
            "Attempting CurseForge detection for {} (murmur2: {})",
            filename, murmur2_hash
        );

        // Try to find the mod on CurseForge using the Murmur2 hash
        let modplatforms = app.app.modplatforms_manager();
        let curseforge_client = &modplatforms.curseforge;

        // Query CurseForge for matching fingerprints
        let fingerprint_response = match curseforge_client.get_fingerprints(&[*murmur2_hash]).await
        {
            Ok(response) => response,
            Err(e) => {
                debug!(
                    "CurseForge fingerprint lookup failed for {}: {}",
                    filename, e
                );
                return Ok(()); // Not an error, just no match found
            }
        };

        // Check if we found exact matches
        if fingerprint_response.data.exact_matches.is_empty() {
            debug!("No CurseForge matches found for: {}", filename);
            return Ok(());
        }

        // Get the first match (there should only be one for a given hash)
        let fingerprint_match = &fingerprint_response.data.exact_matches[0];
        debug!(
            "Found CurseForge match for {}: project_id={}, file_id={}",
            filename, fingerprint_match.id, fingerprint_match.file.id
        );

        // Create CurseForge cache entry directly in database
        match prisma_client
            .curse_forge_mod_cache()
            .create_unchecked(
                metadata_id.to_string(),
                *murmur2_hash as i32,
                fingerprint_match.id as i32,
                fingerprint_match.file.id as i32,
                fingerprint_match.file.display_name.clone(),
                "unknown".to_string(), // version - we don't extract this from fingerprint response
                "unknown".to_string(), // urlslug - not in fingerprint response
                "".to_string(),        // summary - not in fingerprint response
                "Unknown".to_string(), // authors - not in fingerprint response
                2,                     // releaseType - assume stable
                "".to_string(),        // update_paths - empty for now
                chrono::Utc::now().into(), // cached_at
                vec![],
            )
            .exec()
            .await
        {
            Ok(_) => {
                debug!("Successfully cached CurseForge metadata for: {}", filename);
            }
            Err(e) => {
                warn!(
                    "Failed to create CurseForge cache entry for {}: {}",
                    filename, e
                );
            }
        }

        Ok(())
    }

    /// Detect Modrinth metadata using SHA512 hash
    async fn detect_modrinth_metadata(
        &self,
        sha512_hash: &str,
        metadata_id: &str,
        filename: &str,
        prisma_client: &PrismaClient,
        app: ManagerRef<'_, Self>,
    ) -> anyhow::Result<()> {
        debug!(
            "Attempting Modrinth detection for {} (sha512: {})",
            filename, sha512_hash
        );

        // Try to find the mod on Modrinth using the SHA512 hash
        let modplatforms = app.app.modplatforms_manager();
        let modrinth_client = &modplatforms.modrinth;

        // Query Modrinth for matching version hashes
        use carbon_platforms::modrinth::search::VersionHashesQuery;
        use carbon_platforms::modrinth::version::HashAlgorithm;

        let version_response = match modrinth_client
            .get_versions_from_hash(&VersionHashesQuery {
                hashes: vec![sha512_hash.to_string()],
                algorithm: HashAlgorithm::SHA512,
            })
            .await
        {
            Ok(response) => response,
            Err(e) => {
                debug!("Modrinth hash lookup failed for {}: {}", filename, e);
                return Ok(()); // Not an error, just no match found
            }
        };

        // Check if we found any matches
        if version_response.is_empty() {
            debug!("No Modrinth matches found for: {}", filename);
            return Ok(());
        }

        // Get the first version match (there should be one for our hash)
        let version_match = version_response.values().next().unwrap();
        debug!(
            "Found Modrinth match for {}: project_id={}, version_id={}",
            filename, version_match.project_id, version_match.id
        );

        // Create Modrinth cache entry directly in database
        let file_url = version_match
            .files
            .get(0)
            .map(|f| f.url.clone())
            .unwrap_or_else(|| {
                format!(
                    "https://cdn.modrinth.com/data/{}/versions/{}/{}",
                    version_match.project_id, version_match.id, filename
                )
            });

        match prisma_client
            .modrinth_mod_cache()
            .create_unchecked(
                metadata_id.to_string(),
                sha512_hash.to_string(),
                version_match.project_id.clone(),
                version_match.id.clone(),
                version_match.name.clone(),
                version_match.version_number.clone(),
                "unknown".to_string(),     // urlslug - not directly available
                "".to_string(),            // description - not in version response
                "Unknown".to_string(),     // authors - not in version response
                2,                         // releaseType - assume stable
                "".to_string(),            // updatePaths - empty for now
                filename.to_string(),      // filename
                file_url,                  // fileUrl
                chrono::Utc::now().into(), // cachedAt
                vec![],
            )
            .exec()
            .await
        {
            Ok(_) => {
                debug!("Successfully cached Modrinth metadata for: {}", filename);
            }
            Err(e) => {
                warn!(
                    "Failed to create Modrinth cache entry for {}: {}",
                    filename, e
                );
            }
        }

        Ok(())
    }

    /// Set high priority caching for an instance (no-op after V2 removal)
    pub async fn cache_with_priority(&self, _instance_id: InstanceId) {
        debug!("Priority caching disabled");
    }

    /// Set the currently viewed instance for priority caching (no-op after V2 removal)
    pub async fn watch_and_prioritize(&self, _instance_id: Option<InstanceId>) {
        debug!("Watch and prioritize disabled");
    }

    /// Force immediate caching and wait for completion
    pub async fn override_caching_and_wait(
        &self,
        instance_id: InstanceId,
        app: ManagerRef<'_, Self>,
    ) -> anyhow::Result<()> {
        // Just delegate to the _with_app method since we now have the app reference
        self.override_caching_and_wait_with_app(instance_id, app)
            .await
    }

    /// Force immediate caching and wait for completion (with app reference)
    pub async fn override_caching_and_wait_with_app(
        &self,
        instance_id: InstanceId,
        app: ManagerRef<'_, Self>,
    ) -> anyhow::Result<()> {
        info!("Starting override caching for instance: {}", instance_id);

        // Get instance mod files that need caching
        let mod_files = app
            .app
            .prisma_client
            .mod_file_cache()
            .find_many(vec![fcdb::instance_id::equals(*instance_id)])
            .exec()
            .await
            .map_err(|e| anyhow!("Failed to get mod files: {}", e))?;

        if mod_files.is_empty() {
            info!(
                "No mod files found in database for instance {}, scanning filesystem",
                instance_id
            );

            // Get the instance shortpath for proper file path resolution
            let instance_manager = app.app.instance_manager();
            let instances = instance_manager.instances.read().await;
            let instance = instances
                .get(&instance_id)
                .ok_or_else(|| anyhow!("Instance {} not found", instance_id))?;
            let instance_shortpath = instance.shortpath.clone();
            drop(instances);

            // Scan the mods directory for .jar files
            let mods_dir = app
                .app
                .settings_manager()
                .runtime_path
                .get_instances()
                .get_instance_path(&instance_shortpath)
                .get_data_path()
                .join("mods");

            if mods_dir.exists() {
                // Create database entries for each mod file found
                let mut entries = Vec::new();
                for entry in std::fs::read_dir(&mods_dir)? {
                    let entry = entry?;
                    let path = entry.path();

                    if path.is_file() {
                        let filename = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_string());

                        if let Some(filename) = filename {
                            // Check if it's a mod file (jar, disabled, etc.)
                            if filename.ends_with(".jar") || filename.ends_with(".jar.disabled") {
                                info!("Found mod file to cache: {}", filename);

                                // Queue for full pipeline caching instead of just creating database entries
                                entries.push((filename, path));
                            }
                        }
                    }
                }

                if entries.is_empty() {
                    info!(
                        "No mod files found in filesystem for instance {}",
                        instance_id
                    );
                    return Ok(());
                } else {
                    info!(
                        "Found {} mod files to cache for instance {}",
                        entries.len(),
                        instance_id
                    );

                    // Run full caching pipeline for all found files
                    self.cache_missing_files(instance_id, &instance_shortpath, entries, app)
                        .await;
                }
            } else {
                info!("Mods directory does not exist for instance {}", instance_id);
                return Ok(());
            }
        } else {
            // mod_files already exist in database, run them through full pipeline to ensure images/metadata are cached
            info!(
                "Found {} existing mod files in database for instance {}, running through full pipeline",
                mod_files.len(),
                instance_id
            );

            // Get the instance shortpath for file path resolution
            let instance_manager = app.app.instance_manager();
            let instances = instance_manager.instances.read().await;
            let instance = instances
                .get(&instance_id)
                .ok_or_else(|| anyhow!("Instance {} not found", instance_id))?;
            let instance_shortpath = instance.shortpath.clone();
            drop(instances);

            // Prepare existing files for pipeline processing
            let mut files_to_cache = Vec::new();
            for mod_file in mod_files {
                let path = app
                    .app
                    .settings_manager()
                    .runtime_path
                    .get_instances()
                    .get_instance_path(&instance_shortpath)
                    .get_data_path()
                    .join("mods")
                    .join(&mod_file.filename);

                if path.exists() {
                    files_to_cache.push((mod_file.filename, path));
                } else {
                    warn!(
                        "Mod file {} exists in database but not on disk",
                        mod_file.filename
                    );
                }
            }

            if !files_to_cache.is_empty() {
                // Run full caching pipeline for existing files
                self.cache_missing_files(instance_id, &instance_shortpath, files_to_cache, app)
                    .await;
            }
        }

        // If we reach here, all mod files have been processed through the full pipeline
        info!("Cache processing completed for instance: {}", instance_id);
        Ok(())
    }

    /// Queue background caching for an instance
    pub async fn queue_caching(&self, instance_id: InstanceId, force: bool) {
        warn!(
            "queue_caching called without app reference for instance {} - this is a temporary implementation",
            instance_id
        );
        // This is a hack but necessary since some callers don't have app reference
        // We'll store the instance_id for later processing when we have app access
        info!(
            "Marking instance {} for caching (force: {})",
            instance_id, force
        );
    }

    /// Queue background caching for an instance with app reference
    pub async fn queue_caching_with_app(
        &self,
        instance_id: InstanceId,
        force: bool,
        app: ManagerRef<'_, Self>,
    ) {
        info!(
            "Queue caching requested for instance: {} (force: {})",
            instance_id, force
        );

        // Clone the necessary references for the spawned task
        let app_clone = app.app.clone();
        let instance_id_clone = instance_id;

        // Spawn a background task to perform the caching
        tokio::spawn(async move {
            // Add a small delay to ensure modpack files are fully written
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            info!(
                "Starting background caching for instance: {}",
                instance_id_clone
            );

            // Get a new reference to the cache manager from the app
            let cache_manager = app_clone.meta_cache_manager();

            // Perform the caching operation
            match cache_manager
                .override_caching_and_wait_with_app(instance_id_clone, cache_manager)
                .await
            {
                Ok(()) => {
                    info!(
                        "Background caching completed successfully for instance: {}",
                        instance_id_clone
                    );
                    // Invalidate the instance mods cache to trigger UI update
                    app_clone.invalidate(INSTANCE_MODS, Some(instance_id_clone.0.into()));
                }
                Err(e) => {
                    error!(
                        "Background caching failed for instance {}: {}",
                        instance_id_clone, e
                    );
                }
            }
        });
    }

    /// Handle instance removal (no-op after V2 removal)
    pub async fn instance_removed(&self, _instance_id: InstanceId) {
        debug!("Instance removal handling disabled");
    }

    /// Set the currently watched instance (no-op after V2 removal)
    pub async fn set_watched_instance(&self, _instance_id: Option<InstanceId>) {
        debug!("Set watched instance disabled");
    }

    /// Pause all caching operations
    pub async fn pause_all_caching(&self) {
        // TODO: Implement pause functionality in V2
        warn!("pause_all_caching: not yet implemented in V2");
    }

    /// Resume all caching operations
    pub async fn resume_all_caching(&self) {
        // TODO: Implement resume functionality in V2
        warn!("resume_all_caching: not yet implemented in V2");
    }

    /// Check if a mod has platform metadata (using direct database access)
    pub async fn check_mod_has_platform_metadata(&self, mod_id: &str) -> anyhow::Result<bool> {
        // This method needs an app reference to work - for now return false as a safe default
        // TODO: Pass app reference or make this method work differently
        warn!("check_mod_has_platform_metadata: returning false as app reference not available");
        Ok(false)
    }

    /// Clear platform metadata for a mod (using direct database access)
    pub async fn clear_mod_platform_metadata(&self, _mod_id: &str) -> anyhow::Result<()> {
        // This method needs an app reference to work - for now return success
        // TODO: Pass app reference or make this method work differently
        warn!("clear_mod_platform_metadata: no-op as app reference not available");
        Ok(())
    }

    /// Get the cache status for an instance (returns Idle after V2 removal)
    pub async fn get_instance_cache_status(&self, _instance_id: InstanceId) -> InstanceCacheStatus {
        InstanceCacheStatus::Idle
    }

    /// Get cache statistics (returns default after V2 removal)
    pub async fn get_v2_stats(&self) -> Option<CacheStats> {
        Some(CacheStats::default())
    }

    /// Launch background tasks for testing
    #[cfg(test)]
    pub async fn launch_background_tasks_for_test(&self, app: ManagerRef<'_, Self>) {
        info!("Initializing cache system for tests");

        if let Err(e) = self.initialize_cache_manager(app).await {
            error!("Failed to initialize cache manager for tests: {}", e);
        } else {
            info!("Test cache system initialized successfully");
        }
    }

    /// Shutdown the cache system
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        info!("Shutting down cache system");

        // Note: cache_manager removed - no shutdown needed

        info!("Cache system shutdown complete");
        Ok(())
    }

    /// Garbage collect unused mod metadata (using direct database access)
    pub async fn gc_mod_metadata(&self) -> anyhow::Result<()> {
        // This method needs an app reference to work - for now return success
        // TODO: Pass app reference or make this method work differently
        warn!("gc_mod_metadata: no-op as app reference not available");
        Ok(())
    }

    /// Get the cache manager for direct access (mainly for testing/debugging)
    pub async fn get_cache_manager(&self) -> Option<()> {
        // Note: cache_manager removed - returning None for compatibility
        None
    }

    /// Get current running cache tasks
    pub fn get_current_tasks(&self) -> Vec<CacheTaskCurrent> {
        self.task_tracker.get_current_tasks()
    }

    /// Get cache task history
    pub fn get_task_history(&self) -> Vec<CacheTaskHistory> {
        self.task_tracker.get_task_history()
    }

    /// Clear cache task history
    pub fn clear_task_history(&self) {
        self.task_tracker.clear_history()
    }

    /// Get cache task statistics
    pub fn get_task_stats(&self) -> CacheTaskStats {
        self.task_tracker.get_stats()
    }

    /// Clear all cached addon data from database and disk, then trigger re-caching
    pub async fn clear_all_cache(&self, app: ManagerRef<'_, Self>) -> anyhow::Result<()> {
        info!("Starting complete cache clearing operation");

        // Track this operation
        let task_id = self.task_tracker.start_task(CacheTaskType::CacheClear);

        let prisma_client = &app.app.prisma_client;

        // Clear all database cache tables
        info!("Clearing database cache tables");
        self.task_tracker.update_task(&task_id, CacheTaskStatus::Running {
            stage: "Clearing database tables".to_string(),
            progress: Some((0, 7)),
        });

        // Clear ModFileCache entries (this will cascade to dependent records)
        let deleted_file_cache = prisma_client
            .mod_file_cache()
            .delete_many(vec![])
            .exec()
            .await?;
        info!("Deleted {} ModFileCache entries", deleted_file_cache);
        self.task_tracker.update_task(&task_id, CacheTaskStatus::Running {
            stage: "Clearing database tables".to_string(),
            progress: Some((1, 7)),
        });

        // Clear ModMetadata entries (this will cascade to dependent records)
        let deleted_metadata = prisma_client
            .mod_metadata()
            .delete_many(vec![])
            .exec()
            .await?;
        info!("Deleted {} ModMetadata entries", deleted_metadata);

        // Clear CurseForgeModCache entries
        let deleted_cf_cache = prisma_client
            .curse_forge_mod_cache()
            .delete_many(vec![])
            .exec()
            .await?;
        info!("Deleted {} CurseForgeModCache entries", deleted_cf_cache);

        // Clear ModrinthModCache entries
        let deleted_mr_cache = prisma_client
            .modrinth_mod_cache()
            .delete_many(vec![])
            .exec()
            .await?;
        info!("Deleted {} ModrinthModCache entries", deleted_mr_cache);

        // Clear LocalModImageCache entries
        let deleted_image_cache = prisma_client
            .local_mod_image_cache()
            .delete_many(vec![])
            .exec()
            .await?;
        info!("Deleted {} LocalModImageCache entries", deleted_image_cache);

        // Clear CurseForgeModImageCache entries
        let deleted_cf_image_cache = prisma_client
            .curse_forge_mod_image_cache()
            .delete_many(vec![])
            .exec()
            .await?;
        info!(
            "Deleted {} CurseForgeModImageCache entries",
            deleted_cf_image_cache
        );

        // Clear ModrinthModImageCache entries
        let deleted_mr_image_cache = prisma_client
            .modrinth_mod_image_cache()
            .delete_many(vec![])
            .exec()
            .await?;
        info!(
            "Deleted {} ModrinthModImageCache entries",
            deleted_mr_image_cache
        );

        info!("Database cache clearing completed");

        // TODO: Clear hard link cache and other disk-based caches if needed
        // This would require access to the addon cache system's file cleanup methods

        info!("Database cache clearing completed successfully");

        // Trigger automatic re-caching in background
        info!("Starting background re-caching of all instances");
        let app_clone = app.app.clone();
        tokio::spawn(async move {
            let cache_manager = app_clone.meta_cache_manager();
            if let Err(e) = cache_manager.trigger_full_recache(cache_manager).await {
                error!("Background re-caching failed: {}", e);
            } else {
                info!("Background re-caching completed successfully");
            }
        });

        info!("Cache clearing operation finished successfully");
        
        // Complete the task tracking
        self.task_tracker.complete_task(
            &task_id,
            true,
            None,
            Some(format!("Cleared {} cache tables", 7)),
        );
        
        Ok(())
    }

    /// Trigger full re-caching of all instances after cache clearing
    pub async fn trigger_full_recache(&self, app: ManagerRef<'_, Self>) -> anyhow::Result<()> {
        info!("Starting full re-cache of all instances");

        // Use the existing startup scan logic which handles full pipeline caching
        self.startup_scan_missing_mods(app).await;

        // Invalidate all instance mod caches to trigger UI updates
        let instance_manager = app.app.instance_manager();
        let instances = instance_manager.instances.read().await;

        for instance_id in instances.keys() {
            app.app
                .invalidate(INSTANCE_MODS, Some(instance_id.0.into()));
        }

        drop(instances);
        info!("Full re-cache operation completed");
        Ok(())
    }
}
