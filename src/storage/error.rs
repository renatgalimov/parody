use crate::error::CommonError;
use std::fmt;

#[derive(Debug)]
pub(crate) enum StorageError {
    StatusFileNotFound,
    Common(CommonError),
}

impl<T: Into<CommonError>> From<T> for StorageError {
    fn from(source: T) -> StorageError {
        StorageError::Common(source.into())
    }
}
