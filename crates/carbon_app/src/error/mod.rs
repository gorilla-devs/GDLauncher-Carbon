pub mod json;
pub mod request;
pub mod sentry;

use std::fmt::{Debug, Display};

use serde::Serialize;
use specta::Type;

#[derive(Debug, Type, Serialize)]
pub struct FeError {
    cause: Vec<CauseSegment>,
    backtrace: String,
}

#[derive(Debug, Type, Serialize)]
pub struct CauseSegment {
    pub display: String,
    pub debug: String,
    /// Optional error code for structured error handling (e.g., "QUOTA_EXCEEDED")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Optional structured data for frontend consumption
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Trait for errors that carry a structured error code (and optional data) for the frontend.
pub trait FeErrorCode {
    fn error_code(&self) -> &'static str;
    fn error_data(&self) -> Option<serde_json::Value> {
        None
    }
}

pub type AxumError = (axum::http::StatusCode, String);

/// Try to extract an error code and data from an entry in the anyhow error chain.
fn extract_fe_error(
    entry: &(dyn std::error::Error + 'static),
) -> (Option<String>, Option<serde_json::Value>) {
    use crate::managers::account::gdl_account::{AvatarUploadError, InstanceShareError};
    use crate::managers::instance::run::InsufficientMemoryError;
    use crate::managers::server::EulaNotAcceptedError;

    macro_rules! try_downcast {
        ($entry:expr, $($ty:ty),+ $(,)?) => {
            $(
                if let Some(e) = $entry.downcast_ref::<$ty>() {
                    return (Some(e.error_code().to_string()), e.error_data());
                }
            )+
        };
    }

    try_downcast!(
        entry,
        InstanceShareError,
        AvatarUploadError,
        InsufficientMemoryError,
        EulaNotAcceptedError
    );

    (None, None)
}

impl FeError {
    pub fn from_anyhow(error: &anyhow::Error) -> Self {
        Self {
            cause: error
                .chain()
                .map(|entry| {
                    let (code, data) = extract_fe_error(entry);

                    CauseSegment {
                        display: format!("{entry}"),
                        debug: format!("{entry:#?}"),
                        code,
                        data,
                    }
                })
                .collect(),
            backtrace: format!("{}", error.backtrace()),
        }
    }

    pub fn extend(&mut self, segment: CauseSegment) {
        self.cause.push(segment);
    }

    pub fn make_rspc(&self) -> rspc::Error {
        rspc::Error::new(
            rspc::ErrorCode::InternalServerError,
            serde_json::to_string_pretty(self).expect("could not convert FeError to json"),
        )
    }

    pub fn make_axum(&self) -> AxumError {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::to_string_pretty(self).expect("could not convert FeError to json"),
        )
    }
}

impl CauseSegment {
    pub fn from_display(v: impl Display) -> Self {
        Self {
            display: format!("{v}"),
            debug: String::new(),
            code: None,
            data: None,
        }
    }

    pub fn from_display_debug(v: impl Display + Debug) -> Self {
        Self {
            display: format!("{v}"),
            debug: format!("{v:#?}"),
            code: None,
            data: None,
        }
    }
}

impl From<anyhow::Error> for FeError {
    fn from(value: anyhow::Error) -> Self {
        FeError::from_anyhow(&value)
    }
}
