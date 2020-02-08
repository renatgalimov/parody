use std::fmt;

#[derive(Debug)]
pub enum UtilError {
    DomainMissing,
    InvalidCurrentFilePath,
}

#[derive(Debug)]
pub enum Error {
    AlreadyListening,
    CacheMiss,
    Common(CommonError),
    Util(UtilError),
}

#[derive(Debug)]
pub enum CommonError {
    IoError(std::io::Error),
    YamlError(serde_yaml::Error),
    Utf8Error(std::str::Utf8Error),
    ParseIntError(std::num::ParseIntError),
    HyperError(hyper::Error),
    UrlError(url::ParseError),
    ReqwestError(reqwest::Error),
}

impl From<UtilError> for Error {
    fn from(source: UtilError) -> Error {
        Error::Util(source)
    }
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

impl From<hyper::Error> for CommonError {
    fn from(source: hyper::Error) -> CommonError {
        CommonError::HyperError(source)
    }
}

impl From<url::ParseError> for CommonError {
    fn from(source: url::ParseError) -> CommonError {
        CommonError::UrlError(source)
    }
}

impl From<reqwest::Error> for CommonError {
    fn from(source: reqwest::Error) -> CommonError {
        CommonError::ReqwestError(source)
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
            Error::AlreadyListening => write!(f, "Server is already listening"),
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
            CommonError::HyperError(error) => error.fmt(f),
            CommonError::UrlError(error) => error.fmt(f),
            CommonError::ReqwestError(error) => error.fmt(f),
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
            CommonError::HyperError(error) => Some(error),
            CommonError::UrlError(error) => Some(error),
            CommonError::ReqwestError(error) => Some(error),
        }
    }
}

impl Into<iron::IronError> for CommonError {
    fn into(self) -> iron::IronError {
        iron::IronError::new(Box::new(self), iron::status::InternalServerError)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::AlreadyListening => None,
            Error::Common(error) => error.source(),
            Error::CacheMiss => None,
            Error::Util(error) => error.source(),
        }
    }
}

impl std::fmt::Display for UtilError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UtilError::DomainMissing => write!(f, "Domain is missing in the URL"),
            UtilError::InvalidCurrentFilePath => write!(f, "Current file path is invalid"),
        }
    }
}


impl std::error::Error for UtilError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
