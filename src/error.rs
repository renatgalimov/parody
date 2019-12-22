use std::fmt;

#[derive(Debug)]
pub enum Error {
    CacheMiss,
    Common(CommonError),
}

#[derive(Debug)]
pub enum CommonError {
    IoError(std::io::Error),
    YamlError(serde_yaml::Error),
    Utf8Error(std::str::Utf8Error),
    ParseIntError(std::num::ParseIntError),
}

impl From<std::num::ParseIntError> for CommonError {
    fn from(source: std::num::ParseIntError) -> CommonError {
        CommonError::ParseIntError(source)
    }
}

impl From<std::io::Error> for CommonError {
    fn from(source: std::io::Error) -> CommonError {
        CommonError::IoError(source)
    }
}

impl From<serde_yaml::Error> for CommonError {
    fn from(source: serde_yaml::Error) -> CommonError {
        CommonError::YamlError(source)
    }
}

impl From<std::str::Utf8Error> for CommonError {
    fn from(source: std::str::Utf8Error) -> CommonError {
        CommonError::Utf8Error(source)
    }
}

impl<T: Into<CommonError>> From<T> for Error {
    fn from(source: T) -> Error {
        Error::Common(source.into())
    }
}

impl<'a> std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::CacheMiss => write!(f, "Response not found in cache"),
            error => std::fmt::Display::fmt(error, f),
        }
    }
}


impl fmt::Display for CommonError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommonError::IoError(error) => error.fmt(f),
            CommonError::ParseIntError(error) => error.fmt(f),
            CommonError::Utf8Error(error) => error.fmt(f),
            CommonError::YamlError(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for CommonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CommonError::IoError(error) => Some(error),
            CommonError::ParseIntError(error) => Some(error),
            CommonError::Utf8Error(error) => Some(error),
            CommonError::YamlError(error) => Some(error),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Common(error) => error.source(),
            Error::CacheMiss => None
        }
    }
}
