use crate::events::CacheEvent;
use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::time::{Duration, interval};
use tracing::{debug, error, info, warn};

/// Event persistence for crash recovery
/// Stores unprocessed events to disk for recovery after crashes
pub struct EventPersistence {
    persistence_path: PathBuf,
    event_queue: Arc<RwLock<VecDeque<PersistedEvent>>>,
    max_events: usize,
    flush_interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedEvent {
    event: CacheEvent,
    timestamp: chrono::DateTime<chrono::Utc>,
    retry_count: u32,
}

impl EventPersistence {
    pub fn new(runtime_path: &Path, max_events: usize) -> Result<Self> {
        let persistence_path = runtime_path.join("cache_events.json");

        // Create parent directory if needed
        if let Some(parent) = persistence_path.parent() {
            fs::create_dir_all(parent)?;
        }

        Ok(Self {
            persistence_path,
            event_queue: Arc::new(RwLock::new(VecDeque::new())),
            max_events,
            flush_interval: Duration::from_secs(5),
        })
    }

    /// Load persisted events from disk on startup
    pub async fn load_events(&self) -> Result<Vec<CacheEvent>> {
        if !self.persistence_path.exists() {
            debug!("No persisted events found");
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.persistence_path)?;
        let persisted: Vec<PersistedEvent> = serde_json::from_str(&content)?;

        info!("Loaded {} persisted events from disk", persisted.len());

        // Filter out events that have been retried too many times
        let events: Vec<CacheEvent> = persisted
            .into_iter()
            .filter(|e| e.retry_count < 3)
            .map(|e| e.event)
            .collect();

        // Clear the persistence file after loading
        self.clear_persistence_file()?;

        Ok(events)
    }

    /// Persist an event to disk
    pub async fn persist_event(&self, event: CacheEvent) -> Result<()> {
        let persisted = PersistedEvent {
            event,
            timestamp: chrono::Utc::now(),
            retry_count: 0,
        };

        {
            let mut queue = self.event_queue.write();

            // Limit queue size
            if queue.len() >= self.max_events {
                warn!("Event persistence queue full, dropping oldest event");
                queue.pop_front();
            }

            queue.push_back(persisted);
        }

        // Don't flush immediately for performance
        Ok(())
    }

    /// Start background persistence task
    pub async fn start_persistence_task(&self) {
        let persistence_path = self.persistence_path.clone();
        let event_queue = self.event_queue.clone();
        let flush_interval = self.flush_interval;

        tokio::spawn(async move {
            let mut interval = interval(flush_interval);

            loop {
                interval.tick().await;

                let events: Vec<PersistedEvent> = {
                    let queue = event_queue.read();
                    queue.iter().cloned().collect()
                };

                if events.is_empty() {
                    continue;
                }

                // Write to disk
                match serde_json::to_string_pretty(&events) {
                    Ok(json) => {
                        if let Err(e) = fs::write(&persistence_path, json) {
                            error!("Failed to persist events: {}", e);
                        } else {
                            debug!("Persisted {} events to disk", events.len());
                        }
                    }
                    Err(e) => {
                        error!("Failed to serialize events: {}", e);
                    }
                }
            }
        });
    }

    /// Mark an event as processed and remove from persistence
    pub async fn mark_processed(&self, event: &CacheEvent) -> Result<()> {
        let mut queue = self.event_queue.write();

        // Remove the event from the queue
        queue.retain(|persisted| !Self::events_equal(&persisted.event, event));

        Ok(())
    }

    /// Clear all persisted events
    pub fn clear_persistence_file(&self) -> Result<()> {
        if self.persistence_path.exists() {
            fs::remove_file(&self.persistence_path)?;
            debug!("Cleared persistence file");
        }
        Ok(())
    }

    /// Check if two events are equal (for removal)
    fn events_equal(a: &CacheEvent, b: &CacheEvent) -> bool {
        // Simple comparison - in practice you might want more sophisticated matching
        match (a, b) {
            (
                CacheEvent::AddAddon {
                    path: p1,
                    instance_id: i1,
                },
                CacheEvent::AddAddon {
                    path: p2,
                    instance_id: i2,
                },
            ) => p1 == p2 && i1 == i2,
            (
                CacheEvent::PrioritizeInstance { instance_id: i1 },
                CacheEvent::PrioritizeInstance { instance_id: i2 },
            ) => i1 == i2,
            _ => false,
        }
    }
}

/// Cache status persistence for UI state restoration
pub struct CacheStatusPersistence {
    status_path: PathBuf,
    current_status: Arc<RwLock<CacheStatusSnapshot>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheStatusSnapshot {
    pub instances: std::collections::HashMap<String, InstanceCacheState>,
    pub global_stats: GlobalCacheStats,
    pub last_updated: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceCacheState {
    pub instance_id: String,
    pub status: String, // Serialized InstanceCacheStatus
    pub current_addon: Option<String>,
    pub progress: CacheProgress,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheProgress {
    pub current: usize,
    pub total: usize,
    pub stage: String,
    pub percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalCacheStats {
    pub total_addons_cached: u64,
    pub total_instances_processed: u64,
    pub cache_hit_rate: f64,
    pub average_cache_time_ms: u64,
}

impl CacheStatusPersistence {
    pub fn new(runtime_path: &Path) -> Result<Self> {
        let status_path = runtime_path.join("cache_status.json");

        Ok(Self {
            status_path,
            current_status: Arc::new(RwLock::new(CacheStatusSnapshot::default())),
        })
    }

    /// Load cache status from disk on startup
    pub async fn load_status(&self) -> Result<CacheStatusSnapshot> {
        if !self.status_path.exists() {
            debug!("No persisted cache status found");
            return Ok(CacheStatusSnapshot::default());
        }

        let content = fs::read_to_string(&self.status_path)?;
        let status: CacheStatusSnapshot = serde_json::from_str(&content)?;

        info!(
            "Loaded cache status for {} instances",
            status.instances.len()
        );

        // Update internal state
        *self.current_status.write() = status.clone();

        Ok(status)
    }

    /// Update instance cache state
    pub async fn update_instance_state(
        &self,
        instance_id: &str,
        status: &str,
        current_addon: Option<String>,
        progress: CacheProgress,
    ) -> Result<()> {
        let mut snapshot = self.current_status.write();

        let state = InstanceCacheState {
            instance_id: instance_id.to_string(),
            status: status.to_string(),
            current_addon,
            progress,
            last_activity: chrono::Utc::now(),
        };

        snapshot.instances.insert(instance_id.to_string(), state);
        snapshot.last_updated = Some(chrono::Utc::now());

        Ok(())
    }

    /// Update global statistics
    pub async fn update_global_stats(&self, stats: GlobalCacheStats) -> Result<()> {
        let mut snapshot = self.current_status.write();
        snapshot.global_stats = stats;
        snapshot.last_updated = Some(chrono::Utc::now());
        Ok(())
    }

    /// Start background persistence task
    pub async fn start_persistence_task(&self) {
        let status_path = self.status_path.clone();
        let current_status = self.current_status.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));

            loop {
                interval.tick().await;

                let snapshot = current_status.read().clone();

                // Write to disk
                match serde_json::to_string_pretty(&snapshot) {
                    Ok(json) => {
                        if let Err(e) = fs::write(&status_path, json) {
                            error!("Failed to persist cache status: {}", e);
                        } else {
                            debug!("Persisted cache status to disk");
                        }
                    }
                    Err(e) => {
                        error!("Failed to serialize cache status: {}", e);
                    }
                }
            }
        });
    }

    /// Clear instance state (when caching completes)
    pub async fn clear_instance_state(&self, instance_id: &str) -> Result<()> {
        let mut snapshot = self.current_status.write();
        snapshot.instances.remove(instance_id);
        Ok(())
    }

    /// Get current status snapshot
    pub fn get_current_status(&self) -> CacheStatusSnapshot {
        self.current_status.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Priority;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_event_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = EventPersistence::new(temp_dir.path(), 100).unwrap();

        // Persist an event
        let event = CacheEvent::AddAddon {
            path: PathBuf::from("/test/addon.jar"),
            instance_id: Some("test_instance".to_string()),
        };

        persistence.persist_event(event.clone()).await.unwrap();

        // Save to disk
        persistence.start_persistence_task().await;
        tokio::time::sleep(Duration::from_secs(6)).await;

        // Create new instance and load
        let persistence2 = EventPersistence::new(temp_dir.path(), 100).unwrap();
        let loaded_events = persistence2.load_events().await.unwrap();

        assert_eq!(loaded_events.len(), 1);
    }

    #[tokio::test]
    async fn test_cache_status_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let status_persistence = CacheStatusPersistence::new(temp_dir.path()).unwrap();

        // Update instance state
        let progress = CacheProgress {
            current: 5,
            total: 10,
            stage: "metadata".to_string(),
            percentage: 50.0,
        };

        status_persistence
            .update_instance_state(
                "test_instance",
                "caching",
                Some("test_addon.jar".to_string()),
                progress,
            )
            .await
            .unwrap();

        // Save to disk
        status_persistence.start_persistence_task().await;
        tokio::time::sleep(Duration::from_secs(11)).await;

        // Create new instance and load
        let status_persistence2 = CacheStatusPersistence::new(temp_dir.path()).unwrap();
        let loaded_status = status_persistence2.load_status().await.unwrap();

        assert_eq!(loaded_status.instances.len(), 1);
        assert!(loaded_status.instances.contains_key("test_instance"));
    }
}
