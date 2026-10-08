use std::time::Instant;

use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::services::resource_manager::ResourceManager;

use super::constants::should_load_table;
use super::types::{GameData, LoadedTable, ProgressCallback};

/// Loads every table the editor reads through the resource manager, with the
/// base, English and custom TLKs it holds; `None` when it has no base TLK.
pub async fn load_game_data(
    resource_manager: &RwLock<ResourceManager>,
    progress: Option<ProgressCallback>,
) -> Option<GameData> {
    let report = |message: &str, percent: f32| {
        if let Some(callback) = &progress {
            callback(message, percent);
        }
    };
    let start_time = Instant::now();

    report("Scanning 2DA files...", 5.0);
    let (mut names, strings, english_strings, custom_strings) = {
        let rm = resource_manager.read().await;
        let strings = rm.get_tlk_parser()?;
        let names: Vec<String> = rm
            .get_available_2da_files()
            .into_iter()
            .map(|name| name.to_lowercase())
            .filter(|name| should_load_table(name))
            .collect();
        (
            names,
            strings,
            rm.get_english_tlk_parser(),
            rm.get_custom_tlk_parser(),
        )
    };
    names.sort_unstable();

    let mut game_data = GameData::new(strings);
    game_data.english_strings = english_strings;
    game_data.custom_strings = custom_strings;

    let total_tables = names.len();
    for (idx, name) in names.into_iter().enumerate() {
        if idx % 25 == 0 {
            let percent = 15.0 + (idx as f32 / total_tables as f32) * 70.0;
            report(&format!("Loading {name}..."), percent);
        }

        let parser = resource_manager.read().await.get_2da_with_overrides(&name);
        match parser {
            Ok(parser) => {
                game_data
                    .tables
                    .insert(name.clone(), LoadedTable::new(name, parser));
            }
            Err(e) => warn!("Failed to load table {}: {}", name, e),
        }

        if idx % 50 == 0 {
            tokio::task::yield_now().await;
        }
    }

    report("Loading complete", 100.0);
    info!(
        "Loaded {} tables in {:.1}ms",
        game_data.table_count(),
        start_time.elapsed().as_secs_f64() * 1000.0
    );

    Some(game_data)
}
