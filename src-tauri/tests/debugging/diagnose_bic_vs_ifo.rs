use app_lib::parsers::gff::parser::GffParser;
use app_lib::parsers::gff::types::GffValue;
use app_lib::services::savegame_handler::SaveGameHandler;
use indexmap::IndexMap;
use std::collections::BTreeSet;
use std::path::Path;

const FIXTURE_SAVES: &[&str] = &[
    "tests/fixtures/saves/Classic_Campaign",
    "tests/fixtures/saves/Community_Campaign",
    "tests/fixtures/saves/MOTB",
    "tests/fixtures/saves/STORM_Campaign",
    "tests/fixtures/saves/Westgate_Campaign",
];

fn load_ifo_character(save_path: &Path) -> IndexMap<String, GffValue<'static>> {
    let handler = SaveGameHandler::new(save_path, false).expect("handler");
    let data = handler.extract_player_data().expect("extract ifo");
    let gff = GffParser::from_bytes(data).expect("parse ifo");
    let root = gff.read_struct_fields(0).expect("root");
    let list = match root.get("Mod_PlayerList").expect("Mod_PlayerList") {
        GffValue::List(lazy_structs) => lazy_structs,
        _ => panic!("not a list"),
    };
    let fields = list.first().expect("empty list").force_load().expect("load struct");
    fields
        .into_iter()
        .map(|(k, v)| (k, v.force_owned().expect("force owned")))
        .collect()
}

fn load_bic_character(save_path: &Path) -> Option<IndexMap<String, GffValue<'static>>> {
    let handler = SaveGameHandler::new(save_path, false).expect("handler");
    let data = handler.extract_player_bic().ok()??;
    let gff = GffParser::from_bytes(data).expect("parse bic");
    let fields = gff.read_struct_fields(0).expect("root");
    Some(
        fields
            .into_iter()
            .map(|(k, v)| (k, v.force_owned().expect("force owned")))
            .collect(),
    )
}

fn variant_name(v: &GffValue) -> &'static str {
    match v {
        GffValue::Byte(_) => "Byte",
        GffValue::Char(_) => "Char",
        GffValue::Word(_) => "Word",
        GffValue::Short(_) => "Short",
        GffValue::Dword(_) => "Dword",
        GffValue::Int(_) => "Int",
        GffValue::Dword64(_) => "Dword64",
        GffValue::Int64(_) => "Int64",
        GffValue::Float(_) => "Float",
        GffValue::Double(_) => "Double",
        GffValue::String(_) => "String",
        GffValue::ResRef(_) => "ResRef",
        GffValue::LocString(_) => "LocString",
        GffValue::Void(_) => "Void",
        GffValue::Struct(_) | GffValue::StructOwned(_) | GffValue::StructRef(_) => "Struct",
        GffValue::List(_) | GffValue::ListOwned(_) | GffValue::ListRef(_) => "List",
    }
}

fn count_struct_fields(v: &GffValue) -> usize {
    match v {
        GffValue::StructOwned(m) => m.len(),
        GffValue::Struct(lazy) => lazy.force_load().expect("load struct").len(),
        _ => 0,
    }
}

fn count_list_items(v: &GffValue) -> usize {
    match v {
        GffValue::ListOwned(items) => items.len(),
        GffValue::List(lazys) => lazys.len(),
        _ => 0,
    }
}

fn get_struct_keys(v: &GffValue) -> BTreeSet<String> {
    match v {
        GffValue::StructOwned(m) => m.keys().cloned().collect(),
        GffValue::Struct(lazy) => lazy.force_load().expect("load struct").keys().cloned().collect(),
        _ => BTreeSet::new(),
    }
}

fn get_list_struct_ids(v: &GffValue) -> Vec<u32> {
    match v {
        GffValue::ListOwned(items) => items
            .iter()
            .map(|item| match item.get("__struct_id__") {
                Some(GffValue::Dword(id)) => *id,
                _ => 0,
            })
            .collect(),
        GffValue::List(lazys) => lazys.iter().map(|l| l.struct_id).collect(),
        _ => vec![],
    }
}

/// Q1: Field-by-field comparison between BIC and IFO
#[test]
fn compare_top_level_fields() {
    for save_name in FIXTURE_SAVES {
        let path = Path::new(save_name);
        if !path.exists() {
            println!("SKIP: {save_name}");
            continue;
        }

        let name = path.file_name().unwrap().to_str().unwrap();
        println!("\n{}", "=".repeat(60));
        println!("=== {name} ===");

        let ifo = load_ifo_character(path);
        let bic = match load_bic_character(path) {
            Some(b) => b,
            None => {
                println!("  NO player.bic!");
                continue;
            }
        };

        let ifo_keys: BTreeSet<_> = ifo.keys().cloned().collect();
        let bic_keys: BTreeSet<_> = bic.keys().cloned().collect();

        let only_ifo: BTreeSet<_> = ifo_keys.difference(&bic_keys).collect();
        let only_bic: BTreeSet<_> = bic_keys.difference(&ifo_keys).collect();

        if !only_ifo.is_empty() {
            println!("  Fields ONLY in IFO: {:?}", only_ifo);
        }
        if !only_bic.is_empty() {
            println!("  Fields ONLY in BIC: {:?}", only_bic);
        }

        let common_keys: BTreeSet<_> = ifo_keys.intersection(&bic_keys).collect();
        println!("  Common fields: {}", common_keys.len());
        println!(
            "  IFO total: {}, BIC total: {}",
            ifo_keys.len(),
            bic_keys.len()
        );

        // Check type mismatches and structural differences
        let mut type_mismatches = vec![];
        let mut struct_diffs = vec![];
        let mut list_diffs = vec![];

        for key in &common_keys {
            let ifo_val = ifo.get(*key).unwrap();
            let bic_val = bic.get(*key).unwrap();

            let ifo_type = variant_name(ifo_val);
            let bic_type = variant_name(bic_val);

            if ifo_type != bic_type {
                type_mismatches.push(format!("    {key}: IFO={ifo_type}, BIC={bic_type}"));
            }

            if ifo_type == "Struct" {
                let ifo_count = count_struct_fields(ifo_val);
                let bic_count = count_struct_fields(bic_val);
                if ifo_count != bic_count {
                    let ifo_keys_s = get_struct_keys(ifo_val);
                    let bic_keys_s = get_struct_keys(bic_val);
                    let missing: BTreeSet<_> = bic_keys_s.difference(&ifo_keys_s).collect();
                    let extra: BTreeSet<_> = ifo_keys_s.difference(&bic_keys_s).collect();
                    struct_diffs.push(format!(
                        "    {key}: IFO has {ifo_count} fields, BIC has {bic_count} fields \
                         (IFO missing: {missing:?}, IFO extra: {extra:?})"
                    ));
                }
            }

            if ifo_type == "List" {
                let ifo_count = count_list_items(ifo_val);
                let bic_count = count_list_items(bic_val);
                let ifo_ids = get_list_struct_ids(ifo_val);
                let bic_ids = get_list_struct_ids(bic_val);
                if ifo_count != bic_count || ifo_ids != bic_ids {
                    list_diffs.push(format!(
                        "    {key}: IFO={ifo_count} items (ids={ifo_ids:?}), \
                         BIC={bic_count} items (ids={bic_ids:?})"
                    ));
                }
            }
        }

        if !type_mismatches.is_empty() {
            println!("  TYPE MISMATCHES:");
            for m in &type_mismatches {
                println!("{m}");
            }
        }
        if !struct_diffs.is_empty() {
            println!("  STRUCT FIELD COUNT DIFFERENCES:");
            for d in &struct_diffs {
                println!("{d}");
            }
        }
        if !list_diffs.is_empty() {
            println!("  LIST DIFFERENCES (count or struct_ids):");
            for d in &list_diffs {
                println!("{d}");
            }
        }
    }
}

/// Q2/Q3: Check if all fixture saves have player.bic
#[test]
fn check_player_bic_presence() {
    println!("\n=== player.bic presence across all fixture saves ===\n");
    for save_name in FIXTURE_SAVES {
        let path = Path::new(save_name);
        if !path.exists() {
            println!("  SKIP (not found): {save_name}");
            continue;
        }
        let handler = SaveGameHandler::new(path, false).expect("handler");

        let has_ifo = handler.extract_player_data().is_ok();
        let has_bic = handler.extract_player_bic().ok().and_then(|v| v).is_some();

        let ifo_size = handler.extract_player_data().map(|d| d.len()).unwrap_or(0);
        let bic_size = handler
            .extract_player_bic()
            .ok()
            .and_then(|v| v)
            .map(|d| d.len())
            .unwrap_or(0);

        let name = Path::new(save_name).file_name().unwrap().to_str().unwrap();
        println!("  {name}: ifo={has_ifo} ({ifo_size} bytes), bic={has_bic} ({bic_size} bytes)");
    }
}

/// Q4: Deep scan for ALL nested struct/list differences (not just Equip_ItemList/Tintable)
#[test]
fn deep_nested_struct_comparison() {
    for save_name in FIXTURE_SAVES {
        let path = Path::new(save_name);
        if !path.exists() {
            continue;
        }

        let name = path.file_name().unwrap().to_str().unwrap();
        println!("\n=== {name}: Deep nested comparison ===");

        let ifo = load_ifo_character(path);
        let bic = match load_bic_character(path) {
            Some(b) => b,
            None => continue,
        };

        compare_recursive(&ifo, &bic, "root", 0);
    }
}

fn compare_recursive(
    ifo_fields: &IndexMap<String, GffValue>,
    bic_fields: &IndexMap<String, GffValue>,
    path: &str,
    depth: usize,
) {
    if depth > 3 {
        return; // Limit recursion depth
    }

    let common_keys: BTreeSet<_> = ifo_fields
        .keys()
        .filter(|k| bic_fields.contains_key(*k))
        .cloned()
        .collect();

    for key in &common_keys {
        let ifo_val = ifo_fields.get(key).unwrap();
        let bic_val = bic_fields.get(key).unwrap();
        let field_path = format!("{path}.{key}");

        match (ifo_val, bic_val) {
            (GffValue::StructOwned(ifo_map), GffValue::StructOwned(bic_map)) => {
                let ifo_k: BTreeSet<_> = ifo_map.keys().cloned().collect();
                let bic_k: BTreeSet<_> = bic_map.keys().cloned().collect();
                let missing: Vec<_> = bic_k.difference(&ifo_k).collect();
                let extra: Vec<_> = ifo_k.difference(&bic_k).collect();
                if !missing.is_empty() || !extra.is_empty() {
                    println!("  STRUCT {field_path}: IFO missing {missing:?}, IFO extra {extra:?}");
                }
                compare_recursive(ifo_map, bic_map, &field_path, depth + 1);
            }
            (GffValue::ListOwned(ifo_items), GffValue::ListOwned(bic_items)) => {
                if ifo_items.len() != bic_items.len() {
                    println!(
                        "  LIST {field_path}: IFO={} items, BIC={} items",
                        ifo_items.len(),
                        bic_items.len()
                    );
                }
                // Compare struct_ids
                let ifo_ids: Vec<u32> = ifo_items
                    .iter()
                    .map(|i| match i.get("__struct_id__") {
                        Some(GffValue::Dword(id)) => *id,
                        _ => 0,
                    })
                    .collect();
                let bic_ids: Vec<u32> = bic_items
                    .iter()
                    .map(|i| match i.get("__struct_id__") {
                        Some(GffValue::Dword(id)) => *id,
                        _ => 0,
                    })
                    .collect();
                if ifo_ids != bic_ids {
                    println!("  LIST {field_path} struct_ids: IFO={ifo_ids:?}, BIC={bic_ids:?}");
                }

                // Compare field presence in first item of each
                let min_len = ifo_items.len().min(bic_items.len());
                for i in 0..min_len {
                    compare_recursive(
                        &ifo_items[i],
                        &bic_items[i],
                        &format!("{field_path}[{i}]"),
                        depth + 1,
                    );
                }
            }
            _ => {}
        }
    }
}

/// Specific deep-dive: Equip_ItemList Tintable comparison
#[test]
fn equip_item_list_tintable_detail() {
    for save_name in FIXTURE_SAVES {
        let path = Path::new(save_name);
        if !path.exists() {
            continue;
        }

        let name = path.file_name().unwrap().to_str().unwrap();
        println!("\n=== {name}: Equip_ItemList tintable detail ===");

        let ifo = load_ifo_character(path);
        let bic = match load_bic_character(path) {
            Some(b) => b,
            None => continue,
        };

        for (source_name, fields) in [("IFO", &ifo), ("BIC", &bic)] {
            println!("  --- {source_name} ---");
            if let Some(GffValue::ListOwned(items)) = fields.get("Equip_ItemList") {
                for (i, item) in items.iter().enumerate() {
                    let struct_id = match item.get("__struct_id__") {
                        Some(GffValue::Dword(v)) => *v,
                        _ => 0,
                    };
                    let tag = match item.get("Tag") {
                        Some(GffValue::String(s)) => s.to_string(),
                        _ => "-".into(),
                    };
                    let has_tintable = item.contains_key("Tintable");
                    let has_tint =
                        if let Some(GffValue::StructOwned(tintable)) = item.get("Tintable") {
                            tintable.contains_key("Tint")
                        } else {
                            false
                        };
                    println!(
                        "    [{i}] struct_id={struct_id}, tag={tag}, \
                         tintable={has_tintable}, has_tint={has_tint}"
                    );
                }
            } else {
                println!("    No Equip_ItemList");
            }
        }
    }
}
