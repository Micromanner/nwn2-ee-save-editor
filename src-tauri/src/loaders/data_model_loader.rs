use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::parsers::tlk::TLKParser;
use crate::services::RuleDetector;
use crate::services::resource_manager::ResourceManager;

use super::constants::{PRIORITY_TABLES, is_priority_table, should_load_table};
use super::error::{LoaderError, LoaderResult};
use super::types::{GameData, LoadedTable, LoadingProgress, LoadingStats, TableMetadata};

type ProgressCallback = Box<dyn Fn(&str, f32) + Send + Sync>;

pub struct DataModelLoader {
    resource_manager: Arc<RwLock<ResourceManager>>,
    rule_detector: RuleDetector,
    progress: LoadingProgress,
    priority_only: bool,
}

impl DataModelLoader {
    pub fn new(resource_manager: Arc<RwLock<ResourceManager>>) -> Self {
        Self {
            resource_manager,
            rule_detector: RuleDetector::new(),
            progress: LoadingProgress::default(),
            priority_only: false,
        }
    }

    pub fn with_options(
        resource_manager: Arc<RwLock<ResourceManager>>,
        priority_only: bool,
    ) -> Self {
        Self {
            resource_manager,
            rule_detector: RuleDetector::new(),
            progress: LoadingProgress::default(),
            priority_only,
        }
    }

    pub fn set_progress_callback(&mut self, callback: ProgressCallback) {
        self.progress = LoadingProgress::new(Some(callback));
    }

    pub async fn load_game_data(
        &mut self,
        tlk_parser: Arc<std::sync::RwLock<TLKParser>>,
    ) -> LoaderResult<GameData> {
        let start_time = Instant::now();
        let mut stats = LoadingStats::default();

        self.progress.update("Scanning 2DA files...", 5.0);
        let tables_to_load = self.scan_2da_files().await?;

        self.progress.update("Sorting tables...", 10.0);
        let sorted_tables = Self::sort_priority_first(&tables_to_load);

        self.progress.update("Loading table data...", 15.0);
        let mut loaded_tables = HashMap::new();
        let total_tables = sorted_tables.len();

        for (idx, metadata) in sorted_tables.iter().enumerate() {
            let progress = 15.0 + (idx as f32 / total_tables as f32) * 70.0;
            if idx % 25 == 0 || is_priority_table(&metadata.name) {
                self.progress
                    .update(&format!("Loading {}...", metadata.name), progress);
            }

            match self.load_table(&metadata.name).await {
                Ok(table) => {
                    stats.total_rows += table.row_count();
                    if is_priority_table(&metadata.name) {
                        stats.priority_tables_loaded += 1;
                    }
                    loaded_tables.insert(metadata.name.clone(), table);
                }
                Err(e) => {
                    warn!("Failed to load table {}: {}", metadata.name, e);
                }
            }

            if idx % 50 == 0 {
                tokio::task::yield_now().await;
            }
        }

        stats.tables_loaded = loaded_tables.len();

        let mut game_data = GameData::new(tlk_parser);
        game_data.tables = loaded_tables;
        game_data.rule_detector = Some(self.rule_detector.clone());
        game_data.priority_tables = PRIORITY_TABLES.iter().map(|s| (*s).to_string()).collect();

        stats.load_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;
        self.progress.update("Loading complete", 100.0);

        info!(
            "Loaded {} tables ({} rows) in {:.1}ms",
            stats.tables_loaded, stats.total_rows, stats.load_time_ms
        );

        Ok(game_data)
    }

    async fn scan_2da_files(&self) -> LoaderResult<Vec<TableMetadata>> {
        let rm = self.resource_manager.read().await;

        let available_files = rm.get_available_2da_files();
        let mut tables = Vec::new();

        for name in available_files {
            let name_lower = name.to_lowercase().replace(".2da", "");

            if self.priority_only && !is_priority_table(&name_lower) {
                continue;
            }

            if !should_load_table(&name_lower) {
                continue;
            }

            match rm.get_2da_with_overrides(&name_lower) {
                Ok(parser) => {
                    tables.push(TableMetadata::new(
                        name_lower,
                        parser.row_count(),
                        parser.column_count(),
                    ));
                }
                Err(e) => {
                    debug!("Skipping {}: {}", name_lower, e);
                }
            }
        }

        info!("Found {} tables to load", tables.len());
        Ok(tables)
    }

    fn sort_priority_first(tables: &[TableMetadata]) -> Vec<TableMetadata> {
        let mut sorted: Vec<_> = tables.to_vec();
        sorted.sort_by(|a, b| {
            is_priority_table(&b.name)
                .cmp(&is_priority_table(&a.name))
                .then_with(|| a.name.cmp(&b.name))
        });
        sorted
    }

    async fn load_table(&self, name: &str) -> LoaderResult<LoadedTable> {
        let rm = self.resource_manager.read().await;

        let parser = rm
            .get_2da_with_overrides(name)
            .map_err(|e| LoaderError::Parse(format!("Failed to get 2DA {name}: {e}")))?;

        Ok(LoadedTable::new(name.to_string(), parser))
    }

    pub fn get_stats(&self) -> LoadingStats {
        LoadingStats::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_table_check() {
        assert!(is_priority_table("classes"));
        assert!(is_priority_table("feat"));
        assert!(!is_priority_table("cls_feat_fighter"));
    }
}
