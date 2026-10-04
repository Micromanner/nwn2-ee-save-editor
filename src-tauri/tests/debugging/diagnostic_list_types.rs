//! Diagnostic: dump actual GFF field types from real `.cam` and `module.ifo` files.
//!
//! Purpose: confirm whether the writers in `services/campaign/settings.rs` and
//! `services/campaign/content.rs` are mutating GFF field types on save (same bug
//! class as commits be01e0f / 9fbe545 fixed for `player.bic`).
//!
//! NOTE: files in `tests/debugging/` are NOT compiled as integration tests by
//! default (no aggregator). To run, copy this file to `tests/diagnostic_list_types.rs`
//! then:
//!   cargo test --test diagnostic_list_types -- --ignored --nocapture

use app_lib::parsers::erf::ErfParser;
use app_lib::parsers::gff::{GffParser, GffValue, variant_name};
use indexmap::IndexMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/saves")
}

fn dump_field(prefix: &str, fields: &IndexMap<String, GffValue<'_>>, key: &str) {
    match fields.get(key) {
        Some(v) => println!("  {prefix}{key:20} -> {}", variant_name(v)),
        None => println!("  {prefix}{key:20} -> <missing>"),
    }
}

fn decompress_z(path: &Path) -> Vec<u8> {
    let file = fs::File::open(path).expect("open .z file");
    let mut out = Vec::new();
    lzma_rust2::LzmaReader::new_mem_limit(file, u32::MAX, None)
        .expect("lzma decoder")
        .read_to_end(&mut out)
        .expect("decompress .z");
    out
}

#[test]
#[ignore = "diagnostic only; run with --ignored --nocapture"]
fn diagnostic_dump_cam_field_types() {
    let cam_files: Vec<PathBuf> = [
        "STORM_Campaign",
        "Classic_Campaign",
        "Westgate_Campaign",
        "Community_Campaign",
    ]
    .iter()
    .map(|s| fixtures_root().join(s).join("campaign.cam"))
    .filter(|p| p.exists())
    .collect();

    if cam_files.is_empty() {
        println!("No campaign.cam fixtures found");
        return;
    }

    let writer_keys = [
        "LvlCap",
        "XPCap",
        "CompXPWt",
        "HenchXPWt",
        "AttackNeut",
        "AutoXPAwd",
        "JournalSynch",
        "NoCharChanging",
        "UsePersonalRep",
        "GUID",
        "StartModule",
    ];

    for cam in &cam_files {
        println!("\n=== {} ===", cam.display());
        let bytes = fs::read(cam).expect("read .cam");
        let parser = GffParser::from_bytes(bytes).expect("parse .cam GFF");
        let root = parser.read_struct_fields(0).expect("root struct");

        println!("Root field types (writer-touched fields):");
        for k in &writer_keys {
            dump_field("", &root, k);
        }

        println!("\nAll root fields:");
        for (k, v) in &root {
            println!("  {k:25} -> {}", variant_name(v));
        }
    }
}

#[test]
#[ignore = "diagnostic only; run with --ignored --nocapture"]
fn diagnostic_dump_module_ifo_field_types() {
    let z_files: Vec<(PathBuf, PathBuf)> = [
        ("STORM_Campaign", "g_x2.z"),
        ("Classic_Campaign", "0_tutorial.z"),
        ("Classic_Campaign", "1100_west_harbor.z"),
        ("Westgate_Campaign", "westgate_ar1500.z"),
        ("Community_Campaign", "poe_intro.z"),
    ]
    .iter()
    .map(|(d, z)| {
        let dir = fixtures_root().join(d);
        (dir.clone(), dir.join(z))
    })
    .filter(|(_, p)| p.exists())
    .collect();

    if z_files.is_empty() {
        println!("No module .z fixtures found");
        return;
    }

    let writer_keys = [
        "Mod_StartYear",
        "Mod_StartMonth",
        "Mod_StartDay",
        "Mod_StartHour",
        "VarTable",
    ];

    let var_entry_keys = ["Name", "Type", "Value"];

    for (campaign_dir, z_path) in &z_files {
        let _ = campaign_dir;
        println!("\n=== {} ===", z_path.display());
        let decompressed = decompress_z(z_path);

        let mut erf = ErfParser::new();
        erf.parse_from_bytes(&decompressed).expect("parse ERF");
        let module_ifo_bytes = match erf.extract_resource("module.ifo") {
            Ok(b) => b,
            Err(e) => {
                println!("  no module.ifo in this .z: {e}");
                continue;
            }
        };

        let gff = GffParser::from_bytes(module_ifo_bytes).expect("parse module.ifo");
        let root = gff.read_struct_fields(0).expect("root struct");

        println!("Root field types (writer-touched fields):");
        for k in &writer_keys {
            dump_field("", &root, k);
        }

        if let Some(GffValue::List(entries)) = root.get("VarTable") {
            println!("\nVarTable entry count: {}", entries.len());
            let mut type_field_variants: std::collections::BTreeMap<String, usize> =
                std::collections::BTreeMap::new();
            let mut value_field_variants: std::collections::BTreeMap<String, usize> =
                std::collections::BTreeMap::new();

            for (i, entry) in entries.iter().take(5).enumerate() {
                let fields = entry.force_load().expect("load struct");
                println!("  -- Entry {i}:");
                for k in &var_entry_keys {
                    dump_field("    ", &fields, k);
                }
            }
            for entry in entries {
                let fields = entry.force_load().expect("load struct");
                if let Some(v) = fields.get("Type") {
                    *type_field_variants
                        .entry(variant_name(v).to_string())
                        .or_insert(0) += 1;
                }
                if let Some(v) = fields.get("Value") {
                    *value_field_variants
                        .entry(variant_name(v).to_string())
                        .or_insert(0) += 1;
                }
            }
            println!("  Type field variant counts: {type_field_variants:?}");
            println!("  Value field variant counts: {value_field_variants:?}");
        } else if let Some(GffValue::ListOwned(entries)) = root.get("VarTable") {
            println!("\nVarTable (owned) entry count: {}", entries.len());
        } else {
            println!("\n(no VarTable on this module)");
        }
    }
}
