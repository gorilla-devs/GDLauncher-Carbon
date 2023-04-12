use std::fmt::{self, Display, Formatter};

use reqwest::{Response, StatusCode, Url};
use rspc::Type;
use serde::Serialize;
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

    #[error("unknown reqwest error: {0}")]
    Unknown(String),
}

impl Display for RequestContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.url
                .as_ref()
                .map(|u| u.to_string())
                .unwrap_or_else(|| String::from("<no url>"))
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
            Self::UnexpectedStatus {
                status: error.status().unwrap(),
                details: None,
            }
        } else if error.is_timeout() {
            Self::Timeout
        } else if error.is_connect() {
            Self::ConnectionFailed
        } else if error.is_decode() {
            Self::MalformedResponse
        } else {
            Self::Unknown(format!("{error:#?}"))
        }
    }

    pub fn from_status(response: &Response) -> Self {
        RequestErrorDetails::UnexpectedStatus {
            status: response.status(),
            details: None,
        }
    }

    pub fn from_error_censored(error: reqwest::Error) -> Self {
        Self::from_error(error.without_url())
    }
}

impl RequestError {
    pub fn from_status(response: &Response) -> Self {
        Self {
            context: RequestContext::from_response(response),
            error: RequestErrorDetails::from_status(response),
        }
    }

    pub fn from_error(value: reqwest::Error) -> Self {
        Self {
            context: RequestContext::from_error(&value),
            error: RequestErrorDetails::from_error(value),
        }
    }

    pub fn from_error_censored(value: reqwest::Error) -> Self {
        Self {
            context: RequestContext::from_error(&value),
            error: RequestErrorDetails::from_error_censored(value),
        }
    }
}

#[derive(Type, Serialize)]
pub struct FERequestError {
    url: Option<String>,
    type_: FERequestErrorType,
}

#[derive(Type, Serialize)]
pub enum FERequestErrorType {
    UnexpectedStatus {
        status: u16,
        details: Option<String>,
    },

    Timeout,
    ConnectionFailed,
    MalformedResponse,
    Unknown(String),
}

impl From<RequestError> for FERequestError {
    fn from(value: RequestError) -> Self {
        Self {
            url: value.context.url.as_ref().map(ToString::to_string),
            type_: match value.error {
                RequestErrorDetails::UnexpectedStatus { status, details } => {
                    FERequestErrorType::UnexpectedStatus {
                        status: status.as_u16(),
                        details,
                    }
                }
                RequestErrorDetails::Timeout => FERequestErrorType::Timeout,
                RequestErrorDetails::ConnectionFailed => FERequestErrorType::ConnectionFailed,
                RequestErrorDetails::MalformedResponse => FERequestErrorType::MalformedResponse,
                RequestErrorDetails::Unknown(x) => FERequestErrorType::Unknown(x),
            },
        }
    }
}

pub fn censor_error(error: reqwest_middleware::Error) -> reqwest_middleware::Error {
    match error {
        reqwest_middleware::Error::Reqwest(e) => {
            reqwest_middleware::Error::Reqwest(e.without_url())
        }
        e => e,
    }
}
