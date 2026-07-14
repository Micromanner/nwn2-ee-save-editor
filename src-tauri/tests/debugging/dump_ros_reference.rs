//! Dump companion (.ros) files and ROSTER.rst from the OC "SAVE GAME COMPLETE"
//! fixture to understand the companion file structure for the companion editor.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use app_lib::parsers::gff::{GffParser, GffValue};
use indexmap::IndexMap;

use super::dump_gff_reference::{dump_character_fields, gff_type_name, output_dir, write_json};

fn companion_save_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/saves/000055 - SAVE GAME COMPLETE")
}

fn parse_root(path: &std::path::Path) -> IndexMap<String, GffValue<'static>> {
    let bytes = fs::read(path).expect("read file");
    let parser = GffParser::from_bytes(bytes).expect("parse GFF");
    println!(
        "  Header: type={}, version={}",
        parser.file_type, parser.file_version
    );
    parser
        .read_struct_fields(0)
        .expect("root struct")
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect()
}

#[test]
fn dump_ros_structured() {
    let save_dir = companion_save_dir();
    for name in ["khelgar", "neeshka", "construct"] {
        let path = save_dir.join(format!("{name}.ros"));
        println!("Dumping {name}.ros:");
        let fields = parse_root(&path);
        dump_character_fields(&fields, &output_dir().join("ros").join(name));
    }
}

#[test]
fn dump_roster_rst() {
    let path = companion_save_dir().join("ROSTER.rst");
    println!("Dumping ROSTER.rst:");
    let fields = parse_root(&path);

    let rst_dir = output_dir().join("ros").join("_roster");
    fs::create_dir_all(&rst_dir).expect("create dir");

    let index: BTreeMap<&str, &str> = fields
        .iter()
        .map(|(k, v)| (k.as_str(), gff_type_name(v)))
        .collect();
    write_json(
        &rst_dir.join("_field_index.json"),
        &serde_json::to_value(&index).expect("serialize"),
    );

    let json = serde_json::to_value(&fields).expect("serialize");
    write_json(&rst_dir.join("roster.json"), &json);
}

/// Compare field sets: what does a .ros have that player.bic lacks and vice versa.
#[test]
fn diff_ros_vs_bic_fields() {
    let save_dir = companion_save_dir();
    let ros_fields = parse_root(&save_dir.join("khelgar.ros"));
    let bic_fields = parse_root(&save_dir.join("player.bic"));

    let ros_keys: std::collections::BTreeSet<&str> =
        ros_fields.keys().map(|k| k.as_str()).collect();
    let bic_keys: std::collections::BTreeSet<&str> =
        bic_fields.keys().map(|k| k.as_str()).collect();

    println!(
        "\n=== Fields only in khelgar.ros ({}):",
        ros_keys.difference(&bic_keys).count()
    );
    for k in ros_keys.difference(&bic_keys) {
        println!("  {k}: {}", gff_type_name(&ros_fields[*k]));
    }
    println!(
        "\n=== Fields only in player.bic ({}):",
        bic_keys.difference(&ros_keys).count()
    );
    for k in bic_keys.difference(&ros_keys) {
        println!("  {k}: {}", gff_type_name(&bic_fields[*k]));
    }
    println!(
        "\n=== Shared fields: {}",
        ros_keys.intersection(&bic_keys).count()
    );
}
