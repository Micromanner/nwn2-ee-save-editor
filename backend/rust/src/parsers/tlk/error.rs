use thiserror::Error;
use std::io;

/// Result type for TLK operations
pub type TLKResult<T> = Result<T, TLKError>;

/// Comprehensive error types for TLK parsing operations
#[derive(Error, Debug)]
pub enum TLKError {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Invalid TLK header: expected 'TLK V3.0', found '{found}'")]
    InvalidHeader { found: String },

    #[error("File too short: expected at least {expected} bytes, found {actual}")]
    FileTooShort { expected: usize, actual: usize },

    #[error("Corrupted string entry at index {index}: offset {offset} + size {size} exceeds file bounds")]
    CorruptedStringEntry { index: usize, offset: u32, size: u32 },

    #[error("String reference {str_ref} out of bounds (max: {max_strings})")]
    StringRefOutOfBounds { str_ref: usize, max_strings: usize },

    #[error("Invalid UTF-8 sequence in string {str_ref}: {source}")]
    InvalidUtf8 { str_ref: usize, source: std::string::FromUtf8Error },

    #[error("Cache serialization error: {0}")]
    SerializationError(#[from] rmp_serde::encode::Error),

    #[error("Cache deserialization error: {0}")]
    DeserializationError(#[from] rmp_serde::decode::Error),

    #[error("Compression error: {0}")]
    CompressionError(#[from] flate2::CompressError),

    #[error("Decompression error: {0}")]
    DecompressionError(#[from] flate2::DecompressError),

    #[error("Memory mapping error: {0}")]
    MemoryMapError(String),

    #[error("Security violation: {message}")]
    SecurityViolation { message: String },

    #[error("File size exceeded: {actual} bytes > {limit} bytes")]
    FileSizeExceeded { actual: usize, limit: usize },
}

/// Security limits for TLK parsing
#[derive(Debug, Clone)]
pub struct SecurityLimits {
    /// Maximum file size to process (default: 100MB)
    pub max_file_size: usize,
    /// Maximum number of strings (default: 1,000,000)
    pub max_strings: usize,
    /// Maximum string size (default: 64KB)
    pub max_string_size: usize,
}

impl Default for SecurityLimits {
    fn default() -> Self {
        Self {
            max_file_size: 100 * 1024 * 1024, // 100MB
            max_strings: 1_000_000,
            max_string_size: 64 * 1024, // 64KB
        }
    }
}

impl SecurityLimits {
    /// Create new security limits with custom values
    pub fn new(max_file_size: usize, max_strings: usize, max_string_size: usize) -> Self {
        Self {
            max_file_size,
            max_strings,
            max_string_size,
        }
    }

    /// Validate file size against limits
    pub fn validate_file_size(&self, size: usize) -> TLKResult<()> {
        if size > self.max_file_size {
            return Err(TLKError::FileSizeExceeded {
                actual: size,
                limit: self.max_file_size,
            });
        }
        Ok(())
    }

    /// Validate string count against limits
    pub fn validate_string_count(&self, count: usize) -> TLKResult<()> {
        if count > self.max_strings {
            return Err(TLKError::SecurityViolation {
                message: format!("String count {} exceeds limit {}", count, self.max_strings),
            });
        }
        Ok(())
    }

    /// Validate string size against limits
    pub fn validate_string_size(&self, size: usize) -> TLKResult<()> {
        if size > self.max_string_size {
            return Err(TLKError::SecurityViolation {
                message: format!("String size {} exceeds limit {}", size, self.max_string_size),
            });
        }
        Ok(())
    }
}