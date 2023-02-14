//! Shared error types

use std::fmt::{self, Display, Formatter};

use carbon_domain::error::UnhandledError;
use reqwest::{Response, StatusCode, Url};
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
    pub url: Option<Url>,
}

#[derive(Error, Debug, Clone)]
pub enum RequestErrorDetails {
    #[error("unexpected status received: {status}: details: {details:?}")]
    UnexpectedStatus {
        status: StatusCode,
        details: Option<String>,
    },

    #[error("connection timed out")]
    Timeout,

    #[error("connection failed")]
    ConnectionFailed,

    #[error("malformed response")]
    MalformedResponse, // TODO: get body

    // Unexpected errors are converted to strings so RequestError can be Clone.
    #[error("unexpected error returned from reqwest: {0}")]
    Unexpected(UnhandledError),
}

impl Display for RequestContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.url
                .as_ref()
                .map(|u| u.to_string())
                .unwrap_or(String::from("<no url>"))
        )
    }
}

impl RequestContext {
    pub fn none() -> Self {
        Self { url: None }
    }

    pub fn from_error(error: &reqwest::Error) -> Self {
        Self {
            url: error.url().cloned(),
        }
    }

    pub fn from_response(response: &Response) -> Self {
        Self {
            url: Some(response.url().clone()),
        }
    }
}

impl RequestErrorDetails {
    pub fn from_error(error: reqwest::Error) -> Self {
        if error.is_status() {
            RequestErrorDetails::UnexpectedStatus {
                status: error.status().unwrap(),
                details: None,
            }
        } else if error.is_timeout() {
            RequestErrorDetails::Timeout
        } else if error.is_connect() {
            RequestErrorDetails::ConnectionFailed
        } else if error.is_decode() {
            RequestErrorDetails::MalformedResponse
        } else {
            RequestErrorDetails::Unexpected(UnhandledError::new(error))
        }
    }

    pub fn from_status(response: &Response) -> Self {
        RequestErrorDetails::UnexpectedStatus {
            status: response.status(),
            details: None,
        }
    }
}

impl From<reqwest::Error> for RequestError {
    fn from(value: reqwest::Error) -> Self {
        Self {
            context: RequestContext::from_error(&value),
            error: RequestErrorDetails::from_error(value),
        }
    }
}

impl RequestError {
    pub fn from_status(response: &Response) -> Self {
        Self {
            context: RequestContext::from_response(response),
            error: RequestErrorDetails::from_status(response),
        }
    }
}
