//! Integration tests for standalone .bic loading and saving (vault import, issue #55).

use std::path::{Path, PathBuf};
use tempfile::TempDir;

use app_lib::character::Character;
use app_lib::state::standalone_bic;

fn fixture_bic() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/gff/occidiooctavon/occidiooctavon1.bic"
    ))
}

#[test]
fn load_fields_parses_fixture_bic() {
    let fields = standalone_bic::load_fields(&fixture_bic()).unwrap();
    assert!(!fields.is_empty());
    let character = Character::from_gff(fields);
    assert!(character.total_level() > 0);
}

#[test]
fn load_fields_rejects_non_bic_bytes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fake.bic");
    std::fs::write(&path, b"XXXX not a bic").unwrap();
    let err = standalone_bic::load_fields(&path).unwrap_err();
    assert!(err.contains("BIC"), "unexpected error: {err}");
}

#[test]
fn load_fields_errors_on_missing_file() {
    let err = standalone_bic::load_fields(Path::new("/nope/missing.bic")).unwrap_err();
    assert!(err.contains("Failed to read"), "unexpected error: {err}");
}

#[test]
fn save_round_trips_gold_change_and_creates_backup() {
    let dir = TempDir::new().unwrap();
    let bic_path = dir.path().join("Hero.bic");
    std::fs::copy(fixture_bic(), &bic_path).unwrap();

    let mut character = Character::from_gff(standalone_bic::load_fields(&bic_path).unwrap());
    let original_gold = character.gold();
    character.set_gold(original_gold + 111);

    standalone_bic::save(&bic_path, &character.clone_gff(), 3, true).unwrap();

    let reloaded = Character::from_gff(standalone_bic::load_fields(&bic_path).unwrap());
    assert_eq!(reloaded.gold(), original_gold + 111);

    let backup_dir = dir.path().join("backups").join("Hero");
    let backups: Vec<_> = std::fs::read_dir(&backup_dir).unwrap().flatten().collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(
        std::fs::read(backups[0].path()).unwrap(),
        std::fs::read(fixture_bic()).unwrap(),
        "backup must contain the pre-save bytes"
    );
}

#[test]
fn save_skips_backup_when_disabled() {
    let dir = TempDir::new().unwrap();
    let bic_path = dir.path().join("Hero.bic");
    std::fs::copy(fixture_bic(), &bic_path).unwrap();

    let character = Character::from_gff(standalone_bic::load_fields(&bic_path).unwrap());
    standalone_bic::save(&bic_path, &character.clone_gff(), 3, false).unwrap();

    assert!(!dir.path().join("backups").exists());
}

#[test]
fn save_prunes_old_backups_to_keep_count() {
    let dir = TempDir::new().unwrap();
    let bic_path = dir.path().join("Hero.bic");
    std::fs::copy(fixture_bic(), &bic_path).unwrap();

    let backup_dir = dir.path().join("backups").join("Hero");
    std::fs::create_dir_all(&backup_dir).unwrap();
    std::fs::write(backup_dir.join("backup_20200101_000000.bic"), b"old1").unwrap();
    std::fs::write(backup_dir.join("backup_20200102_000000.bic"), b"old2").unwrap();
    std::fs::write(backup_dir.join("backup_20200103_000000.bic"), b"old3").unwrap();

    let character = Character::from_gff(standalone_bic::load_fields(&bic_path).unwrap());
    standalone_bic::save(&bic_path, &character.clone_gff(), 3, true).unwrap();

    let mut names: Vec<String> = std::fs::read_dir(&backup_dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    assert_eq!(
        names.len(),
        3,
        "should keep exactly backup_keep_count files"
    );
    assert!(
        !names.contains(&"backup_20200101_000000.bic".to_string()),
        "oldest backup should be pruned"
    );
}

use std::sync::Arc;
use tokio::sync::RwLock;

use app_lib::config::NWN2Paths;
use app_lib::loaders::GameData;
use app_lib::parsers::tlk::TLKParser;
use app_lib::services::load_diagnostics::{LoadInput, LoadReport};
use app_lib::services::resource_manager::ResourceManager;
use app_lib::state::SessionState;
use app_lib::state::session_state::CharacterSource;

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

fn empty_game_data() -> GameData {
    GameData::new(Arc::new(std::sync::RwLock::new(TLKParser::default())))
}

#[test]
fn session_loads_standalone_bic() {
    let path = fixture_bic();
    let path_str = path.to_string_lossy().to_string();
    let mut session = new_session();
    let mut report = new_report(&path_str);

    session
        .load_character(&path_str, None, &mut report)
        .unwrap();

    assert!(session.character.is_some());
    assert!(session.savegame_handler.is_none());
    assert_eq!(session.character_source, CharacterSource::Standalone);
    assert_eq!(session.current_file_path.as_deref(), Some(path.as_path()));
}

#[test]
fn session_saves_standalone_bic_with_single_backup() {
    let dir = TempDir::new().unwrap();
    let bic_path = dir.path().join("Hero.bic");
    std::fs::copy(fixture_bic(), &bic_path).unwrap();
    let path_str = bic_path.to_string_lossy().to_string();

    let mut session = new_session();
    let mut report = new_report(&path_str);
    session
        .load_character(&path_str, None, &mut report)
        .unwrap();

    let game_data = empty_game_data();
    let gold = session.character.as_ref().unwrap().gold();

    session.character.as_mut().unwrap().set_gold(gold + 1);
    session.save_character(&game_data, 3).unwrap();

    session.character.as_mut().unwrap().set_gold(gold + 2);
    session.save_character(&game_data, 3).unwrap();

    let reloaded = Character::from_gff(standalone_bic::load_fields(&bic_path).unwrap());
    assert_eq!(reloaded.gold(), gold + 2);

    let backup_dir = dir.path().join("backups").join("Hero");
    let backups: Vec<_> = std::fs::read_dir(&backup_dir).unwrap().flatten().collect();
    assert_eq!(
        backups.len(),
        1,
        "only the first save per session creates a backup"
    );
}
