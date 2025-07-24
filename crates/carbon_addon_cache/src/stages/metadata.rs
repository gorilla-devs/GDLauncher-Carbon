use crate::events::*;
use crate::scheduler::CacheScheduler;
use crate::storage::StorageConfig;
use anyhow::Result;
use blake3::Hasher as Blake3Hasher;
use crossbeam_channel::{Receiver, Sender, bounded};
use md5;
use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use std::collections::{BinaryHeap, HashMap};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use twox_hash::XxHash32;
use zip::ZipArchive;

pub struct MetadataExtractor {
    task_queue: Arc<RwLock<BinaryHeap<MetadataTask>>>,
    instance_priorities: Arc<RwLock<HashMap<String, Priority>>>,
    event_sender: Sender<CacheEvent>,
    event_receiver: Receiver<CacheEvent>,
    config: StorageConfig,
    worker_handles: Vec<JoinHandle<()>>,
    shutdown_senders: Vec<mpsc::UnboundedSender<()>>,
}

#[derive(Debug)]
struct MetadataTask {
    addon_id: String,
    metadata: BasicMetadata,
    priority: Priority,
    created_at: u64,
}

impl PartialEq for MetadataTask {
    fn eq(&self, other: &Self) -> bool {
        self.addon_id == other.addon_id
    }
}

impl Eq for MetadataTask {}

impl PartialOrd for MetadataTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MetadataTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.created_at.cmp(&other.created_at))
    }
}

impl MetadataExtractor {
    pub fn new(config: StorageConfig) -> Result<Self> {
        let (event_sender, event_receiver) = bounded(1000);

        Ok(Self {
            task_queue: Arc::new(RwLock::new(BinaryHeap::new())),
            instance_priorities: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
            event_receiver,
            config,
            worker_handles: Vec::new(),
            shutdown_senders: Vec::new(),
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting metadata extraction stage");

        // Start worker threads - limit to reasonable number for CPU intensive tasks
        let worker_count = (num_cpus::get() / 4).max(1).min(2);
        for i in 0..worker_count {
            let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();
            self.shutdown_senders.push(shutdown_tx);
            let worker = self.spawn_worker(i, shutdown_rx).await?;
            self.worker_handles.push(worker);
        }

        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down metadata extraction stage");

        for sender in &self.shutdown_senders {
            let _ = sender.send(());
        }

        // Wait for workers to finish with timeout, then force abort
        let shutdown_timeout = std::time::Duration::from_secs(1);
        let handles = self.worker_handles.drain(..).collect::<Vec<_>>();

        for handle in handles {
            let result = tokio::time::timeout(shutdown_timeout, handle).await;

            if let Err(_) = result {
                warn!(
                    "Metadata extraction worker shutdown timed out after {}s",
                    shutdown_timeout.as_secs()
                );
            } else if let Ok(Err(e)) = result {
                if e.is_cancelled() {
                    debug!("Metadata extraction worker was successfully aborted");
                } else {
                    error!("Metadata extraction worker failed: {}", e);
                }
            } else {
                debug!("Metadata extraction worker finished cleanly");
            }
        }

        Ok(())
    }

    pub async fn add_addon(
        &self,
        addon_id: String,
        metadata: BasicMetadata,
        priority: Priority,
    ) -> Result<()> {
        // Validate that the file can be opened as a ZIP/JAR
        if let Err(e) = std::fs::File::open(&metadata.file_path) {
            anyhow::bail!("Cannot open file {:?}: {}", metadata.file_path, e);
        }

        // Quick validation that it's a valid ZIP file
        if let Err(e) = zip::ZipArchive::new(std::fs::File::open(&metadata.file_path)?) {
            anyhow::bail!(
                "File {:?} is not a valid ZIP/JAR file: {}",
                metadata.file_path,
                e
            );
        }

        let task = MetadataTask {
            addon_id,
            metadata,
            priority,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };

        self.task_queue.write().push(task);

        Ok(())
    }

    pub async fn prioritize_instance(&self, instance_id: &str, priority: Priority) -> Result<()> {
        self.instance_priorities
            .write()
            .insert(instance_id.to_string(), priority);

        // Reprioritize existing tasks for this instance
        let mut queue = self.task_queue.write();
        let mut tasks: Vec<MetadataTask> = queue.drain().collect();

        for task in &mut tasks {
            if let Some(ref task_instance_id) = task.metadata.instance_id {
                if task_instance_id == instance_id {
                    task.priority = priority;
                }
            }
        }

        for task in tasks {
            queue.push(task);
        }

        Ok(())
    }

    async fn spawn_worker(
        &self,
        worker_id: usize,
        mut shutdown_receiver: mpsc::UnboundedReceiver<()>,
    ) -> Result<JoinHandle<()>> {
        let task_queue = self.task_queue.clone();
        let event_sender = self.event_sender.clone();
        let config = self.config.clone();

        let handle = tokio::spawn(async move {
            debug!("Metadata extraction worker {} started", worker_id);

            loop {
                let task = {
                    let mut queue = task_queue.write();
                    queue.pop()
                };

                match task {
                    Some(task) => {
                        tokio::select! {
                            _ = shutdown_receiver.recv() => {
                                debug!("Metadata extraction worker {} shutting down during task", worker_id);
                                break;
                            }
                            result = Self::process_metadata_task(task, &event_sender, &config) => {
                                if let Err(e) = result {
                                    error!("Error processing metadata task: {}", e);
                                }
                            }
                        }
                    }
                    None => {
                        tokio::select! {
                            _ = shutdown_receiver.recv() => {
                                debug!("Metadata extraction worker {} shutting down", worker_id);
                                break;
                            }
                            _ = tokio::time::sleep(tokio::time::Duration::from_millis(10)) => {}
                        }
                    }
                }
            }

            debug!("Metadata extraction worker {} finished", worker_id);
        });

        Ok(handle)
    }

    async fn process_metadata_task(
        task: MetadataTask,
        event_sender: &Sender<CacheEvent>,
        config: &StorageConfig,
    ) -> Result<()> {
        debug!(
            "Processing metadata extraction for addon: {}",
            task.addon_id
        );

        // Calculate all checksums in a single pass
        let checksums = Self::calculate_checksums(&task.metadata.file_path).await?;

        // Create hard link to centralized storage
        let central_path = config
            .runtime_path
            .join("addons")
            .join(format!("{}.jar", &checksums.blake3));
        Self::create_hard_link(&task.metadata.file_path, &central_path).await?;

        // Extract metadata from the file
        let (mod_metadata, mod_format) =
            Self::extract_mod_metadata(&task.metadata.file_path).await?;

        let local_metadata = LocalMetadata {
            addon_id: task.addon_id.clone(),
            name: mod_metadata.name.unwrap_or_else(|| "Unknown".to_string()),
            version: mod_metadata
                .version
                .unwrap_or_else(|| "Unknown".to_string()),
            authors: mod_metadata.authors.unwrap_or_default(),
            description: mod_metadata.description,
            dependencies: mod_metadata.dependencies.unwrap_or_default(),
            checksums,
            mod_format,
            minecraft_versions: mod_metadata.minecraft_versions.unwrap_or_default(),
            mod_loaders: mod_metadata.mod_loaders.unwrap_or_default(),
            instance_id: task.metadata.instance_id.clone(),
        };

        // Emit MetadataExtracted event
        let event = CacheEvent::MetadataExtracted {
            addon_id: task.addon_id,
            metadata: local_metadata,
        };

        event_sender.send(event)?;

        Ok(())
    }

    async fn calculate_checksums(file_path: &PathBuf) -> Result<Checksums> {
        debug!("Calculating checksums for file: {:?}", file_path);

        // Use carbon_scheduler's optimized hash computation
        let file_hashes = CacheScheduler::compute_file_hashes_buffered(file_path).await?;

        Ok(file_hashes.to_checksums())
    }

    async fn create_hard_link(source: &PathBuf, target: &PathBuf) -> Result<()> {
        // Create target directory if it doesn't exist
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Skip if target already exists
        if target.exists() {
            return Ok(());
        }

        // Try to create hard link, fall back to copy if it fails
        match fs::hard_link(source, target).await {
            Ok(_) => {
                debug!("Created hard link: {:?} -> {:?}", source, target);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to create hard link, falling back to copy: {}", e);
                fs::copy(source, target).await?;
                Ok(())
            }
        }
    }

    async fn extract_mod_metadata(file_path: &PathBuf) -> Result<(ModMetadata, ModFormat)> {
        let file = File::open(file_path)?;
        let mut archive = ZipArchive::new(file)?;

        // Try to find and parse mod metadata files
        if let Ok(metadata) = Self::try_parse_fabric_metadata(&mut archive).await {
            return Ok((metadata, ModFormat::Fabric));
        }

        if let Ok(metadata) = Self::try_parse_quilt_metadata(&mut archive).await {
            return Ok((metadata, ModFormat::Quilt));
        }

        if let Ok(metadata) = Self::try_parse_forge_metadata(&mut archive).await {
            return Ok((metadata, ModFormat::Forge));
        }

        if let Ok(metadata) = Self::try_parse_legacy_forge_metadata(&mut archive).await {
            return Ok((metadata, ModFormat::Forge));
        }

        // If no metadata found, create minimal metadata
        let filename = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let metadata = ModMetadata {
            name: Some(filename.to_string()),
            version: Some("unknown".to_string()),
            authors: None,
            description: None,
            dependencies: None,
            minecraft_versions: None,
            mod_loaders: None,
        };

        Ok((metadata, ModFormat::Unknown))
    }

    async fn try_parse_fabric_metadata(archive: &mut ZipArchive<File>) -> Result<ModMetadata> {
        let mut file = archive.by_name("fabric.mod.json")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let json: serde_json::Value = serde_json::from_str(&contents)?;

        let metadata = ModMetadata {
            name: json["name"].as_str().map(|s| s.to_string()),
            version: json["version"].as_str().map(|s| s.to_string()),
            authors: json["authors"].as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            }),
            description: json["description"].as_str().map(|s| s.to_string()),
            dependencies: json["depends"].as_object().map(|deps| {
                deps.iter()
                    .map(|(k, v)| Dependency {
                        mod_id: k.clone(),
                        version_requirement: v.as_str().unwrap_or("*").to_string(),
                        dependency_type: DependencyType::Required,
                    })
                    .collect()
            }),
            minecraft_versions: json["depends"]["minecraft"]
                .as_str()
                .map(|v| vec![v.to_string()]),
            mod_loaders: Some(vec!["fabric".to_string()]),
        };

        Ok(metadata)
    }

    async fn try_parse_quilt_metadata(archive: &mut ZipArchive<File>) -> Result<ModMetadata> {
        let mut file = archive.by_name("quilt.mod.json")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let json: serde_json::Value = serde_json::from_str(&contents)?;

        let metadata = ModMetadata {
            name: json["quilt_loader"]["metadata"]["name"]
                .as_str()
                .map(|s| s.to_string()),
            version: json["quilt_loader"]["version"]
                .as_str()
                .map(|s| s.to_string()),
            authors: json["quilt_loader"]["metadata"]["contributors"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v["name"].as_str().map(|s| s.to_string()))
                        .collect()
                }),
            description: json["quilt_loader"]["metadata"]["description"]
                .as_str()
                .map(|s| s.to_string()),
            dependencies: json["quilt_loader"]["depends"].as_array().map(|deps| {
                deps.iter()
                    .filter_map(|dep| {
                        let id = dep["id"].as_str()?;
                        let version = dep["version"].as_str().unwrap_or("*");
                        Some(Dependency {
                            mod_id: id.to_string(),
                            version_requirement: version.to_string(),
                            dependency_type: DependencyType::Required,
                        })
                    })
                    .collect()
            }),
            minecraft_versions: json["quilt_loader"]["depends"].as_array().and_then(|deps| {
                deps.iter()
                    .find(|dep| dep["id"].as_str() == Some("minecraft"))
                    .and_then(|dep| dep["version"].as_str())
                    .map(|v| vec![v.to_string()])
            }),
            mod_loaders: Some(vec!["quilt".to_string()]),
        };

        Ok(metadata)
    }

    async fn try_parse_forge_metadata(archive: &mut ZipArchive<File>) -> Result<ModMetadata> {
        let mut file = archive.by_name("META-INF/mods.toml")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        // Simple TOML parsing for mods.toml
        let toml: toml::Value = contents.parse()?;

        let mods = toml["mods"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No mods array"))?;
        let mod_info = mods.first().ok_or_else(|| anyhow::anyhow!("No mod info"))?;

        let metadata = ModMetadata {
            name: mod_info["displayName"].as_str().map(|s| s.to_string()),
            version: mod_info["version"].as_str().map(|s| s.to_string()),
            authors: mod_info["authors"].as_str().map(|s| vec![s.to_string()]),
            description: mod_info["description"].as_str().map(|s| s.to_string()),
            dependencies: toml["dependencies"].as_table().map(|deps| {
                let empty_vec = vec![];
                deps.iter()
                    .flat_map(|(_, mod_deps)| {
                        mod_deps
                            .as_array()
                            .unwrap_or(&empty_vec)
                            .iter()
                            .filter_map(|dep| {
                                let id = dep["modId"].as_str()?;
                                let version = dep["versionRange"].as_str().unwrap_or("*");
                                Some(Dependency {
                                    mod_id: id.to_string(),
                                    version_requirement: version.to_string(),
                                    dependency_type: DependencyType::Required,
                                })
                            })
                    })
                    .collect()
            }),
            minecraft_versions: toml["dependencies"]["minecraft"]
                .as_array()
                .and_then(|deps| {
                    deps.first()
                        .and_then(|dep| dep["versionRange"].as_str())
                        .map(|v| vec![v.to_string()])
                }),
            mod_loaders: Some(vec!["forge".to_string()]),
        };

        Ok(metadata)
    }

    async fn try_parse_legacy_forge_metadata(
        archive: &mut ZipArchive<File>,
    ) -> Result<ModMetadata> {
        let mut file = archive.by_name("mcmod.info")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let json: serde_json::Value = serde_json::from_str(&contents)?;

        let mod_info = if let Some(array) = json.as_array() {
            array
                .first()
                .ok_or_else(|| anyhow::anyhow!("Empty mod info array"))?
        } else {
            &json
        };

        let metadata = ModMetadata {
            name: mod_info["name"].as_str().map(|s| s.to_string()),
            version: mod_info["version"].as_str().map(|s| s.to_string()),
            authors: mod_info["authorList"].as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            }),
            description: mod_info["description"].as_str().map(|s| s.to_string()),
            dependencies: mod_info["dependencies"].as_array().map(|deps| {
                deps.iter()
                    .filter_map(|dep| {
                        let id = dep.as_str()?;
                        Some(Dependency {
                            mod_id: id.to_string(),
                            version_requirement: "*".to_string(),
                            dependency_type: DependencyType::Required,
                        })
                    })
                    .collect()
            }),
            minecraft_versions: mod_info["mcversion"].as_str().map(|v| vec![v.to_string()]),
            mod_loaders: Some(vec!["forge".to_string()]),
        };

        Ok(metadata)
    }
}

#[derive(Debug)]
struct ModMetadata {
    name: Option<String>,
    version: Option<String>,
    authors: Option<Vec<String>>,
    description: Option<String>,
    dependencies: Option<Vec<Dependency>>,
    minecraft_versions: Option<Vec<String>>,
    mod_loaders: Option<Vec<String>>,
}

impl Drop for MetadataExtractor {
    fn drop(&mut self) {
        for sender in &self.shutdown_senders {
            let _ = sender.send(());
        }
    }
}
