use ahash::AHashMap;
use memmap2::Mmap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::time::Instant;

use super::error::{TDAError, TDAResult, SecurityLimits};
use super::tokenizer::{TDATokenizer, Token};
use super::types::{CellValue, ColumnInfo, TDAParser, TDARow};

/// Core parser implementation with high-performance optimizations
impl TDAParser {
    /// Parse 2DA data from a byte slice with zero-copy optimizations where possible
    pub fn parse_from_bytes(&mut self, data: &[u8]) -> TDAResult<()> {
        let start_time = Instant::now();
        
        // Security validation
        self.security_limits().validate_file_size(data.len())?;
        
        // Clear existing data
        self.clear();
        
        // Convert to string (validates UTF-8)
        let content = std::str::from_utf8(data)
            .map_err(|e| TDAError::InvalidUtf8 { position: e.valid_up_to() })?;
            
        // Parse the content
        self.parse_content(content)?;
        
        // Update metadata
        self.metadata_mut().file_size = data.len();
        self.metadata_mut().parse_time_ns = start_time.elapsed().as_nanos() as u64;
        
        Ok(())
    }

    /// Parse 2DA data from a file path with memory mapping for large files
    pub fn parse_from_file<P: AsRef<Path>>(&mut self, path: P) -> TDAResult<()> {
        let file = File::open(&path)?;
        let metadata = file.metadata()?;
        let file_size = metadata.len() as usize;
        
        // Security validation
        self.security_limits().validate_file_size(file_size)?;
        
        if file_size > 64 * 1024 {
            // Use memory mapping for larger files
            self.parse_from_mmap(file)
        } else {
            // Read smaller files directly
            let mut content = String::new();
            let mut reader = BufReader::new(file);
            reader.read_to_string(&mut content)?;
            self.parse_from_bytes(content.as_bytes())
        }
    }

    /// Parse using memory-mapped file for optimal performance
    fn parse_from_mmap(&mut self, file: File) -> TDAResult<()> {
        let start_time = Instant::now();
        
        // Memory map the file
        let mmap = unsafe {
            Mmap::map(&file)
                .map_err(|e| TDAError::MemoryMapError {
                    details: e.to_string(),
                })?
        };

        // Clear existing data
        self.clear();
        
        // Convert to string slice
        let content = std::str::from_utf8(&mmap)
            .map_err(|e| TDAError::InvalidUtf8 { position: e.valid_up_to() })?;
            
        // Parse the content
        self.parse_content(content)?;
        
        // Update metadata
        self.metadata_mut().file_size = mmap.len();
        self.metadata_mut().parse_time_ns = start_time.elapsed().as_nanos() as u64;
        
        Ok(())
    }

    /// Core parsing logic for 2DA content
    fn parse_content(&mut self, content: &str) -> TDAResult<()> {
        let mut tokenizer = TDATokenizer::new();
        let mut header_parsed = false;
        let mut columns_parsed = false;
        let mut line_count = 0;

        for line in content.lines() {
            line_count += 1;

            // Validate line length
            self.security_limits().validate_line_length(line.len())?;

            // Tokenize the line
            let tokens = tokenizer.tokenize_line(line)?;

            // Skip empty lines and comments
            if tokens.is_empty() {
                continue;
            }

            if !header_parsed {
                // Position-based header parsing - no tokenization needed
                self.parse_header_direct(line.trim())?;
                header_parsed = true;
            } else if !columns_parsed {
                self.parse_columns(&tokens)?;
                // Validate column count immediately after parsing columns
                self.security_limits().validate_column_count(self.column_count())?;
                columns_parsed = true;
            } else {
                self.parse_data_row(&tokens)?;
                // Validate row count incrementally after each row
                self.security_limits().validate_row_count(self.row_count())?;
            }
        }
        
        self.metadata_mut().line_count = line_count;
        
        Ok(())
    }

    /// Parse the 2DA header line using position-based extraction
    /// Handles: "2DA V2.0", "2DA\tV2.0", "2DAMV1.0" (merge files)
    fn parse_header_direct(&mut self, line: &str) -> TDAResult<()> {
        let bytes = line.as_bytes();

        // Minimum header length: "2DA V2.0" = 8 chars, "2DAMV1.0" = 8 chars
        if bytes.len() < 8 {
            return Err(TDAError::InvalidHeader(line.to_string()));
        }

        // Check file type at position 0..3 or 0..4
        let is_2dam = bytes.len() >= 4 && &bytes[0..4] == b"2DAM";
        let is_2da = &bytes[0..3] == b"2DA";

        if !is_2da && !is_2dam {
            return Err(TDAError::InvalidHeader(line.to_string()));
        }

        // Extract version based on format
        let version = if is_2dam {
            // 2DAM format: "2DAMV1.0" - version starts at position 4
            if bytes.len() >= 8 {
                std::str::from_utf8(&bytes[4..8]).unwrap_or("")
            } else {
                ""
            }
        } else {
            // 2DA format: "2DA V2.0" or "2DA\tV2.0" - version starts at position 4
            // Skip the separator (space or tab) at position 3
            if bytes.len() >= 8 && (bytes[3] == b' ' || bytes[3] == b'\t') {
                std::str::from_utf8(&bytes[4..8]).unwrap_or("")
            } else {
                ""
            }
        };

        // Validate version
        if version != "V2.0" && version != "V1.0" {
            return Err(TDAError::InvalidHeader(line.to_string()));
        }

        // Set metadata
        if is_2dam {
            self.metadata_mut().format_version = format!("2DAM{}", version);
            self.metadata_mut().has_warnings = true; // 2DAM is non-standard
        } else {
            self.metadata_mut().format_version = format!("2DA {}", version);
        }

        Ok(())
    }

    /// Parse column headers
    fn parse_columns(&mut self, tokens: &[Token]) -> TDAResult<()> {
        if tokens.is_empty() {
            return Err(TDAError::MalformedLine {
                line_number: 2,
                details: "No column headers found".to_string(),
            });
        }
        
        // Skip the first token if it's empty (common in NWN2 2DA files)
        // This matches Python parser behavior for backward compatibility
        let column_tokens = if tokens.len() > 1 && tokens[0].content.is_empty() {
            &tokens[1..]
        } else {
            tokens
        };
        
        if column_tokens.is_empty() {
            return Err(TDAError::MalformedLine {
                line_number: 2,
                details: "No valid column headers found after skipping empty first column".to_string(),
            });
        }
        
        // Reserve capacity for performance
        self.columns_mut().reserve(column_tokens.len());
        self.column_map_mut().reserve(column_tokens.len());
        
        for (index, token) in column_tokens.iter().enumerate() {
            // Intern the column name
            let symbol = self.interner_mut().get_or_intern(token.content);
            let column_info = ColumnInfo {
                name: symbol,
                index,
            };
            
            self.columns_mut().push(column_info);
            
            // Add to case-insensitive lookup map
            self.column_map_mut().insert(token.content.to_lowercase(), index);
        }
        
        Ok(())
    }

    /// Parse a data row
    fn parse_data_row(&mut self, tokens: &[Token]) -> TDAResult<()> {
        if tokens.is_empty() {
            return Ok(()); // Skip empty rows
        }
        
        // Skip the first token (row index) and process the rest as data cells
        // This matches the Python parser behavior: data_tokens = tokens[1:]
        let data_tokens = if tokens.len() > 1 { &tokens[1..] } else { &[] };
        
        // Create row with appropriate capacity
        let mut row = TDARow::new();
        row.reserve(self.columns().len());
        
        // Process each data token
        for (col_index, token) in data_tokens.iter().enumerate() {
            if col_index >= self.column_count() {
                // Ignore extra tokens beyond column count
                break;
            }
            
            let cell_value = CellValue::new(token.content, self.interner_mut());
            row.push(cell_value);
        }
        
        // Pad with empty values if row has fewer tokens than columns
        while row.len() < self.column_count() {
            row.push(CellValue::Empty);
        }
        
        self.rows_mut().push(row);
        Ok(())
    }

    /// Parse data from a string (convenience method)
    pub fn parse_from_string(&mut self, data: &str) -> TDAResult<()> {
        self.parse_from_bytes(data.as_bytes())
    }

    /// Serialize to MessagePack with compression
    pub fn to_msgpack_compressed(&self) -> TDAResult<Vec<u8>> {
        use flate2::{write::ZlibEncoder, Compression};
        use std::io::Write;
        use super::types::SerializableTDAParser;
        
        // Convert to serializable form
        let serializable = SerializableTDAParser::from_parser(self);
        
        // Serialize to MessagePack
        let msgpack_data = rmp_serde::to_vec(&serializable)?;
        
        // Compress with zlib
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&msgpack_data)
            .map_err(|e| TDAError::CompressionError { details: e.to_string() })?;
        
        encoder.finish()
            .map_err(|e| TDAError::CompressionError { details: e.to_string() })
    }

    /// Deserialize from compressed MessagePack
    pub fn from_msgpack_compressed(data: &[u8]) -> TDAResult<Self> {
        use flate2::read::ZlibDecoder;
        use std::io::Read;
        use super::types::SerializableTDAParser;
        
        // Decompress
        let mut decoder = ZlibDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| TDAError::CompressionError { details: e.to_string() })?;
        
        // Deserialize from MessagePack
        let serializable: SerializableTDAParser = rmp_serde::from_slice(&decompressed)?;
        
        // Convert back to regular parser
        Ok(serializable.to_parser())
    }

    /// Load from cached MessagePack file if available, otherwise parse from source
    pub fn load_with_cache<P: AsRef<Path>>(
        &mut self,
        source_path: P,
        cache_path: Option<P>,
    ) -> TDAResult<bool> {
        // Try loading from cache first
        if let Some(ref cache_path) = cache_path {
            if let Ok(cache_data) = std::fs::read(cache_path) {
                if let Ok(cached_parser) = Self::from_msgpack_compressed(&cache_data) {
                    *self = cached_parser;
                    return Ok(true); // Loaded from cache
                }
            }
        }
        
        // Parse from source
        self.parse_from_file(source_path)?;
        
        // Save cache if path provided
        if let Some(cache_path) = cache_path {
            if let Ok(compressed_data) = self.to_msgpack_compressed() {
                // Propagate write errors instead of ignoring them
                std::fs::write(cache_path, compressed_data)?;
            }
        }
        
        Ok(false) // Parsed from source
    }

    /// Get parser statistics for debugging and optimization
    pub fn statistics(&self) -> ParserStatistics {
        ParserStatistics {
            total_cells: self.rows().len() * self.columns().len(),
            memory_usage: self.memory_usage(),
            interned_strings: self.interner().len(),
            parse_time_ms: self.metadata().parse_time_ns as f64 / 1_000_000.0,
            compression_ratio: if self.metadata().file_size > 0 {
                self.memory_usage() as f64 / self.metadata().file_size as f64
            } else {
                0.0
            },
        }
    }
}

/// Statistics about parser performance and memory usage
#[derive(Debug, Clone)]
pub struct ParserStatistics {
    pub total_cells: usize,
    pub memory_usage: usize,
    pub interned_strings: usize,
    pub parse_time_ms: f64,
    pub compression_ratio: f64,
}

/// Parallel loading function for multiple 2DA files
pub fn load_multiple_files<P: AsRef<Path> + Send + Sync>(
    file_paths: &[P],
    security_limits: Option<SecurityLimits>,
) -> TDAResult<AHashMap<String, TDAParser>> {
    use rayon::prelude::*;
    use std::collections::HashMap;
    
    let limits = security_limits.unwrap_or_default();
    
    let results: Result<HashMap<String, TDAParser>, TDAError> = file_paths
        .par_iter()
        .map(|path| {
            let path_str = path.as_ref().to_string_lossy().to_string();
            let mut parser = TDAParser::with_limits(limits.clone());
            
            parser.parse_from_file(path)
                .map(|_| (path_str, parser))
        })
        .collect();
    
    // Convert HashMap to AHashMap
    results.map(|hashmap| {
        let mut ahashmap = AHashMap::new();
        for (k, v) in hashmap {
            ahashmap.insert(k, v);
        }
        ahashmap
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_2DA: &str = r#"2DA V2.0

Label       Name        Description
0           test1       "Test Item 1"
1           test2       "Test Item 2"
2           ****        "Empty Label"
"#;

    #[test]
    fn test_basic_parsing() {
        let mut parser = TDAParser::new();
        parser.parse_from_string(SAMPLE_2DA).unwrap();
        
        assert_eq!(parser.column_count(), 3);
        assert_eq!(parser.row_count(), 3);
        
        let columns = parser.column_names();
        assert_eq!(columns, vec!["Label", "Name", "Description"]);
    }

    #[test]
    fn test_cell_access() {
        let mut parser = TDAParser::new();
        parser.parse_from_string(SAMPLE_2DA).unwrap();
        
        // Test by index - after row index fix, data starts at column 0
        assert_eq!(parser.get_cell(0, 0).unwrap(), Some("test1"));        // Label column data
        assert_eq!(parser.get_cell(0, 1).unwrap(), Some("Test Item 1"));  // Name column data
        assert_eq!(parser.get_cell(1, 1).unwrap(), Some("Test Item 2"));  // Name column data
        
        // Test by name - column names are as defined in header
        assert_eq!(parser.get_cell_by_name(0, "Label").unwrap(), Some("test1"));
        assert_eq!(parser.get_cell_by_name(0, "Name").unwrap(), Some("Test Item 1"));
        assert_eq!(parser.get_cell_by_name(2, "Label").unwrap(), None); // **** value
    }

    #[test]
    fn test_row_dict() {
        let mut parser = TDAParser::new();
        parser.parse_from_string(SAMPLE_2DA).unwrap();
        
        let row = parser.get_row_dict(0).unwrap();
        assert_eq!(row.get("Label"), Some(&Some("test1".to_string())));
        assert_eq!(row.get("Name"), Some(&Some("Test Item 1".to_string())));
        assert_eq!(row.get("Description"), Some(&Some("".to_string()))); // Empty string in sample data
    }

    #[test]
    fn test_find_row() {
        let mut parser = TDAParser::new();
        parser.parse_from_string(SAMPLE_2DA).unwrap();
        
        assert_eq!(parser.find_row("Label", "test1").unwrap(), Some(0));
        assert_eq!(parser.find_row("Label", "test2").unwrap(), Some(1));
        assert_eq!(parser.find_row("Name", "Test Item 1").unwrap(), Some(0));
        assert_eq!(parser.find_row("Name", "Test Item 2").unwrap(), Some(1));
        assert_eq!(parser.find_row("Name", "nonexistent").unwrap(), None);
    }

    #[test]
    fn test_security_limits() {
        let limits = SecurityLimits {
            max_file_size: 100,
            ..SecurityLimits::default()
        };
        
        let mut parser = TDAParser::with_limits(limits);
        let large_data = "x".repeat(200);
        
        assert!(parser.parse_from_string(&large_data).is_err());
    }
    
    #[test]
    fn test_serialization_roundtrip() {
        let mut parser = TDAParser::new();
        parser.parse_from_string(SAMPLE_2DA).unwrap();
        
        // Serialize to MessagePack
        let compressed = parser.to_msgpack_compressed().unwrap();
        
        // Deserialize back
        let restored = TDAParser::from_msgpack_compressed(&compressed).unwrap();
        
        // Verify the data is the same
        assert_eq!(restored.column_count(), parser.column_count());
        assert_eq!(restored.row_count(), parser.row_count());
        assert_eq!(restored.column_names(), parser.column_names());
        
        // Check some cell values
        assert_eq!(
            restored.get_cell_by_name(0, "Name").unwrap(),
            parser.get_cell_by_name(0, "Name").unwrap()
        );
        assert_eq!(
            restored.get_cell_by_name(2, "Name").unwrap(),
            parser.get_cell_by_name(2, "Name").unwrap()
        );
        
        // Check metadata is preserved
        assert_eq!(restored.metadata().format_version, parser.metadata().format_version);
    }
    
    #[test]
    fn test_row_index_ignored() {
        // Test that the first token (row index) is properly ignored
        let data_with_indices = r#"2DA V2.0

Label   Name        Description
0       item1       "First Item"
999     item2       "Second Item"
abc     item3       "Third Item"
"#;
        
        let mut parser = TDAParser::new();
        parser.parse_from_string(data_with_indices).unwrap();
        
        assert_eq!(parser.column_count(), 3);
        assert_eq!(parser.row_count(), 3);
        
        // The actual data should start from the second token, ignoring row indices
        assert_eq!(parser.get_cell_by_name(0, "Label").unwrap(), Some("item1"));
        assert_eq!(parser.get_cell_by_name(1, "Label").unwrap(), Some("item2")); 
        assert_eq!(parser.get_cell_by_name(2, "Label").unwrap(), Some("item3"));
        
        // Row indices (0, 999, abc) should not appear in the data
        assert_eq!(parser.get_cell_by_name(0, "Name").unwrap(), Some("First Item"));
        assert_eq!(parser.get_cell_by_name(1, "Name").unwrap(), Some("Second Item"));
        assert_eq!(parser.get_cell_by_name(2, "Name").unwrap(), Some("Third Item"));
    }

    #[test]
    fn test_tab_separated_header() {
        // Header with tab separator instead of space
        let data = "2DA\tV2.0\n\nLabel\tName\n0\ttest1\tvalue1\n";

        let mut parser = TDAParser::new();
        parser.parse_from_string(data).unwrap();

        assert_eq!(parser.metadata().format_version, "2DA V2.0");
        assert_eq!(parser.column_count(), 2);
        assert_eq!(parser.row_count(), 1);
    }

    #[test]
    fn test_2dam_merge_file() {
        // 2DAM merge file format (V1.0)
        let data = "2DAMV1.0\n\nLabel\tName\n0\ttest1\tvalue1\n";

        let mut parser = TDAParser::new();
        parser.parse_from_string(data).unwrap();

        assert_eq!(parser.metadata().format_version, "2DAMV1.0");
        assert!(parser.metadata().has_warnings); // 2DAM is non-standard
        assert_eq!(parser.column_count(), 2);
    }

    #[test]
    fn test_v1_format() {
        // V1.0 format support
        let data = "2DA V1.0\n\nLabel\tName\n0\ttest1\tvalue1\n";

        let mut parser = TDAParser::new();
        parser.parse_from_string(data).unwrap();

        assert_eq!(parser.metadata().format_version, "2DA V1.0");
    }

    #[test]
    fn test_tab_separated_with_quoted_commas() {
        // Real-world case: tab-separated with quoted fields containing commas
        let data = "2DA\tV2.0\n\nName\tComment\tResRefs\n0\tBeetles\t\",various beetles\"\t\"a,b,c\"\n";

        let mut parser = TDAParser::new();
        parser.parse_from_string(data).unwrap();

        assert_eq!(parser.column_count(), 3);
        assert_eq!(parser.get_cell_by_name(0, "Comment").unwrap(), Some(",various beetles"));
        assert_eq!(parser.get_cell_by_name(0, "ResRefs").unwrap(), Some("a,b,c"));
    }
}