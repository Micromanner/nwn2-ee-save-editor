mod common;

use app_lib::character::gff_helpers::gff_value_to_i32;
use app_lib::parsers::gff::{GffParser, GffValue};

#[tokio::test]
async fn investigate_skin_deep_scan() {
    println!("\n=== STARTING DEEP GFF SCAN FOR SKILL BONUSES ===");

    let fixtures = vec![
        "player.bic",
        "ryathstrongarm/ryathstrongarm1.bic",
        "ryathstrongarm/ryathstrongarm4.bic",
        "okkugodofbears/okkugodofbears1.bic",
    ];

    for fixture_name in fixtures {
        println!("\n>>> Scanning Fixture: {} <<<", fixture_name);

        let data = match std::panic::catch_unwind(|| common::load_test_gff(fixture_name)) {
            Ok(d) => d,
            Err(_) => {
                println!("  [!] Failed to load fixture.");
                continue;
            }
        };

        let parser = GffParser::from_bytes(data).expect("Failed to parse GFF");
        // Read top level struct
        match parser.read_struct_fields(0) {
            Ok(fields) => {
                let root = GffValue::StructOwned(Box::new(fields));
                scan_value(&root, "Root", 0);
            }
            Err(e) => println!("  [!] Failed to read root struct: {:?}", e),
        }
    }

    println!("\n=== DEEP SCAN COMPLETE ===");
}

fn scan_value(val: &GffValue, path: &str, depth: usize) {
    if depth > 20 {
        return;
    } // Prevent infinite recursion if any

    match val {
        GffValue::StructOwned(fields) => scan_struct(fields, path, depth),
        GffValue::ListOwned(list) => {
            for (idx, item) in list.iter().enumerate() {
                let new_path = format!("{}[{}]", path, idx);
                check_item_properties(item, &new_path);
                scan_struct(item, &new_path, depth + 1);
            }
        }
        GffValue::List(list) => {
            for (idx, lazy) in list.iter().enumerate() {
                let item = lazy.force_load().expect("load struct");
                let new_path = format!("{}[{}]", path, idx);
                check_item_properties(&item, &new_path);
                let struct_val = GffValue::StructOwned(Box::new(item));
                scan_value(&struct_val, &new_path, depth + 1);
            }
        }
        GffValue::Struct(lazy) => {
            let item = lazy.force_load().expect("load struct");
            let struct_val = GffValue::StructOwned(Box::new(item));
            scan_value(&struct_val, path, depth);
        }
        _ => {}
    }
}

fn scan_struct(fields: &indexmap::IndexMap<String, GffValue>, path: &str, depth: usize) {
    for (key, val) in fields {
        let new_path = format!("{}.{}", path, key);
        scan_value(val, &new_path, depth + 1);
    }
}

fn check_item_properties(item: &indexmap::IndexMap<String, GffValue>, path: &str) {
    // Only check if it has PropertiesList
    if let Some(props_val) = item.get("PropertiesList") {
        let mut properties = Vec::new();

        match props_val {
            GffValue::ListOwned(l) => {
                for p in l {
                    properties.push(p.clone());
                }
            }
            GffValue::List(l) => {
                for lazy in l {
                    properties.push(lazy.force_load().expect("load struct"));
                }
            }
            _ => {}
        }

        for (pidx, prop) in properties.iter().enumerate() {
            let name = prop
                .get("PropertyName")
                .and_then(gff_value_to_i32)
                .unwrap_or(-1);
            let subtype = prop.get("Subtype").and_then(gff_value_to_i32).unwrap_or(-1);

            // Check for Skill Bonus (52)
            if name == 52 {
                let cost = prop
                    .get("CostValue")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(-1);
                let chance = prop
                    .get("ChanceAppear")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(100);

                println!(
                    "  [!] FOUND SKILL BONUS (Prop 52) at {}.PropertiesList[{}]",
                    path, pidx
                );
                println!("      Subtype: {} (Skill ID)", subtype);
                println!("      CostValue: {} (Bonus Amount)", cost);
                println!("      Chance: {}%", chance);

                // detailed item info
                let base_item = item
                    .get("BaseItem")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(-1);

                // Use debug format to avoid type issues
                let tag = item
                    .get("Tag")
                    .map(|v| format!("{:?}", v))
                    .unwrap_or_else(|| "?".to_string());

                println!("      Item Info: BaseItem={} Tag={}", base_item, tag);

                // Check if it's on a skin
                if base_item == 69 || base_item == 49 {
                    println!("      ===> THIS IS LIKELY A SKIN/CREATURE ITEM <===");
                }

                // If found in Equip_ItemList, note it
                if path.contains("Equip_ItemList") {
                    println!("      --> Located in EQUIPMENT");
                } else if path.contains("ItemList") {
                    println!("      --> Located in INVENTORY");
                }
            }
        }
    }
}
