use ahash::AHashMap;
use lasso::{Rodeo, Spur};
use smallvec::SmallVec;

use super::error::{SecurityLimits, TDAError, TDAResult};

pub type Symbol = Spur;
pub type TDAStringInterner = Rodeo;

#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Interned(Symbol),
    Raw(String),
    Null,
    Empty,
}

impl CellValue {
    pub fn new(value: &str, interner: &mut TDAStringInterner) -> Self {
        match value {
            "" => Self::Empty,
            "****" => Self::Null,
            _ => {
                if value.len() <= 32 && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    Self::Interned(interner.get_or_intern(value))
                } else {
                    Self::Raw(value.to_string())
                }
            }
        }
    }

    pub fn as_str<'a>(&'a self, interner: &'a TDAStringInterner) -> Option<&'a str> {
        match self {
            Self::Interned(symbol) => Some(interner.resolve(symbol)),
            Self::Raw(string) => Some(string),
            Self::Null => None,
            Self::Empty => Some(""),
        }
    }
}

pub type TDARow = SmallVec<[CellValue; 16]>;

#[derive(Debug, Default)]
pub struct TDAParser {
    pub(super) interner: TDAStringInterner,
    pub(super) columns: Vec<Symbol>,
    pub(super) column_map: AHashMap<String, usize>,
    pub(super) rows: Vec<TDARow>,
    pub(super) security_limits: SecurityLimits,
}

impl TDAParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_column(&mut self, name: &str) {
        let index = self.columns.len();
        let symbol = self.interner.get_or_intern(name);
        self.columns.push(symbol);
        self.column_map.insert(name.to_lowercase(), index);
    }

    #[cfg(test)]
    pub fn add_row(&mut self, values: AHashMap<String, Option<String>>) {
        let mut row = TDARow::new();
        for column in &self.columns {
            let col_name = self.interner.resolve(column);
            let val = values.get(col_name).and_then(|v| v.as_deref());
            let cell = match val {
                Some(s) => CellValue::new(s, &mut self.interner),
                None => CellValue::Null,
            };
            row.push(cell);
        }
        self.rows.push(row);
    }

    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_names(&self) -> Vec<&str> {
        self.columns
            .iter()
            .map(|col| self.interner.resolve(col))
            .collect()
    }

    pub fn find_column_index(&self, name: &str) -> Option<usize> {
        self.column_map.get(&name.to_lowercase()).copied()
    }

    pub fn get_cell(&self, row_index: usize, col_index: usize) -> TDAResult<Option<&str>> {
        let row = self
            .rows
            .get(row_index)
            .ok_or(TDAError::RowIndexOutOfBounds {
                index: row_index,
                max: self.rows.len(),
            })?;

        let cell = row.get(col_index).ok_or(TDAError::ColumnIndexOutOfBounds {
            index: col_index,
            max: row.len(),
        })?;

        Ok(cell.as_str(&self.interner))
    }

    pub fn get_cell_by_name(&self, row_index: usize, column_name: &str) -> TDAResult<Option<&str>> {
        let col_index =
            self.find_column_index(column_name)
                .ok_or_else(|| TDAError::ColumnNotFound {
                    column: column_name.to_string(),
                })?;

        self.get_cell(row_index, col_index)
    }

    pub fn get_row_dict(&self, row_index: usize) -> TDAResult<AHashMap<String, Option<String>>> {
        let row = self
            .rows
            .get(row_index)
            .ok_or(TDAError::RowIndexOutOfBounds {
                index: row_index,
                max: self.rows.len(),
            })?;

        let mut result = AHashMap::with_capacity(self.columns.len());

        for (column, cell) in self.columns.iter().zip(row.iter()) {
            let col_name = self.interner.resolve(column);
            let value = cell
                .as_str(&self.interner)
                .map(std::string::ToString::to_string);
            result.insert(col_name.to_lowercase(), value);
        }

        Ok(result)
    }

    pub fn get_all_rows_dict(&self) -> Vec<AHashMap<String, Option<String>>> {
        let col_names: Vec<String> = self
            .columns
            .iter()
            .map(|column| self.interner.resolve(column).to_lowercase())
            .collect();

        self.rows
            .iter()
            .map(|row| {
                let mut result = AHashMap::with_capacity(self.columns.len());
                for (col_name, cell) in col_names.iter().zip(row.iter()) {
                    let value = cell
                        .as_str(&self.interner)
                        .map(std::string::ToString::to_string);
                    result.insert(col_name.clone(), value);
                }
                result
            })
            .collect()
    }

    pub fn find_row(&self, column_name: &str, value: &str) -> TDAResult<Option<usize>> {
        let col_index =
            self.find_column_index(column_name)
                .ok_or_else(|| TDAError::ColumnNotFound {
                    column: column_name.to_string(),
                })?;

        for (row_index, row) in self.rows.iter().enumerate() {
            if let Some(cell) = row.get(col_index)
                && let Some(cell_value) = cell.as_str(&self.interner)
                && cell_value == value
            {
                return Ok(Some(row_index));
            }
        }

        Ok(None)
    }
}
