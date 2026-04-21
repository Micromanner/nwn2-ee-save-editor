use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SaveGameError {
    #[error("Save game not found: {path}")]
    NotFound { path: PathBuf },

    #[error("Corrupted save game ZIP: {0}")]
    CorruptedZip(String),

    #[error("File not found in save: {filename}")]
    FileNotInSave { filename: String },

    #[error("Invalid file header for {filename}: expected {expected}")]
    InvalidHeader { filename: String, expected: String },

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Backup creation failed: {0}")]
    BackupFailed(String),

    #[error("Backup restoration failed: {0}")]
    RestoreFailed(String),

    #[error("PlayerInfo sync error: {0}")]
    PlayerInfoSync(String),

    #[error("Invalid save directory structure: {0}")]
    InvalidStructure(String),

    #[error("File validation failed: {filename} - {reason}")]
    ValidationFailed { filename: String, reason: String },

    #[error("Temporary file error: {0}")]
    TempFileError(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("GFF parse error: {0}")]
    GffParse(String),

    #[error("{0}")]
    Transform(String),
}

impl serde::Serialize for SaveGameError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<SaveGameError> for crate::error::AppError {
    fn from(err: SaveGameError) -> crate::error::AppError {
        match err {
            SaveGameError::NotFound { path } => {
                crate::error::AppError::PathNotFound(path.display().to_string())
            }
            SaveGameError::Io(e) => crate::error::AppError::Io(e),
            SaveGameError::FileNotInSave { filename } => {
                crate::error::AppError::NotFound(format!("File in save: {filename}"))
            }
            _ => crate::error::AppError::Parse(err.to_string()),
        }
    }
}

pub type SaveGameResult<T> = Result<T, SaveGameError>;
