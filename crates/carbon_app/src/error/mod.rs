//! Shared error types

pub mod request;

use std::{
    error::Error as StdError,
    fmt::{self, Debug, Display},
    result::Result as StdResult,
};

use backtrace::Backtrace;
use serde::Serialize;

pub type UResult<T, E> = StdResult<T, UError<E>>;

#[derive(Debug, Clone)]
pub enum UError<E: StdError> {
    Expected(E),
    Unexpected(UnexpectedError),
}

impl<E: StdError> Display for UError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expected(e) => Display::fmt(e, f),
            Self::Unexpected(e) => Display::fmt(e, f),
        }
    }
}

impl<E: StdError> From<E> for UError<E> {
    fn from(value: E) -> Self {
        Self::Expected(value.into())
    }
}

impl<E: StdError> UError<E> {
    pub fn map<F: StdError + Into<E>>(value: UError<F>) -> Self {
        match value {
            UError::Expected(x) => UError::Expected(x.into()),
            UError::Unexpected(x) => UError::Unexpected(x),
        }
    }
}

pub trait Unexpected<T, E: StdError> {
    fn unexpected(self, actions: HandlingActions) -> UResult<T, E>;
}

impl<T, E: StdError> Unexpected<T, E> for StdResult<T, E> {
    fn unexpected(self, actions: HandlingActions) -> UResult<T, E> {
        match self {
            Ok(x) => Ok(x),
            Err(e) => Err(UError::Unexpected(UnexpectedError::new(e, actions))),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct UnexpectedError {
    display: String,
    debug: String,
    trace: String,
    actions: HandlingActions,
}

/// Actions to suggest when receiving an unhandled error
#[derive(Debug, Clone, Copy, Serialize)]
pub enum HandlingActions {
    None,
}

impl UnexpectedError {
    pub fn new<E: StdError>(error: E, actions: HandlingActions) -> Self {
        Self {
            display: format!("{error}"),
            debug: format!("{error:#?}"),
            trace: format!("{:?}", Backtrace::new()),
            actions,
        }
    }
}

impl Debug for UnexpectedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TracedError")
            .field("display", &self.display)
            .field("debug", &self.debug)
            .field("actions", &self.actions)
            .field("trace", &format_args!("Backtrace {{\n{:?}}}", &self.trace))
            .finish()
    }
}

impl Display for UnexpectedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display)
    }
}

#[cfg(test)]
mod test {
    use std::{error::Error, fmt::Display};

    use crate::error::UnexpectedError;

    use super::{UResult, Unexpected};

    #[test]
    fn test() {
        #[derive(Debug)]
        struct KnownErr;

        #[derive(Debug)]
        struct UnknownErr;

        impl Display for UnknownErr {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{self:?} display")
            }
        }
        impl Error for UnknownErr {}

        fn throw_known() -> UResult<(), UnknownErr> {
            Err(UnknownErr)?;
            Ok(())
        }

        fn throw_unknown() -> UResult<(), UnknownErr> {
            Err(UnknownErr).unexpected(crate::error::HandlingActions::None)?;
            Ok(())
        }

        println!(
            "Debug: {0:?}\nDebug Expanded: {0:#?}\nDisplay:\n{0}",
            UnexpectedError::new(Box::new(UnknownErr), crate::error::HandlingActions::None)
        );
    }
}
