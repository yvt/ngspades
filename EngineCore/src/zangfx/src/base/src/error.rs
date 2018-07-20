//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::error::Error as StdError;
use std::fmt;

/// Generic error types.
///
/// This enumerate type includes common error causes.
///
/// Some causes are intentionally excluded. They are mostly attributed to logic
/// errors and simply returning them would obfucate the exact location where
/// the error was detected, making debugging harder. The following list shows
/// the excluded causes:
///
///  - *Not supported*: The requested feature is not supported by, or exceeds
///    the limits of the hardware or the backend.
///
///  - *Invalid usage*: API contract violation was detected.
///
/// These errors are simply not detected, or in the cases they are detected,
/// they will be escalated to `panic!`.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ErrorKind {
    /// Ran out of device memory during an operation.
    OutOfDeviceMemory,

    /// The device became lost due to hardware/software errors, execution
    /// timeouts, or other reasons.
    ///
    /// Backend implementations may use this value to indicate that the integrity
    /// was compromised because of a software error and cannot proceed a proper
    /// operation.
    DeviceLost,

    /// Any error that is not part of this list.
    Other,
}

impl ErrorKind {
    fn as_str(&self) -> &'static str {
        match *self {
            ErrorKind::OutOfDeviceMemory => "out of device memory",
            ErrorKind::DeviceLost => "device lost",
            ErrorKind::Other => "uncategorized error",
        }
    }
}

/// The generic error type used by ZanGFX backends.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    error: Option<Box<StdError + Send + Sync>>,
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind, error: None }
    }
    pub fn with_detail<E>(kind: ErrorKind, error: E) -> Self
    where
        E: Into<Box<StdError + Send + Sync>>,
    {
        Self {
            kind,
            error: Some(error.into()),
        }
    }

    pub fn get_ref(&self) -> Option<&(StdError + Send + Sync + 'static)> {
        use std::ops::Deref;
        self.error.as_ref().map(Deref::deref)
    }

    pub fn get_mut(&mut self) -> Option<&mut (StdError + Send + Sync + 'static)> {
        use std::ops::DerefMut;
        self.error.as_mut().map(DerefMut::deref_mut)
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref error) = self.error {
            error.fmt(fmt)
        } else {
            write!(fmt, "{}", self.kind.as_str())
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        if let Some(ref error) = self.error {
            error.description()
        } else {
            self.kind.as_str()
        }
    }

    fn cause(&self) -> Option<&StdError> {
        self.error.as_ref().and_then(|x| x.cause())
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error::new(kind)
    }
}

pub type Result<T> = ::std::result::Result<T, Error>;
