//! Core types for the icon cache

use std::sync::Arc;
use serde::{Serialize, Deserialize};

/// Represents the source hierarchy for icon priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SourceType {
    /// Base game icons (lowest priority)
    BaseGame = 0,
    /// Traditional override directory
    Override = 1,
    /// Steam Workshop mods
    Workshop = 2,
    /// HAK files
    Hak = 3,
    /// Module icons (highest priority)
    Module = 4,
}

impl SourceType {
    /// Get a human-readable name for the source type
    pub fn name(&self) -> &'static str {
        match self {
            SourceType::BaseGame => "Base Game",
            SourceType::Override => "Override",
            SourceType::Workshop => "Workshop",
            SourceType::Hak => "HAK",
            SourceType::Module => "Module",
        }
    }
}

/// Supported image formats for conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    WebP,
}

impl ImageFormat {
    /// Get the MIME type for this format
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::WebP => "image/webp",
        }
    }
    
    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::WebP => "webp",
        }
    }
}

/// A cached icon with metadata
#[derive(Debug, Clone)]
pub struct CachedIcon {
    /// The icon data (shared ownership for duplicates)
    pub data: Arc<Vec<u8>>,
    /// The format of the icon data
    pub format: ImageFormat,
    /// The source type (for debugging/statistics)
    pub source_type: SourceType,
}

// Custom serialization for CachedIcon
impl serde::Serialize for CachedIcon {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("CachedIcon", 3)?;
        state.serialize_field("data", self.data.as_ref())?;
        state.serialize_field("format", &self.format)?;
        state.serialize_field("source_type", &self.source_type)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for CachedIcon {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CachedIconData {
            data: Vec<u8>,
            format: ImageFormat,
            source_type: SourceType,
        }
        
        let data = CachedIconData::deserialize(deserializer)?;
        Ok(CachedIcon {
            data: Arc::new(data.data),
            format: data.format,
            source_type: data.source_type,
        })
    }
}

impl CachedIcon {
    /// Create a new cached icon
    pub fn new(data: Vec<u8>, format: ImageFormat, source_type: SourceType) -> Self {
        Self {
            data: Arc::new(data),
            format,
            source_type,
        }
    }
    
    /// Get the size of the icon data in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Icon discovery result
#[derive(Debug, Clone)]
pub struct IconSource {
    /// Path to the source (directory or file)
    pub path: std::path::PathBuf,
    /// Type of source
    pub source_type: SourceType,
    /// Icon names found in this source
    pub icons: Vec<String>,
}

/// Statistics about the cache
#[derive(Debug, Clone, Default)]
pub struct CacheStatistics {
    /// Number of icons per source type
    pub source_counts: std::collections::HashMap<SourceType, usize>,
    /// Total number of icons
    pub total_icons: usize,
    /// Total memory usage in bytes
    pub memory_usage: usize,
    /// Cache hit rate (if tracking)
    pub cache_hit_rate: Option<f64>,
}

/// Input formats that can be converted
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    Tga,
    Dds,
    Png,
    Unknown,
}

impl InputFormat {
    /// Detect format from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "tga" => InputFormat::Tga,
            "dds" => InputFormat::Dds,
            "png" => InputFormat::Png,
            _ => InputFormat::Unknown,
        }
    }
}