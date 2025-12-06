pub mod cache;
pub mod config;
pub mod discovery;
pub mod error;
pub mod index;
pub mod persistence;
pub mod processing;
pub mod types;

#[cfg(feature = "python-bindings")]
pub mod python;

pub use cache::RustIconCache;
pub use config::IconCacheConfig;
pub use discovery::IconDiscovery;
pub use error::IconCacheError;
pub use index::{IconIndex, IndexBuilder, IndexEntry, LazyIconCache};
pub use persistence::DiskCache;
pub use processing::ImageProcessor;
pub use types::{CacheStatistics, CachedIcon, IconKey, IconSource, ImageFormat, InputFormat, SourceType, StringInterner};

#[cfg(feature = "python-bindings")]
pub use python::PyRustIconCache;
