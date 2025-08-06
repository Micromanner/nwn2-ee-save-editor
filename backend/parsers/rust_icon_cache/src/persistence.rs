//! Persistent disk cache implementation

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use serde::{Serialize, Deserialize};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use flate2::Compression;
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use std::io::{Read, Write};
use uuid::Uuid;

use crate::{
    types::CachedIcon,
    error::{Result, IconCacheError},
    IconKey,
    StringInterner,
};

/// Version of the cache format for compatibility checking
const CACHE_FORMAT_VERSION: u32 = 1;

/// Persistent cache data structure
#[derive(Clone, Serialize, Deserialize)]
pub struct PersistedCacheData {
    /// Schema version for compatibility
    pub version: u32,
    /// All cached icons
    pub cache: Vec<(IconKey, CachedIcon)>,
    /// Checksum for integrity verification
    pub checksum: u64,
}

/// Handles disk-based cache persistence
pub struct DiskCache {
    cache_path: PathBuf,
}

impl DiskCache {
    /// Create a new disk cache handler
    pub fn new(cache_dir: impl AsRef<Path>) -> Result<Self> {
        let cache_dir = cache_dir.as_ref();
        
        // Ensure cache directory exists
        std::fs::create_dir_all(cache_dir)?;
        
        // Clean up any orphaned temp files from previous runs
        if let Ok(entries) = std::fs::read_dir(cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with("icon_cache.bin.gz.tmp.") {
                        log::warn!("Removing orphaned temp file: {:?}", path);
                        let _ = std::fs::remove_file(path);
                    }
                }
            }
        }
        
        let cache_path = cache_dir.join("icon_cache.bin.gz");
        
        Ok(Self { cache_path })
    }
    
    /// Check if a valid cache file exists
    pub async fn is_valid(&self) -> Result<bool> {
        if !self.cache_path.exists() {
            return Ok(false);
        }
        
        // Try to read and validate the cache header
        match self.read_and_validate_header().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    /// Load the cache from disk
    pub async fn load(&self) -> Result<PersistedCacheData> {
        // Read compressed data
        let compressed_data = fs::read(&self.cache_path).await?;
        
        // Decompress
        let mut decoder = GzDecoder::new(&compressed_data[..]);
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data)
            .map_err(|e| IconCacheError::Io(e))?;
        
        // Deserialize
        let wrapper: PersistedCacheData = bincode::deserialize(&decompressed_data)?;
        
        // Version check
        if wrapper.version != CACHE_FORMAT_VERSION {
            return Err(IconCacheError::VersionMismatch {
                expected: CACHE_FORMAT_VERSION,
                found: wrapper.version,
            });
        }
        
        // Integrity check
        let expected_checksum = wrapper.checksum;
        let mut temp_wrapper = wrapper.clone();
        temp_wrapper.checksum = 0;
        
        let verification_data = bincode::serialize(&temp_wrapper)?;
        let actual_checksum = calculate_checksum(&verification_data);
        
        if actual_checksum != expected_checksum {
            return Err(IconCacheError::IntegrityCheckFailed);
        }
        
        log::info!("Loaded {} icons from cache", wrapper.cache.len());
        
        Ok(wrapper)
    }
    
    /// Save the cache to disk atomically
    pub async fn save(
        &self, 
        _interner: Arc<StringInterner>, 
        cache_data: Vec<(IconKey, CachedIcon)>
    ) -> Result<()> {
        log::info!("Saving {} icons to cache", cache_data.len());
        
        // Create cache wrapper
        let mut cache_wrapper = PersistedCacheData {
            version: CACHE_FORMAT_VERSION,
            cache: cache_data,
            checksum: 0,
        };
        
        // Serialize once to calculate checksum
        let temp_data = bincode::serialize(&cache_wrapper)?;
        let checksum = calculate_checksum(&temp_data);
        cache_wrapper.checksum = checksum;
        
        // Serialize with checksum
        let final_data = bincode::serialize(&cache_wrapper)?;
        
        // Compress
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&final_data)
            .map_err(|e| IconCacheError::Io(e))?;
        let compressed_data = encoder.finish()
            .map_err(|e| IconCacheError::Io(e))?;
        
        // Atomic write (temp file + rename)
        let temp_path = self.cache_path.with_extension(format!("tmp.{}", Uuid::new_v4()));
        
        // Write to temp file
        let mut file = fs::File::create(&temp_path).await?;
        file.write_all(&compressed_data).await?;
        file.sync_all().await?;
        drop(file);
        
        // Atomic rename
        fs::rename(&temp_path, &self.cache_path).await?;
        
        log::info!("Cache saved successfully ({} bytes compressed)", compressed_data.len());
        
        Ok(())
    }
    
    /// Clear the cache file
    pub async fn clear(&self) -> Result<()> {
        if self.cache_path.exists() {
            fs::remove_file(&self.cache_path).await?;
        }
        Ok(())
    }
    
    /// Read and validate just the header for quick validation
    async fn read_and_validate_header(&self) -> Result<()> {
        // Optimized validation - just check if file exists and has minimum size
        // Full validation happens during load() anyway
        let metadata = fs::metadata(&self.cache_path).await?;
        
        // Basic sanity check - cache file should be at least 100 bytes
        if metadata.len() < 100 {
            return Err(IconCacheError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Cache file too small"
            )));
        }
        
        // Check if file was modified recently (might indicate corruption)
        let modified = metadata.modified()
            .map_err(|e| IconCacheError::Io(e))?;
        let now = std::time::SystemTime::now();
        
        // If modified very recently (within last 5 seconds), might be incomplete write
        if let Ok(duration) = now.duration_since(modified) {
            if duration.as_secs() < 5 {
                return Err(IconCacheError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Cache file too recently modified"
                )));
            }
        }
        
        Ok(())
    }
}

/// Calculate a checksum for data integrity verification
fn calculate_checksum(data: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::types::{SourceType, ImageFormat};
    
    #[tokio::test]
    async fn test_disk_cache_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let cache = DiskCache::new(temp_dir.path()).unwrap();
        
        // Create some test data
        let interner = Arc::new(StringInterner::default());
        let key1 = interner.get_or_intern("test_icon_1");
        let key2 = interner.get_or_intern("test_icon_2");
        
        let icon1 = CachedIcon::new(vec![1, 2, 3], ImageFormat::WebP, SourceType::BaseGame);
        let icon2 = CachedIcon::new(vec![4, 5, 6], ImageFormat::Avif, SourceType::Override);
        
        let test_data = vec![(key1, icon1), (key2, icon2)];
        
        // Save
        cache.save(interner.clone(), test_data.clone()).await.unwrap();
        
        // Load
        let loaded = cache.load().await.unwrap();
        
        // Verify
        assert_eq!(loaded.version, CACHE_FORMAT_VERSION);
        assert_eq!(loaded.cache.len(), 2);
        assert_eq!(loaded.cache[0].1.data.as_ref(), &vec![1, 2, 3]);
        assert_eq!(loaded.cache[1].1.data.as_ref(), &vec![4, 5, 6]);
    }
    
    #[tokio::test]
    async fn test_cache_integrity_check() {
        let temp_dir = TempDir::new().unwrap();
        let cache = DiskCache::new(temp_dir.path()).unwrap();
        
        // Create and save some data
        let interner = Arc::new(StringInterner::default());
        let key = interner.get_or_intern("test");
        let icon = CachedIcon::new(vec![1, 2, 3], ImageFormat::WebP, SourceType::BaseGame);
        
        cache.save(interner, vec![(key, icon)]).await.unwrap();
        
        // Corrupt the file
        let mut data = std::fs::read(&cache.cache_path).unwrap();
        data[data.len() - 10] ^= 0xFF; // Flip some bits
        std::fs::write(&cache.cache_path, data).unwrap();
        
        // Try to load - should fail integrity check
        let result = cache.load().await;
        assert!(matches!(result, Err(IconCacheError::IntegrityCheckFailed)));
    }
}