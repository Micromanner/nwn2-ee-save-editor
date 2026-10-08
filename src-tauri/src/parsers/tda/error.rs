use thiserror::Error;

#[derive(Error, Debug)]
pub enum TDAError {
    #[error("Invalid 2DA header: expected '2DA V2.0', found '{0}'")]
    InvalidHeader(String),

    #[error("File size {size} exceeds maximum allowed size {max_size}")]
    FileSizeExceeded { size: usize, max_size: usize },

    #[error("Column '{column}' not found in table")]
    ColumnNotFound { column: String },

    #[error("Row index {index} is out of bounds (max: {max})")]
    RowIndexOutOfBounds { index: usize, max: usize },

    #[error("Column index {index} is out of bounds (max: {max})")]
    ColumnIndexOutOfBounds { index: usize, max: usize },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Column count {count} exceeds the configured limit of {limit}")]
    ColumnCountExceeded { count: usize, limit: usize },

    #[error("Row count {count} exceeds the configured limit of {limit}")]
    RowCountExceeded { count: usize, limit: usize },

    #[error("Line length {length} exceeds the configured limit of {limit}")]
    LineLengthExceeded { length: usize, limit: usize },
}

pub type TDAResult<T> = Result<T, TDAError>;

#[derive(Debug, Clone)]
pub struct SecurityLimits {
    pub max_file_size: usize,
    pub max_columns: usize,
    pub max_rows: usize,
    pub max_line_length: usize,
}

impl Default for SecurityLimits {
    fn default() -> Self {
        Self {
            max_file_size: 256 * 1024 * 1024,
            max_columns: 1024,
            max_rows: 1_000_000,
            max_line_length: 65536,
        }
    }
}

impl SecurityLimits {
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
