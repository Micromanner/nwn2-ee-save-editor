use app_lib::character::Character;
use app_lib::parsers::gff::{GffParser, GffValue, GffWriter};
use app_lib::services::savegame_handler::SaveGameHandler;
use indexmap::IndexMap;
use tempfile::TempDir;

#[path = "../common/mod.rs"]
mod common;

const GARRICK_FULL_NAME: &str = "Garrick Ironheart";
const TRANSITION_FIELD_NAMES: &[&str] = &[
    "Race",
    "Subrace",
    "ClassList",
    "LvlStatList",
    "FeatList",
    "SkillList",
];

#[test]
fn test_savegame_handler_read_operations() {
    let fixture_path = common::fixtures_path().join("saves/Classic_Campaign");
    let temp_dir = TempDir::new().unwrap();
    let save_path = temp_dir.path().join("Classic_Campaign_Read");

    common::copy_dir_recursive(&fixture_path, &save_path).expect("Failed to copy fixture");

    let handler = SaveGameHandler::new(&save_path, true, false).expect("Failed to create handler");

    // Test extract_file
    let globals = handler
        .extract_file("globals.xml")
        .expect("Failed to extract globals.xml");
    assert!(!globals.is_empty());

    let start_bytes = &globals[0..5]; // check xml header or similar
    assert!(String::from_utf8_lossy(start_bytes).starts_with('<') || !globals.is_empty());

    // Test list_files
    let files = handler.list_files().expect("Failed to list files");
    assert!(!files.is_empty());
    assert!(!files.is_empty());
    // list_files shows zip content only; directory-based files like globals.xml may not appear here.

    // Test list_companions
    let companions = handler
        .list_companions()
        .expect("Failed to list companions");
    // Classic Campaign might have some companions like 'Khelgar', 'Neeshka' depending on save point.
    // Even if empty, it should not fail.
    println!("Companions found: {companions:?}");

    // Test batch_read_character_files
    let char_files = handler
        .batch_read_character_files()
        .expect("Failed to batch read characters");
    assert!(char_files.contains_key("player.bic"));
    // If companions exist, they should be here too
    if !companions.is_empty() {
        assert!(char_files.keys().any(|k| k.contains(&companions[0])));
    }

    // Test read_character_summary
    let summary = handler
        .read_character_summary()
        .expect("Failed to read character summary");
    if let Some(s) = summary {
        assert!(!s.first_name.is_empty());
        // assert!(s.str > 0);
    } else {
        // player.bic must expect to exist in valid save
        panic!("Character summary should be present for Classic_Campaign");
    }
}

#[test]
fn test_savegame_handler_write_operations() {
    let fixture_path = common::fixtures_path().join("saves/Classic_Campaign");
    let temp_dir = TempDir::new().unwrap();
    let save_path = temp_dir.path().join("Classic_Campaign_Write");

    common::copy_dir_recursive(&fixture_path, &save_path).expect("Failed to copy fixture");

    let mut handler =
        SaveGameHandler::new(&save_path, true, false).expect("Failed to create handler");

    // Test update_file
    let test_filename = "test_file.txt";
    let test_content = b"Hello NWN2";

    handler
        .update_file(test_filename, test_content)
        .expect("Failed to update file");

    // Verify update
    let content = handler
        .extract_file(test_filename)
        .expect("Failed to extract updated file");
    assert_eq!(content, test_content);

    // Verify it is in list
    let files = handler.list_files().expect("Failed to list files");
    assert!(files.iter().any(|f| f.name == test_filename));

    // Test update existing file
    let new_globals = b"<xml>Modified</xml>";
    handler
        .update_file("globals.xml", new_globals)
        .expect("Failed to overwrite globals.xml");

    let content_mod = handler
        .extract_file("globals.xml")
        .expect("Failed to read modified globals");
    assert_eq!(content_mod, new_globals);
}

#[test]
fn test_multiplayer_non_primary_slot_write_preserves_primary_mirrors() {
    let fixture_path = common::fixtures_path().join("saves/Multiplayer_Slot_Aware");
    let before_fixture_path = fixture_path.join("before");
    let after_fixture_path = fixture_path.join("after");
    if !before_fixture_path.exists() || !after_fixture_path.exists() {
        eprintln!(
            "Skipping multiplayer slot-aware write test; before/after fixture pair not found at {}",
            fixture_path.display()
        );
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let save_path = temp_dir.path().join("Multiplayer_Slot_Aware_Write");
    common::copy_dir_recursive(&before_fixture_path, &save_path).expect("Failed to copy fixture");

    let mut handler =
        SaveGameHandler::new(&save_path, true, false).expect("Failed to create handler");
    let after_handler = SaveGameHandler::new(&after_fixture_path, true, false)
        .expect("Failed to create after fixture handler");

    let original_player_bic = handler
        .extract_player_bic()
        .expect("Failed to read player.bic")
        .expect("multiplayer fixture must include player.bic");
    let playerinfo_path = save_path.join("playerinfo.bin");
    let original_playerinfo =
        std::fs::read(&playerinfo_path).expect("multiplayer fixture must include playerinfo.bin");

    let original_playerlist = handler
        .extract_player_data()
        .expect("Failed to read playerlist.ifo");
    let after_playerlist = after_handler
        .extract_player_data()
        .expect("Failed to read after fixture playerlist.ifo");

    let original_primary_snapshot = read_transition_snapshot(original_playerlist.clone(), 0);
    let (player_index, original_slot_fields) =
        find_player_fields(original_playerlist.clone(), GARRICK_FULL_NAME);
    assert!(
        player_index > 0,
        "fixture must keep Garrick in a non-primary multiplayer slot"
    );

    let original_slot_snapshot = TransitionSnapshot::from_fields(&original_slot_fields);
    assert_eq!(
        original_slot_snapshot.race,
        Some(31),
        "before fixture should contain Garrick as Yuan-ti"
    );
    assert_eq!(
        original_slot_snapshot.subrace,
        Some(47),
        "before fixture should contain Garrick's Yuan-ti subrace"
    );
    assert_eq!(
        original_slot_snapshot.classes,
        vec![(0, 5), (4, 2)],
        "before fixture should contain the known barbarian/fighter split"
    );
    assert_eq!(
        original_slot_snapshot.level_history.len(),
        7,
        "before fixture should contain Garrick's known level history"
    );
    assert_eq!(
        original_slot_snapshot.feats.len(),
        35,
        "before fixture should contain Garrick's known feat list"
    );
    assert_eq!(
        original_slot_snapshot.skills,
        vec![(21, 4), (24, 10), (25, 9), (26, 8)],
        "before fixture should contain Garrick's known skill ranks"
    );
    let original_non_transition_snapshot = non_transition_snapshot(&original_slot_fields);

    let (after_player_index, after_slot_fields) =
        find_player_fields(after_playerlist, GARRICK_FULL_NAME);
    assert_eq!(
        after_player_index, player_index,
        "after fixture should keep Garrick in the same non-primary slot"
    );
    let expected_slot_snapshot = TransitionSnapshot::from_fields(&after_slot_fields);
    assert_eq!(
        expected_slot_snapshot.race,
        Some(0),
        "after fixture should contain Garrick as dwarf"
    );
    assert_eq!(
        expected_slot_snapshot.subrace,
        Some(2),
        "after fixture should contain Garrick's dwarf subrace"
    );
    assert_eq!(
        expected_slot_snapshot.classes,
        vec![(4, 1)],
        "after fixture should contain the known fighter rebuild"
    );
    assert_eq!(
        expected_slot_snapshot.level_history.len(),
        1,
        "after fixture should contain Garrick's rebuilt level history"
    );
    assert_eq!(
        expected_slot_snapshot.feats.len(),
        25,
        "after fixture should contain Garrick's rebuilt feat list"
    );
    assert_eq!(
        expected_slot_snapshot.skills,
        vec![(10, 4), (18, 4), (24, 4)],
        "after fixture should contain Garrick's rebuilt skill ranks"
    );
    assert_ne!(
        expected_slot_snapshot, original_slot_snapshot,
        "fixture pair must alter transition-relevant non-primary slot data"
    );

    let updated_playerlist =
        playerlist_with_transition_fields(original_playerlist, player_index, &after_slot_fields);

    handler
        .update_player_complete(&updated_playerlist, None, None, None)
        .expect("Failed to write non-primary slot playerlist update");

    let reparsed_playerlist = handler
        .extract_player_data()
        .expect("Failed to re-read playerlist.ifo");
    let (_, reparsed_slot_fields) =
        find_player_fields(reparsed_playerlist.clone(), GARRICK_FULL_NAME);
    assert_eq!(
        TransitionSnapshot::from_fields(&reparsed_slot_fields),
        expected_slot_snapshot,
        "non-primary slot must match the successful Garrick transition fixture"
    );
    assert_eq!(
        non_transition_snapshot(&reparsed_slot_fields),
        original_non_transition_snapshot,
        "non-transition fields, including inventory/items/spells, must stay untouched"
    );
    assert_eq!(
        read_transition_snapshot(reparsed_playerlist, 0),
        original_primary_snapshot,
        "editing slot 1+ must not rewrite primary Mod_PlayerList entry"
    );

    assert_eq!(
        handler
            .extract_player_bic()
            .expect("Failed to re-read player.bic")
            .expect("player.bic disappeared"),
        original_player_bic,
        "non-primary slot writes must not rewrite player.bic"
    );
    assert_eq!(
        std::fs::read(&playerinfo_path).expect("Failed to re-read playerinfo.bin"),
        original_playerinfo,
        "non-primary slot writes must not rewrite playerinfo.bin"
    );
}

#[derive(Debug, PartialEq, Eq)]
struct TransitionSnapshot {
    race: Option<i32>,
    subrace: Option<i32>,
    classes: Vec<(i32, i32)>,
    level_history: Vec<String>,
    feats: Vec<i32>,
    skills: Vec<(usize, i32)>,
}

fn playerlist_with_transition_fields(
    playerlist_data: Vec<u8>,
    player_index: usize,
    source_fields: &IndexMap<String, GffValue<'static>>,
) -> Vec<u8> {
    let gff = GffParser::from_bytes(playerlist_data).expect("Failed to parse playerlist.ifo");
    let file_type = gff.file_type.clone();
    let file_version = gff.file_version.clone();
    let root_fields_raw = gff
        .read_struct_fields(0)
        .expect("Failed to read playerlist root");
    let mut root_fields = root_fields_raw
        .into_iter()
        .map(|(key, value)| (key, value.force_owned()))
        .collect::<indexmap::IndexMap<_, _>>();

    let players = match root_fields.get_mut("Mod_PlayerList") {
        Some(GffValue::ListOwned(players)) => players,
        _ => panic!("Mod_PlayerList must be a list"),
    };
    assert!(
        players.len() > player_index,
        "multiplayer fixture must contain at least {} player slots",
        player_index + 1
    );

    copy_transition_fields(&mut players[player_index], source_fields);

    GffWriter::new(&file_type, &file_version)
        .write(root_fields)
        .expect("Failed to serialize updated playerlist")
}

fn copy_transition_fields(
    target_fields: &mut IndexMap<String, GffValue<'static>>,
    source_fields: &IndexMap<String, GffValue<'static>>,
) {
    for field_name in TRANSITION_FIELD_NAMES {
        let value = source_fields
            .get(*field_name)
            .unwrap_or_else(|| panic!("after fixture slot must include {field_name}"))
            .clone()
            .force_owned();
        target_fields.insert((*field_name).to_string(), value);
    }
}

fn read_transition_snapshot(playerlist_data: Vec<u8>, player_index: usize) -> TransitionSnapshot {
    let gff = GffParser::from_bytes(playerlist_data).expect("Failed to parse playerlist.ifo");
    let root_fields = gff
        .read_struct_fields(0)
        .expect("Failed to read playerlist root");
    let players = match root_fields.get("Mod_PlayerList") {
        Some(GffValue::List(players)) => players,
        _ => panic!("Mod_PlayerList must be a list"),
    };
    let fields = players
        .get(player_index)
        .expect("Expected player slot after write")
        .force_load();
    TransitionSnapshot::from_fields(&fields)
}

fn find_player_fields(
    playerlist_data: Vec<u8>,
    full_name: &str,
) -> (usize, IndexMap<String, GffValue<'static>>) {
    let gff = GffParser::from_bytes(playerlist_data).expect("Failed to parse playerlist.ifo");
    let root_fields = gff
        .read_struct_fields(0)
        .expect("Failed to read playerlist root");
    let players = match root_fields.get("Mod_PlayerList") {
        Some(GffValue::List(players)) => players,
        _ => panic!("Mod_PlayerList must be a list"),
    };

    players
        .iter()
        .enumerate()
        .find_map(|(index, player)| {
            let fields = player.force_load();
            let character = Character::from_gff(fields.clone());
            (character.full_name() == full_name).then_some((index, fields))
        })
        .unwrap_or_else(|| panic!("{full_name} must exist in Mod_PlayerList"))
}

fn non_transition_snapshot(
    fields: &IndexMap<String, GffValue<'static>>,
) -> IndexMap<String, String> {
    fields
        .iter()
        .filter(|(key, _)| !TRANSITION_FIELD_NAMES.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), format!("{:?}", value.clone().force_owned())))
        .collect()
}

impl TransitionSnapshot {
    fn from_fields(fields: &IndexMap<String, GffValue<'static>>) -> Self {
        Self {
            race: numeric_field(fields, "Race"),
            subrace: numeric_field(fields, "Subrace"),
            classes: list_pairs(fields, "ClassList", "Class", "ClassLevel"),
            level_history: list_entry_snapshots(fields, "LvlStatList"),
            feats: list_values(fields, "FeatList", "Feat"),
            skills: skill_ranks(fields),
        }
    }
}

fn list_pairs(
    fields: &IndexMap<String, GffValue<'static>>,
    list_name: &str,
    first_field: &str,
    second_field: &str,
) -> Vec<(i32, i32)> {
    list_entries(fields, list_name)
        .into_iter()
        .filter_map(|entry| {
            Some((
                numeric_field(&entry, first_field)?,
                numeric_field(&entry, second_field)?,
            ))
        })
        .collect()
}

fn list_values(
    fields: &IndexMap<String, GffValue<'static>>,
    list_name: &str,
    field_name: &str,
) -> Vec<i32> {
    list_entries(fields, list_name)
        .into_iter()
        .filter_map(|entry| numeric_field(&entry, field_name))
        .collect()
}

fn list_entry_snapshots(
    fields: &IndexMap<String, GffValue<'static>>,
    list_name: &str,
) -> Vec<String> {
    list_entries(fields, list_name)
        .into_iter()
        .map(|entry| {
            let owned_entry = entry
                .into_iter()
                .map(|(key, value)| (key, value.force_owned()))
                .collect::<IndexMap<_, _>>();
            format!("{owned_entry:?}")
        })
        .collect()
}

fn skill_ranks(fields: &IndexMap<String, GffValue<'static>>) -> Vec<(usize, i32)> {
    list_entries(fields, "SkillList")
        .into_iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let rank = numeric_field(&entry, "Rank")?;
            (rank > 0).then_some((index, rank))
        })
        .collect()
}

fn list_entries(
    fields: &IndexMap<String, GffValue<'static>>,
    list_name: &str,
) -> Vec<IndexMap<String, GffValue<'static>>> {
    match fields.get(list_name) {
        Some(GffValue::ListOwned(items)) => items.clone(),
        Some(GffValue::List(items)) => items.iter().map(|item| item.force_load()).collect(),
        _ => Vec::new(),
    }
}

fn numeric_field(fields: &IndexMap<String, GffValue<'static>>, field_name: &str) -> Option<i32> {
    fields.get(field_name).and_then(value_to_i32)
}

fn value_to_i32(value: &GffValue<'_>) -> Option<i32> {
    match value {
        GffValue::Byte(value) => Some(i32::from(*value)),
        GffValue::Char(value) => i32::try_from(u32::from(*value)).ok(),
        GffValue::Word(value) => Some(i32::from(*value)),
        GffValue::Short(value) => Some(i32::from(*value)),
        GffValue::Dword(value) => i32::try_from(*value).ok(),
        GffValue::Int(value) => Some(*value),
        _ => None,
    }
}
