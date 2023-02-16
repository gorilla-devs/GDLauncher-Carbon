use std::fmt::{self, Display, Formatter};

use reqwest::{Response, StatusCode, Url};
use std::error::Error as StdError;
use thiserror::Error;

use super::{HandlingActions, UError, UnexpectedError};

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
    pub fn from_error(error: reqwest::Error) -> UError<Self> {
        let error = if error.is_status() {
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
            return UError::Unexpected(UnexpectedError::new(error, HandlingActions::None));
        };

        UError::Expected(error)
    }

    pub fn from_status(response: &Response) -> Self {
        RequestErrorDetails::UnexpectedStatus {
            status: response.status(),
            details: None,
        }
    }
}

impl RequestError {
    pub fn from_status(response: &Response) -> Self {
        Self {
            context: RequestContext::from_response(&response),
            error: RequestErrorDetails::from_status(response),
        }
    }

    pub fn from_reqwest(value: reqwest::Error) -> UError<Self> {
        let context = RequestContext::from_error(&value);

        match RequestErrorDetails::from_error(value) {
            UError::Unexpected(e) => UError::Unexpected(e),
            UError::Expected(e) => UError::Expected(RequestError { context, error: e }),
        }
    }

    /// Convenience function for mapping a [reqwest::Error] to any error
    /// that implements From<RequestError>.
    pub fn map<E: StdError + From<Self>>(value: reqwest::Error) -> UError<E> {
        UError::map(Self::from_reqwest(value))
    }

    /// Same as [map], but strips the URL from the error
    pub fn map_sensitive<E: StdError + From<Self>>(value: reqwest::Error) -> UError<E> {
        Self::map(value.without_url())
    }
}

impl<E: StdError + From<RequestError>> From<reqwest::Error> for UError<E> {
    fn from(value: reqwest::Error) -> Self {
        RequestError::map(value)
    }
}
