use std::{
    error::Error as StdError,
    fmt::{Debug, Display},
    result::Result as StdResult,
};

use backtrace::Backtrace;

pub type Result<T, E> = StdResult<T, Error<E>>;

#[derive(Debug, Clone)]
pub enum Error<E: Debug> {
    Known(E),
    Unhandled(UnhandledError),
}

impl<E: Debug> From<E> for Error<E> {
    fn from(value: E) -> Self {
        Self::Known(value)
    }
}

pub trait Unhandle<T, E: Debug> {
    fn unhandle(self) -> Result<T, E>;
}

impl<T, E: Debug, U: StdError + 'static> Unhandle<T, E> for StdResult<T, U> {
    fn unhandle(self) -> Result<T, E> {
        match self {
            Ok(x) => Ok(x),
            Err(e) => Err(Error::Unhandled(UnhandledError::new(Box::new(e)))),
        }
    }
}

#[derive(Clone)]
pub struct UnhandledError {
    display: String,
    debug: String,
}

impl UnhandledError {
    pub fn new<E: StdError>(error: E) -> Self {
        let error = TracedError::new(error);
        Self {
            display: format!("{error}"),
            debug: format!("{error:#?}"),
        }
    }
}

impl Debug for UnhandledError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.debug)
    }
}

impl Display for UnhandledError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display)
    }
}

pub struct TracedError<E: Debug> {
    pub error: E,
    pub trace: Backtrace,
}

impl<E: Debug> TracedError<E> {
    pub fn new(error: E) -> Self {
        Self {
            error,
            trace: Backtrace::new(),
        }
    }
}

impl<E: Debug> Debug for TracedError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TracedError").field(&self.error).finish()
    }
}

impl<E: Debug + Display> Display for TracedError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Error: {error}\nError Debug: {error:#?}\nBacktrace:\n{trace:?}\n",
            error = &self.error,
            trace = &self.trace,
        )
    }
}

#[cfg(test)]
mod test {
    use std::{error::Error, fmt::Display};

    use crate::error::UnhandledError;

    use super::{Result, Unhandle};

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

        fn throw_known() -> Result<(), KnownErr> {
            Err(KnownErr)?;
            Ok(())
        }

        fn throw_unknown() -> Result<(), KnownErr> {
            Err(UnknownErr).unhandle()?;
            Ok(())
        }

        println!(
            "Debug: {0:?}\nDebug Expanded: {0:#?}\nDisplay:\n{0}",
            UnhandledError::new(Box::new(UnknownErr))
        );
    }
}
