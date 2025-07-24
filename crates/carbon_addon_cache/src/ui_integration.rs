use crate::events::*;
use crate::notifier::{InstanceProgress, ProgressNotifier, ProgressUpdate};
use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

// UI Progress data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UICacheProgress {
    pub instance_id: String,
    pub status: UICacheStatus,
    pub stages: Vec<UIStageProgress>,
    pub overall_progress: f32,
    pub estimated_time_remaining: Option<u64>, // seconds
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIStageProgress {
    pub stage: UICacheStage,
    pub current: usize,
    pub total: usize,
    pub completed: bool,
    pub error: Option<String>,
    pub stage_name: String,
    pub stage_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UICacheStatus {
    Idle,
    Initializing,
    Caching {
        current_stage: UICacheStage,
        current: usize,
        total: usize,
    },
    Paused,
    Complete,
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UICacheStage {
    FileScanning,
    MetadataExtraction,
    ImageProcessing,
    OnlineDataFetch,
    UpdateCheck,
}

impl From<CacheStage> for UICacheStage {
    fn from(stage: CacheStage) -> Self {
        match stage {
            CacheStage::FileCache => UICacheStage::FileScanning,
            CacheStage::MetadataExtraction => UICacheStage::MetadataExtraction,
            CacheStage::ImageCache => UICacheStage::ImageProcessing,
            CacheStage::ModplatformData => UICacheStage::OnlineDataFetch,
            CacheStage::Updates => UICacheStage::UpdateCheck,
        }
    }
}

impl UICacheStage {
    pub fn display_name(&self) -> &'static str {
        match self {
            UICacheStage::FileScanning => "Scanning Files",
            UICacheStage::MetadataExtraction => "Reading Metadata",
            UICacheStage::ImageProcessing => "Processing Images",
            UICacheStage::OnlineDataFetch => "Fetching Online Data",
            UICacheStage::UpdateCheck => "Checking for Updates",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            UICacheStage::FileScanning => "Discovering and cataloging addon files",
            UICacheStage::MetadataExtraction => "Extracting mod information and calculating hashes",
            UICacheStage::ImageProcessing => "Extracting and optimizing addon images",
            UICacheStage::OnlineDataFetch => "Fetching project data from CurseForge and Modrinth",
            UICacheStage::UpdateCheck => "Checking for available updates",
        }
    }
}

// UI Event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UIEvent {
    ProgressUpdate(UICacheProgress),
    CacheComplete(String), // instance_id
    CacheError {
        instance_id: String,
        error: String,
    },
    UpdateAvailable {
        addon_id: String,
        latest_version: String,
    },
    GlobalStatus(GlobalCacheStatus),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalCacheStatus {
    pub active_instances: usize,
    pub queued_instances: usize,
    pub completed_instances: usize,
    pub failed_instances: usize,
    pub is_online: bool,
    pub total_addons_cached: usize,
}

// UI Integration Manager
pub struct UIIntegrationManager {
    progress_notifier: ProgressNotifier,
    ui_event_sender: broadcast::Sender<UIEvent>,
    ui_event_receiver: broadcast::Receiver<UIEvent>,
    instance_progress: Arc<RwLock<HashMap<String, UICacheProgress>>>,
    global_status: Arc<RwLock<GlobalCacheStatus>>,
    worker_handle: Option<JoinHandle<()>>,
}

impl UIIntegrationManager {
    pub fn new() -> Self {
        let (ui_event_sender, ui_event_receiver) = broadcast::channel(1000);

        Self {
            progress_notifier: ProgressNotifier::new(),
            ui_event_sender,
            ui_event_receiver,
            instance_progress: Arc::new(RwLock::new(HashMap::new())),
            global_status: Arc::new(RwLock::new(GlobalCacheStatus {
                active_instances: 0,
                queued_instances: 0,
                completed_instances: 0,
                failed_instances: 0,
                is_online: true,
                total_addons_cached: 0,
            })),
            worker_handle: None,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting UI integration manager");

        // Start progress notifier
        self.progress_notifier.start().await;

        // Start UI event processing worker
        let worker_handle = self.start_ui_worker().await?;
        self.worker_handle = Some(worker_handle);

        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down UI integration manager");

        if let Some(handle) = self.worker_handle.take() {
            handle.abort();

            // Wait for the handle to finish with timeout
            let shutdown_timeout = std::time::Duration::from_secs(3);
            let result = tokio::time::timeout(shutdown_timeout, handle).await;
            match result {
                Ok(Ok(())) => {
                    debug!("UI worker finished cleanly");
                }
                Ok(Err(e)) if e.is_cancelled() => {
                    debug!("UI worker was cancelled");
                }
                Ok(Err(e)) => {
                    error!("UI worker panicked: {}", e);
                }
                Err(_) => {
                    error!("UI worker shutdown timed out");
                }
            }
        }

        self.progress_notifier.shutdown().await;

        info!("UI integration manager shutdown complete");
        Ok(())
    }

    // Subscribe to UI events
    pub fn subscribe_to_ui_events(&self) -> broadcast::Receiver<UIEvent> {
        self.ui_event_sender.subscribe()
    }

    // Subscribe to progress updates for a specific instance
    pub fn subscribe_to_instance_progress(
        &self,
        instance_id: String,
    ) -> broadcast::Receiver<UICacheProgress> {
        let (sender, receiver) = broadcast::channel(100);

        // Forward progress updates for this instance
        let instance_progress = self.instance_progress.clone();
        let ui_event_sender = self.ui_event_sender.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));

            loop {
                interval.tick().await;

                if let Some(progress) = instance_progress.read().get(&instance_id) {
                    if sender.send(progress.clone()).is_err() {
                        break; // No more receivers
                    }

                    // Also send to main UI event stream
                    let _ = ui_event_sender.send(UIEvent::ProgressUpdate(progress.clone()));
                }
            }
        });

        receiver
    }

    // Update progress for an instance
    pub fn update_instance_progress(&self, instance_id: &str, update: ProgressUpdate) {
        let ui_progress = self.convert_to_ui_progress(instance_id, &update);

        // Update local cache
        self.instance_progress
            .write()
            .insert(instance_id.to_string(), ui_progress.clone());

        // Send UI event
        let _ = self
            .ui_event_sender
            .send(UIEvent::ProgressUpdate(ui_progress));

        // Update global status
        self.update_global_status();
    }

    // Mark instance as complete
    pub fn mark_instance_complete(&self, instance_id: &str) {
        // Update instance status
        if let Some(mut progress) = self.instance_progress.write().get_mut(instance_id) {
            progress.status = UICacheStatus::Complete;
            progress.overall_progress = 1.0;
        }

        // Send completion event
        let _ = self
            .ui_event_sender
            .send(UIEvent::CacheComplete(instance_id.to_string()));

        // Update global status
        self.update_global_status();
    }

    // Mark instance as failed
    pub fn mark_instance_failed(&self, instance_id: &str, error: &str) {
        // Update instance status
        if let Some(mut progress) = self.instance_progress.write().get_mut(instance_id) {
            progress.status = UICacheStatus::Error {
                message: error.to_string(),
            };
        }

        // Send error event
        let _ = self.ui_event_sender.send(UIEvent::CacheError {
            instance_id: instance_id.to_string(),
            error: error.to_string(),
        });

        // Update global status
        self.update_global_status();
    }

    // Notify about available update
    pub fn notify_update_available(&self, addon_id: &str, latest_version: &str) {
        let _ = self.ui_event_sender.send(UIEvent::UpdateAvailable {
            addon_id: addon_id.to_string(),
            latest_version: latest_version.to_string(),
        });
    }

    // Set online status
    pub fn set_online_status(&self, online: bool) {
        self.global_status.write().is_online = online;
        self.send_global_status_update();
    }

    // Get current progress for an instance
    pub fn get_instance_progress(&self, instance_id: &str) -> Option<UICacheProgress> {
        self.instance_progress.read().get(instance_id).cloned()
    }

    // Get global cache status
    pub fn get_global_status(&self) -> GlobalCacheStatus {
        self.global_status.read().clone()
    }

    // Get all active instance progresses
    pub fn get_all_instance_progress(&self) -> HashMap<String, UICacheProgress> {
        self.instance_progress.read().clone()
    }

    async fn start_ui_worker(&self) -> Result<JoinHandle<()>> {
        let instance_progress = self.instance_progress.clone();
        let ui_event_sender = self.ui_event_sender.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));

            loop {
                interval.tick().await;

                // Clean up completed instances after some time
                let mut to_remove = Vec::new();
                {
                    let progress_map = instance_progress.read();
                    for (instance_id, progress) in progress_map.iter() {
                        if matches!(
                            progress.status,
                            UICacheStatus::Complete | UICacheStatus::Error { .. }
                        ) {
                            // Remove after 5 minutes
                            if progress.last_updated + 300 < chrono::Utc::now().timestamp() as u64 {
                                to_remove.push(instance_id.clone());
                            }
                        }
                    }
                }

                if !to_remove.is_empty() {
                    let mut progress_map = instance_progress.write();
                    for instance_id in to_remove {
                        progress_map.remove(&instance_id);
                        debug!("Cleaned up completed instance progress: {}", instance_id);
                    }
                }
            }
        });

        Ok(handle)
    }

    fn convert_to_ui_progress(
        &self,
        instance_id: &str,
        update: &ProgressUpdate,
    ) -> UICacheProgress {
        let existing = self.instance_progress.read().get(instance_id).cloned();

        let mut stages = existing
            .as_ref()
            .map(|p| p.stages.clone())
            .unwrap_or_default();
        let ui_stage = UICacheStage::from(update.stage.clone());

        // Update or add stage progress
        if let Some(existing_stage) = stages
            .iter_mut()
            .find(|s| matches!(s.stage, ref st if st == &ui_stage))
        {
            existing_stage.current = update.current;
            existing_stage.total = update.total;
            existing_stage.completed = update.completed;
            existing_stage.error = update.error.clone();
        } else {
            stages.push(UIStageProgress {
                stage: ui_stage.clone(),
                current: update.current,
                total: update.total,
                completed: update.completed,
                error: update.error.clone(),
                stage_name: ui_stage.display_name().to_string(),
                stage_description: ui_stage.description().to_string(),
            });
        }

        // Calculate overall progress
        let total_stages = 5.0;
        let completed_stages = stages.iter().filter(|s| s.completed).count() as f32;
        let partial_progress: f32 = stages
            .iter()
            .map(|s| {
                if s.total > 0 {
                    s.current as f32 / s.total as f32
                } else {
                    0.0
                }
            })
            .sum();
        let overall_progress = (completed_stages + partial_progress) / total_stages;

        // Determine status
        let status = if overall_progress >= 1.0 {
            UICacheStatus::Complete
        } else if update.error.is_some() {
            UICacheStatus::Error {
                message: update.error.clone().unwrap(),
            }
        } else if update.current > 0 || update.total > 0 {
            UICacheStatus::Caching {
                current_stage: ui_stage,
                current: update.current,
                total: update.total,
            }
        } else {
            UICacheStatus::Idle
        };

        UICacheProgress {
            instance_id: instance_id.to_string(),
            status,
            stages,
            overall_progress,
            estimated_time_remaining: self.estimate_time_remaining(overall_progress),
            last_updated: chrono::Utc::now().timestamp() as u64,
        }
    }

    fn estimate_time_remaining(&self, progress: f32) -> Option<u64> {
        if progress <= 0.0 || progress >= 1.0 {
            return None;
        }

        // Simple estimation based on progress rate
        // In a real implementation, this would track historical progress rates
        let remaining_work = 1.0 - progress;
        let estimated_rate = 0.01; // 1% per second (rough estimate)
        Some((remaining_work / estimated_rate) as u64)
    }

    fn update_global_status(&self) {
        let progress_map = self.instance_progress.read();
        let mut global_status = self.global_status.write();

        let mut active = 0;
        let mut completed = 0;
        let mut failed = 0;

        for progress in progress_map.values() {
            match &progress.status {
                UICacheStatus::Caching { .. } | UICacheStatus::Initializing => active += 1,
                UICacheStatus::Complete => completed += 1,
                UICacheStatus::Error { .. } => failed += 1,
                _ => {}
            }
        }

        global_status.active_instances = active;
        global_status.completed_instances = completed;
        global_status.failed_instances = failed;

        drop(global_status);
        drop(progress_map);

        self.send_global_status_update();
    }

    fn send_global_status_update(&self) {
        let status = self.global_status.read().clone();
        let _ = self.ui_event_sender.send(UIEvent::GlobalStatus(status));
    }
}

impl Default for UIIntegrationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for UIIntegrationManager {
    fn drop(&mut self) {
        if let Some(handle) = self.worker_handle.take() {
            handle.abort();
        }
    }
}

// Helper functions for UI integration
pub mod ui_helpers {
    use super::*;

    // Format progress as percentage
    pub fn format_progress_percentage(progress: f32) -> String {
        format!("{:.1}%", progress * 100.0)
    }

    // Format estimated time remaining
    pub fn format_time_remaining(seconds: Option<u64>) -> String {
        match seconds {
            Some(secs) if secs > 3600 => format!("{}h {}m", secs / 3600, (secs % 3600) / 60),
            Some(secs) if secs > 60 => format!("{}m {}s", secs / 60, secs % 60),
            Some(secs) => format!("{}s", secs),
            None => "Unknown".to_string(),
        }
    }

    // Get stage progress bar data
    pub fn get_stage_progress_bar(stage: &UIStageProgress) -> (f32, String) {
        let progress = if stage.total > 0 {
            stage.current as f32 / stage.total as f32
        } else {
            0.0
        };

        let text = if stage.completed {
            "Complete".to_string()
        } else if stage.total > 0 {
            format!("{}/{}", stage.current, stage.total)
        } else {
            "In Progress...".to_string()
        };

        (progress, text)
    }

    // Get status color for UI
    pub fn get_status_color(status: &UICacheStatus) -> &'static str {
        match status {
            UICacheStatus::Idle => "gray",
            UICacheStatus::Initializing => "blue",
            UICacheStatus::Caching { .. } => "blue",
            UICacheStatus::Paused => "yellow",
            UICacheStatus::Complete => "green",
            UICacheStatus::Error { .. } => "red",
        }
    }
}
