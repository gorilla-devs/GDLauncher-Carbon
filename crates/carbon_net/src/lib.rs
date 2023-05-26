use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use futures::StreamExt;
use reqwest::Client;
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use sha1::digest::core_api::CoreWrapper;
use sha1::Digest as _;
use sha1::Sha1;
use sha2::Digest as _;
use sha2::Sha256;
use tokio::sync::watch;
use tokio::{
    fs::OpenOptions,
    io::{AsyncReadExt, AsyncWriteExt},
};
use tracing::trace;

use error::DownloadError;

mod error;

#[derive(Debug)]
pub enum Checksum {
    Sha1(String),
    Sha256(String),
}

pub trait IntoVecDownloadable {
    fn into_vec_downloadable(self, base_path: &Path) -> Vec<Downloadable>;
}

pub trait IntoDownloadable {
    fn into_downloadable(self, base_path: &Path) -> Downloadable;
}

#[derive(Debug)]
pub struct Downloadable {
    pub url: String,
    pub path: PathBuf,
    pub checksum: Option<Checksum>,
    pub size: Option<u64>,
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

// Todo: Add checksum/size verification
pub async fn download_file(
    downloadable_file: &Downloadable,
    progress: Option<watch::Sender<Progress>>,
) -> Result<(), DownloadError> {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    let reqwest_client = Client::builder().build()?;
    let client = ClientBuilder::new(reqwest_client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

    let mut response = client.get(&downloadable_file.url).send().await?;

    if !response.status().is_success() {
        return Err(DownloadError::Non200StatusCode(response.status().as_u16()));
    }

    // Ensure the parent directory exists
    if let Some(parent) = downloadable_file.path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&downloadable_file.path)
        .await?;

    let mut buf = vec![];
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        buf.extend_from_slice(&chunk);
        if let Some(progress) = &progress {
            let size_progress = (buf.len() as f64 / downloadable_file.size.unwrap_or(1) as f64)
                .min(1.0)
                .max(0.0)
                * 100.0;

            progress.send(Progress {
                // Special case for single file
                total_count: 1,
                current_count: 0,

                current_size: buf.len() as u64,
                total_size: downloadable_file.size.unwrap_or(0),
            })?;
        }
    }

    // Check size and checksum when provided
    if let Some(size) = downloadable_file.size {
        if size != buf.len() as u64 {
            return Err(DownloadError::SizeMismatch {
                expected: size,
                actual: buf.len() as u64,
            });
        }
    }

    if let Some(checksum) = &downloadable_file.checksum {
        match checksum {
            Checksum::Sha1(expected) => {
                let mut hasher = Sha1::new();
                hasher.update(&buf);
                let actual = hasher.finalize();
                let actual = hex::encode(actual);

                if expected != &actual {
                    return Err(DownloadError::ChecksumMismatch {
                        expected: expected.clone(),
                        actual,
                    });
                }
            }
            Checksum::Sha256(expected) => {
                let mut hasher = Sha256::new();
                hasher.update(&buf);
                let actual = hasher.finalize();
                let actual = hex::encode(actual);

                if expected != &actual {
                    return Err(DownloadError::ChecksumMismatch {
                        expected: expected.clone(),
                        actual,
                    });
                }
            }
        }
    }

    if let Some(progress) = &progress {
        progress.send(Progress {
            total_count: 1,
            current_count: 1,

            current_size: buf.len() as u64,
            total_size: downloadable_file.size.unwrap_or(0),
        })?;
    }

    Ok(())
}

// TODO: improve checksum/size verification
pub async fn download_multiple(
    files: Vec<Downloadable>,
    progress: watch::Sender<Progress>,
) -> Result<(), DownloadError> {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    let reqwest_client = Client::builder().build().unwrap();
    let client = ClientBuilder::new(reqwest_client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

    let downloads = Arc::new(tokio::sync::Semaphore::new(10));

    let mut tasks: Vec<tokio::task::JoinHandle<Result<_, DownloadError>>> = vec![];

    let arced_progress = Arc::new(progress);

    let atomic_size_counter = Arc::new(AtomicU64::new(0));
    let atomic_file_counter = Arc::new(AtomicU64::new(0));
    let atomic_size = Arc::new(AtomicU64::new(files.iter().map(|f| f.size).flatten().sum()));

    let total_size = files
        .iter()
        .fold(0, |acc, file| acc + file.size.unwrap_or(0));

    let total_count = files.len() as u64;

    for file in files {
        let semaphore = Arc::clone(&downloads);
        let progress = Arc::clone(&arced_progress);
        let size_counter = Arc::clone(&atomic_size_counter);
        let file_counter = Arc::clone(&atomic_file_counter);
        let size = Arc::clone(&atomic_size);
        let url = file.url.clone();
        let path = file.path.clone();
        let client = client.clone();

        tasks.push(tokio::spawn(async move {
            let _permit = semaphore
                .acquire()
                .await
                .map_err(|err| DownloadError::GenericDownload(err.to_string()))?;
            let path = path.clone();
            let url = url.clone();

            let file_looks_good = match file.size {
                Some(size) if path.exists() => {
                    let metadata = tokio::fs::metadata(&path).await;
                    if let Ok(metadata) = metadata {
                        metadata.len() == size
                    } else {
                        false
                    }
                }
                Some(_) => false,
                None => path.exists(),
            };

            // verify if file exists and checksum matches
            if file_looks_good {
                let mut sha1 = Sha1::new();
                let mut sha256 = Sha256::new();

                let mut fs_file = tokio::fs::File::open(&path).await?;

                let mut buf = vec![];
                fs_file.read_to_end(&mut buf).await?;

                match file.checksum {
                    Some(Checksum::Sha1(_)) => sha1.update(&buf),
                    Some(Checksum::Sha256(_)) => sha256.update(&buf),
                    None => {}
                }

                match file.checksum {
                    Some(Checksum::Sha1(ref hash)) => {
                        let finalized = sha1.finalize();
                        if hash == &format!("{finalized:x}") {
                            // unwraps will be fine because file_looks_good can't happen without it
                            let downloaded =
                                size_counter.fetch_add(file.size.unwrap(), Ordering::SeqCst);

                            progress.send(Progress {
                                current_count: file_counter.load(Ordering::SeqCst),
                                total_count,
                                current_size: downloaded,
                                total_size: size.load(Ordering::SeqCst),
                            })?;

                            return Ok(());
                        } else {
                            trace!(
                                "Hash mismatch sha1 for file: {} - expected: {hash} - got: {}",
                                path.display(),
                                &format!("{finalized:x}")
                            );
                        }
                    }
                    Some(Checksum::Sha256(ref hash)) => {
                        let finalized = sha256.finalize();
                        if hash == &format!("{finalized:x}") {
                            // unwraps will be fine because file_looks_good can't happen without it
                            let downloaded =
                                size_counter.fetch_add(file.size.unwrap(), Ordering::SeqCst);

                            progress.send(Progress {
                                current_count: file_counter.load(Ordering::SeqCst),
                                total_count,
                                current_size: downloaded,
                                total_size: size.load(Ordering::SeqCst),
                            })?;

                            return Ok(());
                        } else {
                            trace!(
                                "Hash mismatch sha256 for file: {} - expected: {hash} - got: {}",
                                path.display(),
                                &format!("{finalized:x}")
                            );
                        }
                    }
                    None => {}
                }
            }

            let mut file_downloaded = 0u64;
            let mut file_size_reported = file.size.unwrap_or(0);

            let mut resp_stream = client.get(&url).send().await?.bytes_stream();

            tokio::fs::create_dir_all(path.parent().ok_or(DownloadError::GenericDownload(
                "Can't create folder".to_owned(),
            ))?)
            .await?;

            let mut sha1 = Sha1::new();
            let mut sha256 = Sha256::new();

            let mut fs_file = OpenOptions::new()
                .create(!path.exists())
                .write(true)
                .truncate(path.exists())
                .open(&path)
                .await?;

            while let Some(item) = resp_stream.next().await {
                let res = item?;
                match file.checksum {
                    Some(Checksum::Sha1(_)) => sha1.update(&res),
                    Some(Checksum::Sha256(_)) => sha256.update(&res),
                    None => {}
                }

                tokio::io::copy(&mut res.as_ref(), &mut fs_file).await?;

                let downloaded = size_counter.fetch_add(res.len() as u64, Ordering::SeqCst);
                file_downloaded += res.len() as u64;

                if file_downloaded > file_size_reported {
                    let diff = file_downloaded - file_size_reported;
                    file_size_reported = file_downloaded;
                    size.fetch_add(diff, Ordering::SeqCst);
                }

                progress.send(Progress {
                    current_count: size_counter.load(Ordering::SeqCst),
                    total_count,
                    current_size: downloaded,
                    total_size: size.load(Ordering::SeqCst),
                })?;
            }

            let diff = file_size_reported - file_downloaded;
            let total = size.fetch_sub(diff, Ordering::SeqCst) - diff;

            progress.send(Progress {
                current_count: file_counter.fetch_add(1, Ordering::SeqCst),
                total_count,
                current_size: total,
                total_size: size.load(Ordering::SeqCst),
            })?;

            match file.checksum {
                Some(Checksum::Sha1(hash)) => {
                    if hash != hex::encode(sha1.finalize().as_slice()) {
                        return Err(DownloadError::GenericDownload(
                            "Checksum mismatch".to_owned(),
                        ));
                    }
                }
                Some(Checksum::Sha256(hash)) => {
                    if hash != hex::encode(sha256.finalize().as_slice()) {
                        return Err(DownloadError::GenericDownload(
                            "Checksum mismatch".to_owned(),
                        ));
                    }
                }
                None => {}
            }

            Ok(())
        }));
    }

    for task in tasks {
        task.await??;
    }

    Ok(())
}
