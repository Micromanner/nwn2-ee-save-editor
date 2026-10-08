use std::collections::HashMap;
use std::sync::Arc;

use ahash::AHashMap;

use crate::parsers::tda::TDAParser;
use crate::parsers::tlk::TLKParser;

#[derive(Debug, thiserror::Error)]
pub enum LoaderError {
    #[error("Parse error: {0}")]
    Parse(String),
}

pub type LoaderResult<T> = Result<T, LoaderError>;

pub struct LoadedTable {
    pub name: String,
    pub parser: Arc<TDAParser>,
}

impl LoadedTable {
    pub fn new(name: String, parser: Arc<TDAParser>) -> Self {
        Self { name, parser }
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
        self.parser.get_row_dict(usize::try_from(id).ok()?).ok()
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
}

impl GameData {
    pub fn new(strings: Arc<std::sync::RwLock<TLKParser>>) -> Self {
        Self {
            tables: HashMap::new(),
            strings,
            english_strings: None,
            custom_strings: None,
        }
    }

    pub fn get_table(&self, name: &str) -> Option<&LoadedTable> {
        self.tables.get(name)
    }

    pub fn table_count(&self) -> usize {
        self.tables.len()
    }

    pub fn clear(&mut self) {
        self.tables.clear();
        self.english_strings = None;
        self.custom_strings = None;
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

pub type ProgressCallback = Box<dyn Fn(&str, f32) + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::RwLock;

    fn tlk_with(text: &str) -> Arc<RwLock<TLKParser>> {
        Arc::new(RwLock::new(TLKParser::from_strings(&[text])))
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
