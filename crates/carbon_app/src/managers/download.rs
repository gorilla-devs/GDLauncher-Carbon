use std::io;

use thiserror::Error;
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
    sync::mpsc,
};
use uuid::Uuid;

use crate::{error::request::RequestError, managers::AppRef};

struct DownloadManager {
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

    pub async fn start_download(&self, url: String) -> DownloadHandle {
        let id = Uuid::new_v4();
        let path = self
            .app
            .upgrade()
            .configuration_manager
            .runtime_path
            .get_download()
            .to_pathbuf()
            .join(id.to_string());

        let (status_send, status_recv) = mpsc::unbounded_channel::<DownloadStatus>();
        let (cancel_send, mut cancel_recv) = mpsc::channel(1);

        let app = self.app.clone();
        let task = async move {
            let task = || async {
                let client = app.upgrade().reqwest_client.clone();

                let mut response = client
                    .get(url)
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

                let length = response.content_length().map(|x| x as u64);

                tokio::fs::create_dir_all(
                    path.parent()
                        .ok_or(DownloadError::MalformedPath)
                        .map_err(DownloadStatus::FailedToStart)?,
                )
                .await
                .map_err(DownloadError::IoError)
                .map_err(DownloadStatus::FailedToStart)?;

                let file = File::create(path)
                    .await
                    .map_err(DownloadError::IoError)
                    .map_err(DownloadStatus::FailedToStart)?;

                let mut writebuf = BufWriter::new(file);

                let _ = status_send.send(DownloadStatus::Status {
                    downloaded: 0,
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
                        break; // break instead of return to flush writebuf
                    }

                    let _ = status_send.send(DownloadStatus::Status {
                        downloaded: chunk.len() as u64,
                        total: length,
                    });
                }

                // will NOT be flushed on drop, so it is done manually
                writebuf
                    .flush()
                    .await
                    .map_err(DownloadError::IoError)
                    .map_err(DownloadStatus::FailedInProgress)?;

                Ok(())
            };

            match task().await {
                Ok(()) => {}
                Err(e) => {
                    let _ = status_send.send(e);
                }
            }
        };

        tokio::spawn(task);

        DownloadHandle {
            id,
            status_channel: status_recv,
            cancel_channel: cancel_send,
        }
    }
}

pub struct DownloadHandle {
    id: Uuid,
    pub status_channel: mpsc::UnboundedReceiver<DownloadStatus>,
    pub cancel_channel: mpsc::Sender<()>,
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
