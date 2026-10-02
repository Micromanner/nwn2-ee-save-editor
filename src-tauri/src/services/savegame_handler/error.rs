use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SaveGameError {
    #[error("Save game not found: {path}")]
    NotFound { path: PathBuf },

    #[error("File not found in save: {filename}")]
    FileNotInSave { filename: String },

    #[error("Backup creation failed: {0}")]
    BackupFailed(String),

    #[error("Backup restoration failed: {0}")]
    RestoreFailed(String),

    #[error("Invalid save directory structure: {0}")]
    InvalidStructure(String),

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

pub type SaveGameResult<T> = Result<T, SaveGameError>;
