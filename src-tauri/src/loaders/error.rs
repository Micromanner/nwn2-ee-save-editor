use thiserror::Error;

use crate::error::AppError;
use crate::services::resource_manager::ResourceManagerError;

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("Table not found: {name}")]
    TableNotFound { name: String },

    #[error("Row not found in {table}: ID {id}")]
    RowNotFound { table: String, id: i32 },

    #[error("Column not found in {table}: {column}")]
    ColumnNotFound { table: String, column: String },

    #[error("Invalid table filter configuration: {0}")]
    FilterConfigError(String),

    #[error("Loading interrupted: {0}")]
    LoadingInterrupted(String),

    #[error("Resource manager error: {0}")]
    ResourceManager(#[from] ResourceManagerError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),
}

impl serde::Serialize for LoaderError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<LoaderError> for AppError {
    fn from(err: LoaderError) -> AppError {
        match err {
            LoaderError::TableNotFound { name } => AppError::NotFound(format!("Table: {name}")),
            LoaderError::RowNotFound { table, id } => {
                AppError::NotFound(format!("Row {id} in table {table}"))
            }
            LoaderError::ColumnNotFound { table, column } => {
                AppError::NotFound(format!("Column {column} in table {table}"))
            }
            LoaderError::ResourceManager(e) => e.into(),
            LoaderError::Io(e) => AppError::Io(e),
            _ => AppError::Parse(err.to_string()),
        }
    }
}

pub type LoaderResult<T> = Result<T, LoaderError>;
