use thiserror::Error;

/// Ergonomic and performant error types for TDA parsing, with
/// allocation-free variants for common validation paths
#[derive(Error, Debug)]
pub enum TDAError {
    #[error("Invalid 2DA header: expected '2DA V2.0', found '{0}'")]
    InvalidHeader(String),

    #[error("File size {size} exceeds maximum allowed size {max_size}")]
    FileSizeExceeded { size: usize, max_size: usize },

    #[error("Invalid UTF-8 encoding at byte position {position}")]
    InvalidUtf8 { position: usize },

    #[error("Malformed line {line_number}: {details}")]
    MalformedLine { line_number: usize, details: String },

    #[error("Column '{column}' not found in table")]
    ColumnNotFound { column: String },

    #[error("Row index {index} is out of bounds (max: {max})")]
    RowIndexOutOfBounds { index: usize, max: usize },

    #[error("Column index {index} is out of bounds (max: {max})")]
    ColumnIndexOutOfBounds { index: usize, max: usize },

    #[error("Memory mapping failed: {details}")]
    MemoryMapError { details: String },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] rmp_serde::encode::Error),

    #[error("Deserialization error: {0}")]
    DeserializationError(#[from] rmp_serde::decode::Error),

    #[error("Compression error: {details}")]
    CompressionError { details: String },

    #[error("Column count {count} exceeds the configured limit of {limit}")]
    ColumnCountExceeded { count: usize, limit: usize },

    #[error("Row count {count} exceeds the configured limit of {limit}")]
    RowCountExceeded { count: usize, limit: usize },

    #[error("Line length {length} exceeds the configured limit of {limit}")]
    LineLengthExceeded { length: usize, limit: usize },

    #[error("Parse error: {details}")]
    ParseError { details: String },

    #[error("Invalid token at position {position}: '{token}'")]
    InvalidToken { position: usize, token: String },

    #[error("Security violation: {details}")]
    SecurityViolation { details: String },
}

/// Result type alias for TDA operations
pub type TDAResult<T> = Result<T, TDAError>;

/// Security limits for safe parsing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityLimits {
    /// Maximum file size in bytes (default: 256MB)
    pub max_file_size: usize,
    /// Maximum number of columns (default: 1024)
    pub max_columns: usize,
    /// Maximum number of rows (default: 1M)
    pub max_rows: usize,
    /// Maximum line length in characters (default: 64KB)
    pub max_line_length: usize,
}

impl SecurityLimits {
    const DEFAULT_MAX_FILE_SIZE: usize = 256 * 1024 * 1024; // 256MB
    const DEFAULT_MAX_COLUMNS: usize = 1024;
    const DEFAULT_MAX_ROWS: usize = 1_000_000;
    const DEFAULT_MAX_LINE_LENGTH: usize = 65536; // 64KB
}

impl Default for SecurityLimits {
    fn default() -> Self {
        Self {
            max_file_size: Self::DEFAULT_MAX_FILE_SIZE,
            max_columns: Self::DEFAULT_MAX_COLUMNS,
            max_rows: Self::DEFAULT_MAX_ROWS,
            max_line_length: Self::DEFAULT_MAX_LINE_LENGTH,
        }
    }
}

impl SecurityLimits {
    /// Create security limits for testing with smaller bounds
    pub fn testing() -> Self {
        Self {
            max_file_size: 16 * 1024 * 1024, // 16MB
            max_columns: 256,
            max_rows: 100_000,
            max_line_length: 8192, // 8KB
        }
    }

    /// Validate file size against limits
    pub fn validate_file_size(&self, size: usize) -> TDAResult<()> {
        if size > self.max_file_size {
            Err(TDAError::FileSizeExceeded {
                size,
                max_size: self.max_file_size,
            })
        } else {
            Ok(())
        }
    }

    /// Validate column count against limits
    pub fn validate_column_count(&self, count: usize) -> TDAResult<()> {
        if count > self.max_columns {
            Err(TDAError::ColumnCountExceeded {
                count,
                limit: self.max_columns,
            })
        } else {
            Ok(())
        }
    }

    /// Validate row count against limits
    pub fn validate_row_count(&self, count: usize) -> TDAResult<()> {
        if count > self.max_rows {
            Err(TDAError::RowCountExceeded {
                count,
                limit: self.max_rows,
            })
        } else {
            Ok(())
        }
    }

    /// Validate line length against limits
    pub fn validate_line_length(&self, length: usize) -> TDAResult<()> {
        if length > self.max_line_length {
            Err(TDAError::LineLengthExceeded {
                length,
                limit: self.max_line_length,
            })
        } else {
            Ok(())
        }
    }
}