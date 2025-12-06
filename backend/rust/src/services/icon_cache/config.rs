//! Configuration for the icon cache

use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use super::types::ImageFormat;

/// Configuration for the icon cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconCacheConfig {
    // Performance tuning
    /// Maximum number of parallel jobs for processing
    pub max_parallel_jobs: usize,
    /// Image quality (0-100)
    pub image_quality: u8,
    
    // Format preferences
    /// Image format to use (always WebP for performance)
    pub image_format: ImageFormat,
    
    // Disk cache settings
    /// Directory to store the cache
    pub cache_directory: PathBuf,
    /// Enable persistent cache
    pub enable_persistence: bool,
    
    // Source hierarchy control
    /// Enable Steam Workshop icons
    pub enable_workshop: bool,
    /// Enable HAK file icons
    pub enable_haks: bool,
    /// Enable override directory icons
    pub enable_overrides: bool,
    
    // Cache versioning
    /// Force cache rebuild even if valid cache exists
    pub force_cache_rebuild: bool,
}

impl Default for IconCacheConfig {
    fn default() -> Self {
        Self {
            max_parallel_jobs: num_cpus::get(),
            image_quality: 85,
            image_format: ImageFormat::WebP,
            cache_directory: PathBuf::from("cache/icons"),
            enable_persistence: true,
            enable_workshop: true,
            enable_haks: true,
            enable_overrides: true,
            force_cache_rebuild: false,
        }
    }
}

impl IconCacheConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the cache directory
    pub fn with_cache_directory(mut self, dir: PathBuf) -> Self {
        self.cache_directory = dir;
        self
    }
    
    /// Set the image quality
    pub fn with_image_quality(mut self, quality: u8) -> Self {
        self.image_quality = quality.min(100);
        self
    }
    
    /// Force a cache rebuild
    pub fn with_force_rebuild(mut self) -> Self {
        self.force_cache_rebuild = true;
        self
    }
    
    /// Disable persistent caching (useful for testing)
    pub fn without_persistence(mut self) -> Self {
        self.enable_persistence = false;
        self
    }
}