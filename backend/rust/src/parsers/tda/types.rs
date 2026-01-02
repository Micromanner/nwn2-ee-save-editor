use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use lasso::{Spur, ThreadedRodeo};

use super::error::{SecurityLimits, TDAError, TDAResult};

/// Symbol type for interned strings to save memory
pub type Symbol = Spur;

/// High-performance string interner for column names and repeated values
pub type TDAStringInterner = ThreadedRodeo;

/// Compact representation of a table cell value
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    /// Interned string for memory efficiency
    Interned(Symbol),
    /// Raw string for less common values
    Raw(String),
    /// Special marker for missing/null values (****)
    Null,
    /// Empty string value
    Empty,
}

impl CellValue {
    /// Create a new cell value, interning if beneficial
    pub fn new(value: &str, interner: &mut TDAStringInterner) -> Self {
        match value {
            "" => Self::Empty,
            "****" => Self::Null,
            _ => {
                // Intern strings that are likely to be repeated
                if value.len() <= 32 && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    Self::Interned(interner.get_or_intern(value))
                } else {
                    Self::Raw(value.to_string())
                }
            }
        }
    }

    /// Get the string value, resolving interned symbols
    pub fn as_str<'a>(&'a self, interner: &'a TDAStringInterner) -> Option<&'a str> {
        match self {
            Self::Interned(symbol) => Some(interner.resolve(symbol)),
            Self::Raw(string) => Some(string),
            Self::Null => None,
            Self::Empty => Some(""),
        }
    }

    /// Check if this is a null value (****)
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Check if this is empty
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

/// Optimized row storage using SmallVec to avoid heap allocation for small rows
pub type TDARow = SmallVec<[CellValue; 16]>;

/// Column metadata for fast lookups
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    /// Column name (interned)
    pub name: Symbol,
    /// Column index for O(1) access
    pub index: usize,
}

/// Main TDA parser structure optimized for memory efficiency and speed
#[derive(Debug)]
pub struct TDAParser {
    /// String interner for memory efficiency
    interner: TDAStringInterner,
    
    /// Column information with fast lookups
    columns: Vec<ColumnInfo>,
    
    /// Fast column name -> index mapping (case-insensitive)
    column_map: AHashMap<String, usize>,
    
    /// Row data using optimized storage
    rows: Vec<TDARow>,
    
    /// Security limits for safe parsing
    security_limits: SecurityLimits,
    
    /// Metadata about the parsed file
    metadata: TDAMetadata,
}

/// Metadata about the parsed 2DA file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TDAMetadata {
    /// Original file size in bytes
    pub file_size: usize,
    /// Number of lines processed
    pub line_count: usize,
    /// Parse time in nanoseconds
    pub parse_time_ns: u64,
    /// Whether the file had any format warnings
    pub has_warnings: bool,
    /// File format version detected
    pub format_version: String,
}

impl Default for TDAMetadata {
    fn default() -> Self {
        Self {
            file_size: 0,
            line_count: 0,
            parse_time_ns: 0,
            has_warnings: false,
            format_version: "2DA V2.0".to_string(),
        }
    }
}

impl TDAParser {
    /// Create a new parser with default security limits
    pub fn new() -> Self {
        Self::with_limits(SecurityLimits::default())
    }

    /// Create a new parser with custom security limits
    pub fn with_limits(limits: super::error::SecurityLimits) -> Self {
        Self {
            interner: TDAStringInterner::default(),
            columns: Vec::new(),
            column_map: AHashMap::new(),
            rows: Vec::new(),
            security_limits: limits,
            metadata: TDAMetadata::default(),
        }
    }

    /// Get the number of columns
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// Get the number of rows
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Get column names as strings
    pub fn column_names(&self) -> Vec<&str> {
        self.columns
            .iter()
            .map(|col| self.interner.resolve(&col.name))
            .collect()
    }

    /// Find column index by name (case-insensitive)
    pub fn find_column_index(&self, name: &str) -> Option<usize> {
        self.column_map.get(&name.to_lowercase()).copied()
    }

    /// Get a cell value by row and column indices
    pub fn get_cell(&self, row_index: usize, col_index: usize) -> TDAResult<Option<&str>> {
        let row = self.rows.get(row_index).ok_or(TDAError::RowIndexOutOfBounds {
            index: row_index,
            max: self.rows.len(),
        })?;

        let cell = row.get(col_index).ok_or(TDAError::ColumnIndexOutOfBounds {
            index: col_index,
            max: row.len(),
        })?;

        Ok(cell.as_str(&self.interner))
    }

    /// Get a cell value by row index and column name
    pub fn get_cell_by_name(&self, row_index: usize, column_name: &str) -> TDAResult<Option<&str>> {
        let col_index = self.find_column_index(column_name)
            .ok_or_else(|| TDAError::ColumnNotFound {
                column: column_name.to_string(),
            })?;
        
        self.get_cell(row_index, col_index)
    }

    /// Get an entire row as a map of column names to values
    pub fn get_row_dict(&self, row_index: usize) -> TDAResult<AHashMap<String, Option<String>>> {
        let row = self.rows.get(row_index).ok_or(TDAError::RowIndexOutOfBounds {
            index: row_index,
            max: self.rows.len(),
        })?;

        let mut result = AHashMap::with_capacity(self.columns.len());

        for (col_info, cell) in self.columns.iter().zip(row.iter()) {
            let col_name = self.interner.resolve(&col_info.name);
            let value = cell.as_str(&self.interner).map(|s| s.to_string());
            result.insert(col_name.to_string(), value);
        }

        Ok(result)
    }

    pub fn get_all_rows_dict(&self) -> Vec<AHashMap<String, Option<String>>> {
        let col_names: Vec<String> = self.columns.iter()
            .map(|col_info| self.interner.resolve(&col_info.name).to_string())
            .collect();

        self.rows.iter().map(|row| {
            let mut result = AHashMap::with_capacity(self.columns.len());
            for (col_name, cell) in col_names.iter().zip(row.iter()) {
                let value = cell.as_str(&self.interner).map(|s| s.to_string());
                result.insert(col_name.clone(), value);
            }
            result
        }).collect()
    }

    /// Find the first row where a column matches a value
    pub fn find_row(&self, column_name: &str, value: &str) -> TDAResult<Option<usize>> {
        let col_index = self.find_column_index(column_name)
            .ok_or_else(|| TDAError::ColumnNotFound {
                column: column_name.to_string(),
            })?;

        for (row_index, row) in self.rows.iter().enumerate() {
            if let Some(cell) = row.get(col_index) {
                if let Some(cell_value) = cell.as_str(&self.interner) {
                    if cell_value == value {
                        return Ok(Some(row_index));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Get parser metadata
    pub fn metadata(&self) -> &TDAMetadata {
        &self.metadata
    }

    /// Get mutable access to metadata for internal parser operations
    pub(crate) fn metadata_mut(&mut self) -> &mut TDAMetadata {
        &mut self.metadata
    }

    /// Get security limits
    pub fn security_limits(&self) -> &SecurityLimits {
        &self.security_limits
    }

    /// Get mutable access to security limits
    pub fn security_limits_mut(&mut self) -> &mut SecurityLimits {
        &mut self.security_limits
    }

    /// Get access to columns for internal parsing operations
    pub(crate) fn columns(&self) -> &Vec<ColumnInfo> {
        &self.columns
    }

    /// Get mutable access to columns for internal parsing operations
    pub(crate) fn columns_mut(&mut self) -> &mut Vec<ColumnInfo> {
        &mut self.columns
    }

    /// Get access to column map for internal parsing operations
    pub(crate) fn column_map_mut(&mut self) -> &mut AHashMap<String, usize> {
        &mut self.column_map
    }

    /// Get access to rows for internal parsing operations
    pub(crate) fn rows_mut(&mut self) -> &mut Vec<TDARow> {
        &mut self.rows
    }
    
    /// Get access to rows (read-only)
    pub(crate) fn rows(&self) -> &Vec<TDARow> {
        &self.rows
    }

    /// Get access to interner for internal parsing operations
    pub(crate) fn interner_mut(&mut self) -> &mut TDAStringInterner {
        &mut self.interner
    }
    
    /// Get access to interner (read-only)
    pub(crate) fn interner(&self) -> &TDAStringInterner {
        &self.interner
    }

    /// Clear all data and reset the parser
    pub fn clear(&mut self) {
        // Replace interner with a new one since it doesn't have a clear method
        self.interner = TDAStringInterner::default();
        self.columns.clear();
        self.column_map.clear();
        self.rows.clear();
        self.metadata = TDAMetadata::default();
    }

    /// Estimate memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        let interner_size = self.interner.len() * 32; // Rough estimate
        let columns_size = self.columns.len() * std::mem::size_of::<ColumnInfo>();
        let column_map_size = self.column_map.len() * (32 + 8); // Rough estimate
        let rows_size = self.rows.iter()
            .map(|row| row.len() * std::mem::size_of::<CellValue>())
            .sum::<usize>();
        
        interner_size + columns_size + column_map_size + rows_size
    }

    /// Iterator over all rows, yielding string values efficiently
    /// This is more performant than repeated get_cell calls for bulk operations
    pub fn iter_rows(&self) -> impl Iterator<Item = impl Iterator<Item = Option<&str>> + '_> + '_ {
        self.rows.iter().map(move |row| {
            row.iter().map(move |cell| cell.as_str(&self.interner))
        })
    }

    /// Iterator over a specific column by index
    /// Returns None for rows that don't have this column, preserving row alignment
    pub fn iter_column(&self, col_index: usize) -> impl Iterator<Item = Option<&str>> + '_ {
        self.rows.iter().map(move |row| {
            // Use .get() and .and_then() to chain the Options
            row.get(col_index)
               .and_then(|cell| cell.as_str(&self.interner))
        })
    }

    /// Iterator over a specific column by name
    pub fn iter_column_by_name(&self, column_name: &str) -> Option<impl Iterator<Item = Option<&str>> + '_> {
        let col_index = self.find_column_index(column_name)?;
        Some(self.iter_column(col_index))
    }
}

impl Default for TDAParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable representation of a cell value
/// Note: Empty cells are serialized as String("") for compatibility with Python consumers
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SerializableCellValue {
    /// Direct string value (includes empty strings)
    String(String),
    /// Null value marker (****)
    Null,
}

/// Serializable version of TDAParser for MessagePack storage
#[derive(Debug, Serialize, Deserialize)]
pub struct SerializableTDAParser {
    /// Column names
    pub column_names: Vec<String>,
    /// Row data with expanded strings
    pub rows: Vec<Vec<SerializableCellValue>>,
    /// Security limits
    pub security_limits: super::error::SecurityLimits,
    /// Metadata
    pub metadata: TDAMetadata,
}

impl SerializableTDAParser {
    /// Convert a regular TDAParser to serializable form
    pub fn from_parser(parser: &TDAParser) -> Self {
        let column_names = parser.column_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let rows = parser.rows().iter().map(|row| {
            row.iter().map(|cell| {
                match cell {
                    CellValue::Interned(symbol) => {
                        let s = parser.interner().resolve(symbol);
                        SerializableCellValue::String(s.to_string())
                    }
                    CellValue::Raw(s) => SerializableCellValue::String(s.clone()),
                    CellValue::Null => SerializableCellValue::Null,
                    CellValue::Empty => SerializableCellValue::String(String::new()),
                }
            }).collect()
        }).collect();
        
        Self {
            column_names,
            rows,
            security_limits: super::error::SecurityLimits::default(),
            metadata: parser.metadata().clone(),
        }
    }
    
    /// Convert back to a regular TDAParser
    pub fn to_parser(self) -> TDAParser {
        let mut parser = TDAParser::with_limits(self.security_limits);
        parser.metadata = self.metadata;
        
        // Rebuild columns
        for (idx, name) in self.column_names.into_iter().enumerate() {
            let symbol = parser.interner_mut().get_or_intern(&name);
            parser.columns_mut().push(ColumnInfo {
                name: symbol,
                index: idx,
            });
            parser.column_map_mut().insert(name.to_lowercase(), idx);
        }
        
        // Rebuild rows
        for row_data in self.rows {
            let mut row = TDARow::new();
            for cell in row_data {
                let cell_value = match cell {
                    SerializableCellValue::String(s) => {
                        CellValue::new(&s, parser.interner_mut())
                    }
                    SerializableCellValue::Null => CellValue::Null,
                };
                row.push(cell_value);
            }
            parser.rows_mut().push(row);
        }
        
        parser
    }
}