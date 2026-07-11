use app_lib::parsers::gff::{GffParser, GffValue};
use app_lib::services::savegame_handler::SaveGameHandler;
use indexmap::IndexMap;
use serde_json::Value as JsonValue;
use std::fs;
use std::path::PathBuf;

fn output_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/investigation")
}

fn saves_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/saves/Classic_Campaign")
}

fn dump_to_json(fields: &IndexMap<String, GffValue<'static>>, path: &PathBuf) {
    let json = serde_json::to_value(fields).expect("serialize");
    let json_str = serde_json::to_string_pretty(&json).expect("pretty print");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, json_str).unwrap();
}

#[test]
fn compare_khelgar_and_playerlist() {
    let handler = SaveGameHandler::new(saves_dir(), false, false).expect("Failed to open save");

    // 1. Dump Khelgar.ros
    let khelgar_bytes = handler
        .extract_file("khelgar.ros")
        .expect("Failed to extract khelgar.ros");
    let parser = GffParser::from_bytes(khelgar_bytes).expect("Failed to parse ROS");
    let khelgar_fields = parser
        .read_struct_fields(0)
        .expect("Failed to read ROS fields");
    let khelgar_owned: IndexMap<String, GffValue<'static>> = khelgar_fields
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();
    dump_to_json(&khelgar_owned, &output_dir().join("khelgar_ros.json"));

    // 2. Dump playerlist.ifo
    let ifo_bytes = handler
        .extract_file("playerlist.ifo")
        .expect("Failed to extract playerlist.ifo");
    let parser_ifo = GffParser::from_bytes(ifo_bytes).expect("Failed to parse IFO");
    let ifo_fields = parser_ifo
        .read_struct_fields(0)
        .expect("Failed to read IFO fields");
    let ifo_owned: IndexMap<String, GffValue<'static>> = ifo_fields
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();
    dump_to_json(&ifo_owned, &output_dir().join("playerlist_ifo.json"));

    // 3. Dump player.bic
    let bic_bytes = handler
        .extract_file("player.bic")
        .expect("Failed to extract player.bic");
    let parser_bic = GffParser::from_bytes(bic_bytes).expect("Failed to parse BIC");
    let bic_fields = parser_bic
        .read_struct_fields(0)
        .expect("Failed to read BIC fields");
    let bic_owned: IndexMap<String, GffValue<'static>> = bic_fields
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();
    dump_to_json(&bic_owned, &output_dir().join("player_bic.json"));

    println!("Dumping complete. Files in {:?}", output_dir());
}
