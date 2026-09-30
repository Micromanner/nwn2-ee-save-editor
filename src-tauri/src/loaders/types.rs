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
    /// English base talk table when `strings` is another language.
    pub english_strings: Option<Arc<std::sync::RwLock<TLKParser>>>,
    /// The loaded save's module TLK (`Mod_CustomTlk`), if it has one.
    pub custom_strings: Option<Arc<std::sync::RwLock<TLKParser>>>,
    pub rule_detector: Option<RuleDetector>,
    pub priority_tables: Vec<String>,
}

impl GameData {
    pub fn new(strings: Arc<std::sync::RwLock<TLKParser>>) -> Self {
        Self {
            tables: HashMap::new(),
            strings,
            english_strings: None,
            custom_strings: None,
            rule_detector: None,
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
        self.english_strings = None;
        self.custom_strings = None;
        self.rule_detector = None;
        self.priority_tables.clear();
    }

    pub fn table_names(&self) -> impl Iterator<Item = &str> {
        self.tables.keys().map(std::string::String::as_str)
    }

    pub fn get_string(&self, str_ref: i32) -> Option<String> {
        crate::parsers::tlk::resolve_strref(
            Some(&self.strings),
            self.custom_strings.as_deref(),
            str_ref,
        )
    }

    /// A string as English text, for game text the editor parses (feat bonuses).
    pub fn get_english_string(&self, str_ref: i32) -> Option<String> {
        crate::parsers::tlk::resolve_strref(
            Some(self.english_strings.as_deref().unwrap_or(&self.strings)),
            self.custom_strings.as_deref(),
            str_ref,
        )
    }

    pub fn get_strings_batch(&self, str_refs: &[i32]) -> std::collections::HashMap<i32, String> {
        str_refs
            .iter()
            .filter_map(|&str_ref| Some((str_ref, self.get_string(str_ref)?)))
            .collect()
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoadingStats {
    pub tables_loaded: usize,
    pub total_rows: usize,
    pub load_time_ms: f64,
    pub priority_tables_loaded: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::tlk::TLKStringEntry;
    use crate::parsers::tlk::types::CachedString;
    use std::sync::RwLock;

    fn tlk_with(text: &str) -> Arc<RwLock<TLKParser>> {
        let mut tlk = TLKParser::default();
        tlk.entries.push(TLKStringEntry {
            flags: 1,
            sound_resref: None,
            volume_variance: 0,
            pitch_variance: 0,
            data_offset: 0,
            string_size: 0,
        });
        let symbol = tlk.interner.get_or_intern(text);
        tlk.string_cache.insert(
            0,
            CachedString {
                symbol,
                byte_length: 0,
            },
        );
        Arc::new(RwLock::new(tlk))
    }

    #[test]
    fn english_string_reads_english_tlk_when_base_is_localized() {
        let mut game_data = GameData::new(tlk_with("+2 Bonus auf Zähigkeitswürfe"));
        assert_eq!(
            game_data.get_english_string(0).as_deref(),
            Some("+2 Bonus auf Zähigkeitswürfe")
        );

        game_data.english_strings = Some(tlk_with("+2 bonus on Fortitude saving throws"));
        assert_eq!(
            game_data.get_english_string(0).as_deref(),
            Some("+2 bonus on Fortitude saving throws")
        );
        assert_eq!(
            game_data.get_string(0).as_deref(),
            Some("+2 Bonus auf Zähigkeitswürfe")
        );
    }
}
