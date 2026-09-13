use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
struct CacheMetadata {
    cache_key: String,
    created_at: u64,
    total_tables: usize,
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedTable {
    data: Vec<u8>,
    timestamp: u64,
    row_count: usize,
}

type CacheSection = HashMap<String, CachedTable>;

pub struct CacheBuilder {
    cache_dir: PathBuf,
}

impl CacheBuilder {
    pub fn new(cache_dir: String) -> Result<Self, String> {
        let cache_dir = PathBuf::from(cache_dir).join("compiled_cache");

        fs::create_dir_all(&cache_dir)
            .map_err(|e| format!("Failed to create cache directory: {e}"))?;

        Ok(CacheBuilder { cache_dir })
    }

    pub fn build_cache(
        &self,
        tables_data: HashMap<String, HashMap<String, serde_json::Value>>,
        cache_key: String,
    ) -> Result<bool, String> {
        let start = SystemTime::now();

        let mut base_game_cache = CacheSection::new();
        let mut workshop_cache = CacheSection::new();
        let mut override_cache = CacheSection::new();

        let mut total_tables = 0;

        for (table_name, table_info) in tables_data {
            let section = table_info
                .get("section")
                .and_then(|v| v.as_str())
                .unwrap_or("base_game");

            let data = table_info
                .get("data")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_u64().map(|n| n as u8))
                        .collect::<Vec<u8>>()
                })
                .unwrap_or_default();

            let row_count = table_info
                .get("row_count")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as usize;

            let cached_table = CachedTable {
                data,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                row_count,
            };

            match section {
                "workshop" => workshop_cache.insert(table_name, cached_table),
                "override" => override_cache.insert(table_name, cached_table),
                _ => base_game_cache.insert(table_name, cached_table),
            };

            total_tables += 1;
        }

        if !base_game_cache.is_empty() {
            self.write_cache_section("base_game", &base_game_cache)?;
        }

        if !workshop_cache.is_empty() {
            self.write_cache_section("workshop", &workshop_cache)?;
        }

        if !override_cache.is_empty() {
            self.write_cache_section("override", &override_cache)?;
        }

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
        println!(
            "Cache build complete in {:.2}s. Total tables: {}",
            elapsed.as_secs_f64(),
            total_tables
        );

        Ok(true)
    }

    pub fn generate_cache_key(
        &self,
        mod_state: HashMap<String, serde_json::Value>,
    ) -> Result<String, String> {
        let mut hasher = Sha256::new();

        if let Some(install_dir) = mod_state.get("install_dir").and_then(|v| v.as_str()) {
            hasher.update(format!("install:{install_dir}").as_bytes());
        }

        if let Some(workshop_files) = mod_state.get("workshop_files").and_then(|v| v.as_array()) {
            let mut sorted_files: Vec<String> = workshop_files
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            sorted_files.sort();
            hasher.update(format!("workshop:{sorted_files:?}").as_bytes());
        }

        if let Some(override_files) = mod_state.get("override_files").and_then(|v| v.as_array()) {
            let mut sorted_files: Vec<String> = override_files
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            sorted_files.sort();
            hasher.update(format!("override:{sorted_files:?}").as_bytes());
        }

        let result = hasher.finalize();
        Ok(hex::encode(result)[..16].to_string())
    }

    fn write_cache_section(&self, name: &str, section: &CacheSection) -> Result<(), String> {
        let path = self.cache_dir.join(format!("{name}_cache.msgpack"));
        let data = rmp_serde::to_vec(section)
            .map_err(|e| format!("Failed to serialize cache section: {e}"))?;

        fs::write(&path, &data).map_err(|e| format!("Failed to write cache file: {e}"))?;

        let size_mb = data.len() as f64 / (1024.0 * 1024.0);
        println!(
            "Wrote {} cache: {:.2} MB ({} tables)",
            name,
            size_mb,
            section.len()
        );

        Ok(())
    }

    fn write_metadata(&self, metadata: &CacheMetadata) -> Result<(), String> {
        let path = self.cache_dir.join("cache_metadata.json");
        let json = serde_json::to_string_pretty(metadata)
            .map_err(|e| format!("Failed to serialize metadata: {e}"))?;

        fs::write(&path, json).map_err(|e| format!("Failed to write metadata: {e}"))?;
        Ok(())
    }
}

pub struct CacheManager {
    cache_dir: PathBuf,
    loaded_sections: HashMap<String, CacheSection>,
    metadata: Option<CacheMetadata>,
    cache_valid: Option<bool>,
}

impl CacheManager {
    pub fn new(cache_dir: String) -> Result<Self, String> {
        let cache_dir = PathBuf::from(cache_dir).join("compiled_cache");

        Ok(CacheManager {
            cache_dir,
            loaded_sections: HashMap::new(),
            metadata: None,
            cache_valid: None,
        })
    }

    pub fn get_table_data(&mut self, table_name: String) -> Result<Option<Vec<u8>>, String> {
        let table_name = if !table_name.ends_with(".2da") {
            format!("{table_name}.2da")
        } else {
            table_name
        };

        if !self.is_cache_valid()? {
            return Ok(None);
        }

        for section in &["override", "workshop", "base_game"] {
            if !self.loaded_sections.contains_key(*section) {
                self.load_cache_section(section)?;
            }

            if let Some(section_data) = self.loaded_sections.get(*section)
                && let Some(cached_table) = section_data.get(&table_name)
            {
                return Ok(Some(cached_table.data.clone()));
            }
        }

        Ok(None)
    }

    pub fn is_cache_valid(&mut self) -> Result<bool, String> {
        if let Some(valid) = self.cache_valid {
            return Ok(valid);
        }

        if self.metadata.is_none() {
            self.metadata = self.load_metadata()?;
        }

        if self.metadata.is_none() {
            self.cache_valid = Some(false);
            return Ok(false);
        }

        self.cache_valid = Some(true);
        Ok(true)
    }

    pub fn validate_cache_key(&mut self, current_key: String) -> Result<bool, String> {
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

    pub fn invalidate_cache(&mut self) {
        self.loaded_sections.clear();
        self.metadata = None;
        self.cache_valid = None;
    }

    pub fn get_cache_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();

        stats.insert(
            "valid".to_string(),
            serde_json::json!(self.cache_valid.unwrap_or(false)),
        );
        stats.insert(
            "loaded_sections".to_string(),
            serde_json::json!(self.loaded_sections.len()),
        );

        let total_tables: usize = self
            .loaded_sections
            .values()
            .map(std::collections::HashMap::len)
            .sum();
        stats.insert(
            "total_tables_loaded".to_string(),
            serde_json::json!(total_tables),
        );

        let mut total_size = 0u64;
        for section in &["base_game", "workshop", "override"] {
            let path = self.cache_dir.join(format!("{section}_cache.msgpack"));
            if path.exists()
                && let Ok(metadata) = fs::metadata(&path)
            {
                total_size += metadata.len();
            }
        }

        stats.insert(
            "cache_size_mb".to_string(),
            serde_json::json!(total_size as f64 / (1024.0 * 1024.0)),
        );

        if let Some(metadata) = &self.metadata {
            stats.insert(
                "cache_key".to_string(),
                serde_json::json!(&metadata.cache_key),
            );
            stats.insert(
                "created_at".to_string(),
                serde_json::json!(metadata.created_at),
            );
            stats.insert("version".to_string(), serde_json::json!(&metadata.version));
        }

        stats
    }

    fn load_metadata(&self) -> Result<Option<CacheMetadata>, String> {
        let path = self.cache_dir.join("cache_metadata.json");

        if !path.exists() {
            return Ok(None);
        }

        let json =
            fs::read_to_string(&path).map_err(|e| format!("Failed to read metadata: {e}"))?;
        let metadata =
            serde_json::from_str(&json).map_err(|e| format!("Failed to parse metadata: {e}"))?;

        Ok(Some(metadata))
    }

    fn load_cache_section(&mut self, section: &str) -> Result<(), String> {
        let path = self.cache_dir.join(format!("{section}_cache.msgpack"));

        if !path.exists() {
            self.loaded_sections
                .insert(section.to_string(), CacheSection::new());
            return Ok(());
        }

        let data = fs::read(&path).map_err(|e| format!("Failed to read cache section: {e}"))?;
        let section_data: CacheSection = rmp_serde::from_slice(&data)
            .map_err(|e| format!("Failed to deserialize cache section: {e}"))?;

        let size_mb = data.len() as f64 / (1024.0 * 1024.0);
        println!(
            "Loaded {} cache: {:.2} MB ({} tables)",
            section,
            size_mb,
            section_data.len()
        );

        self.loaded_sections
            .insert(section.to_string(), section_data);
        Ok(())
    }
}
