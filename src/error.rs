use std::{fmt, net::AddrParseError};

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
struct OtherError {
    reason: &'static str,
}

impl std::error::Error for OtherError {}

impl fmt::Display for OtherError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.reason)
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ErrorKind {
    Io,
    Serde,
    AddrParseError,
    Other
}

impl std::string::ToString for ErrorKind {
    fn to_string(&self) -> String {
        match self {
            ErrorKind::Io => String::from("io"),
            ErrorKind::Serde => String::from("serde"),
            ErrorKind::AddrParseError => String::from("AddrParseError"),
            ErrorKind::Other => String::from("other"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Error {
    kind: ErrorKind,
    inner: Box<dyn std::error::Error>
}

impl Error {
    pub fn new(reason: &'static str) -> Self {
        Self {
            kind: ErrorKind::Other,
            inner: Box::new(OtherError{reason})
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ephc error, kind: {}, err: {}", self.kind.to_string(), self.inner.to_string())
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(e: serde_yaml::Error) -> Self {
        Self {
            kind: ErrorKind::Serde,
            inner: Box::new(e)
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self {
            kind: ErrorKind::Io,
            inner: Box::new(e)
        }
    }
}

impl From<std::net::AddrParseError> for Error {
    fn from(e: AddrParseError) -> Self {
        Self {
            kind: ErrorKind::AddrParseError,
            inner: Box::new(e)
        }
    }
}