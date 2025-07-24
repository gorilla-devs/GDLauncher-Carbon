use crate::events::*;
use crate::storage::StorageConfig;
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, bounded};
use parking_lot::RwLock;
use reqwest::Client;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep};
use tracing::{debug, error, info, warn};

pub struct UpdateChecker {
    task_queue: Arc<RwLock<BinaryHeap<UpdateTask>>>,
    instance_priorities: Arc<RwLock<HashMap<String, Priority>>>,
    event_sender: Sender<CacheEvent>,
    event_receiver: Receiver<CacheEvent>,
    config: StorageConfig,
    worker_handles: Vec<JoinHandle<()>>,
    shutdown_senders: Vec<mpsc::UnboundedSender<()>>,
    online_status: Arc<RwLock<bool>>,
    http_client: Arc<Client>,
    rate_limiters: Arc<RwLock<HashMap<Platform, RateLimiter>>>,
}

#[derive(Debug)]
struct UpdateTask {
    addon_id: String,
    platform_data: ModplatformData,
    priority: Priority,
    created_at: u64,
}

impl PartialEq for UpdateTask {
    fn eq(&self, other: &Self) -> bool {
        self.addon_id == other.addon_id
    }
}

impl Eq for UpdateTask {}

impl PartialOrd for UpdateTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UpdateTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.created_at.cmp(&other.created_at))
    }
}

#[derive(Debug, Clone)]
struct RateLimiter {
    last_request: Instant,
    requests_per_second: u32,
    burst_capacity: u32,
    current_tokens: u32,
}

impl RateLimiter {
    fn new(requests_per_second: u32, burst_capacity: u32) -> Self {
        Self {
            last_request: Instant::now(),
            requests_per_second,
            burst_capacity,
            current_tokens: burst_capacity,
        }
    }

    async fn acquire(&mut self) -> Result<()> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_request);

        let tokens_to_add = (elapsed.as_secs_f64() * self.requests_per_second as f64) as u32;
        self.current_tokens = (self.current_tokens + tokens_to_add).min(self.burst_capacity);

        if self.current_tokens > 0 {
            self.current_tokens -= 1;
            self.last_request = now;
            Ok(())
        } else {
            let wait_time = Duration::from_secs_f64(1.0 / self.requests_per_second as f64);
            sleep(wait_time).await;
            self.current_tokens = self.burst_capacity - 1;
            self.last_request = Instant::now();
            Ok(())
        }
    }
}

impl UpdateChecker {
    pub fn new(config: StorageConfig) -> Result<Self> {
        let (event_sender, event_receiver) = bounded(1000);

        let http_client = Arc::new(
            Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("GDLauncher-Carbon/1.0")
                .build()?,
        );

        let mut rate_limiters = HashMap::new();
        rate_limiters.insert(Platform::CurseForge, RateLimiter::new(20, 60));
        rate_limiters.insert(Platform::Modrinth, RateLimiter::new(100, 300));

        Ok(Self {
            task_queue: Arc::new(RwLock::new(BinaryHeap::new())),
            instance_priorities: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
            event_receiver,
            config,
            worker_handles: Vec::new(),
            shutdown_senders: Vec::new(),
            online_status: Arc::new(RwLock::new(true)),
            http_client,
            rate_limiters: Arc::new(RwLock::new(rate_limiters)),
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting update checker stage");

        // Start fewer workers to respect rate limits
        for i in 0..2 {
            let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();
            self.shutdown_senders.push(shutdown_tx);
            let worker = self.spawn_worker(i, shutdown_rx).await?;
            self.worker_handles.push(worker);
        }

        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down update checker stage");

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
                    "Update checker worker shutdown timed out after {}s",
                    shutdown_timeout.as_secs()
                );
            } else if let Ok(Err(e)) = result {
                if e.is_cancelled() {
                    debug!("Update checker worker was successfully aborted");
                } else {
                    error!("Update checker worker failed: {}", e);
                }
            } else {
                debug!("Update checker worker finished cleanly");
            }
        }

        Ok(())
    }

    pub async fn add_addon(
        &self,
        addon_id: String,
        platform_data: ModplatformData,
        priority: Priority,
    ) -> Result<()> {
        let task = UpdateTask {
            addon_id,
            platform_data,
            priority,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        };

        self.task_queue.write().push(task);

        Ok(())
    }

    pub async fn prioritize_instance(&self, instance_id: &str, priority: Priority) -> Result<()> {
        self.instance_priorities
            .write()
            .insert(instance_id.to_string(), priority);

        let mut queue = self.task_queue.write();
        let mut tasks: Vec<UpdateTask> = queue.drain().collect();

        for _task in &mut tasks {
            // Would need instance tracking to reprioritize properly
        }

        for task in tasks {
            queue.push(task);
        }

        Ok(())
    }

    pub async fn set_online(&self, online: bool) -> Result<()> {
        *self.online_status.write() = online;
        Ok(())
    }

    async fn spawn_worker(
        &self,
        worker_id: usize,
        mut shutdown_receiver: mpsc::UnboundedReceiver<()>,
    ) -> Result<JoinHandle<()>> {
        let task_queue = self.task_queue.clone();
        let event_sender = self.event_sender.clone();
        let online_status = self.online_status.clone();
        let http_client = self.http_client.clone();
        let rate_limiters = self.rate_limiters.clone();

        let handle = tokio::spawn(async move {
            debug!("Update checker worker {} started", worker_id);

            loop {
                // Skip if offline
                if !*online_status.read() {
                    tokio::select! {
                        _ = shutdown_receiver.recv() => {
                            debug!("Update checker worker {} shutting down while offline", worker_id);
                            break;
                        }
                        _ = sleep(Duration::from_secs(1)) => {}
                    }
                    continue;
                }

                let task = {
                    let mut queue = task_queue.write();
                    queue.pop()
                };

                match task {
                    Some(task) => {
                        tokio::select! {
                            _ = shutdown_receiver.recv() => {
                                debug!("Update checker worker {} shutting down during task", worker_id);
                                break;
                            }
                            result = Self::process_update_task(
                                task,
                                &event_sender,
                                &http_client,
                                &rate_limiters,
                            ) => {
                                if let Err(e) = result {
                                    error!("Error processing update task: {}", e);
                                }
                            }
                        }
                    }
                    None => {
                        tokio::select! {
                            _ = shutdown_receiver.recv() => {
                                debug!("Update checker worker {} shutting down", worker_id);
                                break;
                            }
                            _ = sleep(Duration::from_millis(100)) => {}
                        }
                    }
                }
            }

            debug!("Update checker worker {} finished", worker_id);
        });

        Ok(handle)
    }

    async fn process_update_task(
        task: UpdateTask,
        event_sender: &Sender<CacheEvent>,
        http_client: &Client,
        rate_limiters: &Arc<RwLock<HashMap<Platform, RateLimiter>>>,
    ) -> Result<()> {
        debug!("Processing update check for addon: {}", task.addon_id);

        let updates = match task.platform_data.platform {
            Platform::CurseForge => {
                Self::fetch_curseforge_versions(&task, http_client, rate_limiters).await?
            }
            Platform::Modrinth => {
                Self::fetch_modrinth_versions(&task, http_client, rate_limiters).await?
            }
            Platform::Unknown => {
                debug!("Unknown platform for addon: {}", task.addon_id);
                return Ok(());
            }
        };

        let event = CacheEvent::UpdatesChecked {
            addon_id: task.addon_id,
            updates,
        };

        event_sender.send(event)?;

        Ok(())
    }

    async fn fetch_curseforge_versions(
        task: &UpdateTask,
        http_client: &Client,
        rate_limiters: &Arc<RwLock<HashMap<Platform, RateLimiter>>>,
    ) -> Result<Vec<Version>> {
        // Apply rate limiting
        let limiter_opt = rate_limiters.write().get(&Platform::CurseForge).cloned();
        if let Some(mut limiter) = limiter_opt {
            limiter.acquire().await?;
            rate_limiters.write().insert(Platform::CurseForge, limiter);
        }

        let url = format!(
            "https://api.curseforge.com/v1/mods/{}/files",
            task.platform_data.project_id
        );

        let response = http_client
            .get(&url)
            .header(
                "x-api-key",
                std::env::var("CURSEFORGE_API_KEY").unwrap_or_default(),
            )
            .query(&[("pageSize", "50")])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "CurseForge API error: {}",
                response.status()
            ));
        }

        let json: serde_json::Value = response.json().await?;
        let files = json["data"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No files data found"))?;

        let mut versions = Vec::new();

        for file in files {
            let version = Version {
                version_number: file["displayName"]
                    .as_str()
                    .unwrap_or("Unknown")
                    .to_string(),
                version_type: match file["releaseType"].as_u64().unwrap_or(1) {
                    1 => VersionType::Release,
                    2 => VersionType::Beta,
                    3 => VersionType::Alpha,
                    _ => VersionType::Release,
                },
                minecraft_versions: file["gameVersions"]
                    .as_array()
                    .map(|versions| {
                        versions
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                mod_loaders: file["modLoaders"]
                    .as_array()
                    .map(|loaders| {
                        loaders
                            .iter()
                            .filter_map(|l| l.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                release_date: file["fileDate"].as_str().unwrap_or("").to_string(),
                download_url: file["downloadUrl"].as_str().unwrap_or("").to_string(),
                changelog: None, // Would need separate API call for changelog
            };

            versions.push(version);
        }

        // Sort by release date (newest first)
        versions.sort_by(|a, b| b.release_date.cmp(&a.release_date));

        Ok(versions)
    }

    async fn fetch_modrinth_versions(
        task: &UpdateTask,
        http_client: &Client,
        rate_limiters: &Arc<RwLock<HashMap<Platform, RateLimiter>>>,
    ) -> Result<Vec<Version>> {
        // Apply rate limiting
        let limiter_opt = rate_limiters.write().get(&Platform::Modrinth).cloned();
        if let Some(mut limiter) = limiter_opt {
            limiter.acquire().await?;
            rate_limiters.write().insert(Platform::Modrinth, limiter);
        }

        let url = format!(
            "https://api.modrinth.com/v2/project/{}/version",
            task.platform_data.project_id
        );

        let response = http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Modrinth API error: {}", response.status()));
        }

        let json: serde_json::Value = response.json().await?;
        let versions_data = json
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid versions data"))?;

        let mut versions = Vec::new();

        for version_data in versions_data {
            let version = Version {
                version_number: version_data["version_number"]
                    .as_str()
                    .unwrap_or("Unknown")
                    .to_string(),
                version_type: match version_data["version_type"].as_str().unwrap_or("release") {
                    "release" => VersionType::Release,
                    "beta" => VersionType::Beta,
                    "alpha" => VersionType::Alpha,
                    _ => VersionType::Release,
                },
                minecraft_versions: version_data["game_versions"]
                    .as_array()
                    .map(|versions| {
                        versions
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                mod_loaders: version_data["loaders"]
                    .as_array()
                    .map(|loaders| {
                        loaders
                            .iter()
                            .filter_map(|l| l.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                release_date: version_data["date_published"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                download_url: version_data["files"]
                    .as_array()
                    .and_then(|files| files.first())
                    .and_then(|file| file["url"].as_str())
                    .unwrap_or("")
                    .to_string(),
                changelog: version_data["changelog"].as_str().map(|s| s.to_string()),
            };

            versions.push(version);
        }

        // Sort by release date (newest first)
        versions.sort_by(|a, b| b.release_date.cmp(&a.release_date));

        Ok(versions)
    }
}

impl Drop for UpdateChecker {
    fn drop(&mut self) {
        for sender in &self.shutdown_senders {
            let _ = sender.send(());
        }
    }
}
