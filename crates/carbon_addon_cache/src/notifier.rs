use crate::events::*;
use crossbeam_channel::{Receiver, Sender, bounded};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tracing::{debug, error};

pub struct ProgressNotifier {
    update_sender: Sender<ProgressUpdate>,
    update_receiver: Receiver<ProgressUpdate>,
    subscribers: Arc<RwLock<HashMap<String, Sender<InstanceProgress>>>>,
    instance_progress: Arc<RwLock<HashMap<String, InstanceProgress>>>,
    last_batch_time: Arc<RwLock<Instant>>,
    batch_interval: Duration,
    worker_handle: Option<JoinHandle<()>>,
    shutdown_sender: Option<oneshot::Sender<()>>,
}

#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub instance_id: String,
    pub stage: CacheStage,
    pub current: usize,
    pub total: usize,
    pub completed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InstanceProgress {
    pub instance_id: String,
    pub status: InstanceCacheStatus,
    pub stages: HashMap<CacheStage, StageProgress>,
    pub overall_progress: f32,
    pub last_updated: Instant,
}

#[derive(Debug, Clone)]
pub struct StageProgress {
    pub stage: CacheStage,
    pub current: usize,
    pub total: usize,
    pub completed: bool,
    pub error: Option<String>,
}

impl ProgressNotifier {
    pub fn new() -> Self {
        let (update_sender, update_receiver) = bounded(1000);

        Self {
            update_sender,
            update_receiver,
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            instance_progress: Arc::new(RwLock::new(HashMap::new())),
            last_batch_time: Arc::new(RwLock::new(Instant::now())),
            batch_interval: Duration::from_millis(100),
            worker_handle: None,
            shutdown_sender: None,
        }
    }

    pub async fn start(&mut self) {
        let update_receiver = self.update_receiver.clone();
        let subscribers = self.subscribers.clone();
        let instance_progress = self.instance_progress.clone();
        let last_batch_time = self.last_batch_time.clone();
        let batch_interval = self.batch_interval;

        let (shutdown_sender, shutdown_receiver) = oneshot::channel();
        self.shutdown_sender = Some(shutdown_sender);

        self.worker_handle = Some(tokio::spawn(async move {
            Self::run_notification_loop(
                update_receiver,
                subscribers,
                instance_progress,
                last_batch_time,
                batch_interval,
                shutdown_receiver,
            )
            .await;
        }));
    }

    pub async fn shutdown(&mut self) {
        // Send shutdown signal first
        if let Some(sender) = self.shutdown_sender.take() {
            let _ = sender.send(());
        }

        // Then wait for the worker to finish
        if let Some(handle) = self.worker_handle.take() {
            // Give it a chance to shutdown gracefully
            match tokio::time::timeout(Duration::from_secs(2), handle).await {
                Ok(Ok(())) => debug!("ProgressNotifier worker shut down gracefully"),
                Ok(Err(e)) => error!("ProgressNotifier worker panicked: {:?}", e),
                Err(_) => {
                    error!("ProgressNotifier worker shutdown timeout, aborting");
                    // Timeout expired, force abort is already done by dropping the handle
                }
            }
        }
    }

    pub fn update_progress(&self, update: ProgressUpdate) {
        if let Err(e) = self.update_sender.send(update) {
            error!("Failed to send progress update: {}", e);
        }
    }

    pub fn subscribe(&self, instance_id: String) -> Receiver<InstanceProgress> {
        let (sender, receiver) = bounded(100);
        self.subscribers.write().insert(instance_id, sender);
        receiver
    }

    pub fn unsubscribe(&self, instance_id: &str) {
        self.subscribers.write().remove(instance_id);
    }

    pub fn get_instance_progress(&self, instance_id: &str) -> Option<InstanceProgress> {
        self.instance_progress.read().get(instance_id).cloned()
    }

    pub fn list_active_instances(&self) -> Vec<String> {
        self.instance_progress.read().keys().cloned().collect()
    }

    async fn run_notification_loop(
        update_receiver: Receiver<ProgressUpdate>,
        subscribers: Arc<RwLock<HashMap<String, Sender<InstanceProgress>>>>,
        instance_progress: Arc<RwLock<HashMap<String, InstanceProgress>>>,
        last_batch_time: Arc<RwLock<Instant>>,
        batch_interval: Duration,
        mut shutdown_receiver: oneshot::Receiver<()>,
    ) {
        let mut pending_updates = Vec::new();

        loop {
            // Collect updates with timeout
            let timeout = {
                let last_batch = *last_batch_time.read();
                let elapsed = last_batch.elapsed();
                if elapsed >= batch_interval {
                    Duration::from_millis(0)
                } else {
                    batch_interval - elapsed
                }
            };

            // Use tokio::select to check both update channel and shutdown signal
            tokio::select! {
                // Check for shutdown signal
                _ = &mut shutdown_receiver => {
                    debug!("ProgressNotifier received shutdown signal");
                    // Process any remaining updates before shutting down
                    if !pending_updates.is_empty() {
                        Self::process_update_batch(&mut pending_updates, &subscribers, &instance_progress);
                    }
                    break;
                }
                // Wait for timeout to process batch
                _ = tokio::time::sleep(timeout) => {
                    // Check for any pending updates in the channel
                    while let Ok(update) = update_receiver.try_recv() {
                        pending_updates.push(update);
                    }

                    // Process batch if we have updates or timeout reached
                    if !pending_updates.is_empty() || last_batch_time.read().elapsed() >= batch_interval {
                        Self::process_update_batch(&mut pending_updates, &subscribers, &instance_progress);
                        *last_batch_time.write() = Instant::now();
                    }
                }
                // Also poll for updates continuously
                _ = async {
                    // This future completes when we receive an update
                    loop {
                        match update_receiver.try_recv() {
                            Ok(update) => {
                                pending_updates.push(update);
                                // Continue collecting updates
                                while let Ok(update) = update_receiver.try_recv() {
                                    pending_updates.push(update);
                                }
                                break;
                            }
                            Err(_) => {
                                // No updates available, sleep briefly and retry
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            }
                        }
                    }
                } => {
                    // We got updates, continue the loop to either process them or collect more
                }
            }
        }
    }

    fn process_update_batch(
        pending_updates: &mut Vec<ProgressUpdate>,
        subscribers: &Arc<RwLock<HashMap<String, Sender<InstanceProgress>>>>,
        instance_progress: &Arc<RwLock<HashMap<String, InstanceProgress>>>,
    ) {
        if pending_updates.is_empty() {
            return;
        }

        debug!("Processing {} progress updates", pending_updates.len());

        // Group updates by instance
        let mut instance_updates: HashMap<String, Vec<ProgressUpdate>> = HashMap::new();
        for update in pending_updates.drain(..) {
            instance_updates
                .entry(update.instance_id.clone())
                .or_insert_with(Vec::new)
                .push(update);
        }

        // Process each instance's updates
        for (instance_id, updates) in instance_updates {
            if let Some(progress) =
                Self::update_instance_progress(&instance_id, updates, instance_progress)
            {
                // Notify subscribers
                if let Some(sender) = subscribers.read().get(&instance_id) {
                    if let Err(e) = sender.try_send(progress) {
                        error!(
                            "Failed to send progress notification for instance {}: {}",
                            instance_id, e
                        );
                    }
                }
            }
        }
    }

    fn update_instance_progress(
        instance_id: &str,
        updates: Vec<ProgressUpdate>,
        instance_progress: &Arc<RwLock<HashMap<String, InstanceProgress>>>,
    ) -> Option<InstanceProgress> {
        let mut progress_map = instance_progress.write();

        let progress = progress_map
            .entry(instance_id.to_string())
            .or_insert_with(|| InstanceProgress {
                instance_id: instance_id.to_string(),
                status: InstanceCacheStatus::Idle,
                stages: HashMap::new(),
                overall_progress: 0.0,
                last_updated: Instant::now(),
            });

        // Apply all updates
        for update in updates {
            let stage_progress = StageProgress {
                stage: update.stage.clone(),
                current: update.current,
                total: update.total,
                completed: update.completed,
                error: update.error,
            };

            progress.stages.insert(update.stage, stage_progress);
        }

        // Calculate overall progress
        let total_stages = 5; // FileCache, MetadataExtraction, ImageCache, ModplatformData, Updates
        let completed_stages = progress.stages.values().filter(|s| s.completed).count();

        let stage_progress: f32 = progress
            .stages
            .values()
            .map(|s| {
                if s.total > 0 {
                    s.current as f32 / s.total as f32
                } else {
                    0.0
                }
            })
            .sum();

        progress.overall_progress =
            (completed_stages as f32 + stage_progress) / total_stages as f32;

        // Update status
        progress.status = if progress.overall_progress >= 1.0 {
            InstanceCacheStatus::Complete
        } else if progress.stages.is_empty() {
            InstanceCacheStatus::Idle
        } else {
            // Find the current active stage
            let current_stage = progress
                .stages
                .iter()
                .find(|(_, s)| !s.completed && s.current > 0)
                .map(|(stage, s)| (stage.clone(), s.current, s.total))
                .unwrap_or((CacheStage::FileCache, 0, 0));

            InstanceCacheStatus::Caching {
                stage: current_stage.0,
                current: current_stage.1,
                total: current_stage.2,
            }
        };

        progress.last_updated = Instant::now();

        Some(progress.clone())
    }
}

impl Default for ProgressNotifier {
    fn default() -> Self {
        Self::new()
    }
}
