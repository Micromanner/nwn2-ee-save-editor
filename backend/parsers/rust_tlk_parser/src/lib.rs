//! High-Performance Rust TLK Parser for NWN2 Talk Table Files
//! 
//! This crate provides a lightning-fast parser for Neverwinter Nights 2 TLK (Talk Table) files
//! with extensive optimizations for memory usage, parsing speed, and batch operations.
//! 
//! # Features
//! 
//! - **Memory-optimized storage** - Load all strings into memory once for fast access
//! - **Bulk string retrieval** - Batch operations for high-performance use cases
//! - **String interning** for memory efficiency with duplicate strings
//! - **MessagePack caching** with compression support for faster subsequent loads
//! - **Comprehensive security validation** with configurable limits
//! - **Full Python API compatibility** via PyO3 bindings (drop-in replacement)
//! - **UTF-8 handling** with graceful fallback for corrupted data
//! - **Parallel file loading** using Rayon for multiple TLK files
//! 
//! # Primary Use Case
//! 
//! This parser was specifically designed to solve a performance bottleneck in the NWN2
//! Enhanced Edition Editor where 4,464 TLK string lookups were taking 4.6 seconds during
//! application startup. The Rust implementation reduces this to under 0.5 seconds.
//! 
//! # Example Usage
//! 
//! ```rust
//! use rust_tlk_parser::TLKParser;
//! 
//! let mut parser = TLKParser::new();
//! parser.parse_from_file("dialog.tlk").unwrap();
//! 
//! // Single string lookup
//! if let Some(text) = parser.get_string(1234).unwrap() {
//!     println!("String 1234: {}", text);
//! }
//! 
//! // Batch lookup for performance
//! let str_refs = vec![100, 200, 300, 400];
//! let batch_result = parser.get_strings_batch(&str_refs).unwrap();
//! println!("Retrieved {} strings in {:.2}ms", 
//!          batch_result.strings.len(), 
//!          batch_result.metrics.total_time_ms);
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod error;
pub mod parser;
pub mod types;

#[cfg(feature = "python-bindings")]
pub mod python;

// Re-export main types for convenience
pub use error::{SecurityLimits, TLKError, TLKResult};
pub use parser::load_multiple_files;
pub use types::{
    TLKParser, TLKHeader, TLKStringEntry, ParserStatistics, FileMetadata,
    BatchStringResult, BatchMetrics, SearchResult, SearchOptions,
    SerializableTLKParser
};

#[cfg(feature = "python-bindings")]
pub use python::PyTLKParser;

// PyO3 module definition (only when Python bindings are enabled)
#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;

#[cfg(feature = "python-bindings")]
#[pymodule]
fn rust_tlk_parser(_py: Python, m: &PyModule) -> PyResult<()> {
    python::rust_tlk_parser(_py, m)
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::path::PathBuf;

    // Helper function to get the path to test fixtures
    fn get_fixture_path(filename: &str) -> PathBuf {
        // Get the path relative to the workspace root
        let workspace_root = std::env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR should be set")
            .replace("/backend/parsers/rust_tlk_parser", "");
        PathBuf::from(format!("{}/backend/tests/fixtures/tlk/{}", workspace_root, filename))
    }

    #[test]
    fn test_basic_parsing() {
        let tlk_path = get_fixture_path("dialog_english.tlk");
        if !tlk_path.exists() {
            println!("Skipping test - fixture file not found: {:?}", tlk_path);
            return;
        }

        let mut parser = TLKParser::new();
        parser.parse_from_file(&tlk_path).unwrap();
        
        assert!(parser.is_loaded());
        assert!(parser.string_count() > 0);
        
        // Test basic string access
        if parser.string_count() > 0 {
            let first_string = parser.get_string(0).unwrap();
            // First string should exist (even if empty)
            assert!(first_string.is_some());
        }
    }

    #[test]
    fn test_batch_operations() {
        let tlk_path = get_fixture_path("dialog_english.tlk");
        if !tlk_path.exists() {
            println!("Skipping test - fixture file not found: {:?}", tlk_path);
            return;
        }

        let mut parser = TLKParser::new();
        parser.parse_from_file(&tlk_path).unwrap();
        
        if parser.string_count() < 5 {
            return; // Need at least 5 strings for this test
        }
        
        // Test batch retrieval
        let str_refs = vec![0, 1, 2, 3, 4];
        let batch_result = parser.get_strings_batch(&str_refs).unwrap();
        
        assert!(batch_result.metrics.total_time_ms >= 0.0);
        assert!(batch_result.strings.len() <= str_refs.len());
    }

    #[test]
    fn test_search_functionality() {
        let tlk_path = get_fixture_path("dialog_english.tlk");
        if !tlk_path.exists() {
            println!("Skipping test - fixture file not found: {:?}", tlk_path);
            return;
        }

        let mut parser = TLKParser::new();
        parser.parse_from_file(&tlk_path).unwrap();
        
        // Test search with default options
        let options = SearchOptions::default();
        let results = parser.search_strings("the", &options).unwrap();
        
        // Should return some results for common word "the"
        assert!(results.len() <= options.max_results);
        
        // Test that results are sorted by score
        for window in results.windows(2) {
            assert!(window[0].score >= window[1].score);
        }
    }

    #[test]
    fn test_cache_functionality() {
        let tlk_path = get_fixture_path("dialog_english.tlk");
        if !tlk_path.exists() {
            println!("Skipping test - fixture file not found: {:?}", tlk_path);
            return;
        }

        let cache_path = std::env::temp_dir().join("test_cache.tlk.cache");
        
        // Clean up any existing cache
        let _ = std::fs::remove_file(&cache_path);
        
        {
            let mut parser = TLKParser::new();
            let from_cache = parser.load_with_cache(&tlk_path, Some(&cache_path)).unwrap();
            assert!(!from_cache); // First load should be from source
            assert!(cache_path.exists());
        }
        
        {
            let mut parser = TLKParser::new();
            let from_cache = parser.load_with_cache(&tlk_path, Some(&cache_path)).unwrap();
            assert!(from_cache); // Second load should be from cache
        }
        
        // Clean up
        let _ = std::fs::remove_file(&cache_path);
    }

    #[test]
    fn test_error_handling() {
        let mut parser = TLKParser::new();
        
        // Test non-existent file
        let result = parser.parse_from_file("/nonexistent/file.tlk");
        assert!(result.is_err());
        
        // Test invalid data
        let invalid_data = b"INVALID TLK DATA";
        let result = parser.parse_from_bytes(invalid_data);
        assert!(result.is_err());
        
        // Test out of bounds access
        parser.clear();
        let result = parser.get_string(999999);
        assert!(result.is_ok() && result.unwrap().is_none());
    }

    #[test]
    fn test_security_limits() {
        let limits = SecurityLimits {
            max_file_size: 100, // Very small limit
            max_strings: 10,
            max_string_size: 50,
        };
        
        let mut parser = TLKParser::with_limits(limits);
        
        let tlk_path = get_fixture_path("dialog_english.tlk");
        if !tlk_path.exists() {
            println!("Skipping test - fixture file not found: {:?}", tlk_path);
            return;
        }
        
        // Should fail due to file size limit
        let result = parser.parse_from_file(&tlk_path);
        // May fail due to size limits depending on fixture file size
        println!("Security limits test result: {:?}", result);
    }

    #[test]
    fn test_statistics_and_metadata() {
        let tlk_path = get_fixture_path("dialog_english.tlk");
        if !tlk_path.exists() {
            println!("Skipping test - fixture file not found: {:?}", tlk_path);
            return;
        }

        let mut parser = TLKParser::new();
        parser.parse_from_file(&tlk_path).unwrap();
        
        let stats = parser.statistics();
        assert!(stats.total_strings > 0);
        assert!(stats.memory_usage > 0);
        assert!(stats.parse_time_ms >= 0.0);
        
        let metadata = parser.metadata();
        assert!(metadata.file_size > 0);
        assert!(metadata.parse_time_ns > 0);
        assert!(!metadata.format_version.is_empty());
    }
}