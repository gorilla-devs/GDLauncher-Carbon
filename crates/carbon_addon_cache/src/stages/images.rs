use crate::events::*;
use crate::scheduler::CacheScheduler;
use crate::storage::StorageConfig;
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, bounded};
use image;
use parking_lot::RwLock;
use std::collections::{BinaryHeap, HashMap};
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use zip::ZipArchive;

pub struct ImageCache {
    task_queue: Arc<RwLock<BinaryHeap<ImageTask>>>,
    instance_priorities: Arc<RwLock<HashMap<String, Priority>>>,
    event_sender: Sender<CacheEvent>,
    event_receiver: Receiver<CacheEvent>,
    config: StorageConfig,
    worker_handles: Vec<JoinHandle<()>>,
    shutdown_senders: Vec<mpsc::UnboundedSender<()>>,
}

#[derive(Debug)]
struct ImageTask {
    addon_id: String,
    metadata: LocalMetadata,
    priority: Priority,
    created_at: u64,
}

impl PartialEq for ImageTask {
    fn eq(&self, other: &Self) -> bool {
        self.addon_id == other.addon_id
    }
}

impl Eq for ImageTask {}

impl PartialOrd for ImageTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ImageTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.created_at.cmp(&other.created_at))
    }
}

impl ImageCache {
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
        info!("Starting image cache stage");

        // Start minimal workers for image processing since it's very CPU intensive
        let worker_count = (num_cpus::get() / 8).max(1).min(2);
        for i in 0..worker_count {
            let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();
            self.shutdown_senders.push(shutdown_tx);
            let worker = self.spawn_worker(i, shutdown_rx).await?;
            self.worker_handles.push(worker);
        }

        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down image cache stage");

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
                    "Image cache worker shutdown timed out after {}s",
                    shutdown_timeout.as_secs()
                );
            } else if let Ok(Err(e)) = result {
                if e.is_cancelled() {
                    debug!("Image cache worker was successfully aborted");
                } else {
                    error!("Image cache worker failed: {}", e);
                }
            } else {
                debug!("Image cache worker finished cleanly");
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
        let task = ImageTask {
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

        let mut queue = self.task_queue.write();
        let mut tasks: Vec<ImageTask> = queue.drain().collect();

        for _task in &mut tasks {
            // Note: We can't easily get instance_id from LocalMetadata, so we'll skip reprioritization
            // This would need to be tracked separately if needed
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
            debug!("Image cache worker {} started", worker_id);

            loop {
                let task = {
                    let mut queue = task_queue.write();
                    queue.pop()
                };

                match task {
                    Some(task) => {
                        tokio::select! {
                            _ = shutdown_receiver.recv() => {
                                debug!("Image cache worker {} shutting down during task", worker_id);
                                break;
                            }
                            result = Self::process_image_task(task, &event_sender, &config) => {
                                if let Err(e) = result {
                                    error!("Error processing image task: {}", e);
                                }
                            }
                        }
                    }
                    None => {
                        tokio::select! {
                            _ = shutdown_receiver.recv() => {
                                debug!("Image cache worker {} shutting down", worker_id);
                                break;
                            }
                            _ = tokio::time::sleep(tokio::time::Duration::from_millis(10)) => {}
                        }
                    }
                }
            }

            debug!("Image cache worker {} finished", worker_id);
        });

        Ok(handle)
    }

    async fn process_image_task(
        task: ImageTask,
        event_sender: &Sender<CacheEvent>,
        config: &StorageConfig,
    ) -> Result<()> {
        debug!("Processing image cache for addon: {}", task.addon_id);

        let mut images = Vec::new();

        // Extract images from addon file
        let addon_path = config
            .runtime_path
            .join("addons")
            .join(format!("{}.jar", &task.metadata.checksums.blake3));

        if addon_path.exists() {
            // Extract icon from JAR/ZIP using carbon_scheduler
            if let Ok(icon_data) =
                CacheScheduler::process_zip_entries(&addon_path, |mut archive| {
                    Self::extract_icon_from_archive_sync(&mut archive)
                })
                .await
            {
                let optimized_icon = Self::optimize_image(&icon_data, ImageType::Icon).await?;
                images.push(ImageInfo {
                    image_type: ImageType::Icon,
                    url: None,
                    data: Some(optimized_icon),
                });
            }

            // Extract other images if they exist using carbon_scheduler
            if let Ok(gallery_images) =
                CacheScheduler::process_zip_entries(&addon_path, |mut archive| {
                    Self::extract_gallery_images_sync(&mut archive)
                })
                .await
            {
                for image_data in gallery_images.iter() {
                    let optimized_image =
                        Self::optimize_image(image_data, ImageType::Gallery).await?;
                    images.push(ImageInfo {
                        image_type: ImageType::Gallery,
                        url: None,
                        data: Some(optimized_image),
                    });
                }
            }
        }

        // Emit ImagesProcessed event
        let event = CacheEvent::ImagesProcessed {
            addon_id: task.addon_id,
            images,
        };

        event_sender.send(event)?;

        Ok(())
    }

    async fn extract_icon_from_archive(addon_path: &std::path::Path) -> Result<Vec<u8>> {
        let file = File::open(addon_path)?;
        let mut archive = ZipArchive::new(file)?;

        // Common icon file names in mods
        let icon_names = [
            "icon.png",
            "assets/icon.png",
            "pack.png",
            "logo.png",
            "mod_icon.png",
        ];

        for icon_name in &icon_names {
            if let Ok(mut file) = archive.by_name(icon_name) {
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                return Ok(buffer);
            }
        }

        // Try to find any PNG file that might be an icon
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name().to_string();
            drop(file); // Release the borrow

            if name.ends_with(".png") && name.len() < 50 {
                let mut file = archive.by_index(i)?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                return Ok(buffer);
            }
        }

        Err(anyhow::anyhow!("No icon found in archive"))
    }

    async fn extract_gallery_images(addon_path: &std::path::Path) -> Result<Vec<Vec<u8>>> {
        let file = File::open(addon_path)?;
        let mut archive = ZipArchive::new(file)?;

        let mut images = Vec::new();

        // Collect indices first, then read files
        let mut image_indices = Vec::new();
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name().to_string();
            drop(file); // Release the borrow

            if (name.contains("screenshots") || name.contains("gallery") || name.contains("images"))
                && (name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg"))
            {
                image_indices.push(i);

                // Limit number of gallery images to prevent excessive memory usage
                if image_indices.len() >= 5 {
                    break;
                }
            }
        }

        // Now read the image files
        for index in image_indices {
            let mut file = archive.by_index(index)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            images.push(buffer);
        }

        Ok(images)
    }

    async fn optimize_image(image_data: &[u8], image_type: ImageType) -> Result<Vec<u8>> {
        debug!(
            "Optimizing image of type {:?}, size: {} bytes",
            image_type,
            image_data.len()
        );

        // Use carbon_scheduler for CPU-intensive image processing
        let input_data = image_data.to_vec();

        CacheScheduler::process_image(input_data, move |data| {
            // Load the image
            let img = image::load_from_memory(&data)?;

            // Determine target size based on image type
            let target_size = match image_type {
                ImageType::Icon => (64, 64),
                ImageType::Gallery => (400, 300),
                ImageType::Featured => (800, 600),
            };

            // Resize the image
            let resized = img.resize(
                target_size.0,
                target_size.1,
                image::imageops::FilterType::Lanczos3,
            );

            // Convert to WebP for better compression
            let mut output = Vec::new();
            let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut output);
            resized.write_with_encoder(encoder)?;

            Ok(output)
        })
        .await
    }

    // Synchronous versions for use with carbon_scheduler
    fn extract_icon_from_archive_sync(archive: &mut ZipArchive<File>) -> Result<Vec<u8>> {
        // Common icon file names in mods
        let icon_names = [
            "icon.png",
            "assets/icon.png",
            "pack.png",
            "logo.png",
            "mod_icon.png",
        ];

        for icon_name in &icon_names {
            if let Ok(mut file) = archive.by_name(icon_name) {
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                return Ok(buffer);
            }
        }

        // Try to find any PNG file that might be an icon
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name().to_string();
            drop(file); // Release the borrow

            if name.ends_with(".png") && (name.contains("icon") || name.contains("logo")) {
                let mut file = archive.by_index(i)?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                return Ok(buffer);
            }
        }

        Err(anyhow::anyhow!("No icon found in archive"))
    }

    fn extract_gallery_images_sync(archive: &mut ZipArchive<File>) -> Result<Vec<Vec<u8>>> {
        let mut images = Vec::new();
        let mut image_indices = Vec::new();

        // Find image files in gallery/screenshots directories
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name().to_lowercase();
            drop(file); // Release the borrow

            if (name.contains("screenshots") || name.contains("gallery") || name.contains("images"))
                && (name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg"))
            {
                image_indices.push(i);

                // Limit number of gallery images to prevent excessive memory usage
                if image_indices.len() >= 5 {
                    break;
                }
            }
        }

        // Now read the image files
        for index in image_indices {
            let mut file = archive.by_index(index)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            images.push(buffer);
        }

        Ok(images)
    }

    async fn download_remote_image(url: &str) -> Result<Vec<u8>> {
        let client = reqwest::Client::new();
        let response = client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download image: {}",
                response.status()
            ));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
}

impl Drop for ImageCache {
    fn drop(&mut self) {
        for sender in &self.shutdown_senders {
            let _ = sender.send(());
        }
    }
}
