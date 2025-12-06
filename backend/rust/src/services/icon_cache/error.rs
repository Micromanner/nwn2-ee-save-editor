//! Error types for the icon cache

use thiserror::Error;
use std::io;

/// Result type for icon cache operations
pub type Result<T> = std::result::Result<T, IconCacheError>;

/// Errors that can occur in the icon cache
#[derive(Error, Debug)]
pub enum IconCacheError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    
    /// Image processing error
    #[error("Image processing error: {0}")]
    ImageError(#[from] image::ImageError),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] bincode::error::EncodeError),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    DeserializationError(#[from] bincode::error::DecodeError),
    
    /// Cache version mismatch
    #[error("Cache version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: u32, found: u32 },
    
    /// Cache integrity check failed
    #[error("Cache integrity check failed")]
    IntegrityCheckFailed,
    
    /// Icon not found
    #[error("Icon not found: {0}")]
    IconNotFound(String),
    
    /// Invalid path
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    /// Async runtime error
    #[error("Async runtime error: {0}")]
    RuntimeError(String),
    
    /// Generic error
    #[error("Icon cache error: {0}")]
    Other(String),
}

impl From<walkdir::Error> for IconCacheError {
    fn from(err: walkdir::Error) -> Self {
        IconCacheError::Io(io::Error::new(io::ErrorKind::Other, err.to_string()))
    }
}

impl From<anyhow::Error> for IconCacheError {
    fn from(err: anyhow::Error) -> Self {
        IconCacheError::Other(err.to_string())
    }
}