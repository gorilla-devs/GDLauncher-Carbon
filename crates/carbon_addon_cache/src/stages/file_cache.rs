use crate::events::*;
use crate::scheduler::CacheScheduler;
use crate::storage::StorageConfig;
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, bounded};
use parking_lot::RwLock;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[derive(Debug)]
struct CacheTask {
    addon_id: String,
    path: PathBuf,
    instance_id: Option<String>,
    priority: Priority,
    created_at: u64,
}

impl PartialEq for CacheTask {
    fn eq(&self, other: &Self) -> bool {
        self.addon_id == other.addon_id
    }
}

impl Eq for CacheTask {}

impl PartialOrd for CacheTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CacheTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then older tasks first
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.created_at.cmp(&other.created_at))
    }
}

pub struct FileCache {
    task_queue: Arc<RwLock<BinaryHeap<CacheTask>>>,
    instance_priorities: Arc<RwLock<HashMap<String, Priority>>>,
    event_sender: Sender<CacheEvent>,
    event_receiver: Receiver<CacheEvent>,
    config: StorageConfig,
    worker_handles: Vec<JoinHandle<()>>,
    shutdown_senders: Vec<mpsc::UnboundedSender<()>>,
}

impl FileCache {
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
        info!("Starting file cache stage");

        // Start worker threads - limit to reasonable number for caching
        let worker_count = (num_cpus::get() / 4).max(1).min(4);
        for i in 0..worker_count {
            let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();
            let worker = self.spawn_worker(i, shutdown_rx).await?;
            self.worker_handles.push(worker);
            self.shutdown_senders.push(shutdown_tx);
        }

        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down file cache stage");

        // Signal shutdown to all workers
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
                    "File cache worker shutdown timed out after {}s",
                    shutdown_timeout.as_secs()
                );
            } else if let Ok(Err(e)) = result {
                if e.is_cancelled() {
                    debug!("File cache worker was successfully aborted");
                } else {
                    error!("File cache worker failed: {}", e);
                }
            } else {
                debug!("File cache worker finished cleanly");
            }
        }

        Ok(())
    }

    pub async fn add_file(
        &self,
        path: PathBuf,
        instance_id: Option<String>,
        priority: Priority,
    ) -> Result<()> {
        // Validate file exists before adding to queue
        if !path.exists() {
            anyhow::bail!("File does not exist: {:?}", path);
        }

        let addon_id = Uuid::new_v4().to_string();
        let created_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        let task = CacheTask {
            addon_id,
            path,
            instance_id,
            priority,
            created_at,
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
        let mut tasks: Vec<CacheTask> = queue.drain().collect();

        for task in &mut tasks {
            if let Some(ref task_instance_id) = task.instance_id {
                if task_instance_id == instance_id {
                    task.priority = priority;
                }
            }
        }

        // Rebuild heap with updated priorities
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

        let handle = tokio::spawn(async move {
            debug!("File cache worker {} started", worker_id);

            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = shutdown_receiver.recv() => {
                        debug!("File cache worker {} received shutdown signal", worker_id);
                        break;
                    }
                    // Process tasks
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(10)) => {
                        // Get next task
                        let task = {
                            let mut queue = task_queue.write();
                            queue.pop()
                        };

                        if let Some(task) = task {
                            if let Err(e) = Self::process_file_task(task, &event_sender).await {
                                error!("Error processing file task: {}", e);
                            }
                        }
                    }
                }
            }

            debug!("File cache worker {} finished", worker_id);
        });

        Ok(handle)
    }

    async fn process_file_task(task: CacheTask, event_sender: &Sender<CacheEvent>) -> Result<()> {
        debug!("Processing file task for path: {:?}", task.path);

        // Use carbon_scheduler for CPU-intensive file operations
        let file_info = CacheScheduler::cpu_intensive({
            let path = task.path.clone();
            move || -> Result<(u64, u64, AddonType)> {
                // Check if file exists
                if !path.exists() {
                    anyhow::bail!("File does not exist: {:?}", path);
                }

                // Get file metadata
                let metadata = std::fs::metadata(&path)?;
                let file_size = metadata.len();
                let modified_time = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();

                // Determine addon type
                let addon_type = Self::determine_addon_type(&path);

                Ok((file_size, modified_time, addon_type))
            }
        })
        .await??;

        let (file_size, modified_time, addon_type) = file_info;

        // Create basic metadata
        let basic_metadata = BasicMetadata {
            addon_id: task.addon_id.clone(),
            file_path: task.path.clone(),
            file_size,
            modified_time,
            addon_type,
            instance_id: task.instance_id.clone(),
        };

        // Emit FilesCached event
        let event = CacheEvent::FilesCached {
            addon_id: task.addon_id,
            metadata: basic_metadata,
        };

        event_sender.send(event)?;

        Ok(())
    }

    fn determine_addon_type(path: &PathBuf) -> AddonType {
        let path_str = path.to_string_lossy().to_lowercase();

        if path_str.contains("mods") && path_str.ends_with(".jar") {
            AddonType::Mod
        } else if path_str.contains("resourcepacks") || path_str.contains("resource_packs") {
            AddonType::ResourcePack
        } else if path_str.contains("datapacks") || path_str.contains("data_packs") {
            AddonType::DataPack
        } else if path_str.contains("shaderpacks") || path_str.contains("shaders") {
            AddonType::ShaderPack
        } else {
            AddonType::Unknown
        }
    }
}

impl Drop for FileCache {
    fn drop(&mut self) {
        // Best effort cleanup - signal all workers to shut down
        for sender in &self.shutdown_senders {
            let _ = sender.send(());
        }
    }
}
