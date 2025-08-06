//! Index-based lazy loading for icon cache

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use dashmap::DashMap;
use lru::LruCache;
use tokio::sync::RwLock;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use crate::{
    types::{CachedIcon, SourceType, ImageFormat},
    error::{Result, IconCacheError},
    IconKey,
    StringInterner,
};

/// Version of the index format
const INDEX_FORMAT_VERSION: u32 = 1;

/// Entry in the icon index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Icon name/path
    pub name: String,
    /// Byte offset in data file
    pub offset: u64,
    /// Compressed size in bytes
    pub size: u32,
    /// Image format
    pub format: ImageFormat,
    /// Source type
    pub source_type: SourceType,
    /// Optional checksum for integrity
    pub checksum: Option<u32>,
}

/// Icon index for fast lookups
#[derive(Debug, Serialize, Deserialize)]
pub struct IconIndex {
    /// Format version
    pub version: u32,
    /// All icon entries
    pub entries: Vec<IndexEntry>,
    /// Total data file size for validation
    pub data_file_size: u64,
}

impl IconIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self {
            version: INDEX_FORMAT_VERSION,
            entries: Vec::new(),
            data_file_size: 0,
        }
    }
    
    /// Build a HashMap for O(1) lookups
    pub fn build_lookup_map(&self) -> HashMap<String, IndexEntry> {
        self.entries
            .iter()
            .map(|entry| (entry.name.clone(), entry.clone()))
            .collect()
    }
}

/// Lazy-loading icon cache with index-based access
pub struct LazyIconCache {
    /// Index for all icons (small, loaded at startup)
    index: Arc<RwLock<IconIndex>>,
    
    /// Lookup map for O(1) access
    lookup_map: Arc<RwLock<HashMap<String, IndexEntry>>>,
    
    /// Path to the data file
    data_file_path: PathBuf,
    
    /// Path to the index file
    index_file_path: PathBuf,
    
    /// LRU cache for loaded icons (memory cache)
    lru_cache: Arc<RwLock<LruCache<String, CachedIcon>>>,
    
    /// String interner for efficient memory usage
    interner: Arc<StringInterner>,
    
    /// Filename-to-fullpath mapping for fallback lookups
    filename_map: Arc<DashMap<String, String>>,
    
    /// Statistics
    cache_hits: Arc<RwLock<u64>>,
    cache_misses: Arc<RwLock<u64>>,
}

impl LazyIconCache {
    /// Create a new lazy-loading cache
    pub fn new(cache_dir: impl AsRef<Path>, max_memory_items: usize) -> Result<Self> {
        let cache_dir = cache_dir.as_ref();
        std::fs::create_dir_all(cache_dir)?;
        
        Ok(Self {
            index: Arc::new(RwLock::new(IconIndex::new())),
            lookup_map: Arc::new(RwLock::new(HashMap::new())),
            data_file_path: cache_dir.join("icon_data.bin"),
            index_file_path: cache_dir.join("icon_index.bin"),
            lru_cache: Arc::new(RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(max_memory_items).unwrap()
            ))),
            interner: Arc::new(StringInterner::default()),
            filename_map: Arc::new(DashMap::new()),
            cache_hits: Arc::new(RwLock::new(0)),
            cache_misses: Arc::new(RwLock::new(0)),
        })
    }
    
    /// Load only the index file (fast startup)
    pub async fn load_index(&self) -> Result<()> {
        if !self.index_file_path.exists() {
            return Err(IconCacheError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Index file not found"
            )));
        }
        
        // Read index file (should be small, ~100KB)
        let index_data = tokio::fs::read(&self.index_file_path).await?;
        let index: IconIndex = bincode::deserialize(&index_data)?;
        
        // Validate version
        if index.version != INDEX_FORMAT_VERSION {
            return Err(IconCacheError::VersionMismatch {
                expected: INDEX_FORMAT_VERSION,
                found: index.version,
            });
        }
        
        // Build lookup map
        let lookup_map = index.build_lookup_map();
        
        // Build filename mappings
        for entry in &index.entries {
            if let Some(filename) = entry.name.split('/').last() {
                if filename != entry.name {
                    self.filename_map.insert(filename.to_string(), entry.name.clone());
                }
            }
        }
        
        // Store index and lookup map
        *self.index.write().await = index;
        *self.lookup_map.write().await = lookup_map;
        
        log::info!("Loaded icon index with {} entries", self.index.read().await.entries.len());
        
        Ok(())
    }
    
    /// Get an icon by name (lazy load if not in memory)
    pub async fn get_icon(&self, name: &str) -> Option<CachedIcon> {
        // Check LRU cache first
        {
            let mut cache = self.lru_cache.write().await;
            if let Some(icon) = cache.get(name) {
                *self.cache_hits.write().await += 1;
                return Some(icon.clone());
            }
        }
        
        // Try filename fallback if not found
        let actual_name = if !name.contains('/') {
            if let Some(full_path) = self.filename_map.get(name) {
                full_path.value().clone()
            } else {
                name.to_string()
            }
        } else {
            name.to_string()
        };
        
        // Look up in index
        let lookup_map = self.lookup_map.read().await;
        let entry = lookup_map.get(&actual_name)?;
        
        // Load from disk
        match self.load_icon_from_disk(entry).await {
            Ok(icon) => {
                // Store in LRU cache
                let mut cache = self.lru_cache.write().await;
                cache.put(actual_name.clone(), icon.clone());
                cache.put(name.to_string(), icon.clone()); // Also cache under original name
                
                *self.cache_misses.write().await += 1;
                Some(icon)
            }
            Err(e) => {
                log::error!("Failed to load icon '{}' from disk: {}", name, e);
                None
            }
        }
    }
    
    /// Load a specific icon from disk using index entry
    async fn load_icon_from_disk(&self, entry: &IndexEntry) -> Result<CachedIcon> {
        let mut file = File::open(&self.data_file_path).await?;
        
        // Seek to icon position
        file.seek(tokio::io::SeekFrom::Start(entry.offset)).await?;
        
        // Read icon data
        let mut buffer = vec![0u8; entry.size as usize];
        file.read_exact(&mut buffer).await?;
        
        // Optional: verify checksum
        if let Some(expected_checksum) = entry.checksum {
            let actual_checksum = crc32fast::hash(&buffer);
            if actual_checksum != expected_checksum {
                return Err(IconCacheError::IntegrityCheckFailed);
            }
        }
        
        Ok(CachedIcon::new(buffer, entry.format, entry.source_type))
    }
    
    /// Get cache statistics
    pub async fn get_stats(&self) -> (u64, u64, f64) {
        let hits = *self.cache_hits.read().await;
        let misses = *self.cache_misses.read().await;
        let total = hits + misses;
        let hit_rate = if total > 0 {
            (hits as f64) / (total as f64)
        } else {
            0.0
        };
        (hits, misses, hit_rate)
    }
    
    /// Clear the memory cache (but keep index)
    pub async fn clear_memory_cache(&self) {
        self.lru_cache.write().await.clear();
        *self.cache_hits.write().await = 0;
        *self.cache_misses.write().await = 0;
    }
    
    /// Clone references for spawning tasks
    fn clone_refs(&self) -> Self {
        Self {
            index: self.index.clone(),
            lookup_map: self.lookup_map.clone(),
            data_file_path: self.data_file_path.clone(),
            index_file_path: self.index_file_path.clone(),
            lru_cache: self.lru_cache.clone(),
            interner: self.interner.clone(),
            filename_map: self.filename_map.clone(),
            cache_hits: self.cache_hits.clone(),
            cache_misses: self.cache_misses.clone(),
        }
    }
}

/// Builder for creating index and data files from existing cache
pub struct IndexBuilder {
    entries: Vec<IndexEntry>,
    data_buffer: Vec<u8>,
    current_offset: u64,
}

impl IndexBuilder {
    /// Create a new index builder
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            data_buffer: Vec::new(),
            current_offset: 0,
        }
    }
    
    /// Add an icon to the index
    pub fn add_icon(&mut self, name: String, icon: &CachedIcon) {
        let data = icon.data.as_ref();
        let size = data.len() as u32;
        let checksum = Some(crc32fast::hash(data));
        
        // Create index entry
        let entry = IndexEntry {
            name,
            offset: self.current_offset,
            size,
            format: icon.format,
            source_type: icon.source_type,
            checksum,
        };
        
        // Add to index
        self.entries.push(entry);
        
        // Append data
        self.data_buffer.extend_from_slice(data);
        self.current_offset += size as u64;
    }
    
    /// Build and save the index and data files
    pub async fn save(&self, cache_dir: impl AsRef<Path>) -> Result<()> {
        let cache_dir = cache_dir.as_ref();
        std::fs::create_dir_all(cache_dir)?;
        
        let index_path = cache_dir.join("icon_index.bin");
        let data_path = cache_dir.join("icon_data.bin");
        
        // Create index
        let index = IconIndex {
            version: INDEX_FORMAT_VERSION,
            entries: self.entries.clone(),
            data_file_size: self.current_offset,
        };
        
        // Serialize index
        let index_data = bincode::serialize(&index)?;
        
        // Write index file (small, quick write)
        let mut index_file = File::create(&index_path).await?;
        index_file.write_all(&index_data).await?;
        index_file.sync_all().await?;
        
        // Write data file
        let mut data_file = File::create(&data_path).await?;
        data_file.write_all(&self.data_buffer).await?;
        data_file.sync_all().await?;
        
        log::info!(
            "Saved icon cache: {} entries, {} bytes index, {} bytes data",
            self.entries.len(),
            index_data.len(),
            self.data_buffer.len()
        );
        
        Ok(())
    }
}