//! Shared types used across multiple parsers and services.
//!
//! This module defines common types that are reused throughout the codebase,
//! including security limits, metadata structures, and cache utilities.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

/// Security limits for parser operations.
///
/// These limits prevent denial-of-service attacks and resource exhaustion
/// when parsing untrusted files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityLimits {
    /// Maximum file size in bytes (default: 256 MB)
    pub max_file_size: usize,
    /// Maximum number of items/entries (default: 1,000,000)
    pub max_items: usize,
    /// Maximum size of individual items in bytes (default: 64 KB)
    pub max_item_size: usize,
}

impl Default for SecurityLimits {
    fn default() -> Self {
        Self {
            max_file_size: 256 * 1024 * 1024, // 256 MB
            max_items: 1_000_000,
            max_item_size: 64 * 1024, // 64 KB
        }
    }
}

impl SecurityLimits {
    /// Create security limits suitable for 2DA files.
    pub fn for_tda() -> Self {
        Self {
            max_file_size: 256 * 1024 * 1024,
            max_items: 1_000_000,    // max rows
            max_item_size: 64 * 1024, // max line length
        }
    }

    /// Create security limits suitable for TLK files.
    pub fn for_tlk() -> Self {
        Self {
            max_file_size: 100 * 1024 * 1024, // 100 MB
            max_items: 1_000_000,              // max strings
            max_item_size: 64 * 1024,          // max string size
        }
    }

    /// Create security limits suitable for ERF/HAK/MOD files.
    pub fn for_erf() -> Self {
        Self {
            max_file_size: 500 * 1024 * 1024, // 500 MB
            max_items: 100_000,                // max resources
            max_item_size: 100 * 1024 * 1024,  // max single resource
        }
    }

    /// Validate file size against limits.
    pub fn validate_file_size(&self, size: usize) -> Result<(), String> {
        if size > self.max_file_size {
            Err(format!(
                "File size {} exceeds limit of {} bytes",
                size, self.max_file_size
            ))
        } else {
            Ok(())
        }
    }

    /// Validate item count against limits.
    pub fn validate_item_count(&self, count: usize) -> Result<(), String> {
        if count > self.max_items {
            Err(format!(
                "Item count {} exceeds limit of {}",
                count, self.max_items
            ))
        } else {
            Ok(())
        }
    }
}

/// Common metadata for parsed files.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileMetadata {
    /// Path to the source file (if loaded from disk)
    pub file_path: Option<PathBuf>,
    /// Size of the source file in bytes
    pub file_size: usize,
    /// Time taken to parse in nanoseconds
    pub parse_time_ns: u64,
    /// Format version string
    pub format_version: String,
    /// Any warnings generated during parsing
    pub warnings: Vec<String>,
}

impl FileMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    pub fn with_size(mut self, size: usize) -> Self {
        self.file_size = size;
        self
    }

    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }
}

/// Common statistics for parser operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParserStats {
    /// Total items parsed
    pub total_items: usize,
    /// Memory usage estimate in bytes
    pub memory_usage: usize,
    /// Parse time in milliseconds
    pub parse_time_ms: f64,
    /// Number of interned strings (if applicable)
    pub interned_strings: usize,
    /// Cache hit ratio (0.0 - 1.0)
    pub cache_hit_ratio: f64,
}

impl ParserStats {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Timer utility for measuring operation duration.
#[derive(Debug)]
pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed_ns(&self) -> u64 {
        self.start.elapsed().as_nanos() as u64
    }

    pub fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::start()
    }
}

/// Cache version for detecting incompatible caches.
pub const CACHE_VERSION: u32 = 1;

/// Cache metadata for persisted data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    /// Cache format version
    pub version: u32,
    /// Timestamp when cache was created (Unix epoch)
    pub created_at: u64,
    /// Hash of source files used to generate cache
    pub source_hash: String,
    /// Number of items in cache
    pub item_count: usize,
}

impl CacheMetadata {
    pub fn new(source_hash: String, item_count: usize) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        Self {
            version: CACHE_VERSION,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            source_hash,
            item_count,
        }
    }

    /// Check if cache is valid for given source hash.
    pub fn is_valid(&self, source_hash: &str) -> bool {
        self.version == CACHE_VERSION && self.source_hash == source_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_limits_validation() {
        let limits = SecurityLimits::default();
        assert!(limits.validate_file_size(1000).is_ok());
        assert!(limits.validate_file_size(usize::MAX).is_err());
    }

    #[test]
    fn test_timer() {
        let timer = Timer::start();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(timer.elapsed_ms() >= 10.0);
    }
}
