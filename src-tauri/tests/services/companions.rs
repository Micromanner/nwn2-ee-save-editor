use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use app_lib::config::NWN2Paths;
use app_lib::services::ResourceManager;
use app_lib::state::session_state::{CharacterSource, SessionState};

fn saves_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/saves")
}

fn make_session() -> SessionState {
    let paths = Arc::new(RwLock::new(NWN2Paths::new()));
    SessionState::new(Arc::new(RwLock::new(ResourceManager::new(paths))))
}

fn load_save(session: &mut SessionState, save: &str) {
    let path = saves_dir().join(save);
    let mut report = app_lib::services::load_diagnostics::LoadReport::new(
        app_lib::services::load_diagnostics::LoadInput {
            file_path: path.to_string_lossy().to_string(),
            player_index: None,
            file_size: None,
        },
    );
    session
        .load_character(&path.to_string_lossy(), None, &mut report)
        .expect("load save");
}

// Fixture reality (verified with --nocapture): this save's globals.xml has no
// `*_Joined`/influence vars at all, so `list_roster` falls back to the
// roster.rst availability flags. Under that fallback, all 12 recruited
// companions (including shandra/zhjaeve) are marked available/campaign_npc,
// each with a matching .ros file; only the non-party NPCs npc_bevil and
// 0_amie are unavailable and excluded.
#[test]
fn list_roster_filters_party_members_with_files_original_format() {
    let mut session = make_session();
    load_save(&mut session, "000055 - SAVE GAME COMPLETE");

    let roster = session.list_roster().expect("list roster");
    let names: Vec<&str> = roster.iter().map(|r| r.ros_name.as_str()).collect();

    assert_eq!(roster.len(), 12);
    assert!(names.contains(&"khelgar"));
    assert!(names.contains(&"shandra"));
    assert!(names.contains(&"zhjaeve"));
    assert!(!names.contains(&"npc_bevil"), "not a party member");
    assert!(!names.contains(&"0_amie"), "not a party member");

    let khelgar = roster.iter().find(|r| r.ros_name == "khelgar").unwrap();
    assert_eq!(khelgar.char_name, "Khelgar Ironfist");
    assert_eq!(khelgar.classes, vec![(4, 2)]);
}

// Fixture reality (verified with --nocapture): this save's globals.xml only
// has `00_b*_Joined = 1` for Khelgar, Neeshka, Elanee and Qara — none of the
// other companions' joined_var (per get_companion_definitions, e.g.
// 00_bAmmon_Joined) is present, so ammon_jerro is not recruited even though
// his .ros file exists. `list_roster` now filters by the recruited set from
// globals.xml, so exactly those 4 are listed.
#[test]
fn list_roster_works_on_ee_zip_format() {
    let mut session = make_session();
    load_save(&mut session, "Classic_Campaign");

    let roster = session.list_roster().expect("list roster");
    let mut names: Vec<&str> = roster.iter().map(|r| r.ros_name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, vec!["elanee", "khelgar", "neeshka", "qara"]);
    assert!(
        !names.contains(&"ammon_jerro"),
        "ammon_jerro has no *_Joined var set in this save's globals.xml"
    );
}

#[test]
fn list_roster_empty_when_save_has_no_companions() {
    let mut session = make_session();
    load_save(&mut session, "Westgate_Campaign");
    assert!(session.list_roster().expect("list roster").is_empty());
}

#[test]
fn load_companion_swaps_session_character_and_source() {
    let mut session = make_session();
    load_save(&mut session, "Classic_Campaign");
    assert_eq!(session.character_source, CharacterSource::Player);

    session
        .load_companion("khelgar", false)
        .expect("load companion");
    assert_eq!(
        session.character_source,
        CharacterSource::Companion {
            ros_name: "khelgar".into()
        }
    );
    let character = session.character().expect("character loaded");
    assert!(character.total_level() >= 1);
    assert!(!session.has_unsaved_changes());
}

// Fixture reality (Classic_Campaign resgff.zip): bishop.ros stores
// CompanionName "Karnwyr" / CompanionType 1 (wolf) and carries no Familiar*
// fields at all, while qara.ros stores FamiliarName "Carel" / FamiliarType 4
// (weasel) and no Companion* fields - the game only writes the pair the
// creature actually uses.
#[test]
fn companion_pet_fields_read_from_ros() {
    let mut session = make_session();
    load_save(&mut session, "Classic_Campaign");

    session
        .load_companion("bishop", false)
        .expect("load bishop");
    let bishop = session.character().expect("character loaded");
    assert_eq!(bishop.animal_companion_name(), "Karnwyr");
    assert_eq!(bishop.animal_companion_type(), 1);
    assert_eq!(bishop.familiar_name(), "");
    assert_eq!(bishop.familiar_type(), -1);

    session.load_companion("qara", false).expect("load qara");
    let qara = session.character().expect("character loaded");
    assert_eq!(qara.familiar_name(), "Carel");
    assert_eq!(qara.familiar_type(), 4);
    assert_eq!(qara.animal_companion_type(), -1);
}

#[test]
fn load_companion_rejects_unknown_name() {
    let mut session = make_session();
    load_save(&mut session, "Classic_Campaign");
    assert!(session.load_companion("does_not_exist", false).is_err());
}

// Exercises the file-existence half of the filter: members flagged available
// in roster.rst are excluded when their .ros file is missing from the save.
#[test]
fn list_roster_excludes_available_members_without_ros_file() {
    let fixture = saves_dir().join("000055 - SAVE GAME COMPLETE");
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let save_path = temp_dir.path().join("000055 - SAVE GAME COMPLETE");
    crate::common::copy_dir_recursive(&fixture, &save_path).expect("copy fixture");
    std::fs::remove_file(save_path.join("shandra.ros")).expect("remove shandra.ros");
    std::fs::remove_file(save_path.join("zhjaeve.ros")).expect("remove zhjaeve.ros");

    let mut session = make_session();
    let mut report = app_lib::services::load_diagnostics::LoadReport::new(
        app_lib::services::load_diagnostics::LoadInput {
            file_path: save_path.to_string_lossy().to_string(),
            player_index: None,
            file_size: None,
        },
    );
    session
        .load_character(&save_path.to_string_lossy(), None, &mut report)
        .expect("load save");

    let roster = session.list_roster().expect("list roster");
    let names: Vec<&str> = roster.iter().map(|r| r.ros_name.as_str()).collect();

    assert_eq!(roster.len(), 10);
    assert!(
        !names.contains(&"shandra"),
        "flagged available but .ros file deleted"
    );
    assert!(
        !names.contains(&"zhjaeve"),
        "flagged available but .ros file deleted"
    );
    assert!(names.contains(&"khelgar"));
}

fn temp_save_copy(save: &str) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dst = tmp.path().join("save");
    crate::common::copy_dir_recursive(&saves_dir().join(save), &dst).expect("copy save");
    (tmp, dst)
}

#[test]
fn companion_save_roundtrip_persists_edit_and_syncs_roster() {
    let (_tmp, save_path) = temp_save_copy("Classic_Campaign");

    let mut session = make_session();
    let mut report = app_lib::services::load_diagnostics::LoadReport::new(
        app_lib::services::load_diagnostics::LoadInput {
            file_path: save_path.to_string_lossy().to_string(),
            player_index: None,
            file_size: None,
        },
    );
    session
        .load_character(&save_path.to_string_lossy(), None, &mut report)
        .expect("load save");
    // khelgar is recruited (00_bKhelgar_Joined = 1) in this fixture's
    // globals.xml, so he still shows up in list_roster after the new
    // recruited-status filter below.
    session
        .load_companion("khelgar", false)
        .expect("load companion");

    session
        .character_mut()
        .expect("character")
        .set_ability(app_lib::character::AbilityIndex::STR, 18)
        .expect("set str");
    assert!(session.has_unsaved_changes());

    // Dirty-switch guard (spec requirement): switching while dirty must fail
    // without force.
    assert!(session.load_companion("bishop", false).is_err());

    let warning = session.save_companion().expect("save companion");
    assert!(warning.is_none(), "roster sync should succeed: {warning:?}");
    assert!(!session.has_unsaved_changes());

    // Fresh session sees the persisted edit.
    let mut session2 = make_session();
    let mut report2 = app_lib::services::load_diagnostics::LoadReport::new(
        app_lib::services::load_diagnostics::LoadInput {
            file_path: save_path.to_string_lossy().to_string(),
            player_index: None,
            file_size: None,
        },
    );
    session2
        .load_character(&save_path.to_string_lossy(), None, &mut report2)
        .expect("reload save");
    session2
        .load_companion("khelgar", false)
        .expect("reload companion");
    assert_eq!(
        session2
            .character()
            .unwrap()
            .base_ability(app_lib::character::AbilityIndex::STR),
        18
    );

    // Roster cache reflects the companion's classes after sync.
    let roster = session2.list_roster().expect("roster");
    let khelgar = roster.iter().find(|r| r.ros_name == "khelgar").unwrap();
    let classes: Vec<(i32, i32)> = session2
        .character()
        .unwrap()
        .class_entries()
        .into_iter()
        .map(|e| (e.class_id.0, e.level))
        .collect();
    assert_eq!(khelgar.classes, classes);
}

// The repair case behind the user report: khelgar.ros carries none of the four
// pet fields, so setting a familiar has to create them - and the type-strict
// engine only honors them as CExoString/Int.
#[test]
fn companion_save_roundtrip_creates_missing_familiar_fields() {
    use app_lib::parsers::gff::GffValue;

    let (_tmp, save_path) = temp_save_copy("Classic_Campaign");

    let mut session = make_session();
    let mut report = app_lib::services::load_diagnostics::LoadReport::new(
        app_lib::services::load_diagnostics::LoadInput {
            file_path: save_path.to_string_lossy().to_string(),
            player_index: None,
            file_size: None,
        },
    );
    session
        .load_character(&save_path.to_string_lossy(), None, &mut report)
        .expect("load save");
    session
        .load_companion("khelgar", false)
        .expect("load companion");

    {
        let character = session.character_mut().expect("character");
        assert_eq!(character.familiar_type(), -1, "fixture has no Familiar*");
        character.set_familiar_name("Whiskers".to_string());
        character.set_familiar_type(1);
    }
    assert!(session.save_companion().expect("save companion").is_none());

    let mut session2 = make_session();
    let mut report2 = app_lib::services::load_diagnostics::LoadReport::new(
        app_lib::services::load_diagnostics::LoadInput {
            file_path: save_path.to_string_lossy().to_string(),
            player_index: None,
            file_size: None,
        },
    );
    session2
        .load_character(&save_path.to_string_lossy(), None, &mut report2)
        .expect("reload save");
    session2
        .load_companion("khelgar", false)
        .expect("reload companion");

    let character = session2.character().expect("character");
    assert_eq!(character.familiar_name(), "Whiskers");
    assert_eq!(character.familiar_type(), 1);
    assert!(matches!(
        character.gff().get("FamiliarName"),
        Some(GffValue::String(_))
    ));
    assert!(matches!(
        character.gff().get("FamiliarType"),
        Some(GffValue::Int(1))
    ));
}

#[test]
fn export_to_localvault_rejected_for_companion() {
    let mut session = make_session();
    load_save(&mut session, "Classic_Campaign");
    session
        .load_companion("khelgar", false)
        .expect("load companion");

    let paths = app_lib::config::NWN2Paths::new();
    assert!(session.export_to_localvault(&paths).is_err());
}
