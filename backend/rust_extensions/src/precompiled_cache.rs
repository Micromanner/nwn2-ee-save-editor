use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

/// Cache metadata structure
#[derive(Debug, Serialize, Deserialize)]
struct CacheMetadata {
    cache_key: String,
    created_at: u64,
    total_tables: usize,
    version: String,
}

/// Cached table data
#[derive(Debug, Serialize, Deserialize)]
struct CachedTable {
    data: Vec<u8>,  // Raw 2DA data
    timestamp: u64,
    row_count: usize,
}

/// Cache section containing multiple tables
type CacheSection = HashMap<String, CachedTable>;

/// Pre-compiled cache builder
#[pyclass]
pub struct CacheBuilder {
    cache_dir: PathBuf,
}

#[pymethods]
impl CacheBuilder {
    #[new]
    fn new(cache_dir: String) -> PyResult<Self> {
        let cache_dir = PathBuf::from(cache_dir).join("compiled_cache");
        
        // Create cache directory if it doesn't exist
        fs::create_dir_all(&cache_dir)?;
        
        Ok(CacheBuilder { cache_dir })
    }

    /// Build cache for given tables
    fn build_cache(&self, 
                   py: Python,
                   tables_data: &PyDict,
                   cache_key: String) -> PyResult<bool> {
        
        let start = SystemTime::now();
        
        // Organize tables by section (base_game, workshop, override)
        let mut base_game_cache = CacheSection::new();
        let mut workshop_cache = CacheSection::new();
        let mut override_cache = CacheSection::new();
        
        let mut total_tables = 0;
        
        // Process each table
        for (key, value) in tables_data.iter() {
            let table_name = key.extract::<String>()?;
            let table_info = value.downcast::<PyDict>()?;
            
            let section = match table_info.get_item("section") {
                Ok(Some(s)) => s.extract::<String>().unwrap_or_else(|_| "base_game".to_string()),
                _ => "base_game".to_string(),
            };
            
            let data = match table_info.get_item("data") {
                Ok(Some(d)) => d.extract::<Vec<u8>>().unwrap_or_default(),
                _ => Vec::new(),
            };
            
            let row_count = match table_info.get_item("row_count") {
                Ok(Some(r)) => r.extract::<usize>().unwrap_or(0),
                _ => 0,
            };
            
            let cached_table = CachedTable {
                data,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                row_count,
            };
            
            match section.as_str() {
                "workshop" => workshop_cache.insert(table_name, cached_table),
                "override" => override_cache.insert(table_name, cached_table),
                _ => base_game_cache.insert(table_name, cached_table),
            };
            
            total_tables += 1;
        }
        
        // Write cache sections
        if !base_game_cache.is_empty() {
            self.write_cache_section("base_game", &base_game_cache)?;
        }
        
        if !workshop_cache.is_empty() {
            self.write_cache_section("workshop", &workshop_cache)?;
        }
        
        if !override_cache.is_empty() {
            self.write_cache_section("override", &override_cache)?;
        }
        
        // Write metadata
        let metadata = CacheMetadata {
            cache_key,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            total_tables,
            version: "1.0.0".to_string(),
        };
        
        self.write_metadata(&metadata)?;
        
        let elapsed = start.elapsed().unwrap();
        println!("Cache build complete in {:.2}s. Total tables: {}", 
                 elapsed.as_secs_f64(), total_tables);
        
        Ok(true)
    }
    
    /// Generate cache key based on mod state
    fn generate_cache_key(&self, py: Python, mod_state: &PyDict) -> PyResult<String> {
        let mut hasher = Sha256::new();
        
        // Add install directory
        if let Ok(Some(install_dir)) = mod_state.get_item("install_dir") {
            let dir = install_dir.extract::<String>()?;
            hasher.update(format!("install:{}", dir).as_bytes());
        }
        
        // Add workshop mods
        if let Ok(Some(workshop_files)) = mod_state.get_item("workshop_files") {
            if let Ok(files) = workshop_files.downcast::<PyList>() {
                let mut sorted_files: Vec<String> = files.extract()?;
                sorted_files.sort();
                hasher.update(format!("workshop:{:?}", sorted_files).as_bytes());
            }
        }
        
        // Add override files
        if let Ok(Some(override_files)) = mod_state.get_item("override_files") {
            if let Ok(files) = override_files.downcast::<PyList>() {
                let mut sorted_files: Vec<String> = files.extract()?;
                sorted_files.sort();
                hasher.update(format!("override:{:?}", sorted_files).as_bytes());
            }
        }
        
        let result = hasher.finalize();
        Ok(format!("{:x}", result)[..16].to_string())
    }
    
}

impl CacheBuilder {
    fn write_cache_section(&self, name: &str, section: &CacheSection) -> PyResult<()> {
        let path = self.cache_dir.join(format!("{}_cache.msgpack", name));
        let data = rmp_serde::to_vec(section)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        
        fs::write(&path, &data)?;
        
        let size_mb = data.len() as f64 / (1024.0 * 1024.0);
        println!("Wrote {} cache: {:.2} MB ({} tables)", name, size_mb, section.len());
        
        Ok(())
    }
    
    fn write_metadata(&self, metadata: &CacheMetadata) -> PyResult<()> {
        let path = self.cache_dir.join("cache_metadata.json");
        let json = serde_json::to_string_pretty(metadata)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        
        fs::write(&path, json)?;
        Ok(())
    }
}

/// Pre-compiled cache manager for loading cached data
#[pyclass]
pub struct CacheManager {
    cache_dir: PathBuf,
    loaded_sections: HashMap<String, CacheSection>,
    metadata: Option<CacheMetadata>,
    cache_valid: Option<bool>,
}

#[pymethods]
impl CacheManager {
    #[new]
    fn new(cache_dir: String) -> PyResult<Self> {
        let cache_dir = PathBuf::from(cache_dir).join("compiled_cache");
        
        Ok(CacheManager {
            cache_dir,
            loaded_sections: HashMap::new(),
            metadata: None,
            cache_valid: None,
        })
    }
    
    /// Get table data from cache
    fn get_table_data(&mut self, table_name: String) -> PyResult<Option<Vec<u8>>> {
        let table_name = if !table_name.ends_with(".2da") {
            format!("{}.2da", table_name)
        } else {
            table_name
        };
        
        // Check cache validity
        if !self.is_cache_valid()? {
            return Ok(None);
        }
        
        // Try each section in priority order
        for section in &["override", "workshop", "base_game"] {
            if !self.loaded_sections.contains_key(*section) {
                self.load_cache_section(section)?;
            }
            
            if let Some(section_data) = self.loaded_sections.get(*section) {
                if let Some(cached_table) = section_data.get(&table_name) {
                    return Ok(Some(cached_table.data.clone()));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Check if cache is valid for current mod state
    fn is_cache_valid(&mut self) -> PyResult<bool> {
        if let Some(valid) = self.cache_valid {
            return Ok(valid);
        }
        
        // Load metadata if not loaded
        if self.metadata.is_none() {
            self.metadata = self.load_metadata()?;
        }
        
        // If no metadata, cache is invalid
        if self.metadata.is_none() {
            self.cache_valid = Some(false);
            return Ok(false);
        }
        
        // For now, we'll let Python check the cache key
        // In a full implementation, we'd generate and compare here
        self.cache_valid = Some(true);
        Ok(true)
    }
    
    /// Validate cache key
    fn validate_cache_key(&mut self, current_key: String) -> PyResult<bool> {
        if self.metadata.is_none() {
            self.metadata = self.load_metadata()?;
        }
        
        if let Some(metadata) = &self.metadata {
            let valid = metadata.cache_key == current_key;
            self.cache_valid = Some(valid);
            Ok(valid)
        } else {
            self.cache_valid = Some(false);
            Ok(false)
        }
    }
    
    /// Invalidate cache
    fn invalidate_cache(&mut self) {
        self.loaded_sections.clear();
        self.metadata = None;
        self.cache_valid = None;
    }
    
    /// Get cache statistics
    fn get_cache_stats(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        
        dict.set_item("valid", self.cache_valid.unwrap_or(false))?;
        dict.set_item("loaded_sections", self.loaded_sections.len())?;
        
        let total_tables: usize = self.loaded_sections
            .values()
            .map(|s| s.len())
            .sum();
        dict.set_item("total_tables_loaded", total_tables)?;
        
        // Calculate cache size
        let mut total_size = 0u64;
        for section in &["base_game", "workshop", "override"] {
            let path = self.cache_dir.join(format!("{}_cache.msgpack", section));
            if path.exists() {
                if let Ok(metadata) = fs::metadata(&path) {
                    total_size += metadata.len();
                }
            }
        }
        
        dict.set_item("cache_size_mb", total_size as f64 / (1024.0 * 1024.0))?;
        
        if let Some(metadata) = &self.metadata {
            dict.set_item("cache_key", &metadata.cache_key)?;
            dict.set_item("created_at", metadata.created_at)?;
            dict.set_item("version", &metadata.version)?;
        }
        
        Ok(dict.into())
    }
}

impl CacheManager {
    fn load_metadata(&self) -> PyResult<Option<CacheMetadata>> {
        let path = self.cache_dir.join("cache_metadata.json");
        
        if !path.exists() {
            return Ok(None);
        }
        
        let json = fs::read_to_string(&path)?;
        let metadata = serde_json::from_str(&json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        
        Ok(Some(metadata))
    }
    
    fn load_cache_section(&mut self, section: &str) -> PyResult<()> {
        let path = self.cache_dir.join(format!("{}_cache.msgpack", section));
        
        if !path.exists() {
            self.loaded_sections.insert(section.to_string(), CacheSection::new());
            return Ok(());
        }
        
        let data = fs::read(&path)?;
        let section_data: CacheSection = rmp_serde::from_slice(&data)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        
        let size_mb = data.len() as f64 / (1024.0 * 1024.0);
        println!("Loaded {} cache: {:.2} MB ({} tables)", 
                 section, size_mb, section_data.len());
        
        self.loaded_sections.insert(section.to_string(), section_data);
        Ok(())
    }
}