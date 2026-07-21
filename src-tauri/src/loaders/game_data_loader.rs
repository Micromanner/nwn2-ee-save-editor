use std::sync::Arc;

use ahash::AHashMap;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::parsers::tlk::TLKParser;
use crate::services::resource_manager::ResourceManager;

use super::data_model_loader::DataModelLoader;
use super::error::{LoaderError, LoaderResult};
use super::types::{GameData, LoadedTable, LoadingStats};

pub struct GameDataLoader {
    resource_manager: Arc<RwLock<ResourceManager>>,
    game_data: Option<GameData>,
    tlk_parser: Option<Arc<std::sync::RwLock<TLKParser>>>,
    is_ready: bool,
    is_priority_loaded: bool,
    initialization_error: Option<String>,
}

impl GameDataLoader {
    pub fn new(resource_manager: Arc<RwLock<ResourceManager>>) -> Self {
        Self {
            resource_manager,
            game_data: None,
            tlk_parser: None,
            is_ready: false,
            is_priority_loaded: false,
            initialization_error: None,
        }
    }

    pub async fn initialize(
        &mut self,
        tlk_parser: Arc<std::sync::RwLock<TLKParser>>,
        priority_only: bool,
    ) -> LoaderResult<()> {
        self.tlk_parser = Some(Arc::clone(&tlk_parser));

        let mut loader =
            DataModelLoader::with_options(Arc::clone(&self.resource_manager), priority_only);

        match loader.load_game_data(tlk_parser).await {
            Ok(data) => {
                self.game_data = Some(data);
                self.is_ready = true;
                self.is_priority_loaded = true;
                info!(
                    "GameDataLoader initialized with {} tables",
                    self.table_count()
                );
                Ok(())
            }
            Err(e) => {
                self.initialization_error = Some(e.to_string());
                Err(e)
            }
        }
    }

    pub async fn load_remaining_tables(&mut self) -> LoaderResult<bool> {
        if !self.is_priority_loaded {
            return Err(LoaderError::LoadingInterrupted(
                "Priority tables not loaded yet".into(),
            ));
        }

        let Some(tlk) = &self.tlk_parser else {
            return Err(LoaderError::LoadingInterrupted("TLK parser not set".into()));
        };

        let mut loader = DataModelLoader::with_options(Arc::clone(&self.resource_manager), false);

        match loader.load_game_data(Arc::clone(tlk)).await {
            Ok(new_data) => {
                if let Some(ref mut existing) = self.game_data {
                    for (name, table) in new_data.tables {
                        existing.tables.entry(name).or_insert(table);
                    }
                    info!(
                        "Loaded remaining tables, total now: {}",
                        existing.tables.len()
                    );
                } else {
                    self.game_data = Some(new_data);
                }
                Ok(true)
            }
            Err(e) => {
                warn!("Failed to load remaining tables: {}", e);
                Err(e)
            }
        }
    }

    pub fn get_table(&self, name: &str) -> Option<&LoadedTable> {
        self.game_data.as_ref()?.tables.get(name)
    }

    pub fn game_data(&self) -> Option<&GameData> {
        self.game_data.as_ref()
    }

    pub fn get_table_mut(&mut self, name: &str) -> Option<&mut LoadedTable> {
        self.game_data.as_mut()?.tables.get_mut(name)
    }

    pub fn get_by_id(&self, table_name: &str, id: i32) -> Option<AHashMap<String, Option<String>>> {
        let table = self.get_table(table_name)?;
        table.get_by_id(id)
    }

    pub fn get_row(
        &self,
        table_name: &str,
        row_index: usize,
    ) -> LoaderResult<AHashMap<String, Option<String>>> {
        let table = self
            .get_table(table_name)
            .ok_or_else(|| LoaderError::TableNotFound {
                name: table_name.to_string(),
            })?;
        table.get_row(row_index)
    }

    pub fn table_count(&self) -> usize {
        self.game_data.as_ref().map_or(0, |d| d.tables.len())
    }

    pub fn table_names(&self) -> Vec<String> {
        self.game_data
            .as_ref()
            .map_or_else(Vec::new, |d| d.tables.keys().cloned().collect())
    }

    pub fn get_string(&self, str_ref: i32) -> Option<String> {
        self.game_data.as_ref()?.get_string(str_ref)
    }

    pub fn get_stats(&self) -> LoadingStats {
        if let Some(ref data) = self.game_data {
            let total_rows: usize = data
                .tables
                .values()
                .map(super::types::LoadedTable::row_count)
                .sum();
            LoadingStats {
                tables_loaded: data.tables.len(),
                total_rows,
                load_time_ms: 0.0,
                priority_tables_loaded: data.priority_tables.len(),
            }
        } else {
            LoadingStats::default()
        }
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready && self.game_data.is_some()
    }

    pub fn is_priority_loaded(&self) -> bool {
        self.is_priority_loaded
    }

    pub fn initialization_error(&self) -> Option<&str> {
        self.initialization_error.as_deref()
    }

    pub async fn wait_for_ready(&self, timeout_ms: u64) -> bool {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);

        while !self.is_ready() {
            if start.elapsed() > timeout {
                return false;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        true
    }
}

impl Default for GameDataLoader {
    fn default() -> Self {
        panic!("GameDataLoader requires a ResourceManager. Use GameDataLoader::new() instead.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_not_ready_initially() {
        let rm = Arc::new(RwLock::new(ResourceManager::new(Arc::new(RwLock::new(
            crate::config::NWN2Paths::new(),
        )))));
        let loader = GameDataLoader::new(rm);
        assert!(!loader.is_ready());
        assert_eq!(loader.table_count(), 0);
    }
}
