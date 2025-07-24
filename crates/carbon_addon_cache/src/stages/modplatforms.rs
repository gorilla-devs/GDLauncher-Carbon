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

pub struct ModplatformFetcher {
    task_queue: Arc<RwLock<BinaryHeap<ModplatformTask>>>,
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
struct ModplatformTask {
    addon_id: String,
    metadata: LocalMetadata,
    priority: Priority,
    created_at: u64,
}

impl PartialEq for ModplatformTask {
    fn eq(&self, other: &Self) -> bool {
        self.addon_id == other.addon_id
    }
}

impl Eq for ModplatformTask {}

impl PartialOrd for ModplatformTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ModplatformTask {
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

        // Refill tokens based on elapsed time
        let tokens_to_add = (elapsed.as_secs_f64() * self.requests_per_second as f64) as u32;
        self.current_tokens = (self.current_tokens + tokens_to_add).min(self.burst_capacity);

        if self.current_tokens > 0 {
            self.current_tokens -= 1;
            self.last_request = now;
            Ok(())
        } else {
            // Wait until we can make a request
            let wait_time = Duration::from_secs_f64(1.0 / self.requests_per_second as f64);
            sleep(wait_time).await;
            self.current_tokens = self.burst_capacity - 1;
            self.last_request = Instant::now();
            Ok(())
        }
    }
}

impl ModplatformFetcher {
    pub fn new(config: StorageConfig) -> Result<Self> {
        let (event_sender, event_receiver) = bounded(1000);

        // Skip HTTP client creation in test mode to prevent hanging
        let http_client = if cfg!(test) {
            Arc::new(Client::new()) // Simple client for tests
        } else {
            Arc::new(
                Client::builder()
                    .timeout(Duration::from_secs(30))
                    .user_agent("GDLauncher-Carbon/1.0")
                    .build()?,
            )
        };

        let mut rate_limiters = HashMap::new();
        rate_limiters.insert(Platform::CurseForge, RateLimiter::new(20, 60)); // 20 req/s, burst of 60
        rate_limiters.insert(Platform::Modrinth, RateLimiter::new(100, 300)); // 100 req/s, burst of 300

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
        info!("Starting modplatform fetcher stage");

        // Start minimal workers to respect rate limits and reduce resource usage
        for i in 0..2 {
            let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();
            self.shutdown_senders.push(shutdown_tx);
            let worker = self.spawn_worker(i, shutdown_rx).await?;
            self.worker_handles.push(worker);
        }

        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down modplatform fetcher stage");

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
                    "Modplatform fetcher worker shutdown timed out after {}s",
                    shutdown_timeout.as_secs()
                );
            } else if let Ok(Err(e)) = result {
                if e.is_cancelled() {
                    debug!("Modplatform fetcher worker was successfully aborted");
                } else {
                    error!("Modplatform fetcher worker failed: {}", e);
                }
            } else {
                debug!("Modplatform fetcher worker finished cleanly");
            }
        }

        Ok(())
    }

    pub async fn add_addon(
        &self,
        addon_id: String,
        metadata: LocalMetadata,
        priority: Priority,
    ) -> Result<()> {
        let task = ModplatformTask {
            addon_id,
            metadata,
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
        let mut tasks: Vec<ModplatformTask> = queue.drain().collect();

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
            debug!("Modplatform fetcher worker {} started", worker_id);

            loop {
                // Skip if offline
                if !*online_status.read() {
                    tokio::select! {
                        _ = shutdown_receiver.recv() => {
                            debug!("Modplatform fetcher worker {} shutting down while offline", worker_id);
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
                                debug!("Modplatform fetcher worker {} shutting down during task", worker_id);
                                break;
                            }
                            result = Self::process_modplatform_task(
                                task,
                                &event_sender,
                                &http_client,
                                &rate_limiters,
                            ) => {
                                if let Err(e) = result {
                                    error!("Error processing modplatform task: {}", e);
                                }
                            }
                        }
                    }
                    None => {
                        tokio::select! {
                            _ = shutdown_receiver.recv() => {
                                debug!("Modplatform fetcher worker {} shutting down", worker_id);
                                break;
                            }
                            _ = sleep(Duration::from_millis(100)) => {}
                        }
                    }
                }
            }

            debug!("Modplatform fetcher worker {} finished", worker_id);
        });

        Ok(handle)
    }

    async fn process_modplatform_task(
        task: ModplatformTask,
        event_sender: &Sender<CacheEvent>,
        http_client: &Client,
        rate_limiters: &Arc<RwLock<HashMap<Platform, RateLimiter>>>,
    ) -> Result<()> {
        debug!("Processing modplatform data for addon: {}", task.addon_id);

        // Try CurseForge first
        if let Ok(data) = Self::fetch_curseforge_data(&task, http_client, rate_limiters).await {
            let event = CacheEvent::ModplatformDataFetched {
                addon_id: task.addon_id,
                data,
            };
            event_sender.send(event)?;
            return Ok(());
        }

        // Try Modrinth if CurseForge fails
        if let Ok(data) = Self::fetch_modrinth_data(&task, http_client, rate_limiters).await {
            let event = CacheEvent::ModplatformDataFetched {
                addon_id: task.addon_id,
                data,
            };
            event_sender.send(event)?;
            return Ok(());
        }

        debug!("No modplatform data found for addon: {}", task.addon_id);
        Ok(())
    }

    async fn fetch_curseforge_data(
        task: &ModplatformTask,
        http_client: &Client,
        rate_limiters: &Arc<RwLock<HashMap<Platform, RateLimiter>>>,
    ) -> Result<ModplatformData> {
        // Apply rate limiting
        let limiter_opt = rate_limiters.write().get(&Platform::CurseForge).cloned();
        if let Some(mut limiter) = limiter_opt {
            limiter.acquire().await?;
            rate_limiters.write().insert(Platform::CurseForge, limiter);
        }

        // Use fingerprint API to find the mod
        let fingerprint = task.metadata.checksums.murmur2;
        let url = format!(
            "https://api.curseforge.com/v1/fingerprints/432/{}",
            fingerprint
        );

        let response = http_client
            .get(&url)
            .header(
                "x-api-key",
                std::env::var("CURSEFORGE_API_KEY").unwrap_or_default(),
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "CurseForge API error: {}",
                response.status()
            ));
        }

        let json: serde_json::Value = response.json().await?;

        // Parse the response
        let data = &json["data"];
        let exact_matches = data["exactMatches"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No exact matches found"))?;

        if exact_matches.is_empty() {
            return Err(anyhow::anyhow!("No exact matches found"));
        }

        let file_data = &exact_matches[0];
        let project_id = file_data["modId"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("No project ID found"))?;

        // Fetch project details
        let project_url = format!("https://api.curseforge.com/v1/mods/{}", project_id);
        let project_response = http_client
            .get(&project_url)
            .header(
                "x-api-key",
                std::env::var("CURSEFORGE_API_KEY").unwrap_or_default(),
            )
            .send()
            .await?;

        let project_json: serde_json::Value = project_response.json().await?;
        let project = &project_json["data"];

        let modplatform_data = ModplatformData {
            platform: Platform::CurseForge,
            project_id: project_id.to_string(),
            file_id: file_data["id"].as_u64().unwrap_or(0).to_string(),
            download_url: file_data["downloadUrl"].as_str().map(|s| s.to_string()),
            project_name: project["name"].as_str().unwrap_or("Unknown").to_string(),
            project_description: project["summary"].as_str().map(|s| s.to_string()),
            categories: project["categories"]
                .as_array()
                .map(|cats| {
                    cats.iter()
                        .filter_map(|cat| cat["name"].as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            license: project["license"].as_str().map(|s| s.to_string()),
            website_url: project["websiteUrl"].as_str().map(|s| s.to_string()),
            source_url: project["sourceUrl"].as_str().map(|s| s.to_string()),
            issues_url: project["issuesUrl"].as_str().map(|s| s.to_string()),
        };

        Ok(modplatform_data)
    }

    async fn fetch_modrinth_data(
        task: &ModplatformTask,
        http_client: &Client,
        rate_limiters: &Arc<RwLock<HashMap<Platform, RateLimiter>>>,
    ) -> Result<ModplatformData> {
        // Apply rate limiting
        let limiter_opt = rate_limiters.write().get(&Platform::Modrinth).cloned();
        if let Some(mut limiter) = limiter_opt {
            limiter.acquire().await?;
            rate_limiters.write().insert(Platform::Modrinth, limiter);
        }

        // Use version file hash to find the mod
        let hash = &task.metadata.checksums.sha256;
        let url = format!(
            "https://api.modrinth.com/v2/version_file/{}?algorithm=sha256",
            hash
        );

        let response = http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Modrinth API error: {}", response.status()));
        }

        let version_json: serde_json::Value = response.json().await?;
        let project_id = version_json["project_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No project ID found"))?;

        // Fetch project details
        let project_url = format!("https://api.modrinth.com/v2/project/{}", project_id);
        let project_response = http_client.get(&project_url).send().await?;

        let project_json: serde_json::Value = project_response.json().await?;

        let modplatform_data = ModplatformData {
            platform: Platform::Modrinth,
            project_id: project_id.to_string(),
            file_id: version_json["id"].as_str().unwrap_or("").to_string(),
            download_url: version_json["files"]
                .as_array()
                .and_then(|files| files.first())
                .and_then(|file| file["url"].as_str())
                .map(|s| s.to_string()),
            project_name: project_json["title"]
                .as_str()
                .unwrap_or("Unknown")
                .to_string(),
            project_description: project_json["description"].as_str().map(|s| s.to_string()),
            categories: project_json["categories"]
                .as_array()
                .map(|cats| {
                    cats.iter()
                        .filter_map(|cat| cat.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            license: project_json["license"]["id"]
                .as_str()
                .map(|s| s.to_string()),
            website_url: project_json["project_url"].as_str().map(|s| s.to_string()),
            source_url: project_json["source_url"].as_str().map(|s| s.to_string()),
            issues_url: project_json["issues_url"].as_str().map(|s| s.to_string()),
        };

        Ok(modplatform_data)
    }
}

impl Drop for ModplatformFetcher {
    fn drop(&mut self) {
        for sender in &self.shutdown_senders {
            let _ = sender.send(());
        }
    }
}
