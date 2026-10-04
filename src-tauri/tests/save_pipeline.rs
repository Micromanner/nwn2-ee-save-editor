use std::path::PathBuf;

use indexmap::IndexMap;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("gff")
        .join("sync")
}

fn parse_playerlist_ifo(
    data: Vec<u8>,
) -> IndexMap<String, app_lib::parsers::gff::GffValue<'static>> {
    use app_lib::parsers::gff::{GffParser, GffValue};

    let gff = GffParser::from_bytes(data).expect("Failed to parse IFO");
    let root = gff.read_struct_fields(0).expect("Failed to read root");

    let mod_player_list = root.get("Mod_PlayerList").expect("No Mod_PlayerList");
    if let GffValue::List(lazy_structs) = mod_player_list {
        let first = lazy_structs.first().expect("Mod_PlayerList is empty");
        let fields = first.force_load().expect("load struct");
        fields
            .into_iter()
            .map(|(k, v)| (k, v.force_owned().expect("force owned")))
            .collect()
    } else {
        panic!("Mod_PlayerList is not a list");
    }
}

#[test]
fn test_ifo_round_trip() {
    use app_lib::parsers::gff::{GffValue, GffWriter};

    let ifo_data = std::fs::read(fixtures_dir().join("playerlist.ifo")).expect("read ifo fixture");
    let original_fields = parse_playerlist_ifo(ifo_data);

    // Build IFO bytes from fields
    let mut root = IndexMap::new();
    root.insert(
        "Mod_PlayerList".to_string(),
        GffValue::ListOwned(vec![original_fields.clone()]),
    );
    let mut writer = GffWriter::new("IFO ", "V3.2");
    let ifo_bytes = writer.write(root).expect("Failed to write IFO");

    // Re-parse and compare key fields
    let reparsed = parse_playerlist_ifo(ifo_bytes);

    assert_eq!(
        original_fields.get("FirstName").map(|v| format!("{v:?}")),
        reparsed.get("FirstName").map(|v| format!("{v:?}")),
        "FirstName mismatch after round-trip"
    );
    assert_eq!(
        original_fields.get("Str").map(|v| format!("{v:?}")),
        reparsed.get("Str").map(|v| format!("{v:?}")),
        "Str mismatch after round-trip"
    );
    assert_eq!(
        original_fields.get("Race").map(|v| format!("{v:?}")),
        reparsed.get("Race").map(|v| format!("{v:?}")),
        "Race mismatch after round-trip"
    );

    let orig_count = original_fields
        .keys()
        .filter(|k| !k.starts_with("__"))
        .count();
    let reparse_count = reparsed.keys().filter(|k| !k.starts_with("__")).count();
    assert_eq!(
        orig_count, reparse_count,
        "Field count mismatch after round-trip"
    );
}

#[test]
fn test_bic_sync_preserves_bic_only_fields_and_updates_matching() {
    use app_lib::parsers::gff::{GffParser, GffValue, GffWriter};

    let bic_data = std::fs::read(fixtures_dir().join("player.bic")).expect("read bic fixture");
    let ifo_data = std::fs::read(fixtures_dir().join("playerlist.ifo")).expect("read ifo fixture");

    let bic_gff = GffParser::from_bytes(bic_data).expect("parse bic");
    let bic_fields = bic_gff.read_struct_fields(0).expect("read bic fields");
    let root_struct_id = bic_gff.get_struct_id(0).expect("get root struct_id");

    let char_fields = parse_playerlist_ifo(ifo_data);

    // Collect BIC-only keys before merge
    let bic_only_keys: Vec<String> = bic_fields
        .keys()
        .filter(|k| !k.starts_with("__") && !char_fields.contains_key(*k))
        .cloned()
        .collect();

    // Merge: overwrite matching fields from char_fields
    let mut merged: IndexMap<String, GffValue<'static>> = bic_fields
        .into_iter()
        .map(|(k, v)| (k, v.force_owned().expect("force owned")))
        .collect();

    for (key, value) in &char_fields {
        if key.starts_with("__") {
            continue;
        }
        if merged.contains_key(key) {
            merged.insert(key.clone(), value.clone());
        }
    }

    // BIC-only fields should still be present
    for key in &bic_only_keys {
        assert!(
            merged.contains_key(key),
            "BIC-only field '{key}' was lost during merge"
        );
    }

    // Write and re-parse to verify serialization
    let mut writer = GffWriter::new("BIC ", "V3.2");
    let bic_bytes = writer
        .write_with_struct_id(merged, root_struct_id)
        .expect("Failed to write merged BIC");

    let reparsed_gff = GffParser::from_bytes(bic_bytes).expect("Failed to re-parse BIC");
    let reparsed = reparsed_gff
        .read_struct_fields(0)
        .expect("read reparsed bic");

    for key in &bic_only_keys {
        assert!(
            reparsed.contains_key(key),
            "BIC-only field '{key}' was lost after serialization"
        );
    }
}

#[test]
fn test_playerinfo_bin_sync() {
    use app_lib::parsers::gff::{GffValue, LocalizedString, LocalizedSubstring};
    use app_lib::services::playerinfo::PlayerInfo;

    let fixture_path = fixtures_dir().join("playerinfo.bin");

    // Load original to capture fields that `update_from_gff_data` should preserve
    // when the matching GFF inputs are absent.
    let original = PlayerInfo::load(&fixture_path).expect("load original playerinfo.bin");
    let original_alignment_vertical = original.data.alignment_vertical;
    let original_alignment_horizontal = original.data.alignment_horizontal;
    let original_background_id = original.data.background_id;
    let original_unknown1 = original.data.unknown1;

    // Copy fixture to temp file for modification
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let temp_path = temp_dir.path().join("playerinfo.bin");
    std::fs::copy(&fixture_path, &temp_path).expect("copy fixture");

    // Load and update with test data
    let mut player_info = PlayerInfo::load(&temp_path).expect("load temp playerinfo.bin");

    let mut fields = IndexMap::new();
    fields.insert(
        "FirstName".to_string(),
        GffValue::LocString(LocalizedString {
            string_ref: -1,
            substrings: vec![LocalizedSubstring {
                language: 0,
                gender: 0,
                string: "TestFirst".into(),
            }],
        }),
    );
    fields.insert(
        "LastName".to_string(),
        GffValue::LocString(LocalizedString {
            string_ref: -1,
            substrings: vec![LocalizedSubstring {
                language: 0,
                gender: 0,
                string: "TestLast".into(),
            }],
        }),
    );
    fields.insert("Deity".to_string(), GffValue::String("Tyr".into()));
    fields.insert("Str".to_string(), GffValue::Byte(18));
    fields.insert("Dex".to_string(), GffValue::Byte(14));
    fields.insert("Con".to_string(), GffValue::Byte(16));
    fields.insert("Int".to_string(), GffValue::Byte(12));
    fields.insert("Wis".to_string(), GffValue::Byte(10));
    fields.insert("Cha".to_string(), GffValue::Byte(8));

    player_info.update_from_gff_data(
        &fields,
        "Tiefling",
        "Chaotic Good",
        &[("Fighter".to_string(), 10), ("Rogue".to_string(), 5)],
    );

    player_info
        .save(&temp_path)
        .expect("save updated playerinfo.bin");

    // Reload and verify
    let reloaded = PlayerInfo::load(&temp_path).expect("reload playerinfo.bin");

    assert_eq!(reloaded.data.first_name, "TestFirst");
    assert_eq!(reloaded.data.last_name, "TestLast");
    assert_eq!(reloaded.data.name, "TestFirst TestLast");
    assert_eq!(reloaded.data.deity, "Tyr");
    assert_eq!(reloaded.data.subrace, "Tiefling");
    assert_eq!(reloaded.data.alignment, "Chaotic Good");
    assert_eq!(reloaded.data.str_score, 18);
    assert_eq!(reloaded.data.dex_score, 14);
    assert_eq!(reloaded.data.con_score, 16);
    assert_eq!(reloaded.data.int_score, 12);
    assert_eq!(reloaded.data.wis_score, 10);
    assert_eq!(reloaded.data.cha_score, 8);
    assert_eq!(reloaded.data.classes.len(), 2);
    assert_eq!(reloaded.data.classes[0].name, "Fighter");
    assert_eq!(reloaded.data.classes[0].level, 10);
    assert_eq!(reloaded.data.classes[1].name, "Rogue");
    assert_eq!(reloaded.data.classes[1].level, 5);

    // Fields that must be preserved: no LawfulChaotic/GoodEvil/CharBackground in the GFF
    // inputs above, so their playerinfo.bin counterparts must stay at fixture values.
    assert_eq!(
        reloaded.data.alignment_vertical, original_alignment_vertical,
        "alignment_vertical must be preserved when GoodEvil is absent"
    );
    assert_eq!(
        reloaded.data.alignment_horizontal, original_alignment_horizontal,
        "alignment_horizontal must be preserved when LawfulChaotic is absent"
    );
    assert_eq!(
        reloaded.data.background_id, original_background_id,
        "background_id must be preserved when CharBackground is absent"
    );
    assert_eq!(
        reloaded.data.unknown1, original_unknown1,
        "unknown1 (no-last-name padding) must be preserved"
    );

    // Ability modifiers must be recomputed from the supplied scores.
    assert_eq!(reloaded.data.str_mod, 4); // (18-10)/2
    assert_eq!(reloaded.data.dex_mod, 2); // (14-10)/2
    assert_eq!(reloaded.data.con_mod, 3); // (16-10)/2
    assert_eq!(reloaded.data.int_mod, 1); // (12-10)/2
    assert_eq!(reloaded.data.wis_mod, 0); // (10-10)/2
    assert_eq!(reloaded.data.cha_mod, -1); // (8-10)/2
}
