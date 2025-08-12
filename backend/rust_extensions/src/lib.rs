use pyo3::prelude::*;

mod resource_scanner;
mod zip_indexer;
mod directory_walker;
mod path_discovery;
mod prerequisite_graph;
mod precompiled_cache;
mod zip_content_reader;

use resource_scanner::{ResourceScanner, ResourceLocation, ScanResults};
use path_discovery::{discover_nwn2_paths_rust, profile_path_discovery_rust, DiscoveryResult, PathTiming};
use prerequisite_graph::PrerequisiteGraph;
use precompiled_cache::{CacheBuilder, CacheManager};
use zip_content_reader::{ZipContentReader, ZipReadRequest, ZipReadResult};

/// NWN2 Rust Extensions
/// 
/// High-performance Rust implementations of bottleneck operations identified
/// in the Django backend performance profiling.
#[pymodule]
fn nwn2_rust_extensions(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<ResourceScanner>()?;
    m.add_class::<ResourceLocation>()?;
    m.add_class::<ScanResults>()?;
    
    // Path discovery functions
    m.add_function(wrap_pyfunction!(discover_nwn2_paths_rust, m)?)?;
    m.add_function(wrap_pyfunction!(profile_path_discovery_rust, m)?)?;
    m.add_class::<DiscoveryResult>()?;
    m.add_class::<PathTiming>()?;
    
    // Prerequisite graph for feat validation (10x speedup)
    m.add_class::<PrerequisiteGraph>()?;
    
    // Pre-compiled cache system for 60-70% startup speedup
    m.add_class::<CacheBuilder>()?;
    m.add_class::<CacheManager>()?;
    
    // Efficient ZIP content reader (eliminates open/close overhead)
    m.add_class::<ZipContentReader>()?;
    m.add_class::<ZipReadRequest>()?;
    m.add_class::<ZipReadResult>()?;
    
    // Add version info
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__description__", env!("CARGO_PKG_DESCRIPTION"))?;
    
    Ok(())
}