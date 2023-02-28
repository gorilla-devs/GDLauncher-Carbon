use std::{io, path::Path, str::FromStr};

use prisma_client_rust::QueryError;
use reqwest::{Client, Response};
use thiserror::Error;
use tokio::{
    fs::OpenOptions,
    io::{AsyncWriteExt, BufWriter},
    sync::mpsc,
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

        let (status_send, status_recv) = mpsc::unbounded_channel::<DownloadStatus>();
        let (cancel_send, mut cancel_recv) = mpsc::channel::<()>(1);
        let (complete_send, complete_recv) = mpsc::channel::<()>(1);

        let client = self.app.upgrade().reqwest_client.clone();
        let task = async move {
            let task = || {
                let status_send = &status_send;
                async move {
                    let mut send_complete = true;

                    tokio::fs::create_dir_all(
                        path.parent()
                            .ok_or(DownloadError::MalformedPath)
                            .map_err(DownloadStatus::FailedToStart)?,
                    )
                    .await
                    .map_err(DownloadError::IoError)
                    .map_err(DownloadStatus::FailedToStart)?;

                    let file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .read(true)
                        .open(&path)
                        .await
                        .map_err(DownloadError::IoError)
                        .map_err(DownloadStatus::FailedToStart)?;

                    let mut start_loc = file
                        .metadata()
                        .await
                        .map_err(DownloadError::IoError)
                        .map_err(DownloadStatus::FailedToStart)?
                        .len();

                    async fn init_request(
                        client: &Client,
                        url: &str,
                        start_loc: u64,
                    ) -> Result<Response, DownloadStatus> {
                        let mut builder = client.get(url);

                        if start_loc != 0 {
                            builder = builder.header("Range", format!("bytes={start_loc}-"));
                        }

                        let response = builder
                            .send()
                            .await
                            .map_err(RequestError::from_error)
                            .map_err(DownloadError::Request)
                            .map_err(DownloadStatus::FailedToStart)?;

                        let _ = response
                            .error_for_status_ref()
                            .map_err(RequestError::from_error)
                            .map_err(DownloadError::Request)
                            .map_err(DownloadStatus::FailedToStart)?;

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
                                        file.set_len(0)
                                            .await
                                            .map_err(DownloadError::IoError)
                                            .map_err(DownloadStatus::FailedToStart)?;
                                        response = init_request(&client, &url, 0).await?;
                                    } else if start < start_loc {
                                        // server gave a resume point earlier than we have, truncate the file to the
                                        // server's position.
                                        start_loc = start;
                                        file.set_len(start)
                                            .await
                                            .map_err(DownloadError::IoError)
                                            .map_err(DownloadStatus::FailedToStart)?;
                                    }
                                }
                                Err(()) => {
                                    return Err(DownloadStatus::FailedToStart(
                                        DownloadError::Request(RequestError {
                                            context: RequestContext::from_response(&response),
                                            error: RequestErrorDetails::MalformedResponse,
                                        }),
                                    ))
                                }
                            }
                        }
                        None => {
                            start_loc = 0;
                            file.set_len(0)
                                .await
                                .map_err(DownloadError::IoError)
                                .map_err(DownloadStatus::FailedToStart)?;
                        }
                    }

                    let mut writebuf = BufWriter::new(file);

                    let length = response.content_length().map(|x| x as u64 + start_loc);

                    let mut downloaded = start_loc;

                    let _ = status_send.send(DownloadStatus::Status {
                        downloaded,
                        total: length,
                    });

                    while let Some(chunk) = response
                        .chunk()
                        .await
                        .map_err(RequestError::from_error)
                        .map_err(DownloadError::Request)
                        .map_err(DownloadStatus::FailedInProgress)?
                    {
                        writebuf
                            .write(&chunk)
                            .await
                            .map_err(DownloadError::IoError)
                            .map_err(DownloadStatus::FailedInProgress)?;

                        if let Ok(()) = cancel_recv.try_recv() {
                            send_complete = false;
                            break; // break instead of return to flush writebuf
                        }

                        downloaded += chunk.len() as u64;
                        let _ = status_send.send(DownloadStatus::Status {
                            downloaded,
                            total: length,
                        });
                    }

                    // will NOT be flushed on drop, so it is done manually
                    writebuf
                        .flush()
                        .await
                        .map_err(DownloadError::IoError)
                        .map_err(DownloadStatus::FailedInProgress)?;

                    if send_complete {
                        // the complete flag is set first to avoid a possible race condition
                        let _ = complete_send.send(()).await;
                        let _ = status_send.send(DownloadStatus::Complete);
                    }

                    Ok(())
                }
            };

            match task().await {
                Ok(()) => {}
                Err(e) => {
                    let _ = status_send.send(e);
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
    pub status_channel: mpsc::UnboundedReceiver<DownloadStatus>,
    pub cancel_channel: mpsc::Sender<()>,
    // used to make sure a download was actually completed
    complete_channel: mpsc::Receiver<()>,
}

pub enum DownloadStatus {
    FailedToStart(DownloadError),
    FailedInProgress(DownloadError),
    Status { downloaded: u64, total: Option<u64> },
    Complete,
}

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("request error: {0}")]
    Request(#[from] RequestError),

    #[error("malformed save path")]
    MalformedPath,

    #[error("io error: {0}")]
    IoError(#[from] io::Error),
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

#[cfg(test)]
mod test {
    use super::DownloadStatus;

    #[tokio::test]
    async fn attempt_download() {
        let app = crate::setup_managers_for_test().await;

        let mut handle = app
            .download_manager
            .start_download(String::from("https://gdlauncher.com/"))
            .await
            .unwrap();

        while let Some(msg) = handle.status_channel.recv().await {
            match msg {
                DownloadStatus::FailedToStart(e) => Err(e).unwrap(),
                DownloadStatus::FailedInProgress(e) => Err(e).unwrap(),
                DownloadStatus::Complete => {
                    let tmpfolder = app
                        .configuration_manager
                        .runtime_path
                        .get_temp()
                        .to_pathbuf();

                    tokio::fs::create_dir_all(tmpfolder).await.unwrap();

                    app.download_manager
                        .complete_download(
                            handle,
                            &app.configuration_manager
                                .runtime_path
                                .get_temp()
                                .to_pathbuf()
                                .join("gdl.html"),
                        )
                        .await
                        .unwrap();

                    return;
                }
                DownloadStatus::Status { downloaded, total } => {
                    println!("downloaded: {downloaded}/{total:?}");
                }
            }
        }
        dbg!();

        panic!("channel dropped before completion");
    }
}
