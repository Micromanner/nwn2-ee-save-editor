use super::super::common::create_test_context;
use serde_json::json;
use std::collections::HashMap;
use tempfile::TempDir;

use app_lib::utils::{CacheBuilder, CacheManager};

#[tokio::test]
async fn test_cache_builder_with_real_table_structure() {
    let ctx = create_test_context().await;
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    if let Some(table) = ctx.game_data.get_table("classes") {
        let builder = CacheBuilder::new(temp_dir.path().to_string_lossy().to_string()).unwrap();

        let mut tables_data = HashMap::new();

        let mut table_info = HashMap::new();
        table_info.insert("section".to_string(), json!("base_game"));
        table_info.insert("data".to_string(), json!([1, 2, 3, 4, 5]));
        table_info.insert("row_count".to_string(), json!(table.row_count()));

        tables_data.insert("classes.2da".to_string(), table_info);

        let result = builder.build_cache(tables_data, "test_key".to_string());
        assert!(result.is_ok(), "Should build cache");

        let cache_dir = temp_dir.path().join("compiled_cache");
        assert!(cache_dir.join("cache_metadata.json").exists());
        assert!(cache_dir.join("base_game_cache.msgpack").exists());

        println!(
            "Cache built successfully with {} rows from classes.2da",
            table.row_count()
        );
    }
}

#[tokio::test]
async fn test_cache_key_generation_with_real_install() {
    let paths = app_lib::config::NWN2Paths::new();
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let builder = CacheBuilder::new(temp_dir.path().to_string_lossy().to_string()).unwrap();

    let mut mod_state = HashMap::new();

    if let Some(game_folder) = paths.game_folder() {
        mod_state.insert(
            "install_dir".to_string(),
            json!(game_folder.to_string_lossy().to_string()),
        );
    }

    if let Some(override_dir) = paths.override_dir()
        && override_dir.exists()
    {
        let files: Vec<String> = std::fs::read_dir(&override_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| e.file_name().into_string().ok())
                    .take(10)
                    .collect()
            })
            .unwrap_or_default();

        mod_state.insert("override_files".to_string(), json!(files));
    }

    let key1 = builder.generate_cache_key(mod_state.clone()).unwrap();
    let key2 = builder.generate_cache_key(mod_state).unwrap();

    assert_eq!(key1, key2, "Same input should produce same key");
    assert!(!key1.is_empty(), "Key should not be empty");

    println!("Generated cache key: {key1}");
}

#[tokio::test]
async fn test_cache_manager_with_built_cache() {
    let ctx = create_test_context().await;
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache_path = temp_dir.path().to_string_lossy().to_string();

    if let Some(classes_table) = ctx.game_data.get_table("classes") {
        let builder = CacheBuilder::new(cache_path.clone()).unwrap();

        let test_data: Vec<u8> = (0..100).collect();

        let mut tables_data = HashMap::new();
        let mut table_info = HashMap::new();
        table_info.insert("section".to_string(), json!("base_game"));
        table_info.insert("data".to_string(), json!(test_data.clone()));
        table_info.insert("row_count".to_string(), json!(classes_table.row_count()));
        tables_data.insert("classes.2da".to_string(), table_info);

        builder
            .build_cache(tables_data, "test_cache_key".to_string())
            .unwrap();

        let mut manager = CacheManager::new(cache_path).unwrap();

        assert!(manager.is_cache_valid().unwrap(), "Cache should be valid");

        let data = manager.get_table_data("classes".to_string()).unwrap();
        assert!(data.is_some(), "Should retrieve cached data");
        assert_eq!(data.unwrap(), test_data, "Data should match");

        println!("Successfully cached and retrieved classes.2da data");
    }
}

#[test]
fn test_cache_key_validation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache_path = temp_dir.path().to_string_lossy().to_string();

    let builder = CacheBuilder::new(cache_path.clone()).unwrap();

    let mut tables_data = HashMap::new();
    let mut table_info = HashMap::new();
    table_info.insert("section".to_string(), json!("base_game"));
    table_info.insert("data".to_string(), json!([1, 2, 3]));
    table_info.insert("row_count".to_string(), json!(1));
    tables_data.insert("test.2da".to_string(), table_info);

    builder
        .build_cache(tables_data, "original_cache_key".to_string())
        .unwrap();

    let mut manager = CacheManager::new(cache_path).unwrap();

    assert!(
        manager
            .validate_cache_key("original_cache_key".to_string())
            .unwrap(),
        "Should validate correct key"
    );
    assert!(
        !manager.validate_cache_key("wrong_key".to_string()).unwrap(),
        "Should reject wrong key"
    );
}

#[test]
fn test_cache_invalidation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache_path = temp_dir.path().to_string_lossy().to_string();

    let builder = CacheBuilder::new(cache_path.clone()).unwrap();

    let mut tables_data = HashMap::new();
    let mut table_info = HashMap::new();
    table_info.insert("section".to_string(), json!("base_game"));
    table_info.insert("data".to_string(), json!([1, 2, 3]));
    table_info.insert("row_count".to_string(), json!(1));
    tables_data.insert("test.2da".to_string(), table_info);

    builder.build_cache(tables_data, "key".to_string()).unwrap();

    let mut manager = CacheManager::new(cache_path).unwrap();

    assert!(manager.is_cache_valid().unwrap());

    manager.invalidate_cache();

    let stats = manager.get_cache_stats();
    assert_eq!(
        stats.get("loaded_sections").and_then(|v| v.as_u64()),
        Some(0)
    );

    println!("Cache invalidated successfully");
}

#[tokio::test]
async fn test_cache_stats_with_real_data() {
    let ctx = create_test_context().await;
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache_path = temp_dir.path().to_string_lossy().to_string();

    let builder = CacheBuilder::new(cache_path.clone()).unwrap();

    let mut tables_data = HashMap::new();

    for table_name in ["classes", "feat", "skills"] {
        if ctx.game_data.get_table(table_name).is_some() {
            let data: Vec<u8> = (0..50).collect();

            let mut table_info = HashMap::new();
            table_info.insert("section".to_string(), json!("base_game"));
            table_info.insert("data".to_string(), json!(data));
            table_info.insert("row_count".to_string(), json!(50));
            tables_data.insert(format!("{table_name}.2da"), table_info);
        }
    }

    builder
        .build_cache(tables_data, "multi_table_key".to_string())
        .unwrap();

    let mut manager = CacheManager::new(cache_path).unwrap();

    let _ = manager.get_table_data("classes".to_string());

    let stats = manager.get_cache_stats();

    println!("=== Cache Stats ===");
    for (key, value) in &stats {
        println!("  {key}: {value}");
    }

    assert!(stats.contains_key("valid"));
    assert!(stats.contains_key("cache_key"));
}

#[test]
fn test_multiple_cache_sections() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache_path = temp_dir.path().to_string_lossy().to_string();

    let builder = CacheBuilder::new(cache_path.clone()).unwrap();

    let mut tables_data = HashMap::new();

    let mut base_info = HashMap::new();
    base_info.insert("section".to_string(), json!("base_game"));
    base_info.insert("data".to_string(), json!([1, 2, 3]));
    base_info.insert("row_count".to_string(), json!(1));
    tables_data.insert("base_table.2da".to_string(), base_info);

    let mut workshop_info = HashMap::new();
    workshop_info.insert("section".to_string(), json!("workshop"));
    workshop_info.insert("data".to_string(), json!([4, 5, 6]));
    workshop_info.insert("row_count".to_string(), json!(1));
    tables_data.insert("workshop_table.2da".to_string(), workshop_info);

    let mut override_info = HashMap::new();
    override_info.insert("section".to_string(), json!("override"));
    override_info.insert("data".to_string(), json!([7, 8, 9]));
    override_info.insert("row_count".to_string(), json!(1));
    tables_data.insert("override_table.2da".to_string(), override_info);

    builder
        .build_cache(tables_data, "sections_key".to_string())
        .unwrap();

    let cache_dir = temp_dir.path().join("compiled_cache");
    assert!(cache_dir.join("base_game_cache.msgpack").exists());
    assert!(cache_dir.join("workshop_cache.msgpack").exists());
    assert!(cache_dir.join("override_cache.msgpack").exists());

    let mut manager = CacheManager::new(cache_path).unwrap();

    let base_data = manager.get_table_data("base_table".to_string()).unwrap();
    assert_eq!(base_data, Some(vec![1, 2, 3]));

    let override_data = manager
        .get_table_data("override_table".to_string())
        .unwrap();
    assert_eq!(override_data, Some(vec![7, 8, 9]));

    println!("Multiple cache sections work correctly");
}

#[tokio::test]
async fn test_cache_with_real_row_counts() {
    let ctx = create_test_context().await;
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache_path = temp_dir.path().to_string_lossy().to_string();

    let builder = CacheBuilder::new(cache_path.clone()).unwrap();

    let mut tables_data = HashMap::new();

    if let Some(classes) = ctx.game_data.get_table("classes") {
        let mut info = HashMap::new();
        info.insert("section".to_string(), json!("base_game"));
        info.insert("data".to_string(), json!([1, 2, 3]));
        info.insert("row_count".to_string(), json!(classes.row_count()));
        tables_data.insert("classes.2da".to_string(), info);
    }

    if let Some(feats) = ctx.game_data.get_table("feat") {
        let mut info = HashMap::new();
        info.insert("section".to_string(), json!("base_game"));
        info.insert("data".to_string(), json!([4, 5, 6]));
        info.insert("row_count".to_string(), json!(feats.row_count()));
        tables_data.insert("feat.2da".to_string(), info);
    }

    let result = builder.build_cache(tables_data, "real_counts_key".to_string());
    assert!(result.is_ok());

    let manager = CacheManager::new(cache_path).unwrap();
    let stats = manager.get_cache_stats();

    println!("Cache stats with real row counts: {stats:?}");
}
