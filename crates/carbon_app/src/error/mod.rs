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
}

pub type AxumError = (axum::http::StatusCode, String);

impl FeError {
    pub fn from_anyhow(error: &anyhow::Error) -> Self {
        use crate::managers::account::gdl_account::InstanceShareError;

        Self {
            cause: error
                .chain()
                .map(|entry| {
                    // Try to downcast to InstanceShareError to extract the error code
                    let code = entry
                        .downcast_ref::<InstanceShareError>()
                        .map(|e| e.error_code().to_string());

                    CauseSegment {
                        display: format!("{entry}"),
                        debug: format!("{entry:#?}"),
                        code,
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
        }
    }

    pub fn from_display_debug(v: impl Display + Debug) -> Self {
        Self {
            display: format!("{v}"),
            debug: format!("{v:#?}"),
            code: None,
        }
    }
}

impl From<anyhow::Error> for FeError {
    fn from(value: anyhow::Error) -> Self {
        FeError::from_anyhow(&value)
    }
}
