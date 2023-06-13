use std::{
    collections::HashSet,
    io, mem,
    ops::{Deref, DerefMut},
    path::Path,
    str::FromStr,
};

use anyhow::bail;
use prisma_client_rust::QueryError;
use reqwest::Response;
use reqwest_middleware::ClientWithMiddleware;
use thiserror::Error;
use tokio::{
    fs::OpenOptions,
    io::{AsyncWriteExt, BufWriter},
    sync::{mpsc, watch, Mutex},
};
use uuid::Uuid;

use crate::{
    db::read_filters::StringFilter,
    error::request::{RequestContext, RequestError, RequestErrorDetails},
    once_send::OnceSend,
};

use super::ManagerRef;

pub struct DownloadManager {
    active_downloads: Mutex<HashSet<String>>,
}

impl DownloadManager {
    pub fn new() -> Self {
        Self {
            active_downloads: Mutex::new(HashSet::new()),
        }
    }
}

impl ManagerRef<'_, DownloadManager> {
    pub async fn download(
        &self,
        url: String,
        path: &Path,
        updater: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<(), DownloadError> {
        let mut handle = self.start_download(url).await?;

        while handle.status_channel.changed().await.is_ok() {
            let status = handle.status_channel.borrow();
            match &*status {
                None => {}
                Some(DownloadStatus::Failed(e)) => return Err(DownloadError::Active(e.take().expect("failed download status was replaced twice, which should be impossible due to immediate return"))),
                Some(DownloadStatus::Status { downloaded, total }) => updater(*downloaded, *total),
                Some(DownloadStatus::Complete) => {
                    drop(status);
                    self.complete_download(handle, path).await?;
                    return Ok(());
                }
            }
        }

        return Err(DownloadError::Dropped);
    }

    /// Cancel a download, deleting the file.
    ///
    /// If the download has already finished the files will be deleted anyway.
    pub async fn cancel_download(self, handle: DownloadHandle) -> Result<(), DownloadCancelError> {
        use crate::db::active_downloads::UniqueWhereParam;

        // stop the handle's drop() from being called
        let mut handle = handle.into_inner();

        // cancel active download and wait for comfirmation.
        let _ = handle.cancel_channel.send(()).await;
        let _ = handle.cancel_complete_channel.recv().await;

        let path = self
            .app
            .settings_manager()
            .runtime_path
            .get_download()
            .to_pathbuf()
            .join(&handle.id);

        tokio::fs::remove_file(path).await?;

        self.app
            .prisma_client
            .active_downloads()
            .delete(UniqueWhereParam::FileIdEquals(handle.id))
            .exec()
            .await?;

        Ok(())
    }

    pub async fn complete_download(
        self,
        handle: DownloadHandle,
        target: &Path,
    ) -> Result<(), DownloadCompleteError> {
        use crate::db::active_downloads::UniqueWhereParam;

        let mut handle = handle.into_inner();

        if let Err(_) = handle.complete_channel.try_recv() {
            // no completion flag
            return Err(DownloadCompleteError::DownloadIncomplete);
        }

        let path = self
            .app
            .settings_manager()
            .runtime_path
            .get_download()
            .to_pathbuf()
            .join(&handle.id);

        tokio::fs::rename(path, target)
            .await
            // explicit map_err because this is specifically a rename error,
            // not just an IO error
            .map_err(DownloadCompleteError::RenameError)?;

        self.app
            .prisma_client
            .active_downloads()
            .delete(UniqueWhereParam::FileIdEquals(handle.id))
            .exec()
            .await?;

        Ok(())
    }

    pub async fn start_download(self, url: String) -> Result<DownloadHandle, DownloadStartError> {
        use crate::db::active_downloads::WhereParam;

        // Lock active_downloads. Any future downloads will have to wait here.
        // active_downloads is locked to prevent two downloads attempting to start
        // for the same file. Whichever download gets here later must wait for the
        // first attempt to fail or add itself to the map.
        let mut active_downloads = self.active_downloads.lock().await;

        if active_downloads.contains(&url) {
            return Err(DownloadStartError::AlreadyActive);
        }

        let active_download = self
            .app
            .prisma_client
            .active_downloads()
            .find_first(vec![WhereParam::Url(StringFilter::Equals(url.clone()))])
            .exec()
            .await?;

        let id = match active_download {
            Some(download) => download.file_id,
            None => {
                let id = Uuid::new_v4().to_string();

                self.app
                    .prisma_client
                    .active_downloads()
                    .create(url.clone(), id.clone(), Vec::new())
                    .exec()
                    .await?;

                id
            }
        };

        let path = self
            .app
            .settings_manager()
            .runtime_path
            .get_download()
            .to_pathbuf()
            .join(&id);

        let (status_send, status_recv) = watch::channel::<Option<DownloadStatus>>(None);
        let (cancel_send, mut cancel_recv) = mpsc::channel::<()>(1);
        let (cancel_complete_send, cancel_complete_recv) = mpsc::channel::<()>(1);
        let (complete_send, complete_recv) = mpsc::channel::<()>(1);

        active_downloads.insert(url.clone());
        // All failable operations have finished. If the task itself fails than
        // active_downloads will be updated accordingly.
        drop(active_downloads);

        let app = self.app.clone();
        let task = async move {
            let mut canceled = false;
            let canceled_ref = &mut canceled;
            let task = || {
                let status_send = &status_send;
                let url = &url;
                let app = &app;
                async move {
                    tokio::fs::create_dir_all(
                        path.parent().ok_or(ActiveDownloadError::MalformedPath)?,
                    )
                    .await?;

                    let file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .read(true)
                        .open(&path)
                        .await?;

                    let mut start_loc = file.metadata().await?.len();

                    async fn init_request(
                        client: &ClientWithMiddleware,
                        url: &str,
                        start_loc: u64,
                    ) -> anyhow::Result<Response> {
                        let mut builder = client.get(url).header("avoid-caching", "");

                        if start_loc != 0 {
                            builder = builder.header("Range", format!("bytes={start_loc}-"));
                        }

                        let response = builder.send().await?;

                        let _ = response
                            .error_for_status_ref()
                            .map_err(RequestError::from_error)?;

                        Ok(response)
                    }

                    let mut response = init_request(&app.reqwest_client, &url, start_loc).await?;

                    match response.headers().get("content-range") {
                        Some(range_header) => {
                            let parse_start = || {
                                let mut header_str = range_header.to_str().map_err(|_| ())?;

                                match header_str.get(0..6) {
                                    Some("bytes ") => header_str = &header_str[6..],
                                    _ => return Err(()),
                                }

                                let Some((range, _)) = header_str.split_once('/') else { return Err(()) };
                                let Some((start, _)) = range.split_once('-') else { return Err(()) };
                                let Ok(start) = u64::from_str(start) else { return Err(()) };

                                Ok(start)
                            };

                            match parse_start() {
                                Ok(start) => {
                                    if start > start_loc {
                                        // server gave a resume point later than we have, assuming it will do that
                                        // again if we re-request with our starting point, so start from 0.
                                        start_loc = 0;
                                        file.set_len(0).await?;
                                        response =
                                            init_request(&app.reqwest_client, &url, 0).await?;
                                    } else if start < start_loc {
                                        // server gave a resume point earlier than we have, truncate the file to the
                                        // server's position.
                                        start_loc = start;
                                        file.set_len(start).await?;
                                    }
                                }
                                Err(()) => {
                                    bail!(RequestError {
                                        context: RequestContext::from_response(&response),
                                        error: RequestErrorDetails::MalformedResponse,
                                    });
                                }
                            }
                        }
                        None => {
                            start_loc = 0;
                            file.set_len(0).await?;
                        }
                    }

                    let mut writebuf = BufWriter::new(file);

                    let length = response.content_length().map(|x| x as u64 + start_loc);

                    let mut downloaded = start_loc;

                    let _ = status_send.send(Some(DownloadStatus::Status {
                        downloaded,
                        total: length,
                    }));

                    while let Some(chunk) =
                        response.chunk().await.map_err(RequestError::from_error)?
                    {
                        if let Ok(()) = cancel_recv.try_recv() {
                            *canceled_ref = true;
                            break; // break instead of return to flush writebuf
                        }

                        writebuf.write_all(&chunk).await?;

                        downloaded += chunk.len() as u64;
                        let _ = status_send.send(Some(DownloadStatus::Status {
                            downloaded,
                            total: length,
                        }));
                    }

                    // will NOT be flushed on drop, so it is done manually
                    writebuf.flush().await?;
                    drop(writebuf); // necessary for windows tests, file must be released before we return a complete / err status

                    if !*canceled_ref {
                        // the complete flag is set first to avoid a possible race condition
                        let _ = complete_send.send(()).await;
                        let _ = status_send.send(Some(DownloadStatus::Complete));
                    }

                    Ok(())
                }
            };

            let r = task().await;

            // Remove this download from active downloads *before* cancelation is confirmed.
            // This means after canceling a download it is always valid to restart it.
            app.download_manager()
                .active_downloads
                .lock()
                .await
                .remove(&url);

            if canceled {
                // cancel confirmation is sent to avoid file deletion race
                let _ = cancel_complete_send.send(()).await;
            }

            match r {
                Ok(()) => {}
                Err(e) => {
                    let _ = status_send.send(Some(DownloadStatus::Failed(OnceSend::new(e))));
                }
            }
        };

        tokio::spawn(task);

        Ok(DownloadHandle(DownloadHandleInner {
            id,
            status_channel: status_recv,
            cancel_channel: cancel_send,
            cancel_complete_channel: cancel_complete_recv,
            complete_channel: complete_recv,
        }))
    }
}

pub struct DownloadHandleInner {
    id: String,
    pub status_channel: watch::Receiver<Option<DownloadStatus>>,
    cancel_channel: mpsc::Sender<()>,
    cancel_complete_channel: mpsc::Receiver<()>,
    // used to make sure a download was actually completed
    complete_channel: mpsc::Receiver<()>,
}

pub struct DownloadHandle(DownloadHandleInner);

impl DownloadHandle {
    /// Convert to inner data without dropping
    fn into_inner(self) -> DownloadHandleInner {
        // SAFETY: a reference cast to a pointer and immediately used is
        // always valid.
        unsafe {
            // copy the contained download handle by value
            let inner = (&self.0 as *const DownloadHandleInner).read();
            mem::forget(self);
            // As self is forgotten, it is no longer possible to violate any
            // invariants for inner's fields.
            inner
        }
    }

    /// Cancel this download without deleting files.
    pub async fn cancel(self) {
        let mut handle = self.into_inner();
        let _ = handle.cancel_channel.send(()).await;
        let _ = handle.cancel_complete_channel.recv().await;
    }
}

impl Deref for DownloadHandle {
    type Target = DownloadHandleInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DownloadHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for DownloadHandle {
    fn drop(&mut self) {
        // Canceling a download by dropping its handle will keep the file
        // in the download cache folder.
        // As the download handle is not Clone, and cancel_channel is private,
        // this will never block.
        let channel = self.cancel_channel.clone();
        tokio::task::spawn_blocking(move || {
            let _ = channel.blocking_send(());
        });
    }
}

pub enum DownloadStatus {
    Failed(OnceSend<anyhow::Error>),
    Status { downloaded: u64, total: Option<u64> },
    Complete,
}

#[derive(Error, Debug)]
pub enum DownloadCancelError {
    #[error("not started")]
    NotStarted,

    #[error("query error: {0}")]
    Query(#[from] QueryError),

    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

#[derive(Error, Debug)]
pub enum DownloadStartError {
    #[error("download already active")]
    AlreadyActive,

    #[error("query error")]
    Query(#[from] QueryError),
}

#[derive(Error, Debug)]
pub enum ActiveDownloadError {
    #[error("request error: {0}")]
    Request(#[from] RequestError),

    #[error("malformed save path")]
    MalformedPath,

    #[error("io error: {0}")]
    IoError(#[from] io::Error),
}

impl Clone for ActiveDownloadError {
    fn clone(&self) -> Self {
        match self {
            Self::Request(e) => Self::Request(e.clone()),
            Self::MalformedPath => Self::MalformedPath,
            // this looses information, but is needed to be Clone
            Self::IoError(e) => Self::IoError(io::Error::from(e.kind())),
        }
    }
}

#[derive(Error, Debug)]
pub enum DownloadCompleteError {
    #[error("query error: {0}")]
    Query(#[from] QueryError),

    #[error("download was not completed")]
    DownloadIncomplete,

    #[error("error renaming file: {0}")]
    RenameError(io::Error),
}

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("while starting: {0}")]
    Start(#[from] DownloadStartError),

    #[error("while downloading: {0}")]
    Active(#[from] anyhow::Error),

    #[error("on completion: {0}")]
    Complete(#[from] DownloadCompleteError),

    #[error("channel was dropped")]
    Dropped,
}

#[cfg(test)]
#[cfg(not(target_os = "windows"))] // conflicts with task cleanup
mod test {
    use tracing::error;

    use crate::managers::download::{DownloadError, DownloadStartError};

    #[tokio::test]
    #[timeout(120_000)]
    async fn attempt_download() -> Result<(), DownloadError> {
        let app = crate::setup_managers_for_test().await;

        let tmpfolder = app.settings_manager().runtime_path.get_temp().to_path();

        tokio::fs::create_dir_all(tmpfolder).await.unwrap();

        app.download_manager()
            .download(
                String::from("https://gdlauncher.com"),
                &app.settings_manager()
                    .runtime_path
                    .get_temp()
                    .to_path()
                    .join("gdl.html"),
                &mut |downloaded, total| error!("Downloaded {downloaded}/{total:?}"),
            )
            .await
    }

    #[tokio::test]
    #[should_panic]
    #[timeout(120_000)]
    async fn attempt_download_twice() {
        let app = crate::setup_managers_for_test().await;

        let tmpfolder = app.settings_manager().runtime_path.get_temp().to_path();

        tokio::fs::create_dir_all(tmpfolder).await.unwrap();

        // this file should not instantly download.
        let url = String::from("https://github.com/adoptium/temurin11-binaries/releases/download/jdk-11.0.18%2B10/OpenJDK11U-jdk_x64_linux_hotspot_11.0.18_10.tar.gz");

        app.download_manager()
            .start_download(url.clone())
            .await
            .unwrap();
        app.download_manager()
            .start_download(url.clone())
            .await
            .unwrap();
    }

    #[tokio::test]
    #[timeout(120_000)]
    async fn attempt_download_after_cancel() -> Result<(), DownloadStartError> {
        let app = crate::setup_managers_for_test().await;

        let tmpfolder = app.settings_manager().runtime_path.get_temp().to_path();

        tokio::fs::create_dir_all(tmpfolder).await.unwrap();

        // this file should not instantly download.
        let url = String::from("https://github.com/adoptium/temurin11-binaries/releases/download/jdk-11.0.18%2B10/OpenJDK11U-jdk_x64_linux_hotspot_11.0.18_10.tar.gz");

        let handle = app.download_manager().start_download(url.clone()).await?;
        handle.cancel().await;
        app.download_manager().start_download(url.clone()).await?;

        Ok(())
    }

    #[tokio::test]
    #[timeout(120_000)]
    async fn attempt_cancel_download() {
        let app = crate::setup_managers_for_test().await;

        let tmpfolder = app.settings_manager().runtime_path.get_temp().to_path();

        tokio::fs::create_dir_all(tmpfolder).await.unwrap();

        // this file should not instantly download.
        let url = String::from("https://github.com/adoptium/temurin11-binaries/releases/download/jdk-11.0.18%2B10/OpenJDK11U-jdk_x64_linux_hotspot_11.0.18_10.tar.gz");

        let handle = app
            .download_manager()
            .start_download(url.clone())
            .await
            .unwrap();
        app.download_manager()
            .cancel_download(handle)
            .await
            .unwrap();
    }
}
