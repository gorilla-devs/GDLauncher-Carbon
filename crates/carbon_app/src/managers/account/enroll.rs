use std::sync::Arc;

use async_trait::async_trait;
use futures::{future::abortable, stream::AbortHandle};
use matchout::Extract;
use thiserror::Error;
use tokio::sync::RwLock;

use crate::error::{request::RequestError, UError};

use super::api::{
    DeviceCode, DeviceCodePollError, DeviceCodeRequestError, FullAccount, McAccountPopulateError,
    McAuth, McAuthError, McEntitlementError, McProfileError, XboxError,
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
    Failed(UError<EnrollmentError>),
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
                let device_code = DeviceCode::request_code(&client)
                    .await
                    .map_err(UError::map)?;

                // poll ms auth
                update_status(EnrollmentStatus::PollingCode(device_code.clone())).await;
                let ms_auth = device_code
                    .poll_ms_auth(&client)
                    .await
                    .map_err(UError::map)?;

                // authenticate with MC
                update_status(EnrollmentStatus::McLogin).await;
                let mc_auth = McAuth::auth_ms(&ms_auth, &client)
                    .await
                    .map_err(UError::map)?;

                update_status(EnrollmentStatus::PopulateAccount).await;
                let populated = mc_auth.populate(&client).await.map_err(UError::map)?;

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

#[derive(Error, Debug, Extract)]
pub enum EnrollmentError {
    #[error("request error: {0}")]
    #[extract(DeviceCodeRequestError(self.0))]
    #[extract(DeviceCodePollError::RequestError(self.0))]
    #[extract(McAuthError::Request(self.0))]
    #[extract(McAccountPopulateError::Request(self.0))]
    Request(RequestError),

    #[error("device code expired")]
    #[extract(DeviceCodePollError::CodeExpired)]
    CodeExpired,

    #[error("error getting xbox auth: {0}")]
    XboxAuth(#[extract(McAuthError::Xbox)] XboxError),

    #[error("error checking entitlements: {0}")]
    EntitlementCheckError(#[extract(McAccountPopulateError::Entitlement)] McEntitlementError),

    #[error("no profile attached to account")]
    #[extract(McAccountPopulateError::Profile(McProfileError::NoProfile))]
    NoProfile,
}

/*
mod test {
    use std::sync::Arc;

    use async_trait::async_trait;
    use tokio::sync::RwLock;

    use crate::managers::account::enroll::EnrollmentStatus;

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
                let enrollment1 = self.enrollment.read().await;
                let enrollment = enrollment1.as_ref().unwrap().status.read().await;
                if let EnrollmentStatus::Failed(e) = &*enrollment {
                    println!("{e}");
                }
                println!("Invalidate: {enrollment:#?}",);
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
