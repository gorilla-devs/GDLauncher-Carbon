use md5::Digest as Md5Digest;
use md5::Md5;
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt::Display;
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

/// Maximum time to wait for the next chunk of a response body before treating the connection
/// as stalled. Generous so a slow-but-progressing download is never killed, while a half-open
/// connection that delivers no bytes errors out instead of hanging forever.
const READ_STALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

/// CurseForge requires an API key on direct CDN downloads; unauthenticated requests to
/// `edge.forgecdn.net` are rejected with `401 A valid api-key is required.`
///
/// The key is registered here rather than carried on each [`Downloadable`] so that it is
/// applied at the single point every download request passes through. Resumed range
/// requests reuse the same header map, and no caller can omit it by forgetting to opt in.
static CURSEFORGE_API_KEY: std::sync::OnceLock<reqwest::header::HeaderValue> =
    std::sync::OnceLock::new();

/// Registers the CurseForge API key used to authenticate CDN downloads.
///
/// Called once during app startup: the key lives in the app crate's compile-time
/// environment, which this crate cannot read.
pub fn set_curseforge_api_key(key: &str) -> Result<(), reqwest::header::InvalidHeaderValue> {
    let mut value = reqwest::header::HeaderValue::from_str(key)?;
    value.set_sensitive(true);
    let _ = CURSEFORGE_API_KEY.set(value);
    Ok(())
}

/// Returns the CurseForge API key when `url` points at the enforcing CurseForge CDN host.
///
/// Matches `edge.forgecdn.net` exactly — the only host CurseForge requires the key on.
/// Sibling hosts (`media.forgecdn.net` images, `mediafilez.forgecdn.net` files) are open
/// and stay keyless on direct fetches. A download redirected away from `edge` still
/// carries the key to the redirect target, since reqwest re-sends custom headers on
/// redirects it follows.
///
/// Public so that callers downloading CurseForge files outside this crate's download
/// pipeline (e.g. streamed server-pack fetches) can attach the same `x-api-key` header.
pub fn curseforge_cdn_auth(url: &str) -> Option<reqwest::header::HeaderValue> {
    let parsed = reqwest::Url::parse(url).ok()?;

    if parsed.host_str()? != "edge.forgecdn.net" {
        return None;
    }

    CURSEFORGE_API_KEY.get().cloned()
}

/// Base header map for a download request to `url`: carries the CurseForge API key
/// when the target is a CurseForge CDN host, and is empty otherwise. Resume logic
/// layers the `Range` header on top of this map.
fn download_headers(url: &str) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();

    if let Some(api_key) = curseforge_cdn_auth(url) {
        headers.insert("x-api-key", api_key);
    }

    headers
}

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
    FileNotFound(String),
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
    Sha512(String),
    Md5(String),
}

enum HashDigest {
    Sha256(Sha256),
    Sha512(Sha512),
    Sha1(sha1::Sha1),
    Md5(md5::Md5),
}

impl HashDigest {
    fn update(&mut self, data: &[u8]) {
        match self {
            HashDigest::Sha256(h) => h.update(data),
            HashDigest::Sha512(h) => h.update(data),
            HashDigest::Sha1(h) => h.update(data),
            HashDigest::Md5(h) => h.update(data),
        }
    }

    fn finalize(self) -> Vec<u8> {
        match self {
            HashDigest::Sha256(h) => h.finalize().to_vec(),
            HashDigest::Sha512(h) => h.finalize().to_vec(),
            HashDigest::Sha1(h) => h.finalize().to_vec(),
            HashDigest::Md5(h) => h.finalize().to_vec(),
        }
    }

    fn finalize_reset(&mut self) -> Vec<u8> {
        match self {
            HashDigest::Sha256(h) => h.finalize_reset().to_vec(),
            HashDigest::Sha512(h) => h.finalize_reset().to_vec(),
            HashDigest::Sha1(h) => h.finalize_reset().to_vec(),
            HashDigest::Md5(h) => h.finalize_reset().to_vec(),
        }
    }
}

impl From<&Checksum> for HashDigest {
    fn from(value: &Checksum) -> Self {
        match value {
            Checksum::Sha256(_) => HashDigest::Sha256(Sha256::new()),
            Checksum::Sha512(_) => HashDigest::Sha512(Sha512::new()),
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
    // only one file with the same path can be downloaded concurrently to avoid race conditions
    let file_path_lock = Arc::new(FilePathLock::new());

    // Files with an unknown size contribute nothing to this total up front; their real
    // size is added once known (Content-Length or streamed bytes) so downloaded bytes
    // can never exceed the total and progress can never pass 100%.
    let total_files_size = Arc::new(AtomicU64::new(files.iter().filter_map(|f| f.size).sum()));
    let total_downloaded_size = Arc::new(AtomicU64::new(0));
    let total_files_count = files.len() as u64;
    let current_files_count = Arc::new(AtomicU64::new(0));

    info!(
        "Starting processing of {} files - {} MB",
        files.len(),
        total_files_size.load(Ordering::SeqCst) / 1024 / 1024
    );

    if let Some(sender) = &options.progress_sender {
        let _ = sender.send(Progress {
            total_count: total_files_count,
            current_count: 0,
            total_size: total_files_size.load(Ordering::SeqCst),
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
        &total_files_size,
        total_files_count,
        &file_path_lock,
    );

    process_download_tasks(
        tasks,
        options.only_validate,
        &options.progress_sender,
        total_files_count,
        &total_files_size,
    )
    .await
}

fn create_client(options: &DownloadOptions) -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(options.max_retries);
    let client = Client::builder()
        // Bound connection establishment so a route that never completes the handshake
        // (captive portal, dropped route) errors out instead of hanging forever.
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");
    ClientBuilder::new(client)
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
    total_files_size: &Arc<AtomicU64>,
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
            let total_downloaded_size = Arc::clone(total_downloaded_size);
            let current_files_count = Arc::clone(current_files_count);
            let total_files_size = Arc::clone(total_files_size);
            let file_path_lock = Arc::clone(file_path_lock);

            tokio::spawn(async move {
                let _permit = semaphore.acquire().await?;
                process_file(
                    file,
                    options,
                    client,
                    &total_downloaded_size,
                    &current_files_count,
                    &total_files_size,
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
    downloadable: Downloadable,
    options: DownloadOptions,
    client: ClientWithMiddleware,
    total_downloaded_size: &AtomicU64,
    current_files_count: &AtomicU64,
    total_files_size: &AtomicU64,
    total_files_count: u64,
    file_path_lock: &FilePathLock,
) -> Result<(), DownloadError> {
    let validation = validate_file(
        &downloadable.path,
        downloadable.size,
        &downloadable.checksum,
        options.deep_check,
    )
    .await;

    match validation {
        Err(err) if options.only_validate => {
            return Err(err);
        }
        Err(_) => {
            download_file(
                downloadable,
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
            if let Some(size) = downloadable.size {
                total_downloaded_size.fetch_add(size, Ordering::SeqCst);
            }
            current_files_count.fetch_add(1, Ordering::SeqCst);
            if let Some(sender) = &options.progress_sender {
                let progress = Progress {
                    total_count: total_files_count,
                    current_count: current_files_count.load(Ordering::SeqCst),
                    total_size: total_files_size.load(Ordering::SeqCst),
                    current_size: total_downloaded_size.load(Ordering::SeqCst),
                };

                let _ = sender.send(progress);
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
    total_files_size: &AtomicU64,
    total_files_count: u64,
    file_path_lock: &FilePathLock,
) -> Result<(), DownloadError> {
    // This only locks for the acquisition of the file lock
    let file_lock = file_path_lock.lock(&downloadable.path).await;
    // acquire the lock for the file so no other file with the same path can be downloaded concurrently
    let _guard = file_lock.lock().await;

    let file_processed_bytes = AtomicU64::new(0);
    // Bytes this file contributed to the shared total (for unknown-size files), so a
    // retry can subtract them again instead of inflating the total.
    let file_added_to_total = AtomicU64::new(0);

    let progress = options.progress_sender.as_ref();

    let (part_file_path, mut file, headers, mut hasher, was_resumed) = prepare_download(
        &downloadable,
        false,
        total_downloaded_size,
        &file_processed_bytes,
        current_files_count,
        total_files_size,
        total_files_count,
        progress,
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
        &file_added_to_total,
        current_files_count,
        total_files_size,
        total_files_count,
    )
    .await;

    if let Some(sender) = progress {
        let _ = sender.send(Progress {
            total_count: total_files_count,
            current_count: current_files_count.load(Ordering::SeqCst),
            total_size: total_files_size.load(Ordering::SeqCst),
            current_size: total_downloaded_size.load(Ordering::SeqCst),
        });
    }

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
                    total_size: total_files_size.load(Ordering::SeqCst),
                    current_size: total_downloaded_size.load(Ordering::SeqCst),
                });
            }

            Ok(())
        }
        Err(DownloadError::ChecksumMismatch { .. }) | Err(DownloadError::SizeMismatch { .. })
            if was_resumed =>
        {
            tracing::warn!("Download was resumed, but checksum or size mismatched");

            total_downloaded_size.fetch_sub(
                file_processed_bytes.load(Ordering::SeqCst),
                Ordering::SeqCst,
            );

            file_processed_bytes.store(0, Ordering::SeqCst);

            total_files_size.fetch_sub(
                file_added_to_total.swap(0, Ordering::SeqCst),
                Ordering::SeqCst,
            );

            if let Some(sender) = progress {
                let _ = sender.send(Progress {
                    total_count: total_files_count,
                    current_count: current_files_count.load(Ordering::SeqCst),
                    total_size: total_files_size.load(Ordering::SeqCst),
                    current_size: total_downloaded_size.load(Ordering::SeqCst),
                });
            }

            let (part_file_path, mut file, headers, mut hasher, _) = prepare_download(
                &downloadable,
                true,
                total_downloaded_size,
                &file_processed_bytes,
                current_files_count,
                total_files_size,
                total_files_count,
                progress,
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
                &file_added_to_total,
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
                            total_size: total_files_size.load(Ordering::SeqCst),
                            current_size: total_downloaded_size.load(Ordering::SeqCst),
                        });
                    }

                    Ok(())
                }
                Err(e) => {
                    error!(
                        "Error downloading file {} - {}: {:?}",
                        part_file_path.to_string_lossy().to_string(),
                        downloadable.path.to_string_lossy().to_string(),
                        e
                    );
                    remove_file(&part_file_path).await?;
                    Err(e)
                }
            }
        }
        Err(e) => {
            error!(
                "Error downloading file {} - {}: {:?}",
                part_file_path.to_string_lossy().to_string(),
                downloadable.path.to_string_lossy().to_string(),
                e
            );
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
    file_added_to_total: &AtomicU64,
    current_count: &AtomicU64,
    total_files_size: &AtomicU64,
    total_files_count: u64,
) -> Result<(), DownloadError> {
    let resume_requested = headers.contains_key(reqwest::header::RANGE);

    let mut response = client.get(url).headers(headers).send().await?;

    check_response_status(&response, &downloadable)?;

    // If we asked to resume with a Range header but the server returned the full body
    // (200 instead of 206 Partial Content), the bytes already on disk must be discarded:
    // appending the full response would corrupt the file, and with no size/checksum the
    // corruption would go undetected. Discard the partial bytes and restart from scratch.
    if resume_requested && response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
        warn!(
            "Server ignored range request for `{}` (status {}); restarting download from scratch",
            url,
            response.status()
        );

        let resumed_bytes = file_processed_bytes.swap(0, Ordering::SeqCst);
        total_downloaded_size.fetch_sub(resumed_bytes, Ordering::SeqCst);

        *file = File::options()
            .create(true)
            .write(true)
            .truncate(true)
            .open(part_file_path)
            .await?;

        // Reset the running hash for the restarted download, mirroring the initial hasher
        // (`From<&Checksum> for HashDigest`).
        *hasher = downloadable.checksum.as_ref().map(HashDigest::from);
    }

    // Unknown-size files aren't part of the shared total yet: account them now from
    // Content-Length (plus any resumed prefix), or chunk-by-chunk when the response
    // doesn't declare a length, so the total always covers every byte being counted.
    let mut count_chunks_into_total = false;
    if downloadable.size.is_none() {
        match response.content_length() {
            Some(remaining) => {
                let full_size = file_processed_bytes.load(Ordering::SeqCst) + remaining;
                file_added_to_total.fetch_add(full_size, Ordering::SeqCst);
                total_files_size.fetch_add(full_size, Ordering::SeqCst);
            }
            None => {
                // The resumed prefix is already in the downloaded counter; mirror it
                // into the total, since the chunk counting below only covers new bytes.
                let prefix = file_processed_bytes.load(Ordering::SeqCst);
                if prefix > 0 {
                    file_added_to_total.fetch_add(prefix, Ordering::SeqCst);
                    total_files_size.fetch_add(prefix, Ordering::SeqCst);
                }
                count_chunks_into_total = true;
            }
        }
    }

    download_content(
        &mut response,
        file,
        hasher,
        &options,
        total_downloaded_size,
        file_processed_bytes,
        file_added_to_total,
        count_chunks_into_total,
        total_files_size,
        current_count,
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
            Checksum::Sha256(hash)
            | Checksum::Sha512(hash)
            | Checksum::Sha1(hash)
            | Checksum::Md5(hash) => hash,
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
    total_downloaded_size: &AtomicU64,
    file_processed_bytes: &AtomicU64,
    current_files_count: &AtomicU64,
    total_files_size: &AtomicU64,
    total_files_count: u64,
    progress: Option<&Sender<Progress>>,
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

    let mut headers = download_headers(&downloadable.url);

    let mut processed_bytes = 0;

    let mut hasher = match downloadable.checksum {
        Some(Checksum::Sha256(_)) => Some(HashDigest::Sha256(Sha256::new())),
        Some(Checksum::Sha512(_)) => Some(HashDigest::Sha512(Sha512::new())),
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

            if let Some(progress) = progress {
                let _progress = Progress {
                    total_count: total_files_count,
                    current_count: current_files_count.load(Ordering::SeqCst),
                    total_size: total_files_size.load(Ordering::SeqCst),
                    current_size: total_downloaded_size.load(Ordering::SeqCst),
                };

                let _ = progress.send(_progress);
            }
        }

        total_downloaded_size.fetch_add(processed_bytes, Ordering::SeqCst);
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
    file_added_to_total: &AtomicU64,
    count_chunks_into_total: bool,
    total_size: &AtomicU64,
    current_files_count: &AtomicU64,
    total_count: u64,
) -> Result<(), DownloadError> {
    loop {
        let chunk = match tokio::time::timeout(READ_STALL_TIMEOUT, response.chunk()).await {
            Ok(result) => result?,
            Err(_) => {
                return Err(DownloadError::GenericDownload(
                    "download stalled: no data received before the read timeout".to_string(),
                ));
            }
        };
        let Some(chunk) = chunk else { break };

        if options.cancel_token.is_cancelled() {
            warn!("Download cancelled");
            return Err(DownloadError::Cancelled);
        }

        file.write_all(&chunk).await?;

        if let Some(hasher) = hasher.as_mut() {
            hasher.update(&chunk);
        }

        // Total before downloaded, so a concurrent progress send between the two
        // can never observe current_size > total_size
        if count_chunks_into_total {
            file_added_to_total.fetch_add(chunk.len() as u64, Ordering::SeqCst);
            total_size.fetch_add(chunk.len() as u64, Ordering::SeqCst);
        }

        total_downloaded_size.fetch_add(chunk.len() as u64, Ordering::SeqCst);
        file_processed_bytes.fetch_add(chunk.len() as u64, Ordering::SeqCst);

        if let Some(sender) = &options.progress_sender {
            let progress = Progress {
                total_count,
                current_count: current_files_count.load(Ordering::SeqCst),
                total_size: total_size.load(Ordering::SeqCst),
                current_size: total_downloaded_size.load(Ordering::SeqCst),
            };

            let _ = sender.send(progress);
        }
    }

    Ok(())
}

#[instrument(skip(tasks, progress_sender))]
async fn process_download_tasks(
    tasks: Vec<tokio::task::JoinHandle<Result<(), DownloadError>>>,
    only_validate: bool,
    progress_sender: &Option<tokio::sync::watch::Sender<Progress>>,
    total_count: u64,
    total_size: &AtomicU64,
) -> Result<bool, DownloadError> {
    for task in tasks {
        match task.await? {
            Ok(_) => {}
            Err(
                DownloadError::SizeMismatch { .. }
                | DownloadError::ChecksumMismatch { .. }
                | DownloadError::FileNotFound(_),
            ) if only_validate => {
                return Ok(true);
            }
            Err(e) => {
                error!("File processing failed: {:?}", e);
                return Err(e);
            }
        }
    }

    // Send final progress update
    if let Some(sender) = progress_sender {
        let total_size = total_size.load(Ordering::SeqCst);
        let progress = Progress {
            total_count,
            current_count: total_count,
            total_size,
            current_size: total_size,
        };

        let _ = sender.send(progress);
    }

    info!("All files processed");
    Ok(false)
}

#[instrument(skip(expected_checksum))]
async fn validate_file(
    path: &Path,
    expected_size: Option<u64>,
    expected_checksum: &Option<Checksum>,
    deep_check: bool,
) -> Result<(), DownloadError> {
    if !path.exists() {
        return Err(DownloadError::FileNotFound(
            path.to_string_lossy().to_string(),
        ));
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
                Checksum::Sha256(hash)
                | Checksum::Sha512(hash)
                | Checksum::Sha1(hash)
                | Checksum::Md5(hash) => hash,
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
    use super::*;
    use std::time::Duration;
    use tempfile::tempdir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;
    use tokio::sync::watch;
    use tracing_test::traced_test;

    #[test]
    fn curseforge_key_is_sent_only_to_the_edge_cdn_host() {
        set_curseforge_api_key("test-api-key").unwrap();

        assert!(
            curseforge_cdn_auth("https://edge.forgecdn.net/files/3272/32/jei.jar").is_some(),
            "expected CurseForge key for edge.forgecdn.net"
        );

        for url in [
            // Open CurseForge hosts: enforcement applies to edge only.
            "https://mediafilez.forgecdn.net/files/3272/32/jei.jar",
            "https://media.forgecdn.net/files/3272/32/jei.jar",
            "https://forgecdn.net/files/3272/32/jei.jar",
            "https://cdn.modrinth.com/data/AABBCCDD/versions/1/mod.jar",
            "https://piston-data.mojang.com/v1/objects/abc/client.jar",
            "https://notforgecdn.net/files/3272/32/jei.jar",
            "https://evil.example.com/?x=edge.forgecdn.net",
            "https://forgecdn.net@edge.forgecdn.net.evil.com/files/1/2/x.jar",
            "not a url at all",
        ] {
            assert!(
                curseforge_cdn_auth(url).is_none(),
                "did not expect CurseForge key for {url}"
            );
        }
    }

    #[test]
    fn download_headers_carry_the_curseforge_key_for_cdn_urls() {
        set_curseforge_api_key("test-api-key").unwrap();

        let headers = download_headers("https://edge.forgecdn.net/files/3272/32/jei.jar");
        assert_eq!(
            headers.get("x-api-key").map(|v| v.to_str().unwrap()),
            Some("test-api-key")
        );

        let headers = download_headers("https://cdn.modrinth.com/data/AABBCCDD/versions/1/mod.jar");
        assert!(headers.get("x-api-key").is_none());
    }

    #[tokio::test]
    #[traced_test]
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
    #[traced_test]
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
    #[traced_test]
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
    #[traced_test]
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
    #[traced_test]
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
    #[traced_test]
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
    #[traced_test]
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
    #[traced_test]
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
    #[traced_test]
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

        while !result.is_finished() {
            tokio::select! {
                _ = progress_rx.changed() => {
                    let progress = progress_rx.borrow().clone();
                    last_progress = progress;
                }
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    panic!("Test timed out");
                }
            }
        }

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
    #[traced_test]
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
    #[traced_test]
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
    #[traced_test]
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
    #[traced_test]
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
    #[traced_test]
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

    // Files with an unknown size used to contribute bytes to current_size but nothing to
    // total_size, letting progress exceed 100%. The total must instead grow once the
    // real size is learned, and current_size must never pass it.
    #[tokio::test]
    #[traced_test]
    async fn test_download_progress_with_unknown_size_never_exceeds_total() {
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock1 = server
            .mock("GET", "/known.txt")
            .with_status(200)
            .with_header("content-length", "13")
            .with_body("Hello, World!")
            .create_async()
            .await;

        let mock2 = server
            .mock("GET", "/unknown.txt")
            .with_status(200)
            .with_header("content-length", "15")
            .with_body("Hello, World 2!")
            .create_async()
            .await;

        let temp_dir = tempdir().unwrap();
        let file_path1 = temp_dir.path().join("known.txt");
        let file_path2 = temp_dir.path().join("unknown.txt");

        let downloadable1 = Downloadable {
            url: format!("{}/known.txt", mock_url),
            path: file_path1.clone(),
            checksum: None,
            size: Some(13),
        };

        let downloadable2 = Downloadable {
            url: format!("{}/unknown.txt", mock_url),
            path: file_path2.clone(),
            checksum: None,
            size: None,
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
                    assert!(progress.total_size >= last_progress.total_size);
                    // 13 until the unknown file's Content-Length is learned, 28 after
                    assert!(progress.total_size == 13 || progress.total_size == 28);
                    last_progress = progress;
                }
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    panic!("Test timed out");
                }
            }
        }

        let result = download_handle.await.unwrap();
        assert!(result.is_ok());

        assert_eq!(last_progress.total_size, 28);
        assert_eq!(last_progress.current_size, 28);

        mock1.assert();
        mock2.assert();
    }

    #[tokio::test]
    #[traced_test]
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

        let last_progress = Progress::default();

        tokio::select! {
            _ = progress_rx.changed() => {
                let progress = progress_rx.borrow().clone();
                assert!(progress.current_count <= progress.total_count);
                assert!(progress.current_size <= progress.total_size);
                assert!(progress.current_size >= last_progress.current_size);
                assert_eq!(progress.total_count, 1);
                assert_eq!(progress.total_size, 13);
            }
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                panic!("Test timed out");
            }
        }

        assert_eq!(std::fs::read_to_string(file_path).unwrap(), "Hello, World!");

        mock.assert();
    }

    #[tokio::test]
    #[traced_test]
    async fn test_download_resume_full_response_restarts() {
        // A leftover .part file exists, but the server ignores our Range request and
        // replies 200 with the full body. The partial bytes must be discarded and the
        // file restarted; otherwise the full body would be appended after them and, with
        // no size or checksum to validate against, the corruption would go undetected.
        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        let mock = server
            .mock("GET", "/test.txt")
            .with_status(200)
            .with_body("Hello, World!")
            .create_async()
            .await;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let part_path = temp_dir.path().join(format!("test.txt{}", PART_POSTFIX));

        // Leftover partial download from a previous interrupted attempt.
        {
            let mut file = File::create(&part_path).await.unwrap();
            file.write_all(b"Hello, ").await.unwrap();
            file.flush().await.unwrap();
        }

        let downloadable = Downloadable {
            url: format!("{}/test.txt", mock_url),
            path: file_path.clone(),
            checksum: None,
            size: None,
        };

        let (progress_tx, _progress_rx) = watch::channel(Progress::default());
        let options = DownloadOptions::builder()
            .concurrency(1)
            .progress_sender(progress_tx)
            .build();

        download_multiple(&[downloadable], options).await.unwrap();

        // Restarted from scratch: exactly the server body, not the partial bytes followed
        // by the full body.
        assert_eq!(
            std::fs::read_to_string(&file_path).unwrap(),
            "Hello, World!"
        );

        mock.assert();
    }

    #[tokio::test]
    #[traced_test]
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
