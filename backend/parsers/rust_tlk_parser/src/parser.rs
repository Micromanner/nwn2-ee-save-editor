use crate::error::{TLKError, TLKResult, SecurityLimits};
use crate::types::{
    TLKParser, TLKHeader, TLKStringEntry, CachedString,
    BatchStringResult, BatchMetrics, SearchResult, SearchOptions, SerializableTLKParser
};
use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Cursor};
use std::fs::File;
use std::path::Path;
use std::time::Instant;
use rayon::prelude::*;
use flate2::{Compression, write::GzEncoder, read::GzDecoder};
use std::io::Write;

impl TLKParser {
    /// Parse TLK data from a file path
    pub fn parse_from_file<P: AsRef<Path>>(&mut self, path: P) -> TLKResult<()> {
        let path = path.as_ref();
        let start_time = Instant::now();
        
        // Store file path in metadata
        self.metadata.file_path = Some(path.to_string_lossy().to_string());
        
        // Open and validate file
        let mut file = File::open(path)?;
        let file_size = file.metadata()?.len() as usize;
        
        // Security check
        self.security_limits.validate_file_size(file_size)?;
        self.metadata.file_size = file_size;
        
        // Read entire file into memory for fast processing
        let mut buffer = Vec::with_capacity(file_size);
        file.read_to_end(&mut buffer)?;
        
        self.parse_from_bytes(&buffer)?;
        
        // Update timing
        let elapsed = start_time.elapsed();
        self.stats.parse_time_ms = elapsed.as_secs_f64() * 1000.0;
        self.metadata.parse_time_ns = elapsed.as_nanos();
        
        Ok(())
    }
    
    /// Parse TLK data from byte buffer
    pub fn parse_from_bytes(&mut self, data: &[u8]) -> TLKResult<()> {
        let start_time = Instant::now();
        
        // Clear existing state
        self.clear();
        
        // Minimum file size check
        if data.len() < 20 {
            return Err(TLKError::FileTooShort {
                expected: 20,
                actual: data.len(),
            });
        }
        
        let mut cursor = Cursor::new(data);
        
        // Parse header
        let header = self.parse_header(&mut cursor)?;
        self.security_limits.validate_string_count(header.string_count as usize)?;
        
        // Parse string table entries
        self.parse_string_entries(&mut cursor, header.string_count as usize)?;
        
        // Extract string data section
        let string_data_start = header.string_data_offset as usize;
        if string_data_start >= data.len() {
            return Err(TLKError::CorruptedStringEntry {
                index: 0,
                offset: header.string_data_offset,
                size: 0,
            });
        }
        
        self.string_data = data[string_data_start..].to_vec();
        self.header = Some(header);
        
        // Pre-cache frequently accessed strings (first 100)
        self.pre_cache_strings(100)?;
        
        // Update statistics
        let elapsed = start_time.elapsed();
        self.stats.parse_time_ms = elapsed.as_secs_f64() * 1000.0;
        self.stats.total_strings = self.entries.len();
        self.stats.memory_usage = self.memory_usage();
        self.stats.interned_strings = self.interner.len();
        
        // Update metadata
        self.metadata.parse_time_ns = elapsed.as_nanos();
        if let Some(ref header) = self.header {
            self.metadata.format_version = format!("{} {}", header.file_type.trim(), header.version.trim());
            self.metadata.language_id = header.language_id;
        }
        
        Ok(())
    }
    
    /// Parse TLK header (20 bytes)
    fn parse_header(&self, cursor: &mut Cursor<&[u8]>) -> TLKResult<TLKHeader> {
        // Read file type (4 bytes)
        let mut file_type_bytes = [0u8; 4];
        cursor.read_exact(&mut file_type_bytes)?;
        let file_type = String::from_utf8_lossy(&file_type_bytes).to_string();
        
        // Read version (4 bytes)
        let mut version_bytes = [0u8; 4];
        cursor.read_exact(&mut version_bytes)?;
        let version = String::from_utf8_lossy(&version_bytes).to_string();
        
        // Validate header
        if file_type != "TLK " || version != "V3.0" {
            return Err(TLKError::InvalidHeader {
                found: format!("{}{}", file_type, version),
            });
        }
        
        // Read remaining header fields
        let language_id = cursor.read_u32::<LittleEndian>()?;
        let string_count = cursor.read_u32::<LittleEndian>()?;
        let string_data_offset = cursor.read_u32::<LittleEndian>()?;
        
        Ok(TLKHeader {
            file_type,
            version,
            language_id,
            string_count,
            string_data_offset,
        })
    }
    
    /// Parse string table entries (40 bytes each)
    fn parse_string_entries(&mut self, cursor: &mut Cursor<&[u8]>, count: usize) -> TLKResult<()> {
        self.entries.reserve(count);
        
        for i in 0..count {
            let entry = self.parse_single_string_entry(cursor, i)?;
            self.entries.push(entry);
        }
        
        Ok(())
    }
    
    /// Parse a single string entry (40 bytes)
    fn parse_single_string_entry(&mut self, cursor: &mut Cursor<&[u8]>, _index: usize) -> TLKResult<TLKStringEntry> {
        // Check if we have enough data
        if cursor.position() + 40 > cursor.get_ref().len() as u64 {
            return Err(TLKError::FileTooShort {
                expected: cursor.position() as usize + 40,
                actual: cursor.get_ref().len(),
            });
        }
        
        let flags = cursor.read_u32::<LittleEndian>()?;
        
        // Read sound ResRef (16 bytes)
        let mut sound_resref_bytes = [0u8; 16];
        cursor.read_exact(&mut sound_resref_bytes)?;
        let sound_resref = extract_null_terminated_string(&sound_resref_bytes);
        
        let volume_variance = cursor.read_u32::<LittleEndian>()?;
        let pitch_variance = cursor.read_u32::<LittleEndian>()?;
        let data_offset = cursor.read_u32::<LittleEndian>()?;
        let string_size = cursor.read_u32::<LittleEndian>()?;
        
        // Validate string size
        self.security_limits.validate_string_size(string_size as usize)?;
        
        // Skip reserved bytes (4 bytes)
        cursor.seek(SeekFrom::Current(4))?;
        
        Ok(TLKStringEntry {
            flags,
            sound_resref,
            volume_variance,
            pitch_variance,
            data_offset,
            string_size,
        })
    }
    
    /// Pre-cache frequently accessed strings
    fn pre_cache_strings(&mut self, count: usize) -> TLKResult<()> {
        let cache_count = count.min(self.entries.len());
        
        for i in 0..cache_count {
            if let Some(string) = self.get_string_internal(i)? {
                let symbol = self.interner.get_or_intern(&string);
                self.string_cache.insert(i, CachedString {
                    symbol,
                    byte_length: self.entries[i].string_size,
                });
            }
        }
        
        Ok(())
    }
    
    /// Get a string by reference ID (main public method)
    pub fn get_string(&mut self, str_ref: usize) -> TLKResult<Option<String>> {
        // Check bounds
        if str_ref >= self.entries.len() {
            return Ok(None);
        }
        
        // Check cache first
        if let Some(cached) = self.string_cache.get(&str_ref) {
            if let Some(string) = self.interner.resolve(cached.symbol) {
                return Ok(Some(string.to_string()));
            }
        }
        
        // Load from string data
        let result = self.get_string_internal(str_ref)?;
        
        // Cache the result if successful
        if let Some(ref string) = result {
            let symbol = self.interner.get_or_intern(string);
            self.string_cache.insert(str_ref, CachedString {
                symbol,
                byte_length: self.entries[str_ref].string_size,
            });
        }
        
        Ok(result)
    }
    
    /// Internal string retrieval without caching
    fn get_string_internal(&self, str_ref: usize) -> TLKResult<Option<String>> {
        if str_ref >= self.entries.len() {
            return Ok(None);
        }
        
        let entry = &self.entries[str_ref];
        
        // Check if string is present
        if !entry.is_present() {
            return Ok(Some(String::new())); // Not present strings return empty string
        }
        
        // Check for zero-length strings
        if entry.string_size == 0 {
            return Ok(Some(String::new()));
        }
        
        // Validate bounds
        let start = entry.data_offset as usize;
        let end = start + entry.string_size as usize;
        
        if end > self.string_data.len() {
            // Note: We can't modify stats here as it's immutable, but that's okay for error reporting
            return Ok(None); // Corrupted entry
        }
        
        // Extract string bytes
        let string_bytes = &self.string_data[start..end];
        
        // Convert to UTF-8 string
        match String::from_utf8(string_bytes.to_vec()) {
            Ok(string) => Ok(Some(string)),
            Err(_e) => {
                // Try to recover with lossy conversion
                let string = String::from_utf8_lossy(string_bytes).to_string();
                // Note: We can't modify metadata here as it's immutable, but that's okay for error reporting
                Ok(Some(string))
            }
        }
    }
    
    /// Get multiple strings in one batch operation (high performance)
    pub fn get_strings_batch(&mut self, str_refs: &[usize]) -> TLKResult<BatchStringResult> {
        let start_time = Instant::now();
        let mut strings = HashMap::new();
        let mut errors = HashMap::new();
        let mut cache_hits = 0;
        let mut cache_misses = 0;
        let mut bytes_read = 0;
        
        for &str_ref in str_refs {
            match self.get_string(str_ref) {
                Ok(Some(string)) => {
                    strings.insert(str_ref, string);
                    if self.string_cache.contains_key(&str_ref) {
                        cache_hits += 1;
                    } else {
                        cache_misses += 1;
                        if str_ref < self.entries.len() {
                            bytes_read += self.entries[str_ref].string_size as usize;
                        }
                    }
                }
                Ok(None) => {
                    errors.insert(str_ref, "String not found".to_string());
                }
                Err(e) => {
                    errors.insert(str_ref, e.to_string());
                }
            }
        }
        
        let elapsed = start_time.elapsed();
        let metrics = BatchMetrics {
            total_time_ms: elapsed.as_secs_f64() * 1000.0,
            cache_hits,
            cache_misses,
            bytes_read,
        };
        
        Ok(BatchStringResult {
            strings,
            errors,
            metrics,
        })
    }
    
    /// Search for strings containing the given text
    pub fn search_strings(&mut self, search_text: &str, options: &SearchOptions) -> TLKResult<Vec<SearchResult>> {
        let mut results = Vec::new();
        let search_text_processed = if options.case_sensitive {
            search_text.to_string()
        } else {
            search_text.to_lowercase()
        };
        
        let entry_count = self.entries.len();
        for i in 0..entry_count {
            if results.len() >= options.max_results {
                break;
            }
            
            if let Ok(Some(content)) = self.get_string(i) {
                let content_processed = if options.case_sensitive {
                    content.clone()
                } else {
                    content.to_lowercase()
                };
                
                if content_processed.contains(&search_text_processed) {
                    let score = calculate_match_score(&content_processed, &search_text_processed);
                    if score >= options.min_score {
                        results.push(SearchResult {
                            str_ref: i,
                            content,
                            score,
                        });
                    }
                }
            }
        }
        
        // Sort by score (descending)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(results)
    }
    
    /// Load with MessagePack cache support
    pub fn load_with_cache<P: AsRef<Path>>(&mut self, source_path: P, cache_path: Option<P>) -> TLKResult<bool> {
        let source_path = source_path.as_ref();
        
        // Try to load from cache first
        if let Some(ref cache_path_ref) = cache_path {
            let cache_path = cache_path_ref.as_ref();
            if let Ok(cached_data) = self.load_from_cache(cache_path) {
                // Verify cache is newer than source
                if let Ok(source_metadata) = std::fs::metadata(source_path) {
                    if let Ok(cache_metadata) = std::fs::metadata(cache_path) {
                        if cache_metadata.modified().unwrap_or_else(|_| std::time::SystemTime::UNIX_EPOCH) 
                           >= source_metadata.modified().unwrap_or_else(|_| std::time::SystemTime::UNIX_EPOCH) {
                            *self = cached_data;
                            return Ok(true); // Loaded from cache
                        }
                    }
                }
            }
        }
        
        // Parse from source
        self.parse_from_file(source_path)?;
        
        // Save to cache if path provided
        if let Some(ref cache_path_ref) = cache_path {
            let cache_path = cache_path_ref.as_ref();
            let _ = self.save_to_cache(cache_path); // Ignore cache save errors
        }
        
        Ok(false) // Loaded from source
    }
    
    /// Save parser state to compressed MessagePack cache
    pub fn save_to_cache<P: AsRef<Path>>(&self, cache_path: P) -> TLKResult<()> {
        let serializable = self.to_serializable();
        let encoded = rmp_serde::to_vec(&serializable)?;
        
        // Compress the data
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&encoded)?;
        let compressed = encoder.finish()?;
        
        // Calculate compression ratio
        let _compression_ratio = compressed.len() as f64 / encoded.len() as f64;
        
        std::fs::write(cache_path, compressed)?;
        
        Ok(())
    }
    
    /// Load parser state from compressed MessagePack cache
    pub fn load_from_cache<P: AsRef<Path>>(&self, cache_path: P) -> TLKResult<TLKParser> {
        let compressed = std::fs::read(cache_path)?;
        let mut decoder = GzDecoder::new(&compressed[..]);
        let mut encoded = Vec::new();
        decoder.read_to_end(&mut encoded)?;
        
        let serializable: SerializableTLKParser = rmp_serde::from_slice(&encoded)?;
        Ok(TLKParser::from_serializable(serializable))
    }
    
    /// Get all strings in a range (for compatibility)
    pub fn get_all_strings(&mut self, start: usize, count: usize) -> TLKResult<HashMap<usize, String>> {
        let end = (start + count).min(self.entries.len());
        let str_refs: Vec<usize> = (start..end).collect();
        
        let batch_result = self.get_strings_batch(&str_refs)?;
        Ok(batch_result.strings)
    }
    
    /// Find first string containing the given value
    pub fn find_string(&mut self, search_text: &str) -> TLKResult<Option<usize>> {
        let options = SearchOptions {
            case_sensitive: false,
            max_results: 1,
            ..Default::default()
        };
        
        let results = self.search_strings(search_text, &options)?;
        Ok(results.first().map(|r| r.str_ref))
    }
    
    /// Get file information
    pub fn get_info(&self) -> HashMap<String, serde_json::Value> {
        let mut info = HashMap::new();
        
        if let Some(ref header) = self.header {
            info.insert("language_id".to_string(), serde_json::Value::Number(header.language_id.into()));
            info.insert("string_count".to_string(), serde_json::Value::Number(self.entries.len().into()));
        }
        
        info.insert("cache_size".to_string(), serde_json::Value::Number(self.string_cache.len().into()));
        info.insert("memory_usage".to_string(), serde_json::Value::Number(self.memory_usage().into()));
        
        if let Some(ref path) = self.metadata.file_path {
            info.insert("file_path".to_string(), serde_json::Value::String(path.clone()));
        }
        
        info.insert("file_size".to_string(), serde_json::Value::Number(self.metadata.file_size.into()));
        
        info
    }
}

/// Extract null-terminated string from byte array
fn extract_null_terminated_string(bytes: &[u8]) -> Option<String> {
    let null_pos = bytes.iter().position(|&b| b == 0)?;
    if null_pos == 0 {
        return None;
    }
    
    let string_bytes = &bytes[..null_pos];
    String::from_utf8(string_bytes.to_vec()).ok()
}

/// Calculate match score for search results
fn calculate_match_score(content: &str, search_text: &str) -> f32 {
    if content == search_text {
        return 1.0;
    }
    
    if content.starts_with(search_text) {
        return 0.9;
    }
    
    if content.ends_with(search_text) {
        return 0.8;
    }
    
    // Simple relevance based on search term frequency
    let matches = content.matches(search_text).count();
    let max_possible = content.len() / search_text.len();
    
    if max_possible == 0 {
        return 0.1;
    }
    
    (matches as f32 / max_possible as f32).min(0.7)
}

/// Parallel loading of multiple TLK files
pub fn load_multiple_files(paths: &[&str], limits: Option<SecurityLimits>) -> TLKResult<HashMap<String, TLKParser>> {
    let results: Result<Vec<_>, _> = paths.par_iter()
        .map(|&path| {
            let mut parser = if let Some(ref limits) = limits {
                TLKParser::with_limits(limits.clone())
            } else {
                TLKParser::new()
            };
            
            parser.parse_from_file(path).map(|_| (path.to_string(), parser))
        })
        .collect();
    
    match results {
        Ok(parsers) => Ok(parsers.into_iter().collect()),
        Err(e) => Err(e),
    }
}