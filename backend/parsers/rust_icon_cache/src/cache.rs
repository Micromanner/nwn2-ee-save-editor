//! Main cache implementation

use std::sync::Arc;
use std::path::{Path, PathBuf};
use dashmap::DashMap;
use tokio::sync::RwLock;

use crate::{
    types::{CachedIcon, SourceType, IconSource, CacheStatistics},
    config::IconCacheConfig,
    error::Result,
    persistence::DiskCache,
    index::{LazyIconCache, IndexBuilder},
    discovery::IconDiscovery,
    processing::ImageProcessor,
    IconKey,
    StringInterner,
};

/// High-performance icon cache with persistent storage
pub struct RustIconCache {
    /// Simple, high-performance concurrent HashMap - holds ALL icons in memory
    pub(crate) cache: Arc<DashMap<IconKey, CachedIcon>>,
    
    /// String interner for efficient memory usage
    pub(crate) interner: Arc<StringInterner>,
    
    /// Filename-to-full-path mapping for fallback lookups
    /// Maps "is_calllightning" -> "evocation/spell/is_calllightning"
    filename_map: Arc<DashMap<String, String>>,
    
    /// Configuration
    config: IconCacheConfig,
    
    /// NWN2 home directory path
    nwn2_home: Option<PathBuf>,
    
    /// Persistent disk cache for fast startup
    disk_cache: Arc<DiskCache>,
    
    /// Lazy loading cache (new optimized system)
    lazy_cache: Option<Arc<LazyIconCache>>,
    
    /// Image processor
    processor: Arc<ImageProcessor>,
    
    /// Icon discovery engine
    discovery: Arc<IconDiscovery>,
    
    /// Statistics
    stats: Arc<RwLock<CacheStatistics>>,
    
    /// Flag to use lazy loading
    use_lazy_loading: bool,
}

impl RustIconCache {
    /// Create a new icon cache
    pub fn new(config: IconCacheConfig) -> Result<Self> {
        let cache_dir = config.cache_directory.clone();
        let use_lazy_loading = std::env::var("ICON_CACHE_LAZY_LOADING")
            .unwrap_or_else(|_| "true".to_string()) == "true";
        
        let lazy_cache = if use_lazy_loading {
            Some(Arc::new(LazyIconCache::new(&cache_dir, 1000)?))
        } else {
            None
        };
        
        Ok(Self {
            cache: Arc::new(DashMap::new()),
            interner: Arc::new(StringInterner::default()),
            filename_map: Arc::new(DashMap::new()),
            config: config.clone(),
            nwn2_home: None,
            disk_cache: Arc::new(DiskCache::new(&cache_dir)?),
            lazy_cache,
            processor: Arc::new(ImageProcessor::new(config.clone())),
            discovery: Arc::new(IconDiscovery::new()),
            stats: Arc::new(RwLock::new(CacheStatistics::default())),
            use_lazy_loading,
        })
    }
    
    /// Create a new icon cache with NWN2 home directory
    pub fn with_nwn2_home(config: IconCacheConfig, nwn2_home: PathBuf) -> Result<Self> {
        let cache_dir = config.cache_directory.clone();
        let use_lazy_loading = std::env::var("ICON_CACHE_LAZY_LOADING")
            .unwrap_or_else(|_| "true".to_string()) == "true";
        
        let lazy_cache = if use_lazy_loading {
            Some(Arc::new(LazyIconCache::new(&cache_dir, 1000)?))
        } else {
            None
        };
        
        Ok(Self {
            cache: Arc::new(DashMap::new()),
            interner: Arc::new(StringInterner::default()),
            filename_map: Arc::new(DashMap::new()),
            config: config.clone(),
            nwn2_home: Some(nwn2_home),
            disk_cache: Arc::new(DiskCache::new(&cache_dir)?),
            lazy_cache,
            processor: Arc::new(ImageProcessor::new(config.clone())),
            discovery: Arc::new(IconDiscovery::new()),
            stats: Arc::new(RwLock::new(CacheStatistics::default())),
            use_lazy_loading,
        })
    }
    
    /// Initialize the cache (load from disk or rebuild)
    pub async fn initialize(&self, force_reload: bool) -> Result<()> {
        println!("RustIconCache::initialize called with force_reload={}, lazy_loading={}", 
                 force_reload, self.use_lazy_loading);
        
        // If using lazy loading, just load the index
        if self.use_lazy_loading {
            if let Some(lazy_cache) = &self.lazy_cache {
                if !force_reload {
                    // Try to load existing index
                    log::info!("Loading icon index for lazy loading...");
                    println!("Loading icon index (lazy mode)...");
                    match lazy_cache.load_index().await {
                        Ok(_) => {
                            log::info!("Icon index loaded successfully");
                            println!("Icon index loaded successfully (startup complete)");
                            
                            return Ok(());
                        }
                        Err(e) => {
                            log::warn!("Failed to load index: {}, building cache...", e);
                            println!("Index not found, building cache...");
                        }
                    }
                }
                
                // Build index from scratch
                return self.build_lazy_cache_index().await;
            }
        }
        
        // Original behavior for non-lazy loading
        if !force_reload && self.config.enable_persistence && self.disk_cache.is_valid().await? {
            // Fast path: load pre-processed cache from disk
            log::info!("Loading icon cache from disk...");
            println!("Attempting to load from disk cache...");
            match self.load_from_persistent_cache().await {
                Ok(_) => {
                    log::info!("Icon cache loaded successfully from disk");
                    println!("Loaded from disk cache successfully");
                    return Ok(());
                }
                Err(e) => {
                    log::warn!("Failed to load cache from disk: {}, rebuilding...", e);
                    println!("Failed to load from disk: {}", e);
                }
            }
        }
        
        // Slow path: full rebuild with flattened hierarchy
        log::info!("Building icon cache from scratch...");
        println!("Building icon cache from scratch...");
        self.full_rebuild_and_cache().await
    }
    
    /// Get an icon by name with filename fallback
    pub fn get_icon(&self, name: &str) -> Option<CachedIcon> {
        // Use lazy loading if enabled
        if self.use_lazy_loading {
            if let Some(lazy_cache) = &self.lazy_cache {
                // Create a runtime for async operation
                let runtime = tokio::runtime::Runtime::new().ok()?;
                return runtime.block_on(lazy_cache.get_icon(name));
            }
        }
        
        // Debug cache access for alignment icons
        if name.contains("align") {
            log::info!("CACHE ACCESS DEBUG: Requesting icon '{}', cache size: {}", name, self.cache.len());
        }
        
        // Try exact match first
        if let Some(key) = self.interner.get(name) {
            if let Some(entry) = self.cache.get(&key) {
                if name.contains("align") {
                    log::info!("CACHE ACCESS DEBUG: Found icon '{}' in cache", name);
                }
                return Some(entry.value().clone());
            } else {
                if name.contains("align") {
                    log::info!("CACHE ACCESS DEBUG: Icon '{}' key exists in interner but not in cache", name);
                }
            }
        } else {
            if name.contains("align") {
                log::info!("CACHE ACCESS DEBUG: Icon '{}' key not found in interner", name);
            }
        }
        
        // If not found and name doesn't contain '/', try filename fallback
        if !name.contains('/') {
            if let Some(full_path) = self.filename_map.get(name) {
                log::debug!("Found filename mapping: {} -> {}", name, full_path.value());
                if let Some(key) = self.interner.get(full_path.value()) {
                    if let Some(entry) = self.cache.get(&key) {
                        return Some(entry.value().clone());
                    }
                }
            }
            
            // If still not found, try lowercase filename fallback
            let lowercase_name = name.to_lowercase();
            if lowercase_name != name {
                if let Some(full_path) = self.filename_map.get(&lowercase_name) {
                    log::debug!("Found lowercase filename mapping: {} -> {}", lowercase_name, full_path.value());
                    if let Some(key) = self.interner.get(full_path.value()) {
                        if let Some(entry) = self.cache.get(&key) {
                            return Some(entry.value().clone());
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// Get an icon by name (async version for consistency)
    pub async fn get_icon_async(&self, name: &str) -> Option<CachedIcon> {
        // Use lazy loading if enabled
        if self.use_lazy_loading {
            if let Some(lazy_cache) = &self.lazy_cache {
                return lazy_cache.get_icon(name).await;
            }
        }
        
        self.get_icon(name)
    }
    
    /// Get multiple icons in a batch
    pub fn get_icons_batch(&self, names: &[String]) -> Vec<Option<CachedIcon>> {
        names.iter()
            .map(|name| self.get_icon(name))
            .collect()
    }
    
    /// Set the module HAK list
    pub async fn set_module_haks(&self, hak_list: Vec<String>) -> Result<()> {
        // This would trigger a partial rebuild for just the HAK icons
        // For now, we'll implement this later
        log::info!("Setting module HAKs: {:?}", hak_list);
        Ok(())
    }
    
    /// Load module icons
    pub async fn load_module_icons(&self, module_path: &Path) -> Result<()> {
        // This would load icons from a specific module
        // For now, we'll implement this later
        log::info!("Loading module icons from: {:?}", module_path);
        Ok(())
    }
    
    /// Get cache statistics
    pub async fn get_statistics(&self) -> CacheStatistics {
        self.stats.read().await.clone()
    }
    
    /// Get cache statistics (sync version)
    pub fn get_statistics_sync(&self) -> CacheStatistics {
        // For sync access, we'll create a simple snapshot
        let mut stats = CacheStatistics::default();
        stats.total_icons = self.cache.len();
        
        // Calculate memory usage
        for entry in self.cache.iter() {
            let icon = entry.value();
            stats.memory_usage += icon.size();
            
            *stats.source_counts.entry(icon.source_type).or_insert(0) += 1;
        }
        
        stats
    }
    
    /// Initialize the cache synchronously (creates its own Tokio runtime)
    pub fn initialize_sync(&self, force_reload: bool) -> Result<()> {
        // Create a new Tokio runtime for this operation
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| crate::error::IconCacheError::RuntimeError(
                format!("Failed to create Tokio runtime: {}", e)
            ))?;
        
        // Block on the async initialization
        runtime.block_on(self.initialize(force_reload))
    }
    
    /// Set module HAKs synchronously
    pub fn set_module_haks_sync(&self, hak_list: Vec<String>) -> Result<()> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| crate::error::IconCacheError::RuntimeError(
                format!("Failed to create Tokio runtime: {}", e)
            ))?;
        
        runtime.block_on(self.set_module_haks(hak_list))
    }
    
    /// Load module icons synchronously
    pub fn load_module_icons_sync(&self, module_path: &Path) -> Result<()> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| crate::error::IconCacheError::RuntimeError(
                format!("Failed to create Tokio runtime: {}", e)
            ))?;
        
        runtime.block_on(self.load_module_icons(module_path))
    }
    
    /// Load entire cache from binary file
    async fn load_from_persistent_cache(&self) -> Result<()> {
        let cache_data = self.disk_cache.load().await?;
        
        // Direct assignment - the disk cache handles all the deserialization
        self.cache.clear();
        for (key, icon) in cache_data.cache {
            self.cache.insert(key, icon);
        }
        
        // Update interner - this is safe because we're replacing the entire state
        // In a real implementation, we'd need to be more careful about concurrent access
        // For now, this is a simplified version
        
        Ok(())
    }
    
    /// Full rebuild: scan, process, flatten hierarchy, save cache
    async fn full_rebuild_and_cache(&self) -> Result<()> {
        // 1. Discover all icon sources
        log::info!("Starting full icon cache rebuild...");
        println!("Starting full rebuild...");
        let sources = self.discover_all_sources().await?;
        log::info!("Discovered {} icon sources", sources.len());
        println!("Discovered {} icon sources", sources.len());
        
        // 2. Load with flattened hierarchy (priority-based overwrites)
        self.load_sources_flattened(sources).await?;
        
        // 3. Save to persistent cache for next startup
        if self.config.enable_persistence {
            self.save_persistent_cache().await?;
        }
        
        log::info!("Icon cache rebuild complete. Total icons: {}", self.cache.len());
        println!("Icon cache rebuild complete. Total icons: {}", self.cache.len());
        Ok(())
    }
    
    /// Discover all icon sources
    async fn discover_all_sources(&self) -> Result<Vec<IconSource>> {
        let mut sources = Vec::new();
        
        // Base game icons
        if let Some(base_path) = self.get_base_game_path() {
            log::info!("Scanning base game icons at: {:?}", base_path);
            println!("Scanning base game path: {:?}", base_path);
            let base_sources = self.discovery.scan_directory(&base_path, SourceType::BaseGame).await?;
            log::info!("Found {} base game icon sources", base_sources.len());
            println!("Found {} base game sources", base_sources.len());
            if !base_sources.is_empty() {
                println!("First source has {} icons", base_sources[0].icons.len());
                if base_sources[0].icons.len() > 0 {
                    println!("First few icons: {:?}", &base_sources[0].icons[..5.min(base_sources[0].icons.len())]);
                }
            }
            sources.extend(base_sources);
        } else {
            log::warn!("No base game path found");
            println!("No base game path found!");
        }
        
        // Override directory
        if self.config.enable_overrides {
            if let Some(override_path) = self.get_override_path() {
                log::info!("Scanning override directory at: {:?}", override_path);
                let override_sources = self.discovery.scan_directory(&override_path, SourceType::Override).await?;
                log::info!("Found {} override icon sources", override_sources.len());
                sources.extend(override_sources);
            }
        }
        
        // Workshop icons
        if self.config.enable_workshop {
            for workshop_path in self.get_workshop_paths() {
                sources.extend(self.discovery.scan_directory(&workshop_path, SourceType::Workshop).await?);
            }
        }
        
        Ok(sources)
    }
    
    /// Load sources with flattened hierarchy
    async fn load_sources_flattened(&self, mut sources: Vec<IconSource>) -> Result<()> {
        // Sort by SourceType enum ordering (BaseGame < Override < Workshop < Hak < Module)
        sources.sort_by_key(|source| source.source_type);
        
        log::info!("Processing {} icon sources", sources.len());
        
        // Process in priority order, higher priority overwrites lower
        for source in sources {
            log::info!("Processing {:?} source with {} icons from {:?}", 
                      source.source_type, source.icons.len(), source.path);
            
            match self.process_source_parallel(source).await {
                Ok(icons) => {
                    log::info!("Processed {} icons successfully", icons.len());
                    
                    let align_icons: Vec<_> = icons.iter()
                        .filter(|(name, _)| name.contains("align"))
                        .map(|(name, _)| name.as_str())
                        .collect();
                    
                    if !align_icons.is_empty() {
                        log::info!("CACHE WRITE DEBUG: Processing {} alignment icons: {:?}", 
                                  align_icons.len(), align_icons);
                    }
                    
                    for (name, icon) in icons {
                        let key = self.interner.get_or_intern(name.clone());
                        self.cache.insert(key, icon); // Overwrites if exists = correct priority
                        
                        if name.contains("align") {
                            log::info!("CACHE WRITE DEBUG: Stored icon '{}' in cache with key {:?}", name, key);
                        }
                        
                        // Build filename mapping for fallback lookups
                        // Extract filename from full path (e.g., "evocation/spell/is_calllightning" -> "is_calllightning")
                        if let Some(filename) = name.split('/').last() {
                            if filename != name {  // Only add mapping if name contains directory
                                self.filename_map.insert(filename.to_string(), name.clone());
                                
                                if name.contains("align") {
                                    log::info!("CACHE WRITE DEBUG: Added filename mapping '{}' -> '{}'", filename, name);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to process source: {}", e);
                    // Continue with other sources instead of failing completely
                }
            }
        }
        
        // Update statistics
        self.update_statistics().await;
        
        Ok(())
    }
    
    /// Process a source in parallel
    async fn process_source_parallel(&self, source: IconSource) -> Result<Vec<(String, CachedIcon)>> {
        self.processor.process_source(source).await
    }
    
    /// Save entire cache to binary file atomically
    async fn save_persistent_cache(&self) -> Result<()> {
        // Create a snapshot of the current cache state
        let cache_snapshot: Vec<(IconKey, CachedIcon)> = 
            self.cache.iter()
                .map(|entry| (*entry.key(), entry.value().clone()))
                .collect();
        
        self.disk_cache.save(self.interner.clone(), cache_snapshot).await
    }
    
    /// Update statistics
    async fn update_statistics(&self) {
        let stats = self.get_statistics_sync();
        *self.stats.write().await = stats;
    }
    
    // Path helper methods (these would be configured based on the system)
    
    fn get_base_game_path(&self) -> Option<std::path::PathBuf> {
        self.nwn2_home.as_ref().map(|home| {
            // Try upscaled icons first, then fallback to regular icons
            let upscaled = home.join("ui").join("upscaled").join("icons");
            let default = home.join("ui").join("default").join("icons");
            
            log::info!("Checking for base game icons:");
            log::info!("  Upscaled path: {:?} (exists: {})", upscaled, upscaled.exists());
            log::info!("  Default path: {:?} (exists: {})", default, default.exists());
            
            if upscaled.exists() {
                log::info!("Using upscaled icons (BC7 format supported)");
                upscaled
            } else if default.exists() {
                default
            } else {
                // Check if maybe the icons are directly in the home folder
                log::warn!("Neither upscaled nor default icon paths exist, checking home directory");
                home.clone()
            }
        })
    }
    
    fn get_override_path(&self) -> Option<std::path::PathBuf> {
        self.nwn2_home.as_ref().map(|home| home.join("override"))
    }
    
    fn get_workshop_paths(&self) -> Vec<std::path::PathBuf> {
        // Steam Workshop support would go here
        // For now, return empty
        vec![]
    }
    
    /// Build the lazy cache index from scratch
    async fn build_lazy_cache_index(&self) -> Result<()> {
        log::info!("Building lazy cache index...");
        println!("Building lazy cache index...");
        
        // 1. Discover all icon sources
        let sources = self.discover_all_sources().await?;
        log::info!("Discovered {} icon sources", sources.len());
        
        // 2. Process all sources and build index
        let mut index_builder = IndexBuilder::new();
        let mut total_icons = 0;
        
        // Sort by source type priority
        let mut all_sources = sources;
        all_sources.sort_by_key(|s| s.source_type);
        
        for source in all_sources {
            log::info!("Processing {:?} source with {} icons", source.source_type, source.icons.len());
            
            match self.process_source_parallel(source).await {
                Ok(icons) => {
                    for (name, icon) in icons {
                        index_builder.add_icon(name, &icon);
                        total_icons += 1;
                        
                        // Show progress every 100 icons
                        if total_icons % 100 == 0 {
                            println!("Processed {} icons...", total_icons);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to process source: {}", e);
                }
            }
        }
        
        // 3. Save the index and data files
        index_builder.save(&self.config.cache_directory).await?;
        
        log::info!("Index built with {} icons", total_icons);
        println!("Index built successfully with {} icons", total_icons);
        
        // 4. Load the index we just built
        if let Some(lazy_cache) = &self.lazy_cache {
            lazy_cache.load_index().await?;
        }
        
        Ok(())
    }
}