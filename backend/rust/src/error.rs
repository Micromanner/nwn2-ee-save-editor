//! Shared error types and utilities for the NWN2 Rust extensions.
//!
//! This module provides common error handling patterns used across all parsers
//! and services. Each parser/service defines its own specific error types,
//! but they all implement the traits defined here for consistent error handling.

use pyo3::exceptions::{PyIOError, PyRuntimeError, PyValueError};
use pyo3::PyErr;
use std::fmt;
use thiserror::Error;

/// Common error categories shared across all parsers.
///
/// Individual parsers wrap these in their specific error types.
#[derive(Debug, Error)]
pub enum CommonError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Security violation: {0}")]
    SecurityViolation(String),

    #[error("File size exceeded: {size} bytes (max: {max} bytes)")]
    FileSizeExceeded { size: usize, max: usize },

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Encoding error: {0}")]
    Encoding(String),
}

impl From<CommonError> for PyErr {
    fn from(err: CommonError) -> PyErr {
        match err {
            CommonError::Io(_) => PyIOError::new_err(err.to_string()),
            CommonError::SecurityViolation(_) | CommonError::FileSizeExceeded { .. } => {
                PyValueError::new_err(err.to_string())
            }
            CommonError::NotFound(_) => PyValueError::new_err(err.to_string()),
            _ => PyRuntimeError::new_err(err.to_string()),
        }
    }
}

/// Trait for converting parser-specific errors to Python exceptions.
///
/// All parser error types should implement this trait.
pub trait IntoPyErr {
    fn into_py_err(self) -> PyErr;
}

/// Blanket implementation for any error type that can be displayed.
impl<E: fmt::Display> IntoPyErr for E {
    fn into_py_err(self) -> PyErr {
        PyRuntimeError::new_err(self.to_string())
    }
}

/// Extension trait for Result types to convert to PyResult.
pub trait ResultExt<T, E> {
    fn into_py_result(self) -> Result<T, PyErr>;
}

impl<T, E: fmt::Display> ResultExt<T, E> for Result<T, E> {
    fn into_py_result(self) -> Result<T, PyErr> {
        self.map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }
}

/// Macro for defining parser-specific error types with common variants.
///
/// Usage:
/// ```rust,ignore
/// define_parser_error! {
///     TDAError {
///         InvalidHeader(String) => "Invalid header: {}",
///         MalformedLine { line: usize, content: String } => "Malformed line {}: {}",
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_parser_error {
    (
        $name:ident {
            $(
                $variant:ident $( { $($field:ident : $field_ty:ty),* } )? $( ( $($tuple_ty:ty),* ) )? => $msg:literal
            ),* $(,)?
        }
    ) => {
        #[derive(Debug, thiserror::Error)]
        pub enum $name {
            $(
                #[error($msg)]
                $variant $( { $($field : $field_ty),* } )? $( ( $($tuple_ty),* ) )?,
            )*

            #[error("I/O error: {0}")]
            Io(#[from] std::io::Error),

            #[error("Serialization error: {0}")]
            Serialization(String),

            #[error("Deserialization error: {0}")]
            Deserialization(String),

            #[error("Security violation: {0}")]
            SecurityViolation(String),
        }

        impl From<$name> for pyo3::PyErr {
            fn from(err: $name) -> pyo3::PyErr {
                pyo3::exceptions::PyRuntimeError::new_err(err.to_string())
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_error_display() {
        let err = CommonError::FileSizeExceeded {
            size: 1000,
            max: 500,
        };
        assert!(err.to_string().contains("1000"));
        assert!(err.to_string().contains("500"));
    }
}
