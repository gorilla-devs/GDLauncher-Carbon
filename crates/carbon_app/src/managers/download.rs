use std::{io, path::Path, str::FromStr};

use prisma_client_rust::QueryError;
use reqwest::{Client, Response};
use thiserror::Error;
use tokio::{
    fs::OpenOptions,
    io::{AsyncWriteExt, BufWriter},
    sync::{mpsc, watch},
};
use uuid::Uuid;

use crate::{
    db::read_filters::StringFilter,
    error::request::{RequestContext, RequestError, RequestErrorDetails},
    managers::AppRef,
};

pub struct DownloadManager {
    app: AppRef,
}

impl DownloadManager {
    pub fn new() -> Self {
        Self {
            app: AppRef::uninit(),
        }
    }

    pub fn get_appref(&self) -> &AppRef {
        &self.app
    }

    pub async fn download(
        &self,
        url: String,
        path: &Path,
        updater: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<(), DownloadError> {
        let mut handle = self.start_download(url).await?;

        while handle.status_channel.changed().await.is_ok() {
            let status = handle.status_channel.borrow().clone();
            match status {
                None => {}
                Some(DownloadStatus::Failed(e)) => return Err(DownloadError::Active(e)),
                Some(DownloadStatus::Status { downloaded, total }) => updater(downloaded, total),
                Some(DownloadStatus::Complete) => {
                    self.complete_download(handle, path).await?;
                    return Ok(());
                }
            }
        }

        return Err(DownloadError::Dropped);
    }

    pub async fn complete_download(
        &self,
        mut handle: DownloadHandle,
        target: &Path,
    ) -> Result<(), DownloadCompleteError> {
        use crate::db::active_downloads::UniqueWhereParam;

        if let Err(_) = handle.complete_channel.try_recv() {
            // no completion flag
            return Err(DownloadCompleteError::DownloadIncomplete);
        }

        let path = self
            .app
            .upgrade()
            .configuration_manager
            .runtime_path
            .get_download()
            .to_pathbuf()
            .join(handle.id.to_string());

        tokio::fs::rename(path, target)
            .await
            // explicit map_err because this is specifically a rename error,
            // not just an IO error
            .map_err(DownloadCompleteError::RenameError)?;

        self.app
            .upgrade()
            .prisma_client
            .active_downloads()
            .delete(UniqueWhereParam::FileIdEquals(handle.id.to_string()))
            .exec()
            .await?;

        Ok(())
    }

    pub async fn start_download(&self, url: String) -> Result<DownloadHandle, QueryError> {
        use crate::db::active_downloads::WhereParam;

        let active_download = self
            .app
            .upgrade()
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
                    .upgrade()
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
            .upgrade()
            .configuration_manager
            .runtime_path
            .get_download()
            .to_pathbuf()
            .join(&id);

        let (status_send, status_recv) = watch::channel::<Option<DownloadStatus>>(None);
        let (cancel_send, mut cancel_recv) = mpsc::channel::<()>(1);
        let (complete_send, complete_recv) = mpsc::channel::<()>(1);

        let client = self.app.upgrade().reqwest_client.clone();
        let task = async move {
            let task = || {
                let status_send = &status_send;
                async move {
                    let mut send_complete = true;

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
                        client: &Client,
                        url: &str,
                        start_loc: u64,
                    ) -> Result<Response, ActiveDownloadError> {
                        let mut builder = client.get(url);

                        if start_loc != 0 {
                            builder = builder.header("Range", format!("bytes={start_loc}-"));
                        }

                        let response = builder.send().await.map_err(RequestError::from_error)?;

                        let _ = response
                            .error_for_status_ref()
                            .map_err(RequestError::from_error)?;

                        Ok(response)
                    }

                    let mut response = init_request(&client, &url, start_loc).await?;

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
                                        response = init_request(&client, &url, 0).await?;
                                    } else if start < start_loc {
                                        // server gave a resume point earlier than we have, truncate the file to the
                                        // server's position.
                                        start_loc = start;
                                        file.set_len(start).await?;
                                    }
                                }
                                Err(()) => {
                                    return Err(ActiveDownloadError::Request(RequestError {
                                        context: RequestContext::from_response(&response),
                                        error: RequestErrorDetails::MalformedResponse,
                                    }))
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
                        writebuf.write(&chunk).await?;

                        if let Ok(()) = cancel_recv.try_recv() {
                            send_complete = false;
                            break; // break instead of return to flush writebuf
                        }

                        downloaded += chunk.len() as u64;
                        let _ = status_send.send(Some(DownloadStatus::Status {
                            downloaded,
                            total: length,
                        }));
                    }

                    // will NOT be flushed on drop, so it is done manually
                    writebuf.flush().await?;

                    if send_complete {
                        // the complete flag is set first to avoid a possible race condition
                        let _ = complete_send.send(()).await;
                        let _ = status_send.send(Some(DownloadStatus::Complete));
                    }

                    Ok(())
                }
            };

            match task().await {
                Ok(()) => {}
                Err(e) => {
                    let _ = status_send.send(Some(DownloadStatus::Failed(e)));
                }
            }
        };

        tokio::spawn(task);

        Ok(DownloadHandle {
            id,
            status_channel: status_recv,
            cancel_channel: cancel_send,
            complete_channel: complete_recv,
        })
    }
}

pub struct DownloadHandle {
    id: String,
    pub status_channel: watch::Receiver<Option<DownloadStatus>>,
    pub cancel_channel: mpsc::Sender<()>,
    // used to make sure a download was actually completed
    complete_channel: mpsc::Receiver<()>,
}

#[derive(Clone)]
pub enum DownloadStatus {
    Failed(ActiveDownloadError),
    Status { downloaded: u64, total: Option<u64> },
    Complete,
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
    #[error("while downloading: {0}")]
    Active(#[from] ActiveDownloadError),

    #[error("on completion: {0}")]
    Complete(#[from] DownloadCompleteError),

    #[error("query error: {0}")]
    Query(#[from] QueryError),

    #[error("channel was dropped")]
    Dropped,
}

#[cfg(test)]
mod test {
    use crate::managers::download::DownloadError;

    #[tokio::test]
    async fn attempt_download() -> Result<(), DownloadError> {
        let app = crate::setup_managers_for_test().await;

        let tmpfolder = app
            .configuration_manager
            .runtime_path
            .get_temp()
            .to_pathbuf();

        tokio::fs::create_dir_all(tmpfolder).await.unwrap();

        app.download_manager
            .download(
                String::from("https://gdlauncher.com"),
                &app.configuration_manager
                    .runtime_path
                    .get_temp()
                    .to_pathbuf()
                    .join("gdl.html"),
                &mut |downloaded, total| println!("Downloaded {downloaded}/{total:?}"),
            )
            .await
    }
}
