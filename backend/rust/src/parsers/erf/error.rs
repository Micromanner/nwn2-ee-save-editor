use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ErfError {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    
    #[error("Invalid ERF header: expected signature 'MOD ', 'HAK ', or 'ERF ', found '{found}'")]
    InvalidSignature { found: String },
    
    #[error("Invalid ERF version: expected 'V1.0' or 'V1.1', found '{found}'")]
    InvalidVersion { found: String },
    
    #[error("Invalid resource count: {count} exceeds maximum {max}")]
    InvalidResourceCount { count: u32, max: u32 },
    
    #[error("Invalid file offset: offset {offset} exceeds file size {file_size}")]
    InvalidOffset { offset: usize, file_size: usize },
    
    #[error("Resource not found: '{name}'")]
    ResourceNotFound { name: String },
    
    #[error("Invalid resource name: contains non-ASCII characters")]
    InvalidResourceName,
    
    #[error("Security violation: {message}")]
    SecurityViolation { message: String },
    
    #[error("File too large: {size} bytes exceeds maximum {max} bytes")]
    FileTooLarge { size: usize, max: usize },
    
    #[error("Encoding error: {0}")]
    EncodingError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] rmp_serde::encode::Error),
    
    #[error("Deserialization error: {0}")]
    DeserializationError(#[from] rmp_serde::decode::Error),
    
    #[error("Invalid resource type: {0}")]
    InvalidResourceType(u16),
    
    #[error("Corrupted ERF data: {message}")]
    CorruptedData { message: String },
    
    #[error("Unsupported ERF format: {message}")]
    UnsupportedFormat { message: String },
    
    #[error("Compression error: {0}")]
    CompressionError(String),
}

pub type ErfResult<T> = std::result::Result<T, ErfError>;
pub type Result<T> = ErfResult<T>;

impl ErfError {
    pub fn security_violation(message: impl Into<String>) -> Self {
        ErfError::SecurityViolation {
            message: message.into(),
        }
    }
    
    pub fn corrupted_data(message: impl Into<String>) -> Self {
        ErfError::CorruptedData {
            message: message.into(),
        }
    }
    
    pub fn unsupported_format(message: impl Into<String>) -> Self {
        ErfError::UnsupportedFormat {
            message: message.into(),
        }
    }
}