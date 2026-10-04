use md5::Digest as Md5Digest;
use md5::Md5;
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fmt::Display;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex as SyncMutex, OnceLock, Weak};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::sync::watch::Sender;
use tokio_util::sync::CancellationToken;
use tracing::{error, instrument, warn};

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
    #[error("Invalid partial response range")]
    InvalidRange,
    #[error("Retained partial requires a full restart")]
    RestartPartial,
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
    #[error("{original}; failed to remove owned partial {path:?}: {cleanup}")]
    OwnedPartialCleanup {
        #[source]
        original: Box<DownloadError>,
        path: PathBuf,
        cleanup: std::io::Error,
    },
}

#[derive(Debug, Clone)]
pub enum Checksum {
    Sha1(String),
    Sha256(String),
    Sha512(String),
    Md5(String),
}

#[derive(Clone)]
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
    /// Ordered fallback URLs after `url`. Empty preserves single-URL behavior.
    pub mirrors: Vec<String>,
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
            mirrors: Vec::new(),
            path: path.as_ref().into(),
            checksum: None,
            size: None,
        }
    }

    /// Add ordered fallback candidates. Each is tried at most once after the
    /// primary URL's bounded resume repair; middleware retries remain bounded by
    /// DownloadOptions. Integrity and destination ownership span all candidates.
    pub fn with_mirrors(mut self, mirrors: Vec<String>) -> Self {
        self.mirrors = mirrors;
        self
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

    /// Builds a report whose `current_size` never exceeds `total_size`.
    ///
    /// The two figures live in separate atomics and cannot be read as a pair, so
    /// a reader can always catch the window between a resumed file's prefix
    /// landing in the downloaded counter and the matching contribution reaching
    /// the total a round trip later. Ordering the writes narrows that window but
    /// cannot close it, so the invariant is enforced here, where the value is
    /// built and both figures are finally in hand.
    pub fn sized(total_count: u64, current_count: u64, total_size: u64, current_size: u64) -> Self {
        Self {
            total_count,
            current_count,
            total_size,
            current_size: current_size.min(total_size),
        }
    }
}

// Weak entries are retired only under this mutex, so holders and waiters always
// share one gate. Canonical parents unify relative paths and existing parent links.
// External renames and other processes are outside this in-process ownership contract.
static DESTINATIONS: OnceLock<SyncMutex<HashMap<PathBuf, Weak<Mutex<()>>>>> = OnceLock::new();

#[derive(Clone, Copy)]
enum DestinationPolicy {
    Canonical,
    CaseInsensitive,
}
impl DestinationPolicy {
    fn native() -> Self {
        if cfg!(any(windows, target_os = "macos")) {
            Self::CaseInsensitive
        } else {
            Self::Canonical
        }
    }
}
async fn destination_gate(
    path: &Path,
    policy: DestinationPolicy,
) -> Result<Arc<Mutex<()>>, DownloadError> {
    let key = {
        let absolute = if path.is_absolute() {
            path.to_owned()
        } else {
            std::env::current_dir()?.join(path)
        };
        let mut ancestor = absolute
            .parent()
            .ok_or_else(|| DownloadError::CannotCreateDirectory(path.display().to_string()))?;
        let mut suffix = Vec::new();
        // Off the runtime thread: the workers of a batch share one task, so a
        // blocking resolution here would run every file's one after another.
        let canonical = loop {
            match tokio::fs::canonicalize(ancestor).await {
                Ok(parent) => break parent,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    suffix.push(ancestor.file_name().ok_or(e)?.to_owned());
                    ancestor = ancestor.parent().ok_or_else(|| {
                        DownloadError::CannotCreateDirectory(path.display().to_string())
                    })?;
                }
                // Some Windows volumes (RAM disks, drives the Mount Manager
                // does not map) cannot report a final path. The key is only a
                // contention optimization, so the lexical path stands in; the
                // download itself still reports any real filesystem error.
                Err(_) => break ancestor.to_owned(),
            }
        };
        let mut key = canonical;
        for component in suffix.into_iter().rev() {
            key.push(component);
        }
        key.push(
            absolute
                .file_name()
                .ok_or_else(|| DownloadError::CannotCreateDirectory(path.display().to_string()))?,
        );
        match policy {
            DestinationPolicy::Canonical => key,
            // This is a contention optimization, not a claim of native inode identity.
            // Private partial inodes and the publication boundary protect aliases that
            // a lexical/canonical-parent key cannot recognize.
            DestinationPolicy::CaseInsensitive => {
                PathBuf::from(key.to_string_lossy().to_lowercase())
            }
        }
    };
    let mut gates = DESTINATIONS.get_or_init(Default::default).lock().unwrap();
    gates.retain(|_, gate| gate.strong_count() != 0);
    if let Some(gate) = gates.get(&key).and_then(Weak::upgrade) {
        return Ok(gate);
    }
    let gate = Arc::new(Mutex::new(()));
    gates.insert(key, Arc::downgrade(&gate));
    Ok(gate)
}

// Shared only while selecting a resume snapshot, validating/publishing a complete
// artifact or removing spent leftovers. Never held through a transfer. Unknown
// physical aliases cannot publish simultaneously, no worker appends to an inode
// another worker can publish, and no leftover is removed while one is picked.
static FILE_BOUNDARY: Mutex<()> = Mutex::const_new(());
static ACTIVE_PARTIALS: OnceLock<SyncMutex<HashSet<OsString>>> = OnceLock::new();

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PartialStage {
    Created,
    SnapshotWritten,
    Preparing,
    Publishing,
}
#[cfg(test)]
struct PartialPause {
    stage: PartialStage,
    entered: tokio::sync::Notify,
    release: tokio::sync::Semaphore,
}

/// A resume leftover found next to the destination when an attempt started: the
/// 2.0.40 `<file>.__gdl_part~`, or an owned partial no worker of this process
/// holds (a killed or crashed run left it). Its length and modification time are
/// recorded so a later removal can tell that nothing wrote to it since.
struct Leftover {
    path: PathBuf,
    len: u64,
    modified: Option<std::time::SystemTime>,
    legacy: bool,
}
impl Leftover {
    fn new(path: PathBuf, metadata: &std::fs::Metadata, legacy: bool) -> Self {
        Self {
            path,
            len: metadata.len(),
            modified: metadata.modified().ok(),
            legacy,
        }
    }
}

/// Removes leftovers this destination no longer needs: call it under
/// FILE_BOUNDARY, once the destination validated or a resumed attempt proved the
/// copied bytes unusable. A leftover that changed since it was picked may have a
/// writer this process cannot see, so it stays. Removal is best effort: a
/// leftover that stays costs disk space, never a download.
async fn discard_leftovers<'a>(leftovers: impl IntoIterator<Item = &'a Leftover>) {
    for leftover in leftovers {
        let unchanged = match tokio::fs::symlink_metadata(&leftover.path).await {
            Ok(metadata) => {
                metadata.is_file()
                    && metadata.len() == leftover.len
                    && metadata.modified().ok() == leftover.modified
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                warn!(path = ?leftover.path, ?error, "Cannot inspect download leftover; keeping it");
                continue;
            }
        };
        if !unchanged {
            warn!(path = ?leftover.path, "Download leftover changed since it was picked; keeping it");
            continue;
        }
        match tokio::fs::remove_file(&leftover.path).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                warn!(path = ?leftover.path, ?error, "Failed to remove download leftover");
            }
        }
    }
}

struct OwnedPartial {
    path: PathBuf,
    name: OsString,
    /// Every leftover seen for this destination when the attempt started.
    leftovers: Vec<Leftover>,
    /// Index into `leftovers` of the one copied into this partial.
    source: Option<usize>,
}
impl OwnedPartial {
    /// Drops the leftover this partial was copied from once a resumed attempt
    /// proved its bytes do not belong to the file now served, so it cannot make
    /// every later attempt resume from the same wrong bytes.
    async fn discard_stale_source(&self, options: &DownloadOptions) {
        let Some(source) = self.source.map(|index| &self.leftovers[index]) else {
            return;
        };
        let _boundary = tokio::select! {
            biased;
            _ = options.cancel_token.cancelled() => return,
            guard = FILE_BOUNDARY.lock() => guard,
        };
        discard_leftovers([source]).await;
    }

    // Only call after every handle and entered IO for this newly created inode
    // has settled. Leftovers never go through here: only discard_leftovers
    // removes them, and only once they are proven unneeded.
    async fn cleanup_error(&self, original: DownloadError) -> DownloadError {
        match tokio::fs::remove_file(&self.path).await {
            Ok(()) => original,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => original,
            Err(cleanup) => {
                // Also record secondary worker cleanup failures when the batch
                // must preserve another worker's earlier triggering error.
                warn!(path = ?self.path, error = ?cleanup, triggering_error = ?original,
                    "Failed to remove closed owned partial; artifact remains");
                DownloadError::OwnedPartialCleanup {
                    original: Box::new(original),
                    path: self.path.clone(),
                    cleanup,
                }
            }
        }
    }
}
impl Drop for OwnedPartial {
    fn drop(&mut self) {
        ACTIVE_PARTIALS
            .get_or_init(Default::default)
            .lock()
            .unwrap()
            .remove(&self.name);
    }
}
fn legacy_partial(path: &Path) -> PathBuf {
    let mut extension = path.extension().unwrap_or_default().to_os_string();
    extension.push(PART_POSTFIX);
    path.with_extension(extension)
}

async fn owned_partial(
    download: &Downloadable,
    options: &DownloadOptions,
) -> Result<OwnedPartial, DownloadError> {
    let parent = download
        .path
        .parent()
        .ok_or_else(|| DownloadError::CannotCreateDirectory(download.path.display().to_string()))?;
    tokio::fs::create_dir_all(parent).await?;
    let boundary = tokio::select! {
        biased;
        _ = options.cancel_token.cancelled() => return Err(DownloadError::Cancelled),
        guard = FILE_BOUNDARY.lock() => guard,
    };
    check_cancel(options)?;
    // Gate normalization may conservatively coalesce distinct names. Resume bytes
    // must retain exact filename identity, including non-UTF8 platform encodings.
    let filename = download.path.file_name().unwrap();
    let prefix = format!(
        ".__gdl_owned_part_{}-",
        hex::encode(Sha256::digest(filename.as_encoded_bytes()))
    );
    let mut leftovers = Vec::new();
    let legacy = legacy_partial(&download.path);
    match tokio::fs::symlink_metadata(&legacy).await {
        Ok(metadata) if metadata.is_file() => {
            leftovers.push(Leftover::new(legacy, &metadata, true))
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let mut entries = tokio::fs::read_dir(parent).await?;
    while let Some(entry) = entries.next_entry().await? {
        check_cancel(options)?;
        let name = entry.file_name();
        if !name.to_string_lossy().starts_with(&prefix)
            || ACTIVE_PARTIALS
                .get_or_init(Default::default)
                .lock()
                .unwrap()
                .contains(&name)
        {
            continue;
        }
        // Does not follow links, so only a regular file can become a source. Read
        // from the file itself, as discard_leftovers compares against it later:
        // Windows can report a directory entry's cached size and time instead.
        let path = entry.path();
        match tokio::fs::symlink_metadata(&path).await {
            Ok(metadata) if metadata.is_file() => {
                leftovers.push(Leftover::new(path, &metadata, false))
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    // A leftover longer than the known size cannot be a prefix of this file: resuming
    // from it could only end in a refused range. The 2.0.40 partial is preferred, as
    // before; otherwise the largest owned leftover.
    let usable = |leftover: &Leftover| download.size.is_none_or(|size| leftover.len <= size);
    let source = leftovers
        .iter()
        .position(|leftover| leftover.legacy && usable(leftover))
        .or_else(|| {
            leftovers
                .iter()
                .enumerate()
                .filter(|(_, leftover)| !leftover.legacy && usable(leftover))
                .max_by_key(|(_, leftover)| leftover.len)
                .map(|(index, _)| index)
        });
    let (owned, mut destination) = loop {
        check_cancel(options)?;
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let name = OsString::from(format!(
            "{prefix}{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let path = parent.join(&name);
        match File::options()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
        {
            Ok(file) => {
                ACTIVE_PARTIALS
                    .get_or_init(Default::default)
                    .lock()
                    .unwrap()
                    .insert(name.clone());
                break (
                    OwnedPartial {
                        path,
                        name,
                        leftovers,
                        source,
                    },
                    file,
                );
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    };
    drop(boundary);
    #[cfg(test)]
    options.pause_partial(PartialStage::Created).await;
    // A stale-looking name or vanished owner is not handle-closure proof. Copy a
    // bounded prefix into OUR create_new inode, never import/append the source inode.
    // The original stays until discard_leftovers proves it unneeded; eventual
    // size/hash/range checks still apply.
    let snapshot = async {
        if let Some(source) = owned.source.map(|index| &owned.leftovers[index]) {
            let input = match File::open(&source.path).await {
                Ok(input) => input,
                // A worker publishing this file under an alias the gate cannot
                // recognize may have just removed it. Starting from zero is correct.
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok::<_, DownloadError>(());
                }
                Err(error) => return Err(error.into()),
            };
            let mut input = input.take(source.len);
            let copied = async {
                let mut buffer = [0; 8192];
                loop {
                    check_cancel(options)?;
                    let count = input.read(&mut buffer).await?;
                    if count == 0 {
                        break;
                    }
                    destination.write_all(&buffer[..count]).await?;
                    #[cfg(test)]
                    options.pause_partial(PartialStage::SnapshotWritten).await;
                }
                Ok::<_, DownloadError>(())
            }
            .await;
            drop(input.into_inner().into_std().await);
            copied?;
        }
        Ok::<_, DownloadError>(())
    }
    .await;
    let flush = destination.flush().await;
    drop(destination.into_std().await);
    if let Err(error) = snapshot.and_then(|()| flush.map_err(DownloadError::from)) {
        return Err(owned.cleanup_error(error).await);
    }
    Ok(owned)
}

async fn publish_owned(
    part: &Path,
    download: &Downloadable,
    options: &DownloadOptions,
    leftovers: &[Leftover],
) -> Result<(), DownloadError> {
    #[cfg(test)]
    options.pause_partial(PartialStage::Publishing).await;
    let _boundary = tokio::select! {
        biased;
        _ = options.cancel_token.cancelled() => return Err(DownloadError::Cancelled),
        guard = FILE_BOUNDARY.lock() => guard,
    };
    check_cancel(options)?;
    // Another alias may have published while this independently owned inode was
    // streaming. Revalidate current bytes before deciding whether replacement is needed.
    let current = validate_file(
        &download.path,
        download.size,
        &download.checksum,
        true,
        &options.cancel_token,
    )
    .await;
    check_cancel(options)?;
    if current.is_ok() {
        remove_file(part).await?;
    } else {
        publish(part, &download.path, options)?;
    }
    // The destination now holds verified bytes, so every leftover of an earlier
    // interrupted download of it is spent, as 2.0.40 consumed its partial here.
    discard_leftovers(leftovers).await;
    Ok(())
}

#[derive(Default)]
struct Publication {
    closed: SyncMutex<bool>,
}
struct DownloadOwner {
    publication: Arc<Publication>,
    cancel: CancellationToken,
}
impl Drop for DownloadOwner {
    fn drop(&mut self) {
        // Serialized with the actual OS rename, never with an async dispatch of it.
        *self.publication.closed.lock().unwrap() = true;
        self.cancel.cancel();
    }
}
fn check_cancel(options: &DownloadOptions) -> Result<(), DownloadError> {
    if options.cancel_token.is_cancelled() {
        Err(DownloadError::Cancelled)
    } else {
        Ok(())
    }
}
fn publish(part: &Path, target: &Path, options: &DownloadOptions) -> Result<(), DownloadError> {
    let publication = options.publication.as_ref().expect("owned download");
    let closed = publication.closed.lock().unwrap();
    if *closed {
        return Err(DownloadError::Cancelled);
    }
    check_cancel(options)?;
    // A synchronous metadata operation makes owner-drop closure atomic with commit.
    // Slow/network filesystems can block this runtime thread; this is not fsync durability.
    std::fs::rename(part, target).map_err(|e| {
        DownloadError::CannotMoveFile(e, part.display().to_string(), target.display().to_string())
    })
}

#[derive(Clone)]
pub struct DownloadOptions {
    #[cfg(test)]
    partial_pause: Option<Arc<PartialPause>>,
    concurrency: usize,
    publication: Option<Arc<Publication>>,
    destination_policy: DestinationPolicy,
    read_stall_timeout: std::time::Duration,
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
            publication: None,
            #[cfg(test)]
            partial_pause: None,
            destination_policy: DestinationPolicy::native(),
            read_stall_timeout: READ_STALL_TIMEOUT,
            cancel_token: CancellationToken::new(),
            only_validate: false,
            deep_check: true,
            max_retries: 3,
            progress_sender: None,
        }
    }
}

impl DownloadOptions {
    #[cfg(test)]
    async fn pause_partial(&self, stage: PartialStage) {
        if let Some(pause) = &self.partial_pause {
            if pause.stage == stage {
                pause.entered.notify_one();
                pause.release.acquire().await.unwrap().forget();
            }
        }
    }

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
            publication: None,
            #[cfg(test)]
            partial_pause: None,
            destination_policy: DestinationPolicy::native(),
            read_stall_timeout: READ_STALL_TIMEOUT,
            cancel_token: self.cancel_token.unwrap_or_else(CancellationToken::new),
            only_validate: self.only_validate.unwrap_or(false),
            deep_check: self.deep_check.unwrap_or(false),
            max_retries: self.max_retries.unwrap_or(3),
            progress_sender: self.progress_sender,
        }
    }
}

/// Runs an owned batch. Explicit cancellation returns only after its workers settle.
/// Dropping this future closes publication synchronously; a supervisor then drains
/// cancelled private IO while retaining destination gates. An aborted outer task's
/// join is therefore not itself proof of complete OS IO quiescence.
#[instrument(skip(files, options))]
pub async fn download_multiple(
    files: &[Downloadable],
    mut options: DownloadOptions,
) -> Result<bool, DownloadError> {
    options.cancel_token = options.cancel_token.child_token();
    let publication = Arc::new(Publication::default());
    options.publication = Some(publication.clone());
    let _owner = DownloadOwner {
        publication,
        cancel: options.cancel_token.clone(),
    };
    let files = files.to_vec();
    // This supervisor is never aborted: even after owner drop it awaits already
    // dispatched filesystem work and closes handles before releasing each gate.
    tokio::spawn(async move { run_downloads(files, options).await }).await?
}

async fn run_downloads(
    files: Vec<Downloadable>,
    options: DownloadOptions,
) -> Result<bool, DownloadError> {
    use futures::{StreamExt, stream};
    let client = create_client(&options);
    let total_files_size = AtomicU64::new(files.iter().filter_map(|f| f.size).sum());
    let total_downloaded_size = AtomicU64::new(0);
    let current_files_count = AtomicU64::new(0);
    let total_count = files.len() as u64;
    if let Some(sender) = &options.progress_sender {
        let _ = sender.send(Progress::sized(
            total_count,
            0,
            total_files_size.load(Ordering::SeqCst),
            0,
        ));
    }
    // Zero concurrency retains its queued behavior, but is now cancellable.
    if options.concurrency == 0 {
        options.cancel_token.cancelled().await;
        return Err(DownloadError::Cancelled);
    }
    let mut workers = stream::iter(files)
        .take_while(|_| futures::future::ready(!options.cancel_token.is_cancelled()))
        .map(|file| {
            process_file(
                file,
                options.clone(),
                client.clone(),
                &total_downloaded_size,
                &current_files_count,
                &total_files_size,
                total_count,
            )
        })
        .buffer_unordered(options.concurrency);
    let mut terminal = None;
    while let Some(result) = workers.next().await {
        if let Err(error) = result {
            if terminal.is_none() {
                terminal = Some(match error {
                    DownloadError::SizeMismatch { .. }
                    | DownloadError::ChecksumMismatch { .. }
                    | DownloadError::FileNotFound(_)
                        if options.only_validate =>
                    {
                        Ok(true)
                    }
                    error => Err(error),
                });
                options.cancel_token.cancel();
            }
        }
    }
    if let Some(result) = terminal {
        return result;
    }
    check_cancel(&options)?;
    if let Some(sender) = &options.progress_sender {
        let total_size = total_files_size.load(Ordering::SeqCst);
        let _ = sender.send(Progress::sized(
            total_count,
            total_count,
            total_size,
            total_size,
        ));
    }
    Ok(false)
}

fn create_client(options: &DownloadOptions) -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(options.max_retries);
    let client = Client::builder()
        // Bound connection establishment so a route that never completes the handshake
        // (captive portal, dropped route) errors out instead of hanging forever.
        .connect_timeout(std::time::Duration::from_secs(30))
        .read_timeout(options.read_stall_timeout)
        .build()
        .expect("Failed to build HTTP client");
    ClientBuilder::new(client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}

async fn process_file(
    downloadable: Downloadable,
    options: DownloadOptions,
    client: ClientWithMiddleware,
    total_downloaded_size: &AtomicU64,
    current_files_count: &AtomicU64,
    total_files_size: &AtomicU64,
    total_files_count: u64,
) -> Result<(), DownloadError> {
    check_cancel(&options)?;
    // A metadata-only check opens no handle on the destination, so it cannot get
    // in the way of a publication in flight, and a file already in place needs
    // no gate. That keeps a Play's check of thousands of files from resolving and
    // queueing a gate for each of them. Anything else is decided under the gate.
    if !options.deep_check
        && validate_file(
            &downloadable.path,
            downloadable.size,
            &downloadable.checksum,
            false,
            &options.cancel_token,
        )
        .await
        .is_ok()
    {
        check_cancel(&options)?;
        count_valid(
            &downloadable,
            &options,
            total_downloaded_size,
            current_files_count,
            total_files_size,
            total_files_count,
        );
        return Ok(());
    }
    let gate = destination_gate(&downloadable.path, options.destination_policy).await?;
    let _guard = tokio::select! {
        biased;
        _ = options.cancel_token.cancelled() => return Err(DownloadError::Cancelled),
        guard = gate.lock() => guard,
    };
    check_cancel(&options)?;
    let validation = validate_file(
        &downloadable.path,
        downloadable.size,
        &downloadable.checksum,
        options.deep_check,
        &options.cancel_token,
    )
    .await;

    check_cancel(&options)?;
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
            )
            .await
        }
        Ok(_) => {
            count_valid(
                &downloadable,
                &options,
                total_downloaded_size,
                current_files_count,
                total_files_size,
                total_files_count,
            );
            return Ok(());
        }
    }
}

/// Counts a file that is already valid at its destination as done.
fn count_valid(
    downloadable: &Downloadable,
    options: &DownloadOptions,
    total_downloaded_size: &AtomicU64,
    current_files_count: &AtomicU64,
    total_files_size: &AtomicU64,
    total_files_count: u64,
) {
    if let Some(size) = downloadable.size {
        total_downloaded_size.fetch_add(size, Ordering::SeqCst);
    }
    current_files_count.fetch_add(1, Ordering::SeqCst);
    if let Some(sender) = &options.progress_sender {
        let progress = Progress::sized(
            total_files_count,
            current_files_count.load(Ordering::SeqCst),
            total_files_size.load(Ordering::SeqCst),
            total_downloaded_size.load(Ordering::SeqCst),
        );

        let _ = sender.send(progress);
    }
}

async fn download_file(
    mut downloadable: Downloadable,
    options: DownloadOptions,
    client: ClientWithMiddleware,
    total_downloaded_size: &AtomicU64,
    current_files_count: &AtomicU64,
    total_files_size: &AtomicU64,
    total_files_count: u64,
) -> Result<(), DownloadError> {
    let file_processed_bytes = AtomicU64::new(0);
    let file_added_to_total = AtomicU64::new(0);
    let progress = options.progress_sender.as_ref();
    let owned = owned_partial(&downloadable, &options).await?;
    let candidates: Vec<_> = std::iter::once(downloadable.url.clone())
        .chain(downloadable.mirrors.iter().cloned())
        .collect();
    let result = async {
        for (candidate, url) in candidates.iter().enumerate() {
            downloadable.url = url.clone();
            for attempt in 0..2 {
                check_cancel(&options)?;
                let (part, mut file, headers, mut hasher, resumed) = prepare_download(
                    &downloadable,
                    &owned.path,
                    candidate != 0 || attempt != 0,
                    total_downloaded_size,
                    &file_processed_bytes,
                    current_files_count,
                    total_files_size,
                    total_files_count,
                    progress,
                    &options,
                )
                .await?;
                let complete = resumed
                    && downloadable.checksum.as_ref().is_some_and(|checksum| {
                        let expected = match checksum {
                            Checksum::Sha1(s)
                            | Checksum::Sha256(s)
                            | Checksum::Sha512(s)
                            | Checksum::Md5(s) => s,
                        };
                        downloadable
                            .size
                            .is_none_or(|size| size == file_processed_bytes.load(Ordering::SeqCst))
                            && hex::encode(hasher.as_ref().unwrap().clone().finalize())
                                .eq_ignore_ascii_case(expected)
                    });
                let outcome = if complete {
                    if downloadable.size.is_none() {
                        let size = file_processed_bytes.load(Ordering::SeqCst);
                        file_added_to_total.fetch_add(size, Ordering::SeqCst);
                        total_files_size.fetch_add(size, Ordering::SeqCst);
                    }
                    Ok(())
                } else {
                    _download_file(
                        &downloadable.url,
                        headers,
                        &part,
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
                    .await
                };
                // Tokio writes can have outstanding blocking IO even after write_all resolves.
                // Await it on EVERY outcome and close the OS handle before publication/cleanup.
                let flush = file.flush().await;
                drop(file.into_std().await);
                // A local flush failure must stop admission even if the HTTP
                // transfer also failed. It is not evidence of a mirror outage.
                let outcome = match flush {
                    Ok(()) => outcome,
                    Err(error) => Err(DownloadError::from(error)),
                };
                match outcome {
                    Ok(()) => {
                        publish_owned(&part, &downloadable, &options).await?;
                        current_files_count.fetch_add(1, Ordering::SeqCst);
                        if let Some(sender) = progress {
                            let _ = sender.send(Progress::sized(
                                total_files_count,
                                current_files_count.load(Ordering::SeqCst),
                                total_files_size.load(Ordering::SeqCst),
                                total_downloaded_size.load(Ordering::SeqCst),
                            ));
                        }
                        return Ok(());
                    }
                    Err(
                        DownloadError::ChecksumMismatch { .. }
                        | DownloadError::SizeMismatch { .. }
                        | DownloadError::RestartPartial,
                    ) if resumed && attempt == 0 && !options.cancel_token.is_cancelled() => {
                        total_downloaded_size.fetch_sub(
                            file_processed_bytes.swap(0, Ordering::SeqCst),
                            Ordering::SeqCst,
                        );
                        total_files_size.fetch_sub(
                            file_added_to_total.swap(0, Ordering::SeqCst),
                            Ordering::SeqCst,
                        );
                    }
                    Err(error) if mirror_failure(&error) && candidate + 1 < candidates.len() => {
                        check_cancel(&options)?;
                        total_downloaded_size.fetch_sub(
                            file_processed_bytes.swap(0, Ordering::SeqCst),
                            Ordering::SeqCst,
                        );
                        total_files_size.fetch_sub(
                            file_added_to_total.swap(0, Ordering::SeqCst),
                            Ordering::SeqCst,
                        );
                        // All entered IO is closed above. The next candidate opens
                        // this same owned inode with truncation and fresh URL headers.
                        break;
                    }
                    Err(error) => return Err(error),
                }
            }
        }
        unreachable!("last candidate returns success or its error")
    }
    .await;
    // Includes early cancellation, preparation, transfer, and final publication.
    // Success already consumed the owned pathname; never remove it after success.
    match result {
        Ok(()) => Ok(()),
        Err(error) => Err(owned.cleanup_error(error).await),
    }
}

fn mirror_failure(error: &DownloadError) -> bool {
    matches!(
        error,
        DownloadError::NetworkError(_)
            | DownloadError::NetworkError2(_)
            | DownloadError::GenericDownload(_)
            | DownloadError::Non200StatusCode(..)
            | DownloadError::ChecksumMismatch { .. }
            | DownloadError::SizeMismatch { .. }
            | DownloadError::InvalidRange
            | DownloadError::RestartPartial
    )
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

    let mut response = tokio::select! {
        biased;
        _ = options.cancel_token.cancelled() => return Err(DownloadError::Cancelled),
        response = client.get(url).headers(headers).send() => response?,
    };
    let retained = file_processed_bytes.load(Ordering::SeqCst);
    if resume_requested && response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        let total = response
            .headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("bytes */"))
            .and_then(range_number);
        if total == Some(retained) && downloadable.size.is_none_or(|size| Some(size) == total) {
            return Err(DownloadError::RestartPartial);
        }
        return Err(DownloadError::InvalidRange);
    }
    let range_total = if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
        Some(validate_range(&response, retained, downloadable.size)?)
    } else {
        None
    };

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
    if let Some(total) = range_total {
        if file_processed_bytes.load(Ordering::SeqCst) != total {
            return Err(DownloadError::InvalidRange);
        }
    }

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

        // Hex case carries no meaning, and `expected_hash` is whatever the source
        // handed us verbatim: Mojang, Modrinth, CurseForge, Forge install profiles
        // and Azul all populate `Checksum` straight from their API payload without
        // normalizing it. A source switching to uppercase would otherwise fail every
        // download it serves.
        if !actual_hash.eq_ignore_ascii_case(expected_hash) {
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
    part_file_path: &PathBuf,
    force_overwrite: bool,
    total_downloaded_size: &AtomicU64,
    file_processed_bytes: &AtomicU64,
    current_files_count: &AtomicU64,
    total_files_size: &AtomicU64,
    total_files_count: u64,
    progress: Option<&Sender<Progress>>,
    options: &DownloadOptions,
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
    let should_resume = tokio::fs::metadata(part_file_path).await?.len() > 0 && !force_overwrite;

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

    let prepared = async {
        #[cfg(test)]
        options.pause_partial(PartialStage::Preparing).await;
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
                if options.cancel_token.is_cancelled() {
                    return Err(DownloadError::Cancelled);
                }
                let bytes_read = file.read(&mut buf).await?;
                if bytes_read == 0 {
                    break;
                }
                if let Some(hasher) = hasher.as_mut() {
                    hasher.update(&buf[..bytes_read]);
                }
                processed_bytes += bytes_read as u64;

                if let Some(progress) = progress {
                    let _progress = Progress::sized(
                        total_files_count,
                        current_files_count.load(Ordering::SeqCst),
                        total_files_size.load(Ordering::SeqCst),
                        total_downloaded_size.load(Ordering::SeqCst),
                    );

                    let _ = progress.send(_progress);
                }
            }

            total_downloaded_size.fetch_add(processed_bytes, Ordering::SeqCst);
            file_processed_bytes.fetch_add(processed_bytes, Ordering::SeqCst);

            headers.insert(
                reqwest::header::RANGE,
                reqwest::header::HeaderValue::from_str(&format!("bytes={}-", processed_bytes))
                    .unwrap(),
            );
        }

        Ok::<_, DownloadError>((headers, hasher, should_resume))
    }
    .await;
    match prepared {
        Ok((headers, hasher, resumed)) => {
            Ok((part_file_path.clone(), file, headers, hasher, resumed))
        }
        Err(error) => {
            // Hash/read preparation can fail or be cancelled after opening the
            // owned file. Close it before the outer operation removes its path.
            drop(file.into_std().await);
            Err(error)
        }
    }
}

fn range_number(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

fn validate_range(
    response: &reqwest::Response,
    retained: u64,
    expected: Option<u64>,
) -> Result<u64, DownloadError> {
    let parse = || -> Option<u64> {
        let text = response
            .headers()
            .get(reqwest::header::CONTENT_RANGE)?
            .to_str()
            .ok()?
            .strip_prefix("bytes ")?;
        let (extent, total) = text.split_once('/')?;
        let (start, end) = extent.split_once('-')?;
        let start = range_number(start)?;
        let end = range_number(end)?;
        let total = range_number(total)?;
        if start != retained
            || end < start
            || end.checked_add(1)? != total
            || expected.is_some_and(|size| size != total)
        {
            return None;
        }
        let length = end.checked_sub(start)?.checked_add(1)?;
        if response
            .content_length()
            .is_some_and(|actual| actual != length)
        {
            return None;
        }
        Some(total)
    };
    parse().ok_or(DownloadError::InvalidRange)
}

fn check_response_status(
    response: &reqwest::Response,
    file: &Downloadable,
) -> Result<(), DownloadError> {
    if response.status() != reqwest::StatusCode::OK
        && response.status() != reqwest::StatusCode::PARTIAL_CONTENT
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
        let chunk = match tokio::select! {
            biased;
            _ = options.cancel_token.cancelled() => return Err(DownloadError::Cancelled),
            result = tokio::time::timeout(options.read_stall_timeout, response.chunk()) => result,
        } {
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
            let progress = Progress::sized(
                total_count,
                current_files_count.load(Ordering::SeqCst),
                total_size.load(Ordering::SeqCst),
                total_downloaded_size.load(Ordering::SeqCst),
            );

            let _ = sender.send(progress);
        }
    }

    Ok(())
}

#[instrument(skip(expected_checksum))]
async fn validate_file(
    path: &Path,
    expected_size: Option<u64>,
    expected_checksum: &Option<Checksum>,
    deep_check: bool,
    cancel: &CancellationToken,
) -> Result<(), DownloadError> {
    if !path.exists() {
        return Err(DownloadError::FileNotFound(
            path.to_string_lossy().to_string(),
        ));
    };

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
                    if cancel.is_cancelled() {
                        return Err(DownloadError::Cancelled);
                    }
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

            // Case-insensitive for the same reason as the post-download check above.
            if !actual_hash.eq_ignore_ascii_case(expected_hash) {
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
            mirrors: Vec::new(),
            url: format!("{}/test.txt", mock_url),
            path: file_path.clone(),
            checksum: Some(Checksum::Sha256(
                "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f".to_string(),
            )),
            size: Some(13),
        };

        let downloadable1 = Downloadable {
            mirrors: Vec::new(),
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
            mirrors: Vec::new(),
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

        // The body callback runs once the request has reached the mock. It
        // reports that, then holds the body back until the test has its
        // result, so the download is still in flight when it is cancelled.
        let request_reached = Arc::new(tokio::sync::Notify::new());
        let (release_body, body_released) = std::sync::mpsc::channel::<()>();
        let body_released = std::sync::Mutex::new(body_released);
        let mock = server
            .mock("GET", "/test.txt")
            .with_status(200)
            .with_header("content-length", "13")
            .with_chunked_body({
                let request_reached = request_reached.clone();
                move |w| {
                    request_reached.notify_one();
                    let _ = body_released
                        .lock()
                        .unwrap()
                        .recv_timeout(Duration::from_secs(30));
                    let _ = w.write_all(b"Hello, World!");
                    w.flush()
                }
            })
            .create_async()
            .await;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let downloadable = Downloadable {
            mirrors: Vec::new(),
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

        // Cancel only once the request is on the wire, however slowly the
        // download got there.
        tokio::time::timeout(Duration::from_secs(30), request_reached.notified())
            .await
            .expect("the download request never reached the mock server");
        cancel_token_clone.cancel();

        let result = download_handle.await.unwrap();
        let _ = release_body.send(());
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
            .expect(1)
            .create_async()
            .await;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let downloadable1 = Downloadable {
            mirrors: Vec::new(),
            url: format!("{}/test.txt", mock_url),
            path: file_path.clone(),
            checksum: Some(Checksum::Sha256(
                "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f".into(),
            )),
            size: Some(13),
        };

        let downloadable2 = Downloadable {
            mirrors: Vec::new(),
            url: format!("{}/test.txt", mock_url),
            path: file_path.clone(),
            checksum: Some(Checksum::Sha256(
                "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f".into(),
            )),
            size: Some(13),
        };

        let options = DownloadOptions::builder()
            .concurrency(2)
            .deep_check(true)
            .build();

        download_multiple(&[downloadable1, downloadable2], options)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(&file_path).unwrap(),
            "Hello, World!"
        );
        assert_eq!(std::fs::metadata(&file_path).unwrap().len(), 13);

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

        validate_file(
            &file_path,
            Some(13),
            &Some(checksum),
            true,
            &CancellationToken::new(),
        )
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

        let result = validate_file(
            &file_path,
            Some(14),
            &None,
            false,
            &CancellationToken::new(),
        )
        .await;
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

        let result = validate_file(
            &file_path,
            Some(13),
            &Some(checksum),
            true,
            &CancellationToken::new(),
        )
        .await;
        assert!(matches!(
            result,
            Err(DownloadError::ChecksumMismatch { .. })
        ));
    }

    #[tokio::test]
    #[traced_test]
    async fn test_validate_file_accepts_uppercase_checksum() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = b"Hello, World!";
        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(content).await.unwrap();
        file.flush().await.unwrap();

        let checksum = Checksum::Sha256(
            "DFFD6021BB2BD5B0AF676290809EC3A53191DD81C7F70A4B28688A362182986F".to_string(),
        );

        validate_file(
            &file_path,
            Some(13),
            &Some(checksum),
            true,
            &CancellationToken::new(),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    #[traced_test]
    async fn test_download_accepts_uppercase_checksum() {
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

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let downloadable = Downloadable {
            mirrors: Vec::new(),
            url: format!("{}/test.txt", mock_url),
            path: file_path.clone(),
            checksum: Some(Checksum::Sha256(
                "DFFD6021BB2BD5B0AF676290809EC3A53191DD81C7F70A4B28688A362182986F".to_string(),
            )),
            size: Some(13),
        };

        let options = DownloadOptions::builder().concurrency(1).build();

        download_multiple(&[downloadable], options).await.unwrap();
        assert_eq!(std::fs::read_to_string(file_path).unwrap(), "Hello, World!");

        mock.assert();
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
            .with_header("content-range", "bytes 7-12/13")
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
            mirrors: Vec::new(),
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
            .with_header("content-range", "bytes 7-12/13")
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
            mirrors: Vec::new(),
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
            mirrors: Vec::new(),
            url: format!("{}/test1.txt", mock_url),
            path: file_path1.clone(),
            checksum: Some(Checksum::Sha1(
                "83cfe479c149983ad66bb9c0e4b47cf2870e33c6".to_string(),
            )),
            size: Some(15),
        };

        let downloadable2 = Downloadable {
            mirrors: Vec::new(),
            url: format!("{}/test2.txt", mock_url),
            path: file_path2.clone(),
            checksum: Some(Checksum::Sha256(
                "1aefd92d29317a3119827259177ce85329da746818baf6682d1d455ade4263fd".to_string(),
            )),
            size: Some(15),
        };

        let downloadable3 = Downloadable {
            mirrors: Vec::new(),
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
            mirrors: Vec::new(),
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
            mirrors: Vec::new(),
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
            mirrors: Vec::new(),
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
            mirrors: Vec::new(),
            url: format!("{}/test1.txt", mock_url),
            path: file_path1.clone(),
            checksum: None,
            size: Some(13),
        };

        let downloadable2 = Downloadable {
            mirrors: Vec::new(),
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

    #[test]
    fn progress_never_reports_more_downloaded_than_total() {
        // A resumed file's prefix reaches the downloaded counter a full HTTP
        // round trip before it reaches the total, and every other in-flight
        // download reports from both counters during that window.
        let p = Progress::sized(2, 1, 100, 140);
        assert_eq!(p.current_size, 100);
        assert_eq!(p.total_size, 100);

        let ordinary = Progress::sized(2, 1, 100, 40);
        assert_eq!(ordinary.current_size, 40);
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
            mirrors: Vec::new(),
            url: format!("{}/known.txt", mock_url),
            path: file_path1.clone(),
            checksum: None,
            size: Some(13),
        };

        let downloadable2 = Downloadable {
            mirrors: Vec::new(),
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
            .with_header("content-range", "bytes 7-12/13")
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
            mirrors: Vec::new(),
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
            mirrors: Vec::new(),
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
            mirrors: Vec::new(),
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

#[cfg(test)]
mod task06_tests;

#[cfg(test)]
mod task09_tests;
