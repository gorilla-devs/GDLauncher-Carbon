use md5::digest::generic_array::ArrayLength;
use md5::digest::OutputSizeUser;
use md5::Digest as Md5Digest;
use md5::Md5;
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use sha1::Digest as Sha1Digest;
use sha1::Sha1;
use sha2::{Digest as Sha2Digest, Sha256};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt::Display;
use std::ops::Add;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::watch::Sender;
use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument, warn};

const PART_POSTFIX: &str = ".__gdl_part~";

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("Failed to download {0}")]
    GenericDownload(String),
    #[error("I/O Error {0}")]
    IOError(#[from] std::io::Error),
    #[error("Failed to make network request {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Failed to make network request {0}")]
    NetworkError2(#[from] reqwest_middleware::Error),
    #[error("Join error {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Failed to acquire semaphore")]
    AcquireError(#[from] tokio::sync::AcquireError),
    #[error("Cancelled")]
    Cancelled,
    #[error("File not found")]
    FileNotFound,
    #[error("Size mismatch")]
    SizeMismatch { expected: u64, actual: u64 },
    #[error("Checksum mismatch")]
    ChecksumMismatch {
        expected: String,
        actual: String,
        url: String,
        path: String,
    },
    #[error("Non 200 status code {1}: {0}")]
    Non200StatusCode(Downloadable, u16),
    #[error("Failed to remove file: {0}")]
    CannotRemoveFile(String),
    #[error("Failed to create directory: {0}")]
    CannotCreateDirectory(String),
    #[error("Failed to atomically move file: {0}")]
    CannotMoveFile(std::io::Error, String, String),
}

#[derive(Debug, Clone)]
pub enum Checksum {
    Sha1(String),
    Sha256(String),
    Md5(String),
}

enum HashDigest {
    Sha256(Sha256),
    Sha1(sha1::Sha1),
    Md5(md5::Md5),
}

impl HashDigest {
    fn update(&mut self, data: &[u8]) {
        match self {
            HashDigest::Sha256(h) => h.update(data),
            HashDigest::Sha1(h) => h.update(data),
            HashDigest::Md5(h) => h.update(data),
        }
    }

    fn finalize(self) -> Vec<u8> {
        match self {
            HashDigest::Sha256(h) => h.finalize().to_vec(),
            HashDigest::Sha1(h) => h.finalize().to_vec(),
            HashDigest::Md5(h) => h.finalize().to_vec(),
        }
    }

    fn finalize_reset(&mut self) -> Vec<u8> {
        match self {
            HashDigest::Sha256(h) => h.finalize_reset().to_vec(),
            HashDigest::Sha1(h) => h.finalize_reset().to_vec(),
            HashDigest::Md5(h) => h.finalize_reset().to_vec(),
        }
    }
}

impl From<&Checksum> for HashDigest {
    fn from(value: &Checksum) -> Self {
        match value {
            Checksum::Sha256(_) => HashDigest::Sha256(Sha256::new()),
            Checksum::Sha1(_) => HashDigest::Sha1(Sha1::new()),
            Checksum::Md5(_) => HashDigest::Md5(Md5::new()),
        }
    }
}

pub trait IntoVecDownloadable {
    fn into_vec_downloadable(self, base_path: &Path) -> Vec<Downloadable>;
}

pub trait IntoDownloadable {
    fn into_downloadable(self, base_path: &Path) -> Downloadable;
}

#[derive(Debug, Clone)]
pub struct Downloadable {
    pub url: String,
    pub path: PathBuf,
    pub checksum: Option<Checksum>,
    pub size: Option<u64>,
}

impl Display for Downloadable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -> {}", self.url, self.path.display())
    }
}

impl Downloadable {
    pub fn new(url: impl Into<String>, path: impl AsRef<Path>) -> Self {
        Self {
            url: url.into(),
            path: path.as_ref().into(),
            checksum: None,
            size: None,
        }
    }

    pub fn with_checksum(mut self, checksum: Option<Checksum>) -> Self {
        self.checksum = checksum;
        self
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }
}

#[derive(Debug, Default, Clone)]
pub struct Progress {
    pub total_count: u64,
    pub current_count: u64,
    pub total_size: u64,
    pub current_size: u64,
}

impl Progress {
    pub fn new() -> Self {
        Self::default()
    }
}

struct FilePathLock {
    locks: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
}

impl FilePathLock {
    fn new() -> Self {
        FilePathLock {
            locks: Mutex::new(HashMap::new()),
        }
    }

    async fn lock(&self, path: &PathBuf) -> Arc<Mutex<()>> {
        let mut locks = self.locks.lock().await;
        locks
            .entry(path.clone())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }
}

#[derive(Clone)]
pub struct DownloadOptions {
    concurrency: usize,
    cancel_token: CancellationToken,
    only_validate: bool,
    deep_check: bool,
    max_retries: u32,
    progress_sender: Option<Sender<Progress>>,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            concurrency: 4,
            cancel_token: CancellationToken::new(),
            only_validate: false,
            deep_check: true,
            max_retries: 3,
            progress_sender: None,
        }
    }
}

impl DownloadOptions {
    pub fn builder() -> DownloadOptionsBuilder {
        DownloadOptionsBuilder::default()
    }
}

#[derive(Default)]
pub struct DownloadOptionsBuilder {
    concurrency: Option<usize>,
    bandwidth_limit: Option<u64>,
    cancel_token: Option<CancellationToken>,
    only_validate: Option<bool>,
    deep_check: Option<bool>,
    max_retries: Option<u32>,
    progress_sender: Option<Sender<Progress>>,
}

impl DownloadOptionsBuilder {
    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = Some(concurrency);
        self
    }

    pub fn bandwidth_limit(mut self, bandwidth_limit: u64) -> Self {
        self.bandwidth_limit = Some(bandwidth_limit);
        self
    }

    pub fn cancel_token(mut self, cancel_token: CancellationToken) -> Self {
        self.cancel_token = Some(cancel_token);
        self
    }

    pub fn only_validate(mut self, only_validate: bool) -> Self {
        self.only_validate = Some(only_validate);
        self
    }

    pub fn deep_check(mut self, deep_check: bool) -> Self {
        self.deep_check = Some(deep_check);
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    pub fn progress_sender(mut self, progress_sender: Sender<Progress>) -> Self {
        self.progress_sender = Some(progress_sender);
        self
    }

    pub fn build(self) -> DownloadOptions {
        DownloadOptions {
            concurrency: self.concurrency.unwrap_or(4),
            cancel_token: self.cancel_token.unwrap_or_else(CancellationToken::new),
            only_validate: self.only_validate.unwrap_or(false),
            deep_check: self.deep_check.unwrap_or(false),
            max_retries: self.max_retries.unwrap_or(3),
            progress_sender: self.progress_sender,
        }
    }
}

#[instrument(skip(files, options))]
pub async fn download_multiple(
    files: &[Downloadable],
    options: DownloadOptions,
) -> Result<bool, DownloadError> {
    let client = create_client(&options);
    let semaphore = Arc::new(Semaphore::new(options.concurrency));
    let file_path_lock = Arc::new(FilePathLock::new());

    info!("Starting processing of {} files", files.len());

    let total_files_size: u64 = files.iter().filter_map(|f| f.size).sum();
    let total_downloaded_size = Arc::new(AtomicU64::new(0));
    let total_files_count = files.len() as u64;
    let current_files_count = Arc::new(AtomicU64::new(0));

    if let Some(sender) = &options.progress_sender {
        let _ = sender.send(Progress {
            total_count: total_files_count,
            current_count: 0,
            total_size: total_files_size,
            current_size: 0,
        });
    }

    let tasks = create_download_tasks(
        files,
        &options,
        &client,
        &semaphore,
        &total_downloaded_size,
        &current_files_count,
        total_files_size,
        total_files_count,
        &file_path_lock,
    );

    process_download_tasks(
        tasks,
        &options.progress_sender,
        total_files_count,
        &total_downloaded_size,
        total_files_size,
    )
    .await
}

fn create_client(options: &DownloadOptions) -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(options.max_retries);
    ClientBuilder::new(Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}

fn create_download_tasks(
    files: &[Downloadable],
    options: &DownloadOptions,
    client: &ClientWithMiddleware,
    semaphore: &Arc<Semaphore>,
    total_downloaded_size: &Arc<AtomicU64>,
    current_files_count: &Arc<AtomicU64>,
    total_files_size: u64,
    total_files_count: u64,
    file_path_lock: &Arc<FilePathLock>,
) -> Vec<tokio::task::JoinHandle<Result<(), DownloadError>>> {
    files
        .iter()
        .map(|file| {
            let semaphore = Arc::clone(semaphore);
            let client = client.clone();
            let options = options.clone();
            let file = file.clone();
            let downloaded_size = Arc::clone(total_downloaded_size);
            let current_count = Arc::clone(current_files_count);
            let file_path_lock = Arc::clone(file_path_lock);

            tokio::spawn(async move {
                let _permit = semaphore.acquire().await?;
                process_file(
                    file,
                    options,
                    client,
                    &downloaded_size,
                    &current_count,
                    total_files_size,
                    total_files_count,
                    &file_path_lock,
                )
                .await
            })
        })
        .collect()
}

#[instrument(skip(
    options,
    client,
    total_downloaded_size,
    current_files_count,
    file_path_lock
))]
async fn process_file(
    file: Downloadable,
    options: DownloadOptions,
    client: ClientWithMiddleware,
    total_downloaded_size: &AtomicU64,
    current_files_count: &AtomicU64,
    total_files_size: u64,
    total_files_count: u64,
    file_path_lock: &FilePathLock,
) -> Result<(), DownloadError> {
    let validation = validate_file(&file.path, file.size, &file.checksum, options.deep_check).await;

    match validation {
        Err(err) if options.only_validate => {
            return Err(err);
        }
        Err(_) => {
            download_file(
                file,
                options,
                client,
                total_downloaded_size,
                current_files_count,
                total_files_size,
                total_files_count,
                file_path_lock,
            )
            .await
        }
        Ok(_) => {
            if let Some(size) = file.size {
                total_downloaded_size.fetch_add(size, Ordering::SeqCst);
            }
            current_files_count.fetch_add(1, Ordering::SeqCst);
            if let Some(sender) = &options.progress_sender {
                let _ = sender.send(Progress {
                    total_count: total_files_count,
                    current_count: current_files_count.load(Ordering::SeqCst),
                    total_size: total_files_size,
                    current_size: total_downloaded_size.load(Ordering::SeqCst),
                });
            }
            return Ok(());
        }
    }
}

#[instrument(skip(
    options,
    client,
    total_downloaded_size,
    current_files_count,
    file_path_lock
))]
async fn download_file(
    downloadable: Downloadable,
    options: DownloadOptions,
    client: ClientWithMiddleware,
    total_downloaded_size: &AtomicU64,
    current_files_count: &AtomicU64,
    total_files_size: u64,
    total_files_count: u64,
    file_path_lock: &FilePathLock,
) -> Result<(), DownloadError> {
    let file_lock = file_path_lock.lock(&downloadable.path).await;
    let _guard = file_lock.lock().await;

    let file_processed_bytes = AtomicU64::new(0);

    let (part_file_path, mut file, headers, mut hasher, was_resumed) = prepare_download(
        &downloadable,
        false,
        total_downloaded_size,
        &file_processed_bytes,
    )
    .await?;

    let outcome = _download_file(
        &downloadable.url,
        headers,
        &part_file_path,
        &mut file,
        &mut hasher,
        &downloadable,
        options.clone(),
        client.clone(),
        total_downloaded_size,
        &file_processed_bytes,
        current_files_count,
        total_files_size,
        total_files_count,
    )
    .await;

    let progress = options.progress_sender.as_ref();

    match outcome {
        Ok(()) => {
            tokio::fs::rename(&part_file_path, &downloadable.path)
                .await
                .map_err(|e| {
                    DownloadError::CannotMoveFile(
                        e,
                        part_file_path.to_string_lossy().to_string(),
                        downloadable.path.to_string_lossy().to_string(),
                    )
                })?;

            current_files_count.fetch_add(1, Ordering::SeqCst);
            if let Some(sender) = progress {
                let _ = sender.send(Progress {
                    total_count: total_files_count,
                    current_count: current_files_count.load(Ordering::SeqCst),
                    total_size: total_files_size,
                    current_size: total_downloaded_size.load(Ordering::SeqCst),
                });
            }

            Ok(())
        }
        Err(DownloadError::ChecksumMismatch { .. }) | Err(DownloadError::SizeMismatch { .. })
            if was_resumed =>
        {
            total_downloaded_size.fetch_sub(
                file_processed_bytes.load(Ordering::SeqCst),
                Ordering::SeqCst,
            );

            file_processed_bytes.store(0, Ordering::SeqCst);

            if let Some(sender) = progress {
                let _ = sender.send(Progress {
                    total_count: total_files_count,
                    current_count: current_files_count.load(Ordering::SeqCst),
                    total_size: total_files_size,
                    current_size: total_downloaded_size.load(Ordering::SeqCst),
                });
            }

            let (part_file_path, mut file, headers, mut hasher, _) = prepare_download(
                &downloadable,
                true,
                total_downloaded_size,
                &file_processed_bytes,
            )
            .await?;

            let outcome = _download_file(
                &downloadable.url,
                headers,
                &part_file_path,
                &mut file,
                &mut hasher,
                &downloadable,
                options.clone(),
                client,
                total_downloaded_size,
                &file_processed_bytes,
                current_files_count,
                total_files_size,
                total_files_count,
            )
            .await;

            match outcome {
                Ok(()) => {
                    tokio::fs::rename(&part_file_path, &downloadable.path)
                        .await
                        .map_err(|e| {
                            DownloadError::CannotMoveFile(
                                e,
                                part_file_path.to_string_lossy().to_string(),
                                downloadable.path.to_string_lossy().to_string(),
                            )
                        })?;

                    current_files_count.fetch_add(1, Ordering::SeqCst);
                    if let Some(sender) = progress {
                        let _ = sender.send(Progress {
                            total_count: total_files_count,
                            current_count: current_files_count.load(Ordering::SeqCst),
                            total_size: total_files_size,
                            current_size: total_downloaded_size.load(Ordering::SeqCst),
                        });
                    }

                    Ok(())
                }
                Err(e) => {
                    remove_file(&part_file_path).await?;
                    Err(e)
                }
            }
        }
        Err(e) => {
            remove_file(&part_file_path).await?;
            Err(e)
        }
    }
}

#[instrument(skip(options, client, hasher, total_downloaded_size, current_count))]
async fn _download_file(
    url: &str,
    headers: reqwest::header::HeaderMap,
    part_file_path: &PathBuf,
    file: &mut File,
    hasher: &mut Option<HashDigest>,
    downloadable: &Downloadable,
    options: DownloadOptions,
    client: ClientWithMiddleware,
    total_downloaded_size: &AtomicU64,
    file_processed_bytes: &AtomicU64,
    current_count: &AtomicU64,
    total_files_size: u64,
    total_files_count: u64,
) -> Result<(), DownloadError> {
    let mut response = client.get(url).headers(headers).send().await?;

    check_response_status(&response, &downloadable)?;

    download_content(
        &mut response,
        file,
        hasher,
        &options,
        total_downloaded_size,
        file_processed_bytes,
        total_files_size,
        current_count.load(Ordering::SeqCst),
        total_files_count,
    )
    .await?;

    file.flush().await?;

    if let Some(expected_size) = downloadable.size {
        let processed_bytes = file_processed_bytes.load(Ordering::SeqCst);
        if expected_size != processed_bytes {
            return Err(DownloadError::SizeMismatch {
                expected: expected_size,
                actual: processed_bytes,
            });
        }
    }

    if let Some(expected_checksum) = downloadable.checksum.as_ref() {
        let expected_hash = match expected_checksum {
            Checksum::Sha256(hash) | Checksum::Sha1(hash) | Checksum::Md5(hash) => hash,
        };

        let actual_hash = hex::encode(
            hasher
                .as_mut()
                .expect("downloadable checksum is set, but hasher is not set")
                .finalize_reset(),
        );

        if actual_hash != *expected_hash {
            return Err(DownloadError::ChecksumMismatch {
                expected: expected_hash.clone(),
                actual: actual_hash,
                url: downloadable.url.clone(),
                path: downloadable.path.to_string_lossy().to_string(),
            });
        }
    }

    Ok(())
}

async fn remove_file(path: &Path) -> Result<(), DownloadError> {
    tokio::fs::remove_file(path).await.map_err(|e| {
        error!("Failed to remove file: {:?}", e);
        DownloadError::CannotRemoveFile(e.to_string())
    })
}

async fn prepare_download(
    downloadable: &Downloadable,
    force_overwrite: bool,
    downloaded_size: &AtomicU64,
    file_processed_bytes: &AtomicU64,
) -> Result<
    (
        PathBuf,
        File,
        reqwest::header::HeaderMap,
        Option<HashDigest>,
        bool,
    ),
    DownloadError,
> {
    let initial_ext = downloadable
        .path
        .extension()
        .unwrap_or_default()
        .to_os_string();

    let format_ext = if initial_ext.is_empty() {
        OsString::from(PART_POSTFIX)
    } else {
        let mut ext = OsString::new();

        ext.push(initial_ext);
        ext.push(PART_POSTFIX);

        ext
    };

    let part_file_path = downloadable.path.with_extension(format_ext);

    let should_resume = part_file_path.exists() && !force_overwrite;

    // ensure the parent directory exists
    let parent_dir = part_file_path
        .parent()
        .ok_or(DownloadError::CannotCreateDirectory(
            part_file_path.to_string_lossy().to_string(),
        ))?;
    tokio::fs::create_dir_all(parent_dir).await?;

    let mut file = File::options()
        .create(true)
        .write(!should_resume)
        .truncate(force_overwrite)
        .append(should_resume)
        .read(should_resume)
        .open(&part_file_path)
        .await?;

    let mut headers = reqwest::header::HeaderMap::new();
    let mut processed_bytes = 0;

    let mut hasher = match downloadable.checksum {
        Some(Checksum::Sha256(_)) => Some(HashDigest::Sha256(Sha256::new())),
        Some(Checksum::Sha1(_)) => Some(HashDigest::Sha1(sha1::Sha1::new())),
        Some(Checksum::Md5(_)) => Some(HashDigest::Md5(md5::Md5::new())),
        None => None,
    };

    if should_resume {
        let mut buf = vec![0; 8192];
        loop {
            let bytes_read = file.read(&mut buf).await?;
            if bytes_read == 0 {
                break;
            }
            if let Some(hasher) = hasher.as_mut() {
                hasher.update(&buf[..bytes_read]);
            }
            processed_bytes += bytes_read as u64;
        }

        downloaded_size.fetch_add(processed_bytes, Ordering::SeqCst);
        file_processed_bytes.fetch_add(processed_bytes, Ordering::SeqCst);

        headers.insert(
            reqwest::header::RANGE,
            reqwest::header::HeaderValue::from_str(&format!("bytes={}-", processed_bytes)).unwrap(),
        );
    }

    Ok((part_file_path, file, headers, hasher, should_resume))
}

fn check_response_status(
    response: &reqwest::Response,
    file: &Downloadable,
) -> Result<(), DownloadError> {
    if !response.status().is_success() && response.status() != reqwest::StatusCode::PARTIAL_CONTENT
    {
        error!("Failed to download file: HTTP {}", response.status());
        return Err(DownloadError::Non200StatusCode(
            file.clone(),
            response.status().as_u16(),
        ));
    }
    Ok(())
}

#[instrument(skip(response, file, hasher, options, total_downloaded_size))]
async fn download_content(
    response: &mut reqwest::Response,
    file: &mut File,
    hasher: &mut Option<HashDigest>,
    options: &DownloadOptions,
    total_downloaded_size: &AtomicU64,
    file_processed_bytes: &AtomicU64,
    total_size: u64,
    current_files_count: u64,
    total_count: u64,
) -> Result<(), DownloadError> {
    while let Some(chunk) = response.chunk().await? {
        if options.cancel_token.is_cancelled() {
            warn!("Download cancelled");
            return Err(DownloadError::Cancelled);
        }

        file.write_all(&chunk).await?;

        if let Some(hasher) = hasher.as_mut() {
            hasher.update(&chunk);
        }

        let total_downloaded = total_downloaded_size
            .fetch_add(chunk.len() as u64, Ordering::SeqCst)
            + chunk.len() as u64;

        file_processed_bytes.fetch_add(chunk.len() as u64, Ordering::SeqCst);

        if let Some(sender) = &options.progress_sender {
            let _ = sender.send(Progress {
                total_count,
                current_count: current_files_count,
                total_size,
                current_size: total_downloaded,
            });
        }
    }

    Ok(())
}

#[instrument(skip(tasks, progress_sender, downloaded_size))]
async fn process_download_tasks(
    tasks: Vec<tokio::task::JoinHandle<Result<(), DownloadError>>>,
    progress_sender: &Option<tokio::sync::watch::Sender<Progress>>,
    total_count: u64,
    downloaded_size: &AtomicU64,
    total_size: u64,
) -> Result<bool, DownloadError> {
    let mut download_required = false;

    for task in tasks {
        match task.await? {
            Ok(_) => {
                download_required = true;
            }
            Err(e) => {
                error!("File processing failed: {:?}", e);
                return Err(e);
            }
        }
    }

    // Send final progress update
    if let Some(sender) = progress_sender {
        let _ = sender.send(Progress {
            total_count,
            current_count: total_count,
            total_size,
            current_size: downloaded_size.load(Ordering::SeqCst),
        });
    }

    info!("All files processed");
    Ok(download_required)
}

#[instrument(skip(expected_checksum))]
async fn validate_file(
    path: &Path,
    expected_size: Option<u64>,
    expected_checksum: &Option<Checksum>,
    deep_check: bool,
) -> Result<(), DownloadError> {
    if !path.exists() {
        return Err(DownloadError::FileNotFound);
    }

    let metadata = tokio::fs::metadata(path).await?;

    if let Some(size) = expected_size {
        if metadata.len() != size {
            error!(
                "File size mismatch: expected {}, got {}",
                size,
                metadata.len()
            );
            return Err(DownloadError::SizeMismatch {
                expected: size,
                actual: metadata.len(),
            });
        }
    }

    if deep_check {
        if let Some(expected_checksum) = expected_checksum {
            let mut file = File::open(path).await?;
            let mut buffer = vec![0; 8192];

            let mut hasher = HashDigest::from(expected_checksum);

            let actual_hash = {
                loop {
                    let bytes_read = file.read(&mut buffer).await?;
                    if bytes_read == 0 {
                        break;
                    }
                    hasher.update(&buffer[..bytes_read]);
                }
                Ok::<_, DownloadError>(hex::encode(hasher.finalize()))
            }?;

            let expected_hash = match expected_checksum {
                Checksum::Sha256(hash) | Checksum::Sha1(hash) | Checksum::Md5(hash) => hash,
            };

            if actual_hash != *expected_hash {
                error!(
                    "Checksum mismatch: expected {}, got {}",
                    expected_hash, actual_hash
                );
                return Err(DownloadError::ChecksumMismatch {
                    expected: expected_hash.clone(),
                    actual: actual_hash,
                    url: "".to_string(), // We don't have the URL in this context
                    path: path.to_string_lossy().to_string(),
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use tempfile::tempdir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;
    use tokio::sync::watch;

    #[tokio::test]
    async fn test_download_success() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock = server
            .mock("GET", "/test.txt")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body("Hello, World!")
            .expect(1)
            .create_async()
            .await;

        let mock1 = server
            .mock("GET", "/test1.txt")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body("Hello, World!")
            .expect(1)
            .create_async()
            .await;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path1 = temp_dir.path().join("test1.txt");

        let downloadable = Downloadable {
            url: format!("{}/test.txt", mock_url),
            path: file_path.clone(),
            checksum: Some(Checksum::Sha256(
                "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f".to_string(),
            )),
            size: Some(13),
        };

        let downloadable1 = Downloadable {
            url: format!("{}/test1.txt", mock_url),
            path: file_path1.clone(),
            checksum: Some(Checksum::Sha256(
                "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f".to_string(),
            )),
            size: Some(13),
        };

        let options = DownloadOptions::builder().concurrency(2).build();

        let result = download_multiple(&[downloadable, downloadable1], options).await;
        assert!(result.is_ok());
        assert_eq!(std::fs::read_to_string(file_path).unwrap(), "Hello, World!");
        assert_eq!(
            std::fs::read_to_string(file_path1).unwrap(),
            "Hello, World!"
        );

        mock.assert();
        mock1.assert();
    }

    #[tokio::test]
    async fn test_download_network_error() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock = server
            .mock("GET", "/test.txt")
            .expect(4)
            .with_status(500)
            .create_async()
            .await;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let downloadable = Downloadable {
            url: format!("{}/test.txt", mock_url),
            path: file_path,
            checksum: None,
            size: None,
        };

        let options = DownloadOptions::builder()
            .concurrency(1)
            .max_retries(3)
            .build();

        let result = download_multiple(&[downloadable], options).await;
        assert!(result.is_err());

        mock.assert();
    }

    #[tokio::test]
    async fn test_download_cancellation() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock = server
            .mock("GET", "/test.txt")
            .with_status(200)
            .with_header("content-length", "13")
            .with_chunked_body(|w| {
                // Simulate a slow response
                std::thread::sleep(std::time::Duration::from_secs(5));
                w.write_all(b"Hello, World!").unwrap();
                w.flush()
            })
            .create_async()
            .await;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let downloadable = Downloadable {
            url: format!("{}/test.txt", mock_url),
            path: file_path,
            checksum: None,
            size: Some(13),
        };

        let cancel_token = CancellationToken::new();
        let cancel_token_clone = cancel_token.clone();

        let options = DownloadOptions::builder()
            .concurrency(1)
            .cancel_token(cancel_token)
            .build();

        let download_handle =
            tokio::spawn(async move { download_multiple(&[downloadable], options).await });

        // Cancel the download after a short delay
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        cancel_token_clone.cancel();

        let result = download_handle.await.unwrap();
        assert!(matches!(result, Err(DownloadError::Cancelled)));

        mock.assert();
    }

    #[tokio::test]
    async fn test_download_multiple_concurrent_with_same_name_and_path() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock = server
            .mock("GET", "/test.txt")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body("Hello, World!")
            .expect(2)
            .create_async()
            .await;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let downloadable1 = Downloadable {
            url: format!("{}/test.txt", mock_url),
            path: file_path.clone(),
            checksum: None,
            size: None,
        };

        let downloadable2 = Downloadable {
            url: format!("{}/test.txt", mock_url),
            path: file_path.clone(),
            checksum: None,
            size: None,
        };

        let options = DownloadOptions::builder().concurrency(2).build();

        download_multiple(&[downloadable1, downloadable2], options)
            .await
            .unwrap();
        assert_eq!(std::fs::read_to_string(file_path).unwrap(), "Hello, World!");

        mock.assert();
    }

    #[tokio::test]
    async fn test_validate_file_success() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = b"Hello, World!";
        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(content).await.unwrap();
        file.flush().await.unwrap();

        let checksum = Checksum::Sha256(
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f".to_string(),
        );

        validate_file(&file_path, Some(13), &Some(checksum), true)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_validate_file_size_mismatch() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = b"Hello, World!";
        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(content).await.unwrap();
        file.flush().await.unwrap();

        let result = validate_file(&file_path, Some(14), &None, false).await;
        assert!(matches!(result, Err(DownloadError::SizeMismatch { .. })));
    }

    #[tokio::test]
    async fn test_validate_file_checksum_mismatch() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = b"Hello, World!";
        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(content).await.unwrap();
        file.flush().await.unwrap();

        let checksum = Checksum::Sha256("incorrect_hash".to_string());

        let result = validate_file(&file_path, Some(13), &Some(checksum), true).await;
        assert!(matches!(
            result,
            Err(DownloadError::ChecksumMismatch { .. })
        ));
    }

    #[tokio::test]
    async fn test_download_file_resume() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock = server
            .mock("GET", "/test.txt")
            .match_header("range", "bytes=7-")
            .with_status(206)
            .with_header("content-range", "bytes 7-13/13")
            .with_body("World!")
            .expect(1)
            .create_async()
            .await;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let part_path = temp_dir.path().join(format!("test.txt{}", PART_POSTFIX));

        println!("{}", part_path.display());

        // First, download part of the file
        {
            let mut file = File::create(&part_path).await.unwrap();
            file.write_all(b"Hello, ").await.unwrap();
            file.flush().await.unwrap();
        }

        let downloadable = Downloadable {
            url: format!("{}/test.txt", mock_url),
            path: file_path.clone(),
            checksum: Some(Checksum::Sha256(
                "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f".to_string(),
            )),
            size: Some(13),
        };

        let options = DownloadOptions::builder().concurrency(1).build();

        let res = download_multiple(&[downloadable], options).await;

        res.unwrap();
        assert_eq!(std::fs::read_to_string(file_path).unwrap(), "Hello, World!");

        mock.assert();
    }

    #[tokio::test]
    async fn test_download_file_resume_corrupted_part() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock = server
            .mock("GET", "/test.txt")
            .match_header("range", "bytes=7-")
            .with_status(206)
            .with_header("content-range", "bytes 7-13/13")
            .with_body("World!")
            .expect(1)
            .create_async()
            .await;

        let mock_full = server
            .mock("GET", "/test.txt")
            .with_status(200)
            .with_header("content-length", "13")
            .with_body("Hello, World!")
            .expect(1)
            .create_async()
            .await;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let part_path = temp_dir.path().join(format!("test.txt{}", PART_POSTFIX));

        println!("{}", part_path.display());

        // First, download part of the file
        {
            let mut file = File::create(&part_path).await.unwrap();
            file.write_all(b"wrong, ").await.unwrap();
            file.flush().await.unwrap();
        }

        let (progress_tx, mut progress_rx) = watch::channel(Progress::default());

        let downloadable = Downloadable {
            url: format!("{}/test.txt", mock_url),
            path: file_path.clone(),
            checksum: Some(Checksum::Sha256(
                "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f".to_string(),
            )),
            size: Some(13),
        };

        let options = DownloadOptions::builder()
            .concurrency(1)
            .progress_sender(progress_tx)
            .build();

        let result = tokio::spawn(async move { download_multiple(&[downloadable], options).await });

        let mut last_progress = Progress::default();

        let mut has_reset = false;
        let mut final_progress_reached = false;

        while !result.is_finished() {
            tokio::select! {
                _ = progress_rx.changed() => {
                    let progress = progress_rx.borrow().clone();

                    if progress.current_size == 0 && last_progress.current_size == 13 && !has_reset {
                        has_reset = true;
                        continue;
                    } else if progress.current_size == 13 && has_reset {
                        final_progress_reached = true;
                    }

                    last_progress = progress;
                }
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    panic!("Test timed out");
                }
            }
        }

        assert!(final_progress_reached);
        assert!(has_reset);

        assert_eq!(last_progress.current_count, 1);
        assert_eq!(last_progress.current_size, 13);
        assert_eq!(last_progress.total_count, 1);
        assert_eq!(last_progress.total_size, 13);

        result.await.unwrap().unwrap();

        assert_eq!(std::fs::read_to_string(file_path).unwrap(), "Hello, World!");

        mock.assert();
        mock_full.assert();
    }

    #[tokio::test]
    async fn test_download_multiple_concurrent() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock1 = server
            .mock("GET", "/test1.txt")
            .with_status(200)
            .with_body("Hello, World 1!")
            .expect(1)
            .create_async()
            .await;

        let mock2 = server
            .mock("GET", "/test2.txt")
            .with_status(200)
            .with_body("Hello, World 2!")
            .expect(1)
            .create_async()
            .await;

        let mock3 = server
            .mock("GET", "/test3.txt")
            .with_status(200)
            .with_body("Hello, World 3!")
            .expect(1)
            .create_async()
            .await;

        let temp_dir = tempdir().unwrap();
        let file_path1 = temp_dir.path().join("test1.txt");
        let file_path2 = temp_dir.path().join("test2.txt");
        let file_path3 = temp_dir.path().join("test3.txt");

        let downloadable1 = Downloadable {
            url: format!("{}/test1.txt", mock_url),
            path: file_path1.clone(),
            checksum: Some(Checksum::Sha1(
                "83cfe479c149983ad66bb9c0e4b47cf2870e33c6".to_string(),
            )),
            size: Some(15),
        };

        let downloadable2 = Downloadable {
            url: format!("{}/test2.txt", mock_url),
            path: file_path2.clone(),
            checksum: Some(Checksum::Sha256(
                "1aefd92d29317a3119827259177ce85329da746818baf6682d1d455ade4263fd".to_string(),
            )),
            size: Some(15),
        };

        let downloadable3 = Downloadable {
            url: format!("{}/test3.txt", mock_url),
            path: file_path3.clone(),
            checksum: Some(Checksum::Md5(
                "216a3bcccd4c577dcf40e58f19f31367".to_string(),
            )),
            size: Some(15),
        };

        let options = DownloadOptions::builder().concurrency(2).build();

        download_multiple(&[downloadable1, downloadable2, downloadable3], options)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(file_path1).unwrap(),
            "Hello, World 1!"
        );
        assert_eq!(
            std::fs::read_to_string(file_path2).unwrap(),
            "Hello, World 2!"
        );
        assert_eq!(
            std::fs::read_to_string(file_path3).unwrap(),
            "Hello, World 3!"
        );

        mock1.assert();
        mock2.assert();
        mock3.assert();
    }

    #[tokio::test]
    async fn test_download_file_non_200_status() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock = server
            .mock("GET", "/test.txt")
            .with_status(404)
            .expect(1)
            .create_async()
            .await;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let downloadable = Downloadable {
            url: format!("{}/test.txt", mock_url),
            path: file_path,
            checksum: None,
            size: None,
        };

        let options = DownloadOptions::builder().concurrency(1).build();

        let result = download_multiple(&[downloadable], options).await;
        assert!(matches!(
            result,
            Err(DownloadError::Non200StatusCode(_, 404))
        ));

        mock.assert();
    }

    #[tokio::test]
    async fn test_download_file_with_redirect() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let redirect_mock = server
            .mock("GET", "/original.txt")
            .with_status(302)
            .with_header("Location", "/redirected.txt")
            .create_async()
            .await;

        let content_mock = server
            .mock("GET", "/redirected.txt")
            .with_status(200)
            .with_body("Redirected content")
            .create_async()
            .await;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let downloadable = Downloadable {
            url: format!("{}/original.txt", mock_url),
            path: file_path.clone(),
            checksum: None,
            size: None,
        };

        let options = DownloadOptions::builder().concurrency(1).build();

        let result = download_multiple(&[downloadable], options).await;
        assert!(result.is_ok());
        assert_eq!(
            std::fs::read_to_string(file_path).unwrap(),
            "Redirected content"
        );

        redirect_mock.assert();
        content_mock.assert();
    }

    #[tokio::test]
    async fn test_download_large_file() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        const LARGE_FILE_SIZE: usize = 10 * 1024 * 1024; // 10 MB

        let large_response = vec![5; LARGE_FILE_SIZE];

        let mock = server
            .mock("GET", "/large_file.bin")
            .with_status(200)
            .with_header("content-length", &*LARGE_FILE_SIZE.to_string())
            .with_body(large_response)
            .create_async()
            .await;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("large_file.bin");

        let downloadable = Downloadable {
            url: format!("{}/large_file.bin", mock_url),
            path: file_path.clone(),
            checksum: None,
            size: Some(LARGE_FILE_SIZE as u64),
        };

        let options = DownloadOptions::builder().concurrency(1).build();

        let result = download_multiple(&[downloadable], options).await;
        assert!(result.is_ok());

        let metadata = std::fs::metadata(file_path).unwrap();
        assert_eq!(metadata.len(), LARGE_FILE_SIZE as u64);

        mock.assert();
    }

    #[tokio::test]
    async fn test_download_multiple_with_progress() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock1 = server
            .mock("GET", "/test1.txt")
            .with_status(200)
            .with_header("content-length", "13")
            .with_body("Hello, World!")
            .create_async()
            .await;

        let mock2 = server
            .mock("GET", "/test2.txt")
            .with_status(200)
            .with_header("content-length", "15")
            .with_body("Hello, World 2!")
            .create_async()
            .await;

        let temp_dir = tempdir().unwrap();
        let file_path1 = temp_dir.path().join("test1.txt");
        let file_path2 = temp_dir.path().join("test2.txt");

        let downloadable1 = Downloadable {
            url: format!("{}/test1.txt", mock_url),
            path: file_path1.clone(),
            checksum: None,
            size: Some(13),
        };

        let downloadable2 = Downloadable {
            url: format!("{}/test2.txt", mock_url),
            path: file_path2.clone(),
            checksum: None,
            size: Some(15),
        };

        let (progress_tx, mut progress_rx) = watch::channel(Progress::default());

        let options = DownloadOptions::builder()
            .concurrency(2)
            .progress_sender(progress_tx)
            .build();

        let download_handle = tokio::spawn(async move {
            download_multiple(&[downloadable1, downloadable2], options).await
        });

        let mut last_progress = Progress::default();

        while !download_handle.is_finished() {
            tokio::select! {
                _ = progress_rx.changed() => {
                    let progress = progress_rx.borrow().clone();
                    assert!(progress.current_count <= progress.total_count);
                    assert!(progress.current_size <= progress.total_size);
                    assert!(progress.current_size >= last_progress.current_size);
                    assert_eq!(progress.total_count, 2);
                    assert_eq!(progress.total_size, 28);
                    last_progress = progress;
                }
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    panic!("Test timed out");
                }
            }
        }

        let result = download_handle.await.unwrap();
        assert!(result.is_ok());

        assert_eq!(last_progress.current_count, 2);
        assert_eq!(last_progress.current_size, 28);

        assert_eq!(
            std::fs::read_to_string(file_path1).unwrap(),
            "Hello, World!"
        );
        assert_eq!(
            std::fs::read_to_string(file_path2).unwrap(),
            "Hello, World 2!"
        );

        mock1.assert();
        mock2.assert();
    }

    #[tokio::test]
    async fn test_download_with_progress_and_resume() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock = server
            .mock("GET", "/test.txt")
            .match_header("range", "bytes=7-")
            .with_status(206)
            .with_header("content-range", "bytes 7-13/13")
            .with_body("World!")
            .create_async()
            .await;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let part_path = temp_dir.path().join(format!("test.txt{}", PART_POSTFIX));

        // First, download part of the file
        {
            let mut file = File::create(&part_path).await.unwrap();
            file.write_all(b"Hello, ").await.unwrap();
            file.flush().await.unwrap();
        }

        let downloadable = Downloadable {
            url: format!("{}/test.txt", mock_url),
            path: file_path.clone(),
            checksum: None,
            size: Some(13),
        };

        let (progress_tx, mut progress_rx) = watch::channel(Progress::default());

        let options = DownloadOptions::builder()
            .concurrency(1)
            .progress_sender(progress_tx)
            .build();

        println!("{}", format!("{}/test.txt", mock_url));

        download_multiple(&[downloadable], options).await.unwrap();

        let mut last_progress = Progress::default();

        tokio::select! {
            _ = progress_rx.changed() => {
                let progress = progress_rx.borrow().clone();
                assert!(progress.current_count <= progress.total_count);
                assert!(progress.current_size <= progress.total_size);
                assert!(progress.current_size >= last_progress.current_size);
                assert_eq!(progress.total_count, 1);
                assert_eq!(progress.total_size, 13);
                last_progress = progress;
            }
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                panic!("Test timed out");
            }
        }

        assert_eq!(std::fs::read_to_string(file_path).unwrap(), "Hello, World!");

        mock.assert();
    }

    #[tokio::test]
    async fn test_download_cancellation_with_progress() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock = server
            .mock("GET", "/test.txt")
            .with_status(200)
            .with_header("content-length", "1000000000")
            .with_body(vec![b'a'; 1_000_000_000])
            .create_async()
            .await;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let downloadable = Downloadable {
            url: format!("{}/test.txt", mock_url),
            path: file_path,
            checksum: None,
            size: Some(1_000_000_000),
        };

        let (progress_tx, mut progress_rx) = watch::channel(Progress::default());
        let cancel_token = CancellationToken::new();
        let cancel_token_clone = cancel_token.clone();

        let options = DownloadOptions::builder()
            .concurrency(1)
            .cancel_token(cancel_token)
            .progress_sender(progress_tx)
            .build();

        let download_handle =
            tokio::spawn(async move { download_multiple(&[downloadable], options).await });

        let mut received_progress = false;

        while !download_handle.is_finished() {
            tokio::select! {
                _ = progress_rx.changed() => {
                    let progress = progress_rx.borrow().clone();
                    received_progress = true;
                    println!("Progress: {progress:?}");
                    if progress.current_size > 100_000 {
                        cancel_token_clone.cancel();
                        break;
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    panic!("Test timed out");
                }
            }
        }

        let result = download_handle.await.unwrap();
        assert!(matches!(result, Err(DownloadError::Cancelled)));
        assert!(
            received_progress,
            "Expected to receive progress updates before cancellation"
        );

        mock.assert();
    }
}
