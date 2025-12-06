//! Internal prelude for common imports.
//!
//! This module provides commonly used types and traits that are imported
//! throughout the crate. Use `use crate::prelude::*;` in internal modules.

// Re-export standard library types
pub use std::collections::HashMap;
pub use std::path::{Path, PathBuf};
pub use std::sync::Arc;

// Re-export common external crates
pub use ahash::AHashMap;
pub use serde::{Deserialize, Serialize};
pub use thiserror::Error;

// Re-export shared types
pub use crate::error::{CommonError, IntoPyErr, ResultExt};
pub use crate::types::{CacheMetadata, FileMetadata, ParserStats, SecurityLimits, Timer};

// Re-export PyO3 essentials
pub use pyo3::prelude::*;
pub use pyo3::types::{PyBytes, PyDict, PyList};
