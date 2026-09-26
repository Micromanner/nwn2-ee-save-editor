use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResourceManagerError {
    #[error("2DA file not found: {name}")]
    TdaNotFound { name: String },

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Invalid 2DA format: {0}")]
    InvalidTdaFormat(String),

    #[error("Invalid ERF format: {0}")]
    InvalidErfFormat(String),

    #[error("Invalid TLK format: {0}")]
    InvalidTlkFormat(String),

    #[error("Invalid GFF format: {0}")]
    InvalidGffFormat(String),

    #[error("ZIP error: {0}")]
    ZipError(String),

    #[error("Resource extraction failed: {resource} from {container}")]
    ExtractionFailed { resource: String, container: String },

    #[error("Path configuration missing: {0}")]
    PathNotConfigured(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),
}

impl serde::Serialize for ResourceManagerError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<ResourceManagerError> for crate::error::AppError {
    fn from(err: ResourceManagerError) -> crate::error::AppError {
        match err {
            ResourceManagerError::TdaNotFound { name } => {
                crate::error::AppError::NotFound(format!("2DA: {name}"))
            }
            ResourceManagerError::FileNotFound(path) => {
                crate::error::AppError::PathNotFound(path.display().to_string())
            }
            ResourceManagerError::Io(e) => crate::error::AppError::Io(e),
            _ => crate::error::AppError::Parse(err.to_string()),
        }
    }
}

pub type ResourceManagerResult<T> = Result<T, ResourceManagerError>;
