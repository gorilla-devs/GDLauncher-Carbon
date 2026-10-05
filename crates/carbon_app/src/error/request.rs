use std::fmt::{self, Display, Formatter};

use super::json::get_json_context;
use anyhow::anyhow;
use reqwest::{Response, StatusCode, Url};
use serde::Serialize;
use specta::Type;
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

    #[error("malformed response: {details}")]
    MalformedResponse { details: MalformedResponseDetails }, // TODO: get body

    /// The request could not be made, or led nowhere, whatever the network
    /// does: a malformed address, an unsupported scheme, a redirect loop.
    #[error("unusable request: {0}")]
    UnusableRequest(String),

    #[error("unknown reqwest error: {0}")]
    Unknown(String),
}

#[derive(Debug, Clone)]
pub enum MalformedResponseDetails {
    JsonDecodeError {
        ctx: String,
        line: usize,
        column: usize,
        source: String,
    },
    UnknownDecodeError,
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

impl Display for MalformedResponseDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::JsonDecodeError {
                ctx,
                line,
                column,
                source,
            } => {
                write!(
                    f,
                    "Unable to deserialise json object at {}:{}. Context (failure point marked by `<~~`) `{}` Caused by: {}",
                    line, column, ctx, source
                )
            }
            Self::UnknownDecodeError => {
                write!(f, "Unknown decode error")
            }
        }
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

    pub fn from_url(url: &Url) -> Self {
        Self {
            url: Some(url.clone()),
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
            Self::MalformedResponse {
                details: MalformedResponseDetails::UnknownDecodeError,
            }
        } else if error.is_builder() || error.is_redirect() {
            Self::UnusableRequest(format!("{error:#?}"))
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

    pub fn from_json_decode_error(error: serde_json::Error, body: &str) -> Self {
        Self::MalformedResponse {
            details: MalformedResponseDetails::JsonDecodeError {
                ctx: get_json_context(&error, body, 400),
                line: error.line(),
                column: error.column(),
                source: error.to_string(),
            },
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

    pub fn from_error(value: reqwest::Error) -> Self {
        Self {
            context: RequestContext::from_error(&value),
            error: RequestErrorDetails::from_error(value),
        }
    }

    pub fn from_json_decode_error(error: serde_json::Error, body: &str, url: &Url) -> Self {
        Self {
            context: RequestContext::from_url(url),
            error: RequestErrorDetails::from_json_decode_error(error, body),
        }
    }

    pub fn from_error_censored(value: reqwest::Error) -> Self {
        Self {
            context: RequestContext::from_error(&value),
            error: RequestErrorDetails::from_error_censored(value),
        }
    }

    pub fn report_volatile_malformed(self, ty: &'static str) -> Self {
        if matches!(self.error, RequestErrorDetails::MalformedResponse { .. }) {
            super::sentry::report_volatile_error(ty, anyhow!(self.clone()));
        }

        self
    }
}

#[async_trait::async_trait]
pub trait GoodJsonRequestError {
    async fn json_with_context<T: serde::de::DeserializeOwned>(self) -> Result<T, RequestError>;
    async fn json_with_context_reporting<T: serde::de::DeserializeOwned>(
        self,
        ty: &'static str,
    ) -> Result<T, RequestError>;
}

#[async_trait::async_trait]
impl GoodJsonRequestError for reqwest::Response {
    async fn json_with_context<T: serde::de::DeserializeOwned>(self) -> Result<T, RequestError> {
        let url = self.url().clone();
        let body = self
            .error_for_status()
            .map_err(RequestError::from_error)?
            .text()
            .await
            .map_err(RequestError::from_error)?;
        Ok(serde_json::from_str::<T>(&body)
            .map_err(|err| RequestError::from_json_decode_error(err, &body, &url))?)
    }

    async fn json_with_context_reporting<T: serde::de::DeserializeOwned>(
        self,
        ty: &'static str,
    ) -> Result<T, RequestError> {
        self.json_with_context()
            .await
            .map_err(|e| e.report_volatile_malformed(ty))
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
    MalformedResponse {
        details: FEMalformedResponseDetails,
    },
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
                RequestErrorDetails::MalformedResponse { details } => {
                    FERequestErrorType::MalformedResponse {
                        details: details.into(),
                    }
                }
                RequestErrorDetails::Unknown(x) | RequestErrorDetails::UnusableRequest(x) => {
                    FERequestErrorType::Unknown(x)
                }
            },
        }
    }
}

#[derive(Type, Serialize)]
pub enum FEMalformedResponseDetails {
    JsonDecodeError {
        ctx: String,
        line: usize,
        column: usize,
        source: String,
    },
    UnknownDecodeError,
}

impl From<MalformedResponseDetails> for FEMalformedResponseDetails {
    fn from(value: MalformedResponseDetails) -> Self {
        match value {
            MalformedResponseDetails::JsonDecodeError {
                ctx,
                line,
                column,
                source,
            } => Self::JsonDecodeError {
                ctx,
                line,
                column,
                source,
            },
            MalformedResponseDetails::UnknownDecodeError => Self::UnknownDecodeError,
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

#[cfg(test)]
mod task22_tests {
    use super::*;
    #[derive(Debug, serde::Deserialize)]
    struct Payload {
        #[serde(rename = "bad")]
        _bad: bool,
    }
    #[test]
    fn task22_direct_request_error_preserves_actual_serde_coordinates_and_context() {
        let body = "{\n  \"bad\": \"é\",\n  \"next\": \"innocent\"\n}";
        let error = serde_json::from_str::<Payload>(body).unwrap_err();
        let source = error.to_string();
        let coordinates = (error.line(), error.column());
        let details = RequestErrorDetails::from_json_decode_error(error, body);
        let RequestErrorDetails::MalformedResponse {
            details:
                MalformedResponseDetails::JsonDecodeError {
                    ctx,
                    line,
                    column,
                    source: actual,
                },
        } = details
        else {
            panic!("Wrong diagnostic kind");
        };
        assert_eq!((line, column), coordinates);
        assert_eq!(actual, source);
        assert!(ctx.contains("\"é\"<~~ "), "{ctx}");
    }
}
