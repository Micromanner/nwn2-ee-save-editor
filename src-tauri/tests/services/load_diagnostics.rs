//! Integration tests for the LoadReport lifecycle.

use app_lib::services::load_diagnostics::{LoadInput, LoadReport, LoadStage, LoadStatus};
use app_lib::services::resource_manager::ResourceManager;
use app_lib::state::SessionState;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;

use app_lib::config::NWN2Paths;

#[path = "../common/mod.rs"]
mod common;

fn new_session() -> SessionState {
    let paths = Arc::new(RwLock::new(NWN2Paths::new()));
    let rm = Arc::new(RwLock::new(ResourceManager::new(paths)));
    SessionState::new(rm)
}

fn new_report(path: &str) -> LoadReport {
    LoadReport::new(LoadInput {
        file_path: path.into(),
        player_index: None,
        file_size: std::fs::metadata(path).ok().map(|m| m.len()),
    })
}

#[test]
fn fatal_on_nonexistent_file() {
    let path = "/definitely/not/a/real/save.zip";
    let mut session = new_session();
    let mut report = new_report(path);

    let result = session.load_character(path, None, &mut report);
    assert!(result.is_err());
    assert_eq!(report.status, LoadStatus::Fatal);
    let fatal = report
        .fatal
        .as_ref()
        .expect("fatal field should be populated");
    assert_eq!(fatal.stage, LoadStage::SaveOpen);
}

#[test]
fn fatal_on_corrupted_save_directory() {
    // A directory containing garbage — not a real save. SaveGameHandler will fail
    // to extract playerlist.ifo or parse its contents.
    let dir = TempDir::new().unwrap();
    let fake = dir.path().join("fake_save");
    std::fs::create_dir(&fake).unwrap();
    std::fs::write(fake.join("playerlist.ifo"), b"not gff data").unwrap();

    let path_str = fake.to_string_lossy().to_string();
    let mut session = new_session();
    let mut report = new_report(&path_str);

    let result = session.load_character(&path_str, None, &mut report);
    assert!(result.is_err());
    assert_eq!(report.status, LoadStatus::Fatal);
    assert!(report.fatal.is_some());
}

#[test]
fn happy_path_with_classic_campaign_fixture() {
    let fixture = common::fixtures_path().join("saves/Classic_Campaign");
    if !fixture.exists() {
        eprintln!("skipping: fixture not present at {}", fixture.display());
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let save_path = temp_dir.path().join("Classic_Campaign");
    common::copy_dir_recursive(&fixture, &save_path).expect("copy fixture");

    let path_str = save_path.to_string_lossy().to_string();
    let mut session = new_session();
    let mut report = new_report(&path_str);

    let result = session.load_character(&path_str, None, &mut report);
    assert!(result.is_ok(), "valid save should load: {result:?}");
    assert_eq!(report.status, LoadStatus::Ok);
    assert!(report.warnings.is_empty());
    assert!(report.fatal.is_none());
}

#[test]
fn finalize_populates_timing_and_serializes() {
    let mut report = LoadReport::new(LoadInput {
        file_path: "test.zip".into(),
        player_index: None,
        file_size: None,
    });
    report.finalize();

    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["schema_version"], 1);
    assert!(json["finished_at"].is_string());
    assert!(json["duration_ms"].is_number());
}
