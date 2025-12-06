use std::collections::HashMap;
use std::path::Path;
use std::time::{Instant, UNIX_EPOCH};
use walkdir::WalkDir;

use super::resource_scanner::{ResourceLocation, ResourceScanError};

/// High-performance directory walker for 2DA files
/// 
/// Optimized replacement for Python's directory traversal and workshop scanning
pub struct DirectoryWalker {
    stats: HashMap<String, u64>,
}

impl DirectoryWalker {
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
        }
    }
    
    /// Scan workshop directory for 2DA override files
    /// 
    /// Replicates the Python logic from ResourceManager._scan_workshop_directories
    /// but with native performance.
    /// 
    /// Args:
    ///     workshop_dir: Path to the workshop content directory
    ///     
    /// Returns:
    ///     HashMap mapping resource names to ResourceLocation objects
    pub fn scan_workshop_directory(&mut self, workshop_dir: &Path) -> Result<HashMap<String, ResourceLocation>, ResourceScanError> {
        let start_time = Instant::now();
        let mut resources = HashMap::new();
        let mut workshop_items_scanned = 0;
        let mut override_dirs_found = 0;
        let mut files_found = 0;
        
        if !workshop_dir.is_dir() {
            return Ok(resources);
        }
        
        // Iterate through workshop items (each is a mod/workshop item directory)
        for workshop_item_entry in std::fs::read_dir(workshop_dir)? {
            let workshop_item = workshop_item_entry?;
            
            if !workshop_item.file_type()?.is_dir() {
                continue;
            }
            
            workshop_items_scanned += 1;
            let workshop_item_path = workshop_item.path();
            
            // Check for override directory in this workshop item
            let override_dir = workshop_item_path.join("override");
            
            if override_dir.is_dir() {
                override_dirs_found += 1;
                
                // Check subdirectories like override/2DA/
                let tda_subdir = override_dir.join("2DA");
                if tda_subdir.is_dir() {
                    let subdir_files = self.scan_directory_for_2das(&tda_subdir, true)?;
                    files_found += subdir_files.len();
                    resources.extend(subdir_files);
                }
                
                // Also check root override directory
                let root_files = self.scan_directory_for_2das(&override_dir, false)?;
                files_found += root_files.len();
                resources.extend(root_files);
            }
        }
        
        let scan_time = start_time.elapsed();
        
        // Update statistics
        self.stats.insert("last_workshop_scan_time_ms".to_string(), scan_time.as_millis() as u64);
        self.stats.insert("last_workshop_items_scanned".to_string(), workshop_items_scanned);
        self.stats.insert("last_workshop_override_dirs".to_string(), override_dirs_found);
        self.stats.insert("last_workshop_files_found".to_string(), files_found as u64);
        
        Ok(resources)
    }
    
    /// Index directory for 2DA files
    /// 
    /// Args:
    ///     directory: Directory path to index
    ///     recursive: Whether to scan recursively
    ///     
    /// Returns:
    ///     HashMap mapping resource names to ResourceLocation objects
    pub fn index_directory(&mut self, directory: &Path, recursive: bool) -> Result<HashMap<String, ResourceLocation>, ResourceScanError> {
        let start_time = Instant::now();
        let resources = self.scan_directory_for_2das(directory, recursive)?;
        let scan_time = start_time.elapsed();
        
        // Update statistics
        self.stats.insert("last_dir_index_time_ms".to_string(), scan_time.as_millis() as u64);
        self.stats.insert("last_dir_files_found".to_string(), resources.len() as u64);
        
        Ok(resources)
    }
    
    /// Internal method to scan directory for 2DA files
    /// 
    /// Args:
    ///     directory: Directory to scan
    ///     recursive: Whether to scan subdirectories
    ///     
    /// Returns:
    ///     HashMap of found 2DA files
    fn scan_directory_for_2das(&self, directory: &Path, recursive: bool) -> Result<HashMap<String, ResourceLocation>, ResourceScanError> {
        let mut resources = HashMap::new();
        
        if !directory.is_dir() {
            return Ok(resources);
        }
        
        let walker = if recursive {
            WalkDir::new(directory)
        } else {
            WalkDir::new(directory).max_depth(1)
        };
        
        for entry in walker {
            let entry = entry.map_err(|e| ResourceScanError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("WalkDir error: {}", e)
            )))?;
            
            let path = entry.path();
            
            // Skip if not a file
            if !entry.file_type().is_file() {
                continue;
            }
            
            // Check if it's a 2DA file (case-insensitive)
            if let Some(extension) = path.extension() {
                if extension.to_string_lossy().to_lowercase() == "2da" {
                    let metadata = path.metadata()?;
                    let modified_time = metadata
                        .modified()?
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    
                    // Get the base filename (lowercase for case-insensitive lookup)
                    let base_name = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .ok_or_else(|| ResourceScanError::Path("Invalid filename".to_string()))?
                        .to_lowercase();
                    
                    let resource_location = ResourceLocation {
                        source_type: "file".to_string(),
                        source_path: path.to_string_lossy().to_string(),
                        internal_path: None,
                        size: metadata.len(),
                        modified_time,
                    };
                    
                    resources.insert(base_name, resource_location);
                }
            }
        }
        
        Ok(resources)
    }
    
    /// Scan multiple directories in parallel
    /// 
    /// Args:
    ///     directories: Vector of directory paths to scan
    ///     recursive: Whether to scan recursively
    ///     
    /// Returns:
    ///     Combined HashMap of all resources found
    pub fn scan_directories_parallel(&mut self, directories: Vec<&Path>, recursive: bool) -> Result<HashMap<String, ResourceLocation>, ResourceScanError> {
        use rayon::prelude::*;
        
        let start_time = Instant::now();
        
        // Process directories in parallel
        let results: Result<Vec<_>, ResourceScanError> = directories
            .par_iter()
            .map(|dir_path| {
                let walker = DirectoryWalker::new();
                walker.scan_directory_for_2das(dir_path, recursive)
            })
            .collect();
        
        let parallel_results = results?;
        
        // Combine results
        let mut combined_resources = HashMap::new();
        for dir_resources in parallel_results {
            combined_resources.extend(dir_resources);
        }
        
        let total_time = start_time.elapsed();
        
        // Update parallel processing stats
        self.stats.insert("last_parallel_dir_time_ms".to_string(), total_time.as_millis() as u64);
        self.stats.insert("last_parallel_dir_count".to_string(), directories.len() as u64);
        
        Ok(combined_resources)
    }
    
    /// Get walker performance statistics
    pub fn get_stats(&self) -> HashMap<String, u64> {
        self.stats.clone()
    }
    
    /// Reset performance statistics
    pub fn reset_stats(&mut self) {
        self.stats.clear();
    }
}

impl Default for DirectoryWalker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_directory_walker_creation() {
        let walker = DirectoryWalker::new();
        assert!(walker.get_stats().is_empty());
    }
    
    #[test]
    fn test_scan_empty_directory() {
        let walker = DirectoryWalker::new();
        let temp_dir = TempDir::new().unwrap();
        
        let result = walker.scan_directory_for_2das(temp_dir.path(), false);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
    
    #[test]
    fn test_scan_directory_with_2da_files() {
        let walker = DirectoryWalker::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create a test 2DA file
        let test_file = temp_dir.path().join("test.2da");
        fs::write(&test_file, "2DA V2.0\n\n    Label\n0   TestEntry\n").unwrap();
        
        let result = walker.scan_directory_for_2da_files(temp_dir.path(), false);
        assert!(result.is_ok());
        
        let resources = result.unwrap();
        assert_eq!(resources.len(), 1);
        assert!(resources.contains_key("test.2da"));
    }
}