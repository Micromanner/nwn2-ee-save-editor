use std::collections::HashMap;
use std::sync::Arc;

use ahash::AHashMap;
use serde::{Deserialize, Serialize};

use crate::parsers::tda::TDAParser;
use crate::parsers::tlk::TLKParser;
use crate::services::RuleDetector;

use super::error::{LoaderError, LoaderResult};

pub struct LoadedTable {
    pub name: String,
    pub parser: Arc<TDAParser>,
    pub id_index: HashMap<i32, usize>,
}

impl LoadedTable {
    pub fn new(name: String, parser: Arc<TDAParser>) -> Self {
        let id_index = Self::build_id_index(&parser);
        Self {
            name,
            parser,
            id_index,
        }
    }

    fn build_id_index(parser: &TDAParser) -> HashMap<i32, usize> {
        let mut index = HashMap::new();
        for row_idx in 0..parser.row_count() {
            index.insert(row_idx as i32, row_idx);
        }
        index
    }

    pub fn row_count(&self) -> usize {
        self.parser.row_count()
    }

    pub fn column_names(&self) -> Vec<&str> {
        self.parser.column_names()
    }

    pub fn get_row(&self, row_index: usize) -> LoaderResult<AHashMap<String, Option<String>>> {
        self.parser.get_row_dict(row_index).map_err(|e| {
            LoaderError::Parse(format!(
                "Failed to get row {} from {}: {}",
                row_index, self.name, e
            ))
        })
    }

    pub fn get_by_id(&self, id: i32) -> Option<AHashMap<String, Option<String>>> {
        let row_index = self.id_index.get(&id)?;
        self.parser.get_row_dict(*row_index).ok()
    }

    pub fn get_cell(&self, row_index: usize, column: &str) -> LoaderResult<Option<String>> {
        let value = self
            .parser
            .get_cell_by_name(row_index, column)
            .map_err(|e| {
                LoaderError::Parse(format!(
                    "Failed to get cell {}.{} row {}: {}",
                    self.name, column, row_index, e
                ))
            })?;
        Ok(value.map(std::string::ToString::to_string))
    }

    pub fn find_column_index(&self, column: &str) -> Option<usize> {
        self.parser.find_column_index(column)
    }
}

pub struct GameData {
    pub tables: HashMap<String, LoadedTable>,
    pub strings: Arc<std::sync::RwLock<TLKParser>>,
    pub rule_detector: Option<RuleDetector>,
    pub relationships: ValidationReport,
    pub priority_tables: Vec<String>,
}

impl GameData {
    pub fn new(strings: Arc<std::sync::RwLock<TLKParser>>) -> Self {
        Self {
            tables: HashMap::new(),
            strings,
            rule_detector: None,
            relationships: ValidationReport::default(),
            priority_tables: Vec::new(),
        }
    }

    pub fn get_table(&self, name: &str) -> Option<&LoadedTable> {
        self.tables.get(name)
    }

    pub fn get_table_mut(&mut self, name: &str) -> Option<&mut LoadedTable> {
        self.tables.get_mut(name)
    }

    pub fn table_count(&self) -> usize {
        self.tables.len()
    }

    pub fn clear(&mut self) {
        self.tables.clear();
        self.rule_detector = None;
        self.relationships = ValidationReport::default();
        self.priority_tables.clear();
    }

    pub fn table_names(&self) -> impl Iterator<Item = &str> {
        self.tables.keys().map(std::string::String::as_str)
    }

    pub fn get_string(&self, str_ref: i32) -> Option<String> {
        if str_ref < 0 {
            return None;
        }
        let mut tlk = self.strings.write().ok()?;
        tlk.get_string(str_ref as usize).ok().flatten()
    }

    pub fn get_strings_batch(&self, str_refs: &[i32]) -> std::collections::HashMap<i32, String> {
        let mut out = std::collections::HashMap::with_capacity(str_refs.len());
        let Ok(mut tlk) = self.strings.write() else {
            return out;
        };
        for &str_ref in str_refs {
            if str_ref < 0 {
                continue;
            }
            if let Ok(Some(value)) = tlk.get_string(str_ref as usize) {
                out.insert(str_ref, value);
            }
        }
        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMetadata {
    pub name: String,
    pub row_count: usize,
    pub column_count: usize,
}

impl TableMetadata {
    pub fn new(name: String, row_count: usize, column_count: usize) -> Self {
        Self {
            name,
            row_count,
            column_count,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationReport {
    pub total_relationships: usize,
    pub valid_relationships: usize,
    pub broken_references: Vec<BrokenReference>,
    pub missing_tables: Vec<String>,
    pub dependency_order: Vec<String>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_broken_reference(&mut self, reference: BrokenReference) {
        self.broken_references.push(reference);
    }

    pub fn add_missing_table(&mut self, table: String) {
        if !self.missing_tables.contains(&table) {
            self.missing_tables.push(table);
        }
    }

    pub fn is_valid(&self) -> bool {
        self.broken_references.is_empty() && self.missing_tables.is_empty()
    }

    pub fn summary(&self) -> String {
        format!(
            "Validation: {}/{} relationships valid, {} broken references, {} missing tables",
            self.valid_relationships,
            self.total_relationships,
            self.broken_references.len(),
            self.missing_tables.len()
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokenReference {
    pub source_table: String,
    pub source_column: String,
    pub source_row: usize,
    pub target_table: String,
    pub target_id: i32,
    pub error: String,
}

impl BrokenReference {
    pub fn new(
        source_table: String,
        source_column: String,
        source_row: usize,
        target_table: String,
        target_id: i32,
    ) -> Self {
        let error = format!(
            "Row {source_row} in {source_table}.{source_column} references non-existent {target_table} ID {target_id}"
        );
        Self {
            source_table,
            source_column,
            source_row,
            target_table,
            target_id,
            error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationshipType {
    Lookup,
    TableReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelationshipDefinition {
    pub source_table: String,
    pub source_column: String,
    pub target_table: String,
    pub relationship_type: RelationshipType,
    pub is_nullable: bool,
}

impl RelationshipDefinition {
    pub fn new_lookup(source_table: String, source_column: String, target_table: String) -> Self {
        Self {
            source_table,
            source_column,
            target_table,
            relationship_type: RelationshipType::Lookup,
            is_nullable: true,
        }
    }

    pub fn new_table_reference(
        source_table: String,
        source_column: String,
        target_table: String,
    ) -> Self {
        Self {
            source_table,
            source_column,
            target_table,
            relationship_type: RelationshipType::TableReference,
            is_nullable: true,
        }
    }
}

pub type ProgressCallback = Box<dyn Fn(&str, f32) + Send + Sync>;

pub struct LoadingProgress {
    callback: Option<ProgressCallback>,
    current_message: String,
    current_percent: f32,
}

impl LoadingProgress {
    pub fn new(callback: Option<ProgressCallback>) -> Self {
        Self {
            callback,
            current_message: String::new(),
            current_percent: 0.0,
        }
    }

    pub fn update(&mut self, message: &str, percent: f32) {
        self.current_message = message.to_string();
        self.current_percent = percent;
        if let Some(ref callback) = self.callback {
            callback(message, percent);
        }
    }

    pub fn current_message(&self) -> &str {
        &self.current_message
    }

    pub fn current_percent(&self) -> f32 {
        self.current_percent
    }
}

impl Default for LoadingProgress {
    fn default() -> Self {
        Self::new(None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingStats {
    pub tables_loaded: usize,
    pub total_rows: usize,
    pub load_time_ms: f64,
    pub priority_tables_loaded: usize,
    pub relationships_detected: usize,
}

impl Default for LoadingStats {
    fn default() -> Self {
        Self {
            tables_loaded: 0,
            total_rows: 0,
            load_time_ms: 0.0,
            priority_tables_loaded: 0,
            relationships_detected: 0,
        }
    }
}
