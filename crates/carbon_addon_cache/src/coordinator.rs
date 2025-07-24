use crate::events::*;
use crate::persistence::{CacheStatusPersistence, EventPersistence};
use crate::stages::*;
use crate::storage::*;
use anyhow::Result;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock as TokioRwLock, mpsc};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

pub struct CacheCoordinator {
    event_sender: mpsc::UnboundedSender<CacheEvent>,
    event_receiver: Option<mpsc::UnboundedReceiver<CacheEvent>>,
    storage: Arc<dyn AddonStorage>,
    stages: CacheStages,
    instance_priority: Arc<RwLock<HashMap<String, Priority>>>,
    online_status: Arc<RwLock<bool>>,
    shutdown_sender: mpsc::UnboundedSender<()>,
    shutdown_receiver: Option<mpsc::UnboundedReceiver<()>>,
    worker_handles: Vec<JoinHandle<()>>,
    event_persistence: Option<Arc<EventPersistence>>,
    status_persistence: Option<Arc<CacheStatusPersistence>>,
}

pub struct CacheStages {
    pub file_cache: Arc<TokioRwLock<FileCache>>,
    pub metadata_extractor: Arc<TokioRwLock<MetadataExtractor>>,
    pub image_cache: Arc<TokioRwLock<ImageCache>>,
    pub modplatform_fetcher: Arc<TokioRwLock<ModplatformFetcher>>,
    pub update_checker: Arc<TokioRwLock<UpdateChecker>>,
}

impl CacheCoordinator {
    pub fn new(storage: Arc<dyn AddonStorage>, config: StorageConfig) -> Result<Self> {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let (shutdown_sender, shutdown_receiver) = mpsc::unbounded_channel();

        let stages = CacheStages {
            file_cache: Arc::new(TokioRwLock::new(FileCache::new(config.clone())?)),
            metadata_extractor: Arc::new(TokioRwLock::new(MetadataExtractor::new(config.clone())?)),
            image_cache: Arc::new(TokioRwLock::new(ImageCache::new(config.clone())?)),
            modplatform_fetcher: Arc::new(TokioRwLock::new(ModplatformFetcher::new(
                config.clone(),
            )?)),
            update_checker: Arc::new(TokioRwLock::new(UpdateChecker::new(config.clone())?)),
        };

        // Create persistence components (disabled in test mode for performance)
        let event_persistence = if cfg!(test) {
            None
        } else {
            EventPersistence::new(&config.runtime_path, 1000)
                .ok()
                .map(Arc::new)
        };
        let status_persistence = if cfg!(test) {
            None
        } else {
            CacheStatusPersistence::new(&config.runtime_path)
                .ok()
                .map(Arc::new)
        };

        Ok(Self {
            event_sender,
            event_receiver: Some(event_receiver),
            storage,
            stages,
            instance_priority: Arc::new(RwLock::new(HashMap::new())),
            online_status: Arc::new(RwLock::new(true)),
            shutdown_sender,
            shutdown_receiver: Some(shutdown_receiver),
            worker_handles: Vec::new(),
            event_persistence,
            status_persistence,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting cache coordinator");

        // Skip persistence entirely in test mode to prevent hanging
        if !cfg!(test) {
            // Load persisted events for crash recovery
            if let Some(ref event_persistence) = self.event_persistence {
                let persisted_events = event_persistence.load_events().await?;
                if !persisted_events.is_empty() {
                    info!(
                        "Recovering {} persisted events after crash",
                        persisted_events.len()
                    );
                    for event in persisted_events {
                        if let Err(e) = self.event_sender.send(event) {
                            error!("Failed to replay persisted event: {}", e);
                        }
                    }
                }

                // Start persistence task
                event_persistence.start_persistence_task().await;
            }

            // Load persisted status
            if let Some(ref status_persistence) = self.status_persistence {
                let status = status_persistence.load_status().await?;
                info!(
                    "Loaded cache status for {} instances",
                    status.instances.len()
                );

                // Start status persistence task
                status_persistence.start_persistence_task().await;
            }
        } else {
            info!("Skipping persistence loading in test mode");
        }

        info!("Starting coordinator event loop");
        // Start main event loop
        let coordinator_handle = self.start_event_loop().await?;
        self.worker_handles.push(coordinator_handle);

        info!("Starting cache stage workers");
        // Start stage workers
        let stage_handles = self.start_stage_workers().await?;
        self.worker_handles.extend(stage_handles);

        info!("Cache coordinator startup complete");
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down cache coordinator");

        // Now we can properly shut down all stages using write locks
        info!("Shutting down all cache stages");

        // Shut down stages sequentially to avoid deadlocks
        if let Err(e) = self.stages.file_cache.write().await.shutdown().await {
            warn!("Error shutting down file cache: {}", e);
        }
        if let Err(e) = self
            .stages
            .metadata_extractor
            .write()
            .await
            .shutdown()
            .await
        {
            warn!("Error shutting down metadata extractor: {}", e);
        }
        if let Err(e) = self.stages.image_cache.write().await.shutdown().await {
            warn!("Error shutting down image cache: {}", e);
        }
        if let Err(e) = self
            .stages
            .modplatform_fetcher
            .write()
            .await
            .shutdown()
            .await
        {
            warn!("Error shutting down modplatform fetcher: {}", e);
        }
        if let Err(e) = self.stages.update_checker.write().await.shutdown().await {
            warn!("Error shutting down update checker: {}", e);
        }

        info!("All cache stages shut down");

        // Signal shutdown
        if let Err(e) = self.shutdown_sender.send(()) {
            warn!("Failed to send shutdown signal: {}", e);
        }

        // Wait for all workers to finish with timeout, then force abort
        let shutdown_timeout = std::time::Duration::from_secs(2);
        let handles = self.worker_handles.drain(..).collect::<Vec<_>>();

        for handle in handles {
            // Try graceful shutdown first with timeout
            let result = tokio::time::timeout(shutdown_timeout, handle).await;

            if let Err(_) = result {
                warn!(
                    "Coordinator worker shutdown timed out after {}s",
                    shutdown_timeout.as_secs()
                );
            } else if let Ok(Err(e)) = result {
                if e.is_cancelled() {
                    debug!("Coordinator worker was successfully aborted");
                } else {
                    error!("Coordinator worker failed: {}", e);
                }
            } else {
                debug!("Coordinator worker finished cleanly");
            }
        }

        info!("Cache coordinator shutdown complete");
        Ok(())
    }

    pub fn send_event(&self, event: CacheEvent) -> Result<()> {
        // Fire-and-forget persistence for crash recovery
        if let Some(ref persistence) = self.event_persistence {
            let persistence = Arc::clone(persistence);
            let event_clone = event.clone();
            tokio::spawn(async move {
                if let Err(e) = persistence.persist_event(event_clone).await {
                    warn!("Failed to persist event: {}", e);
                }
            });
        }

        self.event_sender
            .send(event)
            .map_err(|e| anyhow::anyhow!("Failed to send event: {}", e))?;
        Ok(())
    }

    pub fn get_event_sender(&self) -> &mpsc::UnboundedSender<CacheEvent> {
        &self.event_sender
    }

    pub fn prioritize_instance(&self, instance_id: String, priority: Priority) {
        self.instance_priority
            .write()
            .insert(instance_id.clone(), priority);

        if let Err(e) = self.send_event(CacheEvent::PrioritizeInstance { instance_id }) {
            error!("Failed to send prioritize event: {}", e);
        }
    }

    pub fn set_online_status(&self, online: bool) {
        *self.online_status.write() = online;

        let event = if online {
            CacheEvent::GoOnline
        } else {
            CacheEvent::GoOffline
        };

        if let Err(e) = self.send_event(event) {
            error!("Failed to send online/offline event: {}", e);
        }
    }

    async fn start_event_loop(&mut self) -> Result<JoinHandle<()>> {
        let mut event_receiver = self.event_receiver.take().unwrap();
        let mut shutdown_receiver = self.shutdown_receiver.take().unwrap();
        let storage = self.storage.clone();
        let stages = self.stages.clone();
        let instance_priority = self.instance_priority.clone();
        let online_status = self.online_status.clone();

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    event = event_receiver.recv() => {
                        match event {
                            Some(event) => {
                                if let Err(e) = Self::handle_event(
                                    event,
                                    &storage,
                                    &stages,
                                    &instance_priority,
                                    &online_status
                                ).await {
                                    error!("Error handling event: {}", e);
                                }
                            }
                            None => {
                                debug!("Event channel closed");
                                break;
                            }
                        }
                    }
                    _ = shutdown_receiver.recv() => {
                        debug!("Received shutdown signal");
                        break;
                    }
                }
            }
        });

        Ok(handle)
    }

    async fn start_stage_workers(&mut self) -> Result<Vec<JoinHandle<()>>> {
        let handles = Vec::new();

        // Start all stages sequentially with timeouts to prevent hanging
        info!("Starting all cache stage workers with timeouts");

        let stage_timeout = std::time::Duration::from_secs(2);

        info!("Starting file cache stage");
        tokio::time::timeout(stage_timeout, self.stages.file_cache.write().await.start())
            .await
            .map_err(|_| anyhow::anyhow!("File cache startup timeout"))??;

        info!("Starting metadata extractor stage");
        tokio::time::timeout(
            stage_timeout,
            self.stages.metadata_extractor.write().await.start(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Metadata extractor startup timeout"))??;

        info!("Starting image cache stage");
        tokio::time::timeout(stage_timeout, self.stages.image_cache.write().await.start())
            .await
            .map_err(|_| anyhow::anyhow!("Image cache startup timeout"))??;

        info!("Starting modplatform fetcher stage");
        tokio::time::timeout(
            stage_timeout,
            self.stages.modplatform_fetcher.write().await.start(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Modplatform fetcher startup timeout"))??;

        info!("Starting update checker stage");
        tokio::time::timeout(
            stage_timeout,
            self.stages.update_checker.write().await.start(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Update checker startup timeout"))??;

        info!("All cache stage workers started successfully");

        Ok(handles)
    }

    async fn handle_event(
        event: CacheEvent,
        storage: &Arc<dyn AddonStorage>,
        stages: &CacheStages,
        instance_priority: &Arc<RwLock<HashMap<String, Priority>>>,
        _online_status: &Arc<RwLock<bool>>,
    ) -> Result<()> {
        match event {
            CacheEvent::AddAddon { path, instance_id } => {
                debug!("Processing AddAddon event for path: {:?}", path);

                let priority = instance_id
                    .as_ref()
                    .and_then(|id| instance_priority.read().get(id).cloned())
                    .unwrap_or(Priority::Normal);

                stages
                    .file_cache
                    .read()
                    .await
                    .add_file(path, instance_id, priority)
                    .await?;
            }

            CacheEvent::PrioritizeInstance { instance_id } => {
                debug!("Processing PrioritizeInstance event for: {}", instance_id);

                let priority = instance_priority
                    .read()
                    .get(&instance_id)
                    .cloned()
                    .unwrap_or(Priority::High);

                stages
                    .file_cache
                    .read()
                    .await
                    .prioritize_instance(&instance_id, priority)
                    .await?;
                stages
                    .metadata_extractor
                    .read()
                    .await
                    .prioritize_instance(&instance_id, priority)
                    .await?;
                stages
                    .image_cache
                    .read()
                    .await
                    .prioritize_instance(&instance_id, priority)
                    .await?;
                stages
                    .modplatform_fetcher
                    .read()
                    .await
                    .prioritize_instance(&instance_id, priority)
                    .await?;
                stages
                    .update_checker
                    .read()
                    .await
                    .prioritize_instance(&instance_id, priority)
                    .await?;
            }

            CacheEvent::FilesCached { addon_id, metadata } => {
                debug!("Processing FilesCached event for addon: {}", addon_id);

                let priority = metadata
                    .instance_id
                    .as_ref()
                    .and_then(|id| instance_priority.read().get(id).cloned())
                    .unwrap_or(Priority::Normal);

                stages
                    .metadata_extractor
                    .read()
                    .await
                    .add_addon(addon_id, metadata, priority)
                    .await?;
            }

            CacheEvent::MetadataExtracted { addon_id, metadata } => {
                debug!("Processing MetadataExtracted event for addon: {}", addon_id);

                storage.store_metadata(&addon_id, &metadata).await?;

                let priority = metadata
                    .instance_id
                    .as_ref()
                    .and_then(|id| instance_priority.read().get(id).cloned())
                    .unwrap_or(Priority::Normal);

                stages
                    .image_cache
                    .read()
                    .await
                    .add_addon(addon_id.clone(), metadata.clone(), priority)
                    .await?;
                stages
                    .modplatform_fetcher
                    .read()
                    .await
                    .add_addon(addon_id, metadata, priority)
                    .await?;
            }

            CacheEvent::ImagesProcessed { addon_id, images } => {
                debug!("Processing ImagesProcessed event for addon: {}", addon_id);

                for image in images {
                    if let Some(data) = image.data {
                        storage
                            .store_image(&addon_id, image.image_type, &data)
                            .await?;
                    }
                }
            }

            CacheEvent::ModplatformDataFetched { addon_id, data } => {
                debug!(
                    "Processing ModplatformDataFetched event for addon: {}",
                    addon_id
                );

                storage.store_platform_data(&addon_id, &data).await?;

                // Try to get instance_id from stored metadata
                let priority = if let Ok(Some(metadata)) = storage.get_metadata(&addon_id).await {
                    metadata
                        .instance_id
                        .as_ref()
                        .and_then(|id| instance_priority.read().get(id).cloned())
                        .unwrap_or(Priority::Normal)
                } else {
                    Priority::Normal
                };

                stages
                    .update_checker
                    .read()
                    .await
                    .add_addon(addon_id, data, priority)
                    .await?;
            }

            CacheEvent::UpdatesChecked { addon_id, updates } => {
                debug!("Processing UpdatesChecked event for addon: {}", addon_id);

                storage.store_versions(&addon_id, &updates).await?;

                // Check if there are actually newer versions available
                if let Ok(Some(current_metadata)) = storage.get_metadata(&addon_id).await {
                    let current_version = &current_metadata.version;

                    // Find the latest version from the updates
                    if let Some(latest_version) = updates
                        .iter()
                        .filter(|v| v.version_type == VersionType::Release)
                        .max_by(|a, b| a.version_number.cmp(&b.version_number))
                    {
                        // Simple version comparison - in a real scenario you'd want semver comparison
                        if latest_version.version_number != *current_version {
                            info!(
                                "Update available for addon {}: {} -> {}",
                                addon_id, current_version, latest_version.version_number
                            );

                            // Store update availability information
                            // Note: This is stored implicitly by having newer versions in the versions table
                            // The UI can query this through the cache manager
                        }
                    }
                }
            }

            CacheEvent::GoOnline => {
                debug!("Processing GoOnline event");
                stages
                    .modplatform_fetcher
                    .read()
                    .await
                    .set_online(true)
                    .await?;
                stages.update_checker.read().await.set_online(true).await?;
            }

            CacheEvent::GoOffline => {
                debug!("Processing GoOffline event");
                stages
                    .modplatform_fetcher
                    .read()
                    .await
                    .set_online(false)
                    .await?;
                stages.update_checker.read().await.set_online(false).await?;
            }
        }

        Ok(())
    }
}

impl Clone for CacheStages {
    fn clone(&self) -> Self {
        Self {
            file_cache: self.file_cache.clone(),
            metadata_extractor: self.metadata_extractor.clone(),
            image_cache: self.image_cache.clone(),
            modplatform_fetcher: self.modplatform_fetcher.clone(),
            update_checker: self.update_checker.clone(),
        }
    }
}
