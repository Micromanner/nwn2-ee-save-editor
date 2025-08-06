//! High-performance icon cache for NWN2 Enhanced Edition
//! 
//! This crate provides a fast, concurrent icon cache with persistent storage
//! to dramatically improve startup times and icon loading performance.

pub mod types;
pub mod cache;
pub mod discovery;
pub mod processing;
pub mod persistence;
pub mod index;
pub mod error;
pub mod config;

#[cfg(feature = "python-bindings")]
pub mod python;

pub use cache::RustIconCache;
pub use config::IconCacheConfig;
pub use error::{IconCacheError, Result};
pub use types::{CachedIcon, SourceType, ImageFormat};

// Re-export for convenience
pub use dashmap::DashMap;
pub use lasso::Spur as IconKey;

// Type alias for string interner
#[cfg(feature = "multi-threaded")]
pub type StringInterner = lasso::ThreadedRodeo;

#[cfg(not(feature = "multi-threaded"))]
pub type StringInterner = lasso::Rodeo;