use std::{thread::JoinHandle, sync::Arc, mem::MaybeUninit};

use futures::{future::abortable, stream::AbortHandle};
use reqwest::Client;
use thiserror::Error;
use tokio::sync::RwLock;

use super::api::{DeviceCode, MsAuth, DeviceCodePollError, DeviceCodeRequestError};

/// Active process of adding an account
pub struct EnrollmentTask {
    status: Arc<RwLock<EnrollmentStatus>>,
    abort: AbortHandle,
}

enum EnrollmentStatus {
    RequestingCode,
    PollingCode(DeviceCode),
    Aborted,
    Failed(EnrollmentError),
}

impl EnrollmentTask {
    /// Begin account enrollment. `invalidate_fn` will be called
    /// whenever the task's status updates.
    pub fn begin(client: reqwest::Client, invalidate: impl Fn() + Send + Sync + 'static) -> Self {
        let status = Arc::new(RwLock::new(EnrollmentStatus::RequestingCode));
        let task_status = status.clone();

        let task = async move {
            let update_status = |status: EnrollmentStatus| {
                async {
                    *task_status.write().await = status;
                    invalidate();
                }
            };

            let task = || {
                async {
                    // request device code
                    let device_code = DeviceCode::request_code(&client).await?;

                    // poll ms auth
                    update_status(EnrollmentStatus::PollingCode(device_code.clone())).await;
                    let ms_auth = device_code.poll(&client).await?;

                    // TODO

                    Ok::<_, EnrollmentError>(())
                }
            };

            match task().await {
                Ok(()) => {},
                Err(e) => update_status(EnrollmentStatus::Failed(e)).await,
            };
        };

        let (task, abort_handle) = abortable(task);
        tokio::task::spawn(task);


        Self {
            status,
            abort: abort_handle,
        }
    }

    /// Abort the enrollment task
    pub async fn abort(self) {
        *self.status.write().await = EnrollmentStatus::Aborted;
        self.abort.abort();
    }
}

#[derive(Error, Debug)]
pub enum EnrollmentError {
    #[error("error requesting device code: {0}")]
    DeviceCodeRequest(#[from] DeviceCodeRequestError),

    #[error("error polling device code: {0}")]
    DeviceCodePoll(#[from] DeviceCodePollError),
}
