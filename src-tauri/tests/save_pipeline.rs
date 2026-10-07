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
