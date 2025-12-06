use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::{Instant, UNIX_EPOCH};
use zip::ZipArchive;

use super::resource_scanner::{ResourceLocation, ResourceScanError};

/// High-performance ZIP file indexer
/// 
/// Optimized replacement for Python's ZIP scanning logic in ResourceManager
pub struct ZipIndexer {
    stats: HashMap<String, u64>,
}

impl ZipIndexer {
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
        }
    }
    
    /// Index a single ZIP file for 2DA resources
    /// 
    /// Args:
    ///     zip_path: Path to the ZIP file
    ///     
    /// Returns:
    ///     HashMap mapping resource names (lowercase) to ResourceLocation objects
    pub fn index_zip(&mut self, zip_path: &Path) -> Result<HashMap<String, ResourceLocation>, ResourceScanError> {
        let start_time = Instant::now();
        let mut resources = HashMap::new();
        
        // Get ZIP file metadata
        let zip_metadata = zip_path.metadata()?;
        let zip_size = zip_metadata.len();
        let zip_modified = zip_metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Open ZIP file with buffered reader for better performance
        let zip_file = File::open(zip_path)?;
        const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer
        let buffered_reader = BufReader::with_capacity(BUFFER_SIZE, zip_file);
        let mut archive = ZipArchive::new(buffered_reader)?;
        
        let mut files_processed = 0;
        let mut tda_files_found = 0;
        
        // Iterate through all files in the ZIP
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let file_name = file.name();
            
            files_processed += 1;
            
            // Check if it's a 2DA file (case-insensitive)
            if file_name.to_lowercase().ends_with(".2da") {
                tda_files_found += 1;
                
                // Extract the base filename (without path)
                let base_name = Path::new(file_name)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(file_name)
                    .to_lowercase();
                
                let resource_location = ResourceLocation {
                    source_type: "zip".to_string(),
                    source_path: zip_path.to_string_lossy().to_string(),
                    internal_path: Some(file_name.to_string()),
                    size: file.size(),
                    modified_time: zip_modified,
                };
                
                // Store with lowercase name for case-insensitive lookup
                resources.insert(base_name, resource_location);
            }
        }
        
        let index_time = start_time.elapsed();
        
        // Update statistics
        self.stats.insert("last_zip_index_time_ms".to_string(), index_time.as_millis() as u64);
        self.stats.insert("last_zip_size_bytes".to_string(), zip_size);
        self.stats.insert("last_zip_files_processed".to_string(), files_processed);
        self.stats.insert("last_zip_2da_files_found".to_string(), tda_files_found);
        
        // Update cumulative stats
        let total_zips = self.stats.get("total_zips_indexed").unwrap_or(&0) + 1;
        let total_time = self.stats.get("total_zip_index_time_ms").unwrap_or(&0) + index_time.as_millis() as u64;
        let total_2das = self.stats.get("total_2da_files_indexed").unwrap_or(&0) + tda_files_found;
        
        self.stats.insert("total_zips_indexed".to_string(), total_zips);
        self.stats.insert("total_zip_index_time_ms".to_string(), total_time);
        self.stats.insert("total_2da_files_indexed".to_string(), total_2das);
        
        Ok(resources)
    }
    
    /// Index multiple ZIP files in parallel
    /// 
    /// Args:
    ///     zip_paths: Vector of ZIP file paths
    ///     
    /// Returns:
    ///     Combined HashMap of all resources found
    pub fn index_zips_parallel(&mut self, zip_paths: Vec<&Path>) -> Result<HashMap<String, ResourceLocation>, ResourceScanError> {
        use rayon::prelude::*;
        
        let start_time = Instant::now();
        
        // Process ZIP files in parallel
        let results: Result<Vec<_>, ResourceScanError> = zip_paths
            .par_iter()
            .map(|zip_path| {
                let mut indexer = ZipIndexer::new();
                indexer.index_zip(zip_path)
            })
            .collect();
        
        let parallel_results = results?;
        
        // Combine results (later ZIP files override earlier ones, matching Python behavior)
        let mut combined_resources = HashMap::new();
        for zip_resources in parallel_results {
            combined_resources.extend(zip_resources);
        }
        
        let total_time = start_time.elapsed();
        
        // Update parallel processing stats
        self.stats.insert("last_parallel_zip_time_ms".to_string(), total_time.as_millis() as u64);
        self.stats.insert("last_parallel_zip_count".to_string(), zip_paths.len() as u64);
        
        Ok(combined_resources)
    }
    
    /// Get indexer performance statistics
    pub fn get_stats(&self) -> HashMap<String, u64> {
        self.stats.clone()
    }
    
    /// Reset performance statistics
    pub fn reset_stats(&mut self) {
        self.stats.clear();
    }
}

impl Default for ZipIndexer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_zip_indexer_creation() {
        let indexer = ZipIndexer::new();
        assert!(indexer.get_stats().is_empty());
    }
    
    // Additional tests would go here for real ZIP files
    // This is a minimal test structure for compilation
}