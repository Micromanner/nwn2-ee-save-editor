use pyo3::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::zip_indexer::ZipIndexer;
use super::directory_walker::DirectoryWalker;

#[derive(Error, Debug)]
pub enum ResourceScanError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Path error: {0}")]
    Path(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[pyclass]
pub struct ResourceLocation {
    #[pyo3(get)]
    pub source_type: String,  // "zip", "file", "workshop"
    #[pyo3(get)]
    pub source_path: String,
    #[pyo3(get)]
    pub internal_path: Option<String>,  // For ZIP files
    #[pyo3(get)]
    pub size: u64,
    #[pyo3(get)]
    pub modified_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[pyclass]
pub struct ScanResults {
    #[pyo3(get)]
    pub scan_time_ms: u64,
    #[pyo3(get)]
    pub resources_found: u32,
    #[pyo3(get)]
    pub zip_files_scanned: u32,
    #[pyo3(get)]
    pub directories_scanned: u32,
    #[pyo3(get)]
    pub workshop_items_found: u32,
    #[pyo3(get)]
    pub resource_locations: HashMap<String, ResourceLocation>,
}

/// High-performance resource scanner for NWN2 game files
/// 
/// Replaces the Python ResourceManager's bottleneck operations:
/// - ZIP file scanning and indexing
/// - Workshop directory traversal  
/// - Directory indexing for 2DA files
#[pyclass]
pub struct ResourceScanner {
    zip_indexer: ZipIndexer,
    directory_walker: DirectoryWalker,
}

#[pymethods]
impl ResourceScanner {
    #[new]
    pub fn new() -> Self {
        Self {
            zip_indexer: ZipIndexer::new(),
            directory_walker: DirectoryWalker::new(),
        }
    }
    
    /// Scan ZIP files for 2DA resources
    /// 
    /// Args:
    ///     zip_paths: List of ZIP file paths to scan
    ///     
    /// Returns:
    ///     Dictionary mapping resource names to ResourceLocation objects
    pub fn scan_zip_files(&mut self, zip_paths: Vec<String>) -> PyResult<HashMap<String, ResourceLocation>> {
        // Convert strings to paths and filter existing ones
        let valid_paths: Vec<&Path> = zip_paths.iter()
            .map(|s| Path::new(s))
            .filter(|p| p.exists())
            .collect();
        
        if valid_paths.is_empty() {
            return Ok(HashMap::new());
        }
        
        // Always use parallel processing for maximum speed
        match self.zip_indexer.index_zips_parallel(valid_paths) {
            Ok(results) => Ok(results),
            Err(e) => {
                eprintln!("Warning: ZIP parallel scanning failed, falling back to sequential: {}", e);
                // Fallback to sequential if parallel fails
                let mut results = HashMap::new();
                for zip_path_str in zip_paths {
                    let zip_path = Path::new(&zip_path_str);
                    if !zip_path.exists() {
                        continue;
                    }
                    
                    match self.zip_indexer.index_zip(zip_path) {
                        Ok(zip_resources) => {
                            results.extend(zip_resources);
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to index ZIP {}: {}", zip_path_str, e);
                        }
                    }
                }
                Ok(results)
            }
        }
    }
    
    /// Scan workshop directories for override files
    /// 
    /// Args:
    ///     workshop_dirs: List of workshop directory paths to scan
    ///     
    /// Returns:
    ///     Dictionary mapping resource names to ResourceLocation objects
    pub fn scan_workshop_directories(&mut self, workshop_dirs: Vec<String>) -> PyResult<HashMap<String, ResourceLocation>> {
        let mut results = HashMap::new();
        
        for workshop_dir_str in workshop_dirs {
            let workshop_dir = Path::new(&workshop_dir_str);
            
            if !workshop_dir.exists() {
                continue;
            }
            
            match self.directory_walker.scan_workshop_directory(workshop_dir) {
                Ok(workshop_resources) => {
                    for (resource_name, location) in workshop_resources {
                        results.insert(resource_name, location);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to scan workshop directory {}: {}", workshop_dir_str, e);
                }
            }
        }
        
        Ok(results)
    }
    
    /// Index directory for 2DA files
    /// 
    /// Args:
    ///     directory_path: Directory path to index
    ///     recursive: Whether to scan recursively
    ///     
    /// Returns:
    ///     Dictionary mapping resource names to ResourceLocation objects
    pub fn index_directory(&mut self, directory_path: String, recursive: Option<bool>) -> PyResult<HashMap<String, ResourceLocation>> {
        let dir_path = Path::new(&directory_path);
        let is_recursive = recursive.unwrap_or(true);
        
        if !dir_path.exists() {
            return Ok(HashMap::new());
        }
        
        match self.directory_walker.index_directory(dir_path, is_recursive) {
            Ok(resources) => Ok(resources),
            Err(e) => {
                eprintln!("Warning: Failed to index directory {}: {}", directory_path, e);
                Ok(HashMap::new())
            }
        }
    }
    
    /// Comprehensive resource scan
    /// 
    /// Performs all scanning operations and returns combined results with timing info.
    /// 
    /// Args:
    ///     nwn2_data_dir: NWN2 data directory path
    ///     enhanced_data_dir: Enhanced edition data directory (optional)
    ///     workshop_dirs: List of workshop directories
    ///     custom_override_dirs: List of custom override directories
    ///     
    /// Returns:
    ///     ScanResults object with timing and resource information
    #[pyo3(signature = (nwn2_data_dir, workshop_dirs, custom_override_dirs, enhanced_data_dir=None))]
    pub fn comprehensive_scan(
        &mut self,
        nwn2_data_dir: String,
        workshop_dirs: Vec<String>,
        custom_override_dirs: Vec<String>,
        enhanced_data_dir: Option<String>,
    ) -> PyResult<ScanResults> {
        let start_time = Instant::now();
        let mut all_resources = HashMap::new();
        let mut zip_files_scanned = 0;
        let mut directories_scanned = 0;
        let mut workshop_items_found = 0;
        
        // 1. Scan ZIP files in data directories
        let zip_files = vec!["2da.zip", "2da_x1.zip", "2da_x2.zip"];
        let mut zip_paths = Vec::new();
        
        // Base NWN2 data directory
        let data_dir = Path::new(&nwn2_data_dir);
        if data_dir.exists() {
            for zip_name in &zip_files {
                let zip_path = data_dir.join(zip_name);
                if zip_path.exists() {
                    zip_paths.push(zip_path.to_string_lossy().to_string());
                }
            }
        }
        
        // Enhanced edition data directory
        if let Some(enhanced_dir) = enhanced_data_dir {
            let enhanced_path = Path::new(&enhanced_dir);
            if enhanced_path.exists() {
                for zip_name in &zip_files {
                    let zip_path = enhanced_path.join(zip_name);
                    if zip_path.exists() {
                        zip_paths.push(zip_path.to_string_lossy().to_string());
                    }
                }
            }
        }
        
        // Scan ZIP files
        match self.scan_zip_files(zip_paths.clone()) {
            Ok(zip_resources) => {
                zip_files_scanned = zip_paths.len() as u32;
                for (name, location) in zip_resources {
                    all_resources.insert(name, location);
                }
            }
            Err(e) => {
                eprintln!("Warning: ZIP scanning failed: {}", e);
            }
        }
        
        // 2. Scan workshop directories
        match self.scan_workshop_directories(workshop_dirs.clone()) {
            Ok(workshop_resources) => {
                workshop_items_found = workshop_resources.len() as u32;
                for (name, location) in workshop_resources {
                    all_resources.insert(name, location);
                }
            }
            Err(e) => {
                eprintln!("Warning: Workshop scanning failed: {}", e);
            }
        }
        
        // 3. Scan custom override directories
        for override_dir in custom_override_dirs {
            match self.index_directory(override_dir, Some(true)) {
                Ok(override_resources) => {
                    directories_scanned += 1;
                    for (name, location) in override_resources {
                        all_resources.insert(name, location);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Override directory scanning failed: {}", e);
                }
            }
        }
        
        let scan_time_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(ScanResults {
            scan_time_ms,
            resources_found: all_resources.len() as u32,
            zip_files_scanned,
            directories_scanned,
            workshop_items_found,
            resource_locations: all_resources,
        })
    }
    
    /// Get performance statistics from the last scan
    pub fn get_performance_stats(&self) -> PyResult<HashMap<String, u64>> {
        let mut stats = HashMap::new();
        
        // Get stats from sub-components
        stats.extend(self.zip_indexer.get_stats());
        stats.extend(self.directory_walker.get_stats());
        
        Ok(stats)
    }
}

impl Default for ResourceScanner {
    fn default() -> Self {
        Self::new()
    }
}