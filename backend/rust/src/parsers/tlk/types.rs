use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use lasso::{Spur, Rodeo, Key};

/// TLK file header information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TLKHeader {
    /// File type identifier (should be "TLK ")
    pub file_type: String,
    /// Version identifier (should be "V3.0")
    pub version: String,
    /// Language ID (0 = English, 1 = French, etc.)
    pub language_id: u32,
    /// Number of string entries in the file
    pub string_count: u32,
    /// Offset to the beginning of string data
    pub string_data_offset: u32,
}

/// Individual string table entry from TLK file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TLKStringEntry {
    /// Flags indicating if string is present (bit 0)
    pub flags: u32,
    /// Sound resource reference (16 bytes, null-terminated)
    pub sound_resref: Option<String>,
    /// Volume variance (unused in NWN2)
    pub volume_variance: u32,
    /// Pitch variance (unused in NWN2)
    pub pitch_variance: u32,
    /// Offset within string data section
    pub data_offset: u32,
    /// Size of string data in bytes
    pub string_size: u32,
}

impl TLKStringEntry {
    /// Check if this string entry is present
    pub fn is_present(&self) -> bool {
        (self.flags & 0x01) != 0
    }

    /// Get the end offset of this string's data
    pub fn data_end_offset(&self) -> u32 {
        self.data_offset + self.string_size
    }
}

/// Cached string data for fast access
#[derive(Debug, Clone)]
pub struct CachedString {
    /// Interned string symbol for memory efficiency
    pub symbol: Spur,
    /// Original byte length
    pub byte_length: u32,
}

/// Main TLK parser structure
#[derive(Debug)]
pub struct TLKParser {
    /// File header information
    pub header: Option<TLKHeader>,
    /// String table entries
    pub entries: Vec<TLKStringEntry>,
    /// Cached string data - HashMap for O(1) lookups
    pub string_cache: HashMap<usize, CachedString>,
    /// String interner for memory efficiency
    pub interner: Rodeo,
    /// Raw string data (loaded once, indexed by entries)
    pub string_data: Vec<u8>,
    /// Security limits
    pub security_limits: super::error::SecurityLimits,
    /// Parser statistics
    pub stats: ParserStatistics,
    /// File metadata
    pub metadata: FileMetadata,
}

/// Statistics about parser performance and memory usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserStatistics {
    /// Total number of strings processed
    pub total_strings: usize,
    /// Memory usage in bytes
    pub memory_usage: usize,
    /// Number of interned strings
    pub interned_strings: usize,
    /// Parse time in milliseconds
    pub parse_time_ms: f64,
    /// Cache hit ratio (0.0 to 1.0)
    pub cache_hit_ratio: f64,
    /// Compression ratio for cached data
    pub compression_ratio: f64,
    /// Number of corrupted string entries encountered
    pub corrupted_entries: usize,
}

impl Default for ParserStatistics {
    fn default() -> Self {
        Self {
            total_strings: 0,
            memory_usage: 0,
            interned_strings: 0,
            parse_time_ms: 0.0,
            cache_hit_ratio: 0.0,
            compression_ratio: 0.0,
            corrupted_entries: 0,
        }
    }
}

/// File metadata for debugging and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// Original file size in bytes
    pub file_size: usize,
    /// Parse time in nanoseconds (high precision)
    pub parse_time_ns: u128,
    /// Whether parsing encountered warnings
    pub has_warnings: bool,
    /// TLK format version string
    pub format_version: String,
    /// Language identifier
    pub language_id: u32,
    /// File path (if loaded from file)
    pub file_path: Option<String>,
}

impl Default for FileMetadata {
    fn default() -> Self {
        Self {
            file_size: 0,
            parse_time_ns: 0,
            has_warnings: false,
            format_version: String::new(),
            language_id: 0,
            file_path: None,
        }
    }
}

/// Serializable version of TLKParser for caching
#[derive(Serialize, Deserialize)]
pub struct SerializableTLKParser {
    pub header: Option<TLKHeader>,
    pub entries: Vec<TLKStringEntry>,
    pub string_data: Vec<u8>,
    pub stats: ParserStatistics,
    pub metadata: FileMetadata,
    /// Serialized interner data
    pub interner_data: Vec<String>,
    /// String symbol mappings for cache (simplified as usize pairs)
    pub string_mappings: HashMap<usize, usize>,
}

impl Default for TLKParser {
    fn default() -> Self {
        Self::new()
    }
}

impl TLKParser {
    /// Create a new TLK parser with default settings
    pub fn new() -> Self {
        Self {
            header: None,
            entries: Vec::new(),
            string_cache: HashMap::new(),
            interner: Rodeo::default(),
            string_data: Vec::new(),
            security_limits: super::error::SecurityLimits::default(),
            stats: ParserStatistics::default(),
            metadata: FileMetadata::default(),
        }
    }

    /// Create a new TLK parser with custom security limits
    pub fn with_limits(limits: super::error::SecurityLimits) -> Self {
        Self {
            security_limits: limits,
            ..Self::new()
        }
    }

    /// Clear all parser state
    pub fn clear(&mut self) {
        self.header = None;
        self.entries.clear();
        self.string_cache.clear();
        self.interner = Rodeo::default();
        self.string_data.clear();
        self.stats = ParserStatistics::default();
        self.metadata = FileMetadata::default();
    }

    /// Get total number of strings
    pub fn string_count(&self) -> usize {
        self.entries.len()
    }

    /// Check if parser has loaded data
    pub fn is_loaded(&self) -> bool {
        self.header.is_some() && !self.entries.is_empty()
    }

    /// Get memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        let entries_size = self.entries.len() * std::mem::size_of::<TLKStringEntry>();
        let cache_size = self.string_cache.len() * (std::mem::size_of::<usize>() + std::mem::size_of::<CachedString>());
        let string_data_size = self.string_data.len();
        let interner_size = self.interner.len() * 32; // Estimate
        
        entries_size + cache_size + string_data_size + interner_size
    }

    /// Get parser statistics
    pub fn statistics(&self) -> &ParserStatistics {
        &self.stats
    }

    /// Get file metadata
    pub fn metadata(&self) -> &FileMetadata {
        &self.metadata
    }

    /// Get mutable reference to security limits
    pub fn security_limits_mut(&mut self) -> &mut super::error::SecurityLimits {
        &mut self.security_limits
    }

    /// Convert to serializable format for caching
    pub fn to_serializable(&self) -> SerializableTLKParser {
        // Extract interner data
        let interner_data: Vec<String> = self.interner.strings()
            .map(|s| s.to_string())
            .collect();

        // Create simplified string mappings (symbol as usize)
        let mut string_mappings = HashMap::new();
        for (index, cached) in &self.string_cache {
            string_mappings.insert(*index, cached.symbol.into_usize());
        }

        SerializableTLKParser {
            header: self.header.clone(),
            entries: self.entries.clone(),
            string_data: self.string_data.clone(),
            stats: self.stats.clone(),
            metadata: self.metadata.clone(),
            interner_data,
            string_mappings,
        }
    }

    /// Restore from serializable format
    pub fn from_serializable(data: SerializableTLKParser) -> Self {
        let mut parser = Self::new();
        parser.header = data.header;
        parser.entries = data.entries;
        parser.string_data = data.string_data;
        parser.stats = data.stats;
        parser.metadata = data.metadata;

        // Restore interner and build symbol lookup
        let mut symbol_lookup = Vec::new();
        for string_data in data.interner_data {
            let symbol = parser.interner.get_or_intern(&string_data);
            symbol_lookup.push(symbol);
        }

        // Restore string cache
        for (index, symbol_id) in data.string_mappings {
            if let Some(symbol) = symbol_lookup.get(symbol_id).copied() {
                if let Some(entry) = parser.entries.get(index) {
                    parser.string_cache.insert(index, CachedString {
                        symbol,
                        byte_length: entry.string_size,
                    });
                }
            }
        }

        parser
    }
}

/// Batch operation result for bulk string retrieval
#[derive(Debug, Clone)]
pub struct BatchStringResult {
    /// Successfully retrieved strings (str_ref -> string)
    pub strings: HashMap<usize, String>,
    /// Failed string references with error reasons
    pub errors: HashMap<usize, String>,
    /// Performance metrics for the batch operation
    pub metrics: BatchMetrics,
}

/// Performance metrics for batch operations
#[derive(Debug, Clone)]
pub struct BatchMetrics {
    /// Total time taken in milliseconds
    pub total_time_ms: f64,
    /// Number of cache hits
    pub cache_hits: usize,
    /// Number of cache misses requiring disk reads
    pub cache_misses: usize,
    /// Bytes read from string data
    pub bytes_read: usize,
}

/// Search result for string searching operations
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// String reference ID
    pub str_ref: usize,
    /// Matched string content
    pub content: String,
    /// Match score/relevance (0.0 to 1.0)
    pub score: f32,
}

/// Search options for string searching
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Case sensitive search
    pub case_sensitive: bool,
    /// Use regex pattern matching
    pub use_regex: bool,
    /// Maximum number of results to return
    pub max_results: usize,
    /// Minimum match score threshold
    pub min_score: f32,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            use_regex: false,
            max_results: 1000,
            min_score: 0.0,
        }
    }
}