use std::sync::Arc;

use async_trait::async_trait;
use futures::{future::abortable, stream::AbortHandle};
use thiserror::Error;
use tokio::sync::RwLock;

use super::api::{
    DeviceCode, DeviceCodePollError, DeviceCodeRequestError, FullAccount, McAccountPopulateError,
    McAuth, McAuthError,
};

/// Active process of adding an account
pub struct EnrollmentTask {
    pub status: Arc<RwLock<EnrollmentStatus>>,
    abort: AbortHandle,
}

#[derive(Debug)]
pub enum EnrollmentStatus {
    RequestingCode,
    PollingCode(DeviceCode),
    McLogin,
    PopulateAccount,
    Complete(FullAccount),
    Failed(EnrollmentError),
}

impl EnrollmentTask {
    /// Begin account enrollment. `invalidate_fn` will be called
    /// whenever the task's status updates.
    pub fn begin(
        client: reqwest::Client,
        invalidate: impl InvalidateCtx + Send + Sync + 'static,
    ) -> Self {
        let status = Arc::new(RwLock::new(EnrollmentStatus::RequestingCode));
        let task_status = status.clone();

        let task = async move {
            let update_status = |status: EnrollmentStatus| async {
                *task_status.write().await = status;
                invalidate.invalidate().await;
            };

            let task = || async {
                // request device code
                let device_code = DeviceCode::request_code(&client).await?;

                // poll ms auth
                update_status(EnrollmentStatus::PollingCode(device_code.clone())).await;
                let ms_auth = device_code.poll_ms_auth(&client).await?;

                // authenticate with MC
                update_status(EnrollmentStatus::McLogin).await;
                let mc_auth = McAuth::auth_ms(&ms_auth, &client).await?;

                update_status(EnrollmentStatus::PopulateAccount).await;
                let populated = mc_auth.populate(&client).await?;

                update_status(EnrollmentStatus::Complete(FullAccount {
                    ms: ms_auth,
                    mc: populated,
                }))
                .await;

                Ok(())
            };

            match task().await {
                Ok(()) => {}
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
}

impl Drop for EnrollmentTask {
    fn drop(&mut self) {
        self.abort.abort()
    }
}

#[async_trait]
pub trait InvalidateCtx {
    async fn invalidate(&self);
}

#[derive(Error, Debug)]
pub enum EnrollmentError {
    #[error("error requesting device code: {0}")]
    DeviceCodeRequest(#[from] DeviceCodeRequestError),

    #[error("error polling device code: {0}")]
    DeviceCodePoll(#[from] DeviceCodePollError),

    #[error("error getting mc auth: {0}")]
    McAuth(#[from] McAuthError),

    #[error("error populating account details: {0}")]
    Populate(#[from] McAccountPopulateError),
}

/*
mod test {
    use std::sync::Arc;

    use async_trait::async_trait;
    use tokio::sync::RwLock;

    use super::InvalidateCtx;

    use super::EnrollmentTask;

    #[tokio::test]
    async fn test_mc_auth() {
        let enrollment = Arc::new(RwLock::new(Option::<EnrollmentTask>::None));

        struct Printer {
            enrollment: Arc<RwLock<Option<EnrollmentTask>>>,
        }

        #[async_trait]
        impl InvalidateCtx for Printer {
            async fn invalidate(&self) {
                println!(
                    "Invalidate: {:#?}",
                    self.enrollment
                        .read()
                        .await
                        .as_ref()
                        .unwrap()
                        .status
                        .read()
                        .await
                );
            }
        }

        *enrollment.write().await = Some(EnrollmentTask::begin(
            reqwest::Client::new(),
            Printer {
                enrollment: enrollment.clone(),
            },
        ));

        tokio::time::sleep(std::time::Duration::from_secs(10000)).await
    }
}*/
