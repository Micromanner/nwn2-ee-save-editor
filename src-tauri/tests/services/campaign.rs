use app_lib::config::NWN2Paths;
use app_lib::services::campaign::CampaignManager;
use app_lib::services::savegame_handler::SaveGameHandler;
// use std::fs;
use tempfile::TempDir;

#[path = "../common/mod.rs"]
mod common;

#[test]
fn test_campaign_manager_read_operations() {
    let fixture_path = common::fixtures_path().join("saves/Classic_Campaign");

    // Copy to temp dir to avoid SaveGameHandler creating backups in the fixture directory.
    let temp_dir = TempDir::new().unwrap();
    let save_path = temp_dir.path().join("Classic_Campaign");

    // Copy directory
    common::copy_dir_recursive(&fixture_path, &save_path).expect("Failed to copy fixture");

    let handler = SaveGameHandler::new(&save_path, true, false).expect("Failed to create handler");
    let paths = NWN2Paths::new(); // Default paths, likely mostly empty but should work for basic logic

    // Test get_summary
    let summary = CampaignManager::get_summary(&handler).expect("Failed to get summary");
    // Assert on some known values from Classic_Campaign if possible, or just presence
    assert!(
        summary.general_info.contains_key("game_act")
            || summary.general_info.contains_key("player_name")
    );

    // Test get_module_info
    let (info, vars) =
        CampaignManager::get_module_info(&handler, &paths).expect("Failed to get module info");
    assert!(!info.module_name.is_empty());
    assert!(!vars.integers.is_empty() || !vars.strings.is_empty() || !vars.floats.is_empty());

    // Test analyze_quest_progress
    let overview =
        CampaignManager::analyze_quest_progress(&handler).expect("Failed to analyze quests");
    assert!(overview.active_count + overview.completed_count > 0);
}

#[test]
fn test_campaign_manager_write_operations() {
    let fixture_path = common::fixtures_path().join("saves/Classic_Campaign");
    let temp_dir = TempDir::new().unwrap();
    let save_path = temp_dir.path().join("Classic_Campaign_Write");

    common::copy_dir_recursive(&fixture_path, &save_path).expect("Failed to copy fixture");

    let mut handler =
        SaveGameHandler::new(&save_path, true, false).expect("Failed to create handler");

    // Test update_global_int
    let test_var = "TEST_INT_VAR";
    let test_val = 12345;

    CampaignManager::update_global_int(&mut handler, test_var, test_val)
        .expect("Failed to update global int");

    // Verify update
    let xml_content = handler
        .extract_globals_xml()
        .expect("Failed to read globals");
    assert!(xml_content.contains(test_var));
    assert!(xml_content.contains(&test_val.to_string()));

    // Test update_global_string
    let test_str_var = "TEST_STR_VAR";
    let test_str_val = "TestValue";

    CampaignManager::update_global_string(&mut handler, test_str_var, test_str_val)
        .expect("Failed to update global string");

    let xml_content_2 = handler
        .extract_globals_xml()
        .expect("Failed to read globals 2");
    assert!(xml_content_2.contains(test_str_var));
    assert!(xml_content_2.contains(test_str_val));
}
