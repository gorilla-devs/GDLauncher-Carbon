//! Shared error types

use std::fmt::{self, Display, Formatter};

use reqwest::{StatusCode, Url};
use thiserror::Error;

#[derive(Error, Debug, Clone)]
#[error("request error for {context}: {error}")]
pub struct RequestError {
    pub context: RequestContext,
    pub error: RequestErrorDetails,
}

/// Extra context information for request errors
#[derive(Debug, Clone)]
pub struct RequestContext {
    url: Option<Url>,
}

#[derive(Error, Debug, Clone)]
pub enum RequestErrorDetails {
    #[error("unexpected status received: {0}")]
    UnexpectedStatus(StatusCode),

    #[error("connection timed out")]
    Timeout,

    // Unexpected errors are converted to strings so RequestError can be Clone.
    #[error("unexpected error returned from reqwest: {0}")]
    Unexpected(String),
}

impl Display for RequestContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.url
                .map(|u| u.to_string())
                .unwrap_or(String::from("<redacted>"))
        )
    }
}

impl From<reqwest::Error> for RequestError {
    fn from(value: reqwest::Error) -> Self {
        Self {
            context: RequestContext {
                url: value.url().cloned(),
            },
            error: {
                if value.is_status() {
                    RequestErrorDetails::UnexpectedStatus(value.status().unwrap())
                } else if value.is_timeout() {
                    RequestErrorDetails::Timeout
                } else {
                    RequestErrorDetails::Unexpected(value.to_string())
                }
            },
        }
    }
}
