use std::{thread::JoinHandle, sync::Arc, mem::MaybeUninit};

use futures::{future::abortable, stream::AbortHandle};
use reqwest::Client;

use super::api::{DeviceCode, MsAuth, DeviceCodePollError, DeviceCodeRequestError};

/// Active process of adding an account
pub struct EnrollmentTask {
    status: EnrollmentStatus,
    abort: AbortHandle,
}

enum EnrollmentStatus {
    RequestCode,
    PollingCode(DeviceCode),
    Aborted,
    Failed(EnrollmentError),
}

impl EnrollmentTask {
    /// Begin account enrollment. `invalidate_fn` will be called
    /// whenever the task's status updates. `fail` will be called
    /// if there is an error completing the task.
    pub fn begin(client: reqwest::Client, invalidate: Fn()) -> Arc<EnrollmentTask> {
        // Create the arc in an uninitialized state, because it must have the
        // task's abort handle to be created.
        let enrollment = Arc::<EnrollmentTask>::new_uninit();

        let task = async {
            let client = client;
            let invalidate = invalidate;

            // The task does not start until enrollment is initialized.
            let enrollment = unsafe { enrollment.clone().assume_init() };

            let update_status = |status: EnrollmentStatus| {
                enrollment.status = status;
                invalidate();
            };

            let task = || -> Result<(), EnrollmentError> {
                // request device code
                let device_code = DeviceCode::request_code(&client).await?;

                // poll ms auth
                update_status(EnrollmentStatus::PollingCode(device_code.clone()));
                let ms_auth = device_code.poll(&client).await?;

                // TODO

                Ok(())
            };

            match task() {
                Ok(()) => {},
                Err(e) => update_status(EnrollmentStatus::Failed(e)),
            };
        };

        let (task, abort_handle) = abortable(task);

        // Initialize the data in the arc BEFORE starting the task.
        enrollment.write(EnrollmentTask {
            status: EnrollmentStatus::RequestCode,
            abort: abort_handle,
        });

        // Remove the MaybeUninit as it is safe to assume the task has been
        // initialized.
        let enrollment = unsafe { enrollment.assume_init() };

        // Start the task AFTER enrollment has been initialized.
        let task = tokio::task::spawn(task);

        enrollment
    }

    /// Abort the enrollment task
    pub fn abort(self) {
        self.status = EnrollmentStatus::Aborted;
        self.abort.abort();
    }
}

#[derive(Error, Debug)]
pub enum EnrollmentError {
    #[error("error requesting device code: {0}")]
    DeviceCodeRequest(DeviceCodeRequestError),

    #[error("error polling device code: {0}")]
    DeviceCodePoll(DeviceCodePollError),
}
