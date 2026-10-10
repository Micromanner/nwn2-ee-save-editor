use std::path::PathBuf;
use std::sync::Arc;

use app_lib::services::savegame_handler::backup;
use app_lib::state::session_state::{CharacterSource, SessionState};

fn saves_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/saves")
}

fn make_session() -> SessionState {
    SessionState::new()
}

fn load_save(session: &mut SessionState, save: &str) {
    crate::common::load_save(session, &saves_dir().join(save));
}

// Fixture reality (verified with --nocapture): roster.rst lists 14 members,
// all with a .ros file, including the cutscene followers npc_bevil and
// 0_amie. Nothing in roster.rst separates those from real companions (a
// user's save had Casavir in the party with RosAvailable/RosSelectable/
// RosCampaignNPC all 0, the same as the followers), so all 14 are listed.
#[test]
fn list_roster_filters_party_members_with_files_original_format() {
    let mut session = make_session();
    load_save(&mut session, "000055 - SAVE GAME COMPLETE");

    let roster = session.list_roster().expect("list roster");
    let names: Vec<&str> = roster.iter().map(|r| r.ros_name.as_str()).collect();

    assert_eq!(roster.len(), 14);
    assert!(names.contains(&"khelgar"));
    assert!(names.contains(&"shandra"));
    assert!(names.contains(&"zhjaeve"));
    assert!(names.contains(&"npc_bevil"));

    let khelgar = roster.iter().find(|r| r.ros_name == "khelgar").unwrap();
    assert_eq!(khelgar.char_name, "Khelgar Ironfist");
    assert_eq!(khelgar.classes, vec![(4, 2)]);
}

// Fixture reality: this save's globals.xml only has `00_b*_Joined = 1` for
// Khelgar, Neeshka, Elanee and Qara, yet ammon_jerro.ros exists - the game
// does not write a *_Joined var for every companion (user reports 4 and 7),
// so the .ros file alone decides membership.
#[test]
fn list_roster_works_on_ee_zip_format() {
    let mut session = make_session();
    load_save(&mut session, "Classic_Campaign");

    let roster = session.list_roster().expect("list roster");
    let names: Vec<&str> = roster.iter().map(|r| r.ros_name.as_str()).collect();
    for expected in ["elanee", "khelgar", "neeshka", "qara", "ammon_jerro"] {
        assert!(
            names.contains(&expected),
            "{expected} missing from {names:?}"
        );
    }
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

    session.load_companion("khelgar").expect("load companion");
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

    session.load_companion("bishop").expect("load bishop");
    let bishop = session.character().expect("character loaded");
    assert_eq!(bishop.animal_companion_name(), "Karnwyr");
    assert_eq!(bishop.animal_companion_type(), 1);
    assert_eq!(bishop.familiar_name(), "");
    assert_eq!(bishop.familiar_type(), -1);

    session.load_companion("qara").expect("load qara");
    let qara = session.character().expect("character loaded");
    assert_eq!(qara.familiar_name(), "Carel");
    assert_eq!(qara.familiar_type(), 4);
    assert_eq!(qara.animal_companion_type(), -1);
}

#[test]
fn undo_history_survives_character_switch() {
    let mut session = make_session();
    load_save(&mut session, "Classic_Campaign");
    session.record_edit("player edit", |_| Ok(())).unwrap();

    session.load_companion("khelgar").expect("load companion");
    assert!(
        !session.can_undo(),
        "fresh companion starts with empty history"
    );
    session.record_edit("khelgar edit", |_| Ok(())).unwrap();
    session.record_edit("khelgar edit 2", |_| Ok(())).unwrap();

    session.load_player().expect("load player");
    assert_eq!(
        session.history.undo_labels(),
        vec!["player edit".to_string()]
    );

    session.load_companion("khelgar").expect("reload companion");
    assert_eq!(
        session.history.undo_labels(),
        vec!["khelgar edit 2".to_string(), "khelgar edit".to_string()]
    );
    assert_eq!(session.undo().as_deref(), Some("khelgar edit 2"));
    assert_eq!(
        session.history.redo_labels(),
        vec!["khelgar edit 2".to_string()]
    );
}

#[test]
fn load_companion_rejects_unknown_name() {
    let mut session = make_session();
    load_save(&mut session, "Classic_Campaign");
    assert!(session.load_companion("does_not_exist").is_err());
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
        .load_character(&save_path.to_string_lossy(), None, true, &mut report)
        .expect("load save");

    let roster = session.list_roster().expect("list roster");
    let names: Vec<&str> = roster.iter().map(|r| r.ros_name.as_str()).collect();

    assert_eq!(roster.len(), 12);
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

fn empty_game_data() -> app_lib::loaders::GameData {
    app_lib::loaders::GameData::new(Arc::new(std::sync::RwLock::new(
        app_lib::parsers::tlk::TLKParser::default(),
    )))
}

#[test]
fn companion_save_roundtrip_persists_edit_and_syncs_roster() {
    let (_tmp, save_path) = crate::common::temp_save_copy("Classic_Campaign");

    let mut session = make_session();
    let mut report = app_lib::services::load_diagnostics::LoadReport::new(
        app_lib::services::load_diagnostics::LoadInput {
            file_path: save_path.to_string_lossy().to_string(),
            player_index: None,
            file_size: None,
        },
    );
    session
        .load_character(&save_path.to_string_lossy(), None, true, &mut report)
        .expect("load save");
    session.load_companion("khelgar").expect("load companion");

    session
        .character_mut()
        .expect("character")
        .set_ability(app_lib::character::AbilityIndex::STR, 18)
        .expect("set str");
    assert!(session.has_unsaved_changes());

    // Switching parks the edit in memory; nothing hits disk until save.
    session.load_companion("bishop").expect("switch to bishop");
    assert!(session.has_unsaved_changes());
    assert!(!session.character().expect("bishop").is_modified());
    session.load_companion("khelgar").expect("switch back");
    assert_eq!(
        session
            .character()
            .expect("khelgar")
            .base_ability(app_lib::character::AbilityIndex::STR),
        18
    );
    assert!(session.character().expect("khelgar").is_modified());

    let warning = session
        .save_character(&empty_game_data(), &app_lib::config::NWN2Paths::new(), 3)
        .expect("save companion");
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
        .load_character(&save_path.to_string_lossy(), None, true, &mut report2)
        .expect("reload save");
    session2
        .load_companion("khelgar")
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

    let (_tmp, save_path) = crate::common::temp_save_copy("Classic_Campaign");

    let mut session = make_session();
    let mut report = app_lib::services::load_diagnostics::LoadReport::new(
        app_lib::services::load_diagnostics::LoadInput {
            file_path: save_path.to_string_lossy().to_string(),
            player_index: None,
            file_size: None,
        },
    );
    session
        .load_character(&save_path.to_string_lossy(), None, true, &mut report)
        .expect("load save");
    session.load_companion("khelgar").expect("load companion");

    {
        let character = session.character_mut().expect("character");
        assert_eq!(character.familiar_type(), -1, "fixture has no Familiar*");
        character.set_familiar_name("Whiskers".to_string());
        character.set_familiar_type(1);
    }
    assert!(
        session
            .save_character(&empty_game_data(), &app_lib::config::NWN2Paths::new(), 3)
            .expect("save companion")
            .is_none()
    );

    let mut session2 = make_session();
    let mut report2 = app_lib::services::load_diagnostics::LoadReport::new(
        app_lib::services::load_diagnostics::LoadInput {
            file_path: save_path.to_string_lossy().to_string(),
            player_index: None,
            file_size: None,
        },
    );
    session2
        .load_character(&save_path.to_string_lossy(), None, true, &mut report2)
        .expect("reload save");
    session2
        .load_companion("khelgar")
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
    session.load_companion("khelgar").expect("load companion");

    let paths = app_lib::config::NWN2Paths::new();
    assert!(session.export_to_localvault(&paths).is_err());
}

// The CMI workshop mod caches the summoned blueprint on the summoning
// creature itself (`cmi_animcomp`), so a druid companion carries it in its
// own .ros VarTable; saving must retarget it there like the player path does.
#[test]
fn companion_save_retargets_mod_companion_variable_in_ros_vartable() {
    use app_lib::parsers::gff::GffValue;
    use indexmap::IndexMap;

    let (_tmp, save_path) = crate::common::temp_save_copy("Classic_Campaign");
    let mut session = make_session();
    load_save(&mut session, &save_path.to_string_lossy());
    session.load_companion("bishop").expect("load companion");

    let mut var = IndexMap::new();
    var.insert("Name".to_string(), GffValue::String("cmi_animcomp".into()));
    var.insert("Type".to_string(), GffValue::Dword(3));
    var.insert(
        "Value".to_string(),
        GffValue::String("c_ancom_wolf3".into()),
    );
    {
        let character = session.character_mut().expect("character");
        character.set_list("VarTable", vec![var]);
        character.set_animal_companion_type(2);
    }

    let mut hen = app_lib::parsers::tda::TDAParser::new();
    hen.parse_from_string("2DA V2.0\n\n\tLABEL\tbaseresref_prefix\n0\tBadger\tc_ancom_badger\n1\tWolf\tc_ancom_wolf\n2\tBear\tc_ancom_bear\n")
        .expect("parse hen_companion");
    let mut game_data = empty_game_data();
    game_data.tables.insert(
        "hen_companion".to_string(),
        app_lib::loaders::LoadedTable::new("hen_companion".to_string(), Arc::new(hen)),
    );
    session
        .save_character(&game_data, &app_lib::config::NWN2Paths::new(), 3)
        .expect("save companion");

    let mut session2 = make_session();
    load_save(&mut session2, &save_path.to_string_lossy());
    session2.load_companion("bishop").expect("reload companion");
    let GffValue::ListOwned(vars) = &session2.character().unwrap().gff()["VarTable"] else {
        panic!("VarTable missing");
    };
    let value = vars
        .iter()
        .find(|v| matches!(v.get("Name"), Some(GffValue::String(n)) if n == "cmi_animcomp"))
        .and_then(|v| match v.get("Value") {
            Some(GffValue::String(s)) => Some(s.to_string()),
            _ => None,
        })
        .expect("cmi_animcomp present");
    assert_eq!(value, "c_ancom_bear3");
}

#[test]
fn save_character_flushes_parked_companion_and_player_edits() {
    let (_tmp, save_path) = crate::common::temp_save_copy("Classic_Campaign");

    let mut session = make_session();
    let mut report = app_lib::services::load_diagnostics::LoadReport::new(
        app_lib::services::load_diagnostics::LoadInput {
            file_path: save_path.to_string_lossy().to_string(),
            player_index: None,
            file_size: None,
        },
    );
    session
        .load_character(&save_path.to_string_lossy(), None, true, &mut report)
        .expect("load save");

    session
        .character_mut()
        .expect("player")
        .set_ability(app_lib::character::AbilityIndex::STR, 17)
        .expect("set player str");
    session.load_companion("khelgar").expect("load khelgar");
    session
        .character_mut()
        .expect("khelgar")
        .set_ability(app_lib::character::AbilityIndex::STR, 18)
        .expect("set khelgar str");
    session.load_companion("bishop").expect("load bishop");
    assert!(session.has_unsaved_changes());

    session
        .save_character(&empty_game_data(), &app_lib::config::NWN2Paths::new(), 3)
        .expect("save all");
    assert!(!session.has_unsaved_changes());

    let mut session2 = make_session();
    let mut report2 = app_lib::services::load_diagnostics::LoadReport::new(
        app_lib::services::load_diagnostics::LoadInput {
            file_path: save_path.to_string_lossy().to_string(),
            player_index: None,
            file_size: None,
        },
    );
    session2
        .load_character(&save_path.to_string_lossy(), None, true, &mut report2)
        .expect("reload save");
    assert_eq!(
        session2
            .character()
            .expect("player")
            .base_ability(app_lib::character::AbilityIndex::STR),
        17
    );
    session2.load_companion("khelgar").expect("reload khelgar");
    assert_eq!(
        session2
            .character()
            .expect("khelgar")
            .base_ability(app_lib::character::AbilityIndex::STR),
        18
    );
    session2.load_player().expect("back to player");
    assert_eq!(
        session2
            .character()
            .expect("player")
            .base_ability(app_lib::character::AbilityIndex::STR),
        17
    );
}

/// Lowercase tags of the `Creature List` in the Sunken Flagon's saved area state.
fn flagon_creature_tags(save_path: &std::path::Path) -> Vec<String> {
    use app_lib::parsers::gff::{GffParser, GffValue};

    let handler =
        app_lib::services::savegame_handler::SaveGameHandler::new(save_path).expect("handler");
    let (_, mut erf) =
        app_lib::services::campaign::content::open_module_state(&handler, "1000_neverwinter_a1")
            .expect("module state");
    let bytes = erf
        .extract_resource("{1001}sunkenflag.git")
        .expect("flagon git");
    let root = GffParser::from_bytes(bytes)
        .expect("parse git")
        .read_owned_struct(0)
        .expect("git root");
    let Some(GffValue::ListOwned(creatures)) = root.get("Creature List") else {
        panic!("flagon git has no Creature List");
    };
    creatures
        .iter()
        .filter_map(|c| match c.get("Tag") {
            Some(GffValue::String(tag)) => Some(tag.to_ascii_lowercase()),
            _ => None,
        })
        .collect()
}

// Fixture reality: Bishop is benched and parked in the Sunken Flagon, a full creature
// in `{1001}sunkenflag.git` that the game loads instead of bishop.ros.
#[test]
fn companion_save_removes_parked_area_copy() {
    let (_tmp, save_path) = crate::common::temp_save_copy("user_save_linux");
    let before = flagon_creature_tags(&save_path);
    assert!(before.iter().any(|t| t == "bishop"), "fixture parks Bishop");

    let mut session = make_session();
    load_save(&mut session, &save_path.to_string_lossy());
    session.load_companion("bishop").expect("load bishop");
    session
        .character_mut()
        .expect("bishop")
        .set_ability(app_lib::character::AbilityIndex::STR, 18)
        .expect("set str");
    session
        .save_character(&empty_game_data(), &app_lib::config::NWN2Paths::new(), 3)
        .expect("save");

    let after = flagon_creature_tags(&save_path);
    let expected: Vec<String> = before.into_iter().filter(|t| t != "bishop").collect();
    assert_eq!(after, expected, "only Bishop's copy is gone");
}

// The party join is flushed into roster.rst before the copies are removed, so the
// staged roster already lists Bishop in the party; his Flagon copy must still go.
#[test]
fn companion_joining_party_in_same_save_loses_area_copy() {
    let (_tmp, save_path) = crate::common::temp_save_copy("user_save_linux");
    assert!(
        flagon_creature_tags(&save_path)
            .iter()
            .any(|t| t == "bishop")
    );

    let mut session = make_session();
    load_save(&mut session, &save_path.to_string_lossy());
    session.load_companion("bishop").expect("load bishop");
    session
        .character_mut()
        .expect("bishop")
        .set_ability(app_lib::character::AbilityIndex::STR, 18)
        .expect("set str");
    session
        .stage_party_membership(&[("bishop".to_string(), true)])
        .expect("stage join");
    session
        .save_character(&empty_game_data(), &app_lib::config::NWN2Paths::new(), 3)
        .expect("save");

    assert!(
        !flagon_creature_tags(&save_path)
            .iter()
            .any(|t| t == "bishop")
    );
}

// Fixture reality: the unrecruited SoZ cohort Umoja is a full creature in the
// `g_x2.z` bazaar with a locked roster entry (RosSelectable 0); that creature is
// the NPC the player hires, so a cohort edit must not remove it.
#[test]
fn locked_cohort_save_leaves_its_area_creature() {
    let (_tmp, save_path) = crate::common::temp_save_copy("STORM_Campaign");
    let z_path = save_path.join("g_x2.z");
    let before = std::fs::read(&z_path).expect("read .z");

    let mut session = make_session();
    load_save(&mut session, &save_path.to_string_lossy());
    session.load_companion("co_umoja").expect("load umoja");
    session
        .character_mut()
        .expect("umoja")
        .set_ability(app_lib::character::AbilityIndex::STR, 18)
        .expect("set str");
    session
        .save_character(&empty_game_data(), &app_lib::config::NWN2Paths::new(), 3)
        .expect("save");

    assert_eq!(std::fs::read(&z_path).expect("read .z"), before);
}

#[test]
fn player_only_save_leaves_module_state_untouched() {
    let (_tmp, save_path) = crate::common::temp_save_copy("user_save_linux");
    let z_path = save_path.join("1000_neverwinter_a1.z");
    let before = std::fs::read(&z_path).expect("read .z");

    let mut session = make_session();
    load_save(&mut session, &save_path.to_string_lossy());
    session
        .character_mut()
        .expect("player")
        .set_ability(app_lib::character::AbilityIndex::STR, 17)
        .expect("set str");
    session
        .save_character(&empty_game_data(), &app_lib::config::NWN2Paths::new(), 3)
        .expect("save");

    assert_eq!(std::fs::read(&z_path).expect("read .z"), before);
}

#[test]
fn staged_party_membership_counts_as_dirty_and_flushes_on_save() {
    let (_tmp, save_path) = crate::common::temp_save_copy("Classic_Campaign");
    let mut session = make_session();
    load_save(&mut session, &save_path.to_string_lossy());

    let roster = session.list_roster().expect("roster");
    let (name, was_in_party) = {
        let entry = roster.first().expect("roster entry");
        (entry.ros_name.clone(), entry.in_party)
    };
    assert!(!session.has_unsaved_changes());

    session
        .stage_party_membership(&[(name.clone(), !was_in_party)])
        .expect("stage membership");
    assert!(
        session.has_unsaved_changes(),
        "staged party change is dirty"
    );

    session
        .save_character(&empty_game_data(), &app_lib::config::NWN2Paths::new(), 3)
        .expect("save with staged party change");
    assert!(!session.has_unsaved_changes());

    let mut session2 = make_session();
    load_save(&mut session2, &save_path.to_string_lossy());
    let roster2 = session2.list_roster().expect("roster after save");
    let after = roster2
        .iter()
        .find(|r| r.ros_name == name)
        .expect("entry survives save");
    assert_eq!(after.in_party, !was_in_party);
    assert_eq!(after.staged_in_party, None);
}

#[test]
fn save_appends_change_record_to_load_backup_manifest() {
    use app_lib::services::savegame_handler::backup_manifest;

    let (_tmp, save_path) = crate::common::temp_save_copy("Classic_Campaign");
    let mut session = make_session();
    load_save(&mut session, &save_path.to_string_lossy());

    let backups_dir = backup::get_backup_dir_for_save(&save_path);
    let load_backup = std::fs::read_dir(&backups_dir)
        .expect("backups dir")
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("load backup created");
    let initial = backup_manifest::read(&load_backup).expect("initial manifest");
    assert_eq!(initial.reason, backup_manifest::BackupReason::Load);
    assert!(initial.saves.is_empty());

    session.record_edit("Set gold to 5000", |_| Ok(())).unwrap();
    session.undo();
    session.load_companion("khelgar").expect("load khelgar");
    session
        .record_edit("Set strength to 18", |_| Ok(()))
        .unwrap();
    let roster = session.list_roster().expect("roster");
    let (name, was_in_party) = {
        let entry = roster.first().expect("roster entry");
        (entry.ros_name.clone(), entry.in_party)
    };
    session
        .stage_party_membership(&[(name.clone(), !was_in_party)])
        .expect("stage membership");

    session
        .save_character(&empty_game_data(), &app_lib::config::NWN2Paths::new(), 3)
        .expect("save");

    let manifest = backup_manifest::read(&load_backup).expect("manifest after save");
    assert_eq!(manifest.saves.len(), 1);
    let record = &manifest.saves[0];
    assert!(!record.saved_at.is_empty());

    let player = record
        .characters
        .iter()
        .find(|c| c.ros_name.is_none())
        .expect("player changes");
    assert_eq!(
        player.changes,
        vec![
            "Set gold to 5000".to_string(),
            "Undo: Set gold to 5000".to_string()
        ]
    );
    let khelgar = record
        .characters
        .iter()
        .find(|c| c.ros_name.as_deref() == Some("khelgar"))
        .expect("khelgar changes");
    assert_eq!(khelgar.changes, vec!["Set strength to 18".to_string()]);

    assert_eq!(record.party.len(), 1);
    assert!(record.party[0].ros_name.eq_ignore_ascii_case(&name));
    assert_eq!(record.party[0].in_party, !was_in_party);
    assert!(record.quests.is_empty());

    // Nothing pending after the save: a second save with no edits adds no record.
    session
        .save_character(&empty_game_data(), &app_lib::config::NWN2Paths::new(), 3)
        .expect("second save");
    assert_eq!(
        backup_manifest::read(&load_backup)
            .expect("manifest")
            .saves
            .len(),
        1
    );
}

/// Stage the int `name = value` in `module` (`None` = campaign variables).
fn stage_int(session: &mut SessionState, module: Option<&str>, name: &str, value: i32) {
    let paths = app_lib::config::NWN2Paths::new();
    let handler = session.savegame_handler.as_ref().expect("handler");
    session
        .staged_variables
        .stage(
            handler,
            &paths,
            module,
            &[(name.into(), value.to_string(), "int".into())],
        )
        .expect("stage");
}

fn saved_global(session: &SessionState, name: &str) -> Option<i32> {
    let handler = session.savegame_handler.as_ref().expect("handler");
    app_lib::services::campaign::CampaignManager::get_campaign_variables(handler)
        .expect("globals")
        .integers
        .get(name)
        .copied()
}

#[test]
fn staged_variables_stay_in_memory_until_save() {
    use app_lib::services::campaign::CampaignManager;
    use app_lib::services::savegame_handler::backup_manifest;

    let (_tmp, save_path) = crate::common::temp_save_copy("Classic_Campaign");
    let mut session = make_session();
    load_save(&mut session, &save_path.to_string_lossy());
    let load_backup = std::fs::read_dir(backup::get_backup_dir_for_save(&save_path))
        .expect("backups dir")
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("load backup created");
    let before = saved_global(&session, "00_nInfluencekhelgar");

    stage_int(&mut session, None, "00_nInfluencekhelgar", 50);

    assert_eq!(saved_global(&session, "00_nInfluencekhelgar"), before);
    assert!(session.has_unsaved_changes());
    let handler = session.savegame_handler.as_ref().expect("handler");
    let mut data = CampaignManager::get_campaign_variables(handler).expect("globals");
    session.overlay_globals(&mut data);
    assert_eq!(data.integers.get("00_nInfluencekhelgar"), Some(&50));

    session
        .save_character(&empty_game_data(), &app_lib::config::NWN2Paths::new(), 3)
        .expect("save");

    assert!(!session.has_unsaved_changes());
    assert_eq!(saved_global(&session, "00_nInfluencekhelgar"), Some(50));
    let manifest = backup_manifest::read(&load_backup).expect("manifest");
    assert_eq!(manifest.saves.len(), 1);
    let record = &manifest.saves[0];
    assert_eq!(record.variables.len(), 1);
    assert_eq!(record.variables[0].name, "00_nInfluencekhelgar");
    assert_eq!(record.variables[0].value, "50");
}

#[test]
fn staged_module_variables_are_written_on_save() {
    use app_lib::services::campaign::CampaignManager;

    let (_tmp, save_path) = crate::common::temp_save_copy("Classic_Campaign");
    let mut session = make_session();
    load_save(&mut session, &save_path.to_string_lossy());
    let paths = app_lib::config::NWN2Paths::new();
    let module = session
        .savegame_handler
        .as_ref()
        .expect("handler")
        .extract_current_module()
        .expect("current module");
    let saved = |session: &SessionState| {
        let handler = session.savegame_handler.as_ref().expect("handler");
        CampaignManager::get_module_info_by_id(handler, &paths, &module)
            .expect("module variables")
            .1
            .integers
            .get("EDITOR_TEST_VAR")
            .copied()
    };

    stage_int(&mut session, Some(&module), "EDITOR_TEST_VAR", 7);
    assert_eq!(saved(&session), None);

    session
        .save_character(&empty_game_data(), &paths, 3)
        .expect("save");
    assert_eq!(saved(&session), Some(7));
}

#[test]
fn staging_the_saved_value_unstages_the_variable() {
    let mut session = make_session();
    load_save(&mut session, "Classic_Campaign");
    let saved = saved_global(&session, "00_nInfluencekhelgar").unwrap_or(0);

    stage_int(&mut session, None, "00_nInfluencekhelgar", saved + 1);
    stage_int(&mut session, None, "00_nInfluencekhelgar", saved);

    assert!(!session.has_unsaved_changes());
}

#[test]
fn list_roster_errors_on_unparseable_roster() {
    let (_tmp, save_path) = crate::common::temp_save_copy("000055 - SAVE GAME COMPLETE");
    std::fs::write(save_path.join("ROSTER.rst"), b"not a gff").expect("corrupt roster");
    let mut session = make_session();
    load_save(&mut session, &save_path.to_string_lossy());
    assert!(session.list_roster().is_err());
}

#[test]
fn stage_party_membership_stages_nothing_when_a_name_is_unknown() {
    let mut session = make_session();
    load_save(&mut session, "Classic_Campaign");
    let roster = session.list_roster().expect("roster");
    let first = roster.first().expect("roster entry");

    let result = session.stage_party_membership(&[
        (first.ros_name.clone(), !first.in_party),
        ("nobody".to_string(), true),
    ]);
    assert!(result.is_err());
    assert!(!session.has_unsaved_changes());
}

#[test]
fn failed_load_takes_no_load_backup() {
    let (_tmp, save_path) = crate::common::temp_save_copy("000055 - SAVE GAME COMPLETE");
    std::fs::write(save_path.join("playerlist.ifo"), b"not a gff").expect("corrupt playerlist");
    let mut session = make_session();
    let mut report = app_lib::services::load_diagnostics::LoadReport::new(
        app_lib::services::load_diagnostics::LoadInput {
            file_path: save_path.to_string_lossy().to_string(),
            player_index: None,
            file_size: None,
        },
    );
    assert!(
        session
            .load_character(&save_path.to_string_lossy(), None, true, &mut report)
            .is_err()
    );
    assert!(!backup::get_backup_dir_for_save(&save_path).exists());
}
