use std::sync::Arc;

use super::api::{
    get_profile, DeviceCode, DeviceCodeExpiredError, FullAccount, GetProfileError, McAccount,
    McAuth, McEntitlementMissingError, MsAuth, XboxAuth, XboxError,
};
use anyhow::anyhow;
use async_trait::async_trait;
use futures::{future::abortable, stream::AbortHandle};
use thiserror::Error;
use tokio::sync::RwLock;

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
    Failed(anyhow::Result<EnrollmentError>),
}

impl EnrollmentTask {
    /// Begin account enrollment. `invalidate_fn` will be called
    /// whenever the task's status updates.
    pub fn begin(
        client: reqwest_middleware::ClientWithMiddleware,
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
                let ms_auth = device_code.poll_ms_auth(&client).await??;

                update_status(EnrollmentStatus::McLogin).await;

                // authenticate with XBox
                let xbox_auth = XboxAuth::from_ms(&ms_auth, &client).await??;

                // authenticate with MC
                let mc_auth = McAuth::auth_ms(xbox_auth, &client).await?;

                update_status(EnrollmentStatus::PopulateAccount).await;
                let account = McAccount {
                    entitlement: mc_auth.get_entitlement(&client).await??,
                    profile: get_profile(&client, &mc_auth.access_token).await??,
                    auth: mc_auth,
                };

                update_status(EnrollmentStatus::Complete(FullAccount {
                    ms: ms_auth,
                    mc: account,
                }))
                .await;

                Ok(())
            };

            match task().await {
                Ok(()) => {}
                Err(EnrollmentErrorOrAnyhow::EnrollmentError(e)) => {
                    update_status(EnrollmentStatus::Failed(Ok(e))).await
                }
                Err(EnrollmentErrorOrAnyhow::Anyhow(e)) => {
                    update_status(EnrollmentStatus::Failed(Err(e))).await
                }
            };
        };

        let (task, abort_handle) = abortable(task);
        tokio::task::spawn(task);

        Self {
            status,
            abort: abort_handle,
        }
    }

    pub fn refresh(
        client: reqwest_middleware::ClientWithMiddleware,
        refresh_token: String,
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
                // attempt to refresh token
                let ms_auth = MsAuth::refresh(&client, &refresh_token).await?;

                update_status(EnrollmentStatus::McLogin).await;

                // authenticate with XBox
                let xbox_auth = XboxAuth::from_ms(&ms_auth, &client).await??;

                // authenticate with MC
                let mc_auth = McAuth::auth_ms(xbox_auth, &client).await?;

                update_status(EnrollmentStatus::PopulateAccount).await;
                let account = McAccount {
                    entitlement: mc_auth.get_entitlement(&client).await??,
                    profile: get_profile(&client, &mc_auth.access_token).await??,
                    auth: mc_auth,
                };

                update_status(EnrollmentStatus::Complete(FullAccount {
                    ms: ms_auth,
                    mc: account,
                }))
                .await;

                Ok(())
            };

            match task().await {
                Ok(()) => {}
                Err(EnrollmentErrorOrAnyhow::EnrollmentError(e)) => {
                    update_status(EnrollmentStatus::Failed(Ok(e))).await
                }
                Err(EnrollmentErrorOrAnyhow::Anyhow(e)) => {
                    update_status(EnrollmentStatus::Failed(Err(e))).await
                }
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

#[derive(Error, Debug, Clone)]
pub enum EnrollmentError {
    #[error("device code expired")]
    DeviceCodeExpired,
    #[error("xbox error: {0}")]
    XboxError(#[from] XboxError),
    #[error("game entitlement missing")]
    EntitlementMissing,
    #[error("game profile missing")]
    GameProfileMissing,
}

pub enum EnrollmentErrorOrAnyhow {
    EnrollmentError(EnrollmentError),
    Anyhow(anyhow::Error),
}

impl From<DeviceCodeExpiredError> for EnrollmentErrorOrAnyhow {
    fn from(_: DeviceCodeExpiredError) -> Self {
        Self::EnrollmentError(EnrollmentError::DeviceCodeExpired)
    }
}

impl From<XboxError> for EnrollmentErrorOrAnyhow {
    fn from(value: XboxError) -> Self {
        Self::EnrollmentError(EnrollmentError::XboxError(value))
    }
}

impl From<McEntitlementMissingError> for EnrollmentErrorOrAnyhow {
    fn from(_: McEntitlementMissingError) -> Self {
        Self::EnrollmentError(EnrollmentError::EntitlementMissing)
    }
}

impl From<GetProfileError> for EnrollmentErrorOrAnyhow {
    fn from(value: GetProfileError) -> Self {
        match value {
            GetProfileError::GameProfileMissing => {
                Self::EnrollmentError(EnrollmentError::GameProfileMissing)
            }
            GetProfileError::AuthTokenInvalid => {
                Self::Anyhow(anyhow!(GetProfileError::AuthTokenInvalid))
            }
        }
    }
}

impl From<anyhow::Error> for EnrollmentErrorOrAnyhow {
    fn from(value: anyhow::Error) -> Self {
        Self::Anyhow(value)
    }
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
