pub mod request;

use std::fmt::{Debug, Display};

use serde::Serialize;

#[derive(Serialize)]
pub struct FeError {
    cause: Vec<CauseSegment>,
    backtrace: String,
}

#[derive(Serialize)]
pub struct CauseSegment {
    pub display: String,
    pub debug: String,
}

pub type AxumError = (axum::http::StatusCode, String);

impl FeError {
    pub fn from_anyhow(error: &anyhow::Error) -> Self {
        Self {
            cause: error
                .chain()
                .map(|entry| CauseSegment {
                    display: format!("{entry}"),
                    debug: format!("{entry:#?}"),
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
        }
    }

    pub fn from_display_debug(v: impl Display + Debug) -> Self {
        Self {
            display: format!("{v}"),
            debug: format!("{v:#?}"),
        }
    }
}

impl From<anyhow::Error> for FeError {
    fn from(value: anyhow::Error) -> Self {
        FeError::from_anyhow(&value)
    }
}

pub fn anyhow_into_rspc(value: anyhow::Error) -> rspc::Error {
    rspc::Error::new(
        rspc::ErrorCode::InternalServerError,
        format!("backend error: {value:#?}"),
    )
}
