use std::io;
use thiserror::Error;

pub type TLKResult<T> = Result<T, TLKError>;

#[derive(Error, Debug)]
pub enum TLKError {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Invalid TLK header: expected 'TLK V3.0', found '{found}'")]
    InvalidHeader { found: String },

    #[error("File too short: expected at least {expected} bytes, found {actual}")]
    FileTooShort { expected: usize, actual: usize },

    #[error("String count {count} exceeds the limit of {limit}")]
    TooManyStrings { count: usize, limit: usize },

    #[error("File size exceeded: {actual} bytes > {limit} bytes")]
    FileSizeExceeded { actual: usize, limit: usize },
}
