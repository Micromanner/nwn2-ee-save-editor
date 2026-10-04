use app_lib::parsers::gff::parser::GffParser;
use app_lib::parsers::gff::types::GffValue;
use app_lib::services::savegame_handler::SaveGameHandler;
use std::path::PathBuf;

fn load_via_playerlist_ifo(
    save_path: &std::path::Path,
) -> indexmap::IndexMap<String, GffValue<'static>> {
    let handler = SaveGameHandler::new(save_path, false).expect("Failed to create handler");
    let data = handler
        .extract_player_data()
        .expect("Failed to extract playerlist.ifo");
    println!("  playerlist.ifo: {} bytes", data.len());
    let gff = GffParser::from_bytes(data).expect("Failed to parse playerlist.ifo");
    println!(
        "  GFF: type={}, version={}",
        gff.file_type, gff.file_version
    );
    let root = gff.read_struct_fields(0).expect("Failed to read root");
    let list = match root.get("Mod_PlayerList").expect("No Mod_PlayerList") {
        GffValue::List(lazy_structs) => lazy_structs,
        _ => panic!("Mod_PlayerList is not a list"),
    };
    list.first().expect("Empty Mod_PlayerList").force_load().expect("load struct")
}

fn load_via_bic(
    save_path: &std::path::Path,
) -> Option<indexmap::IndexMap<String, GffValue<'static>>> {
    let handler = SaveGameHandler::new(save_path, false).expect("Failed to create handler");
    let bic_data = handler.extract_player_bic().ok()??;
    println!("  player.bic: {} bytes", bic_data.len());
    let gff = GffParser::from_bytes(bic_data).expect("Failed to parse BIC");
    println!(
        "  GFF: type={}, version={}",
        gff.file_type, gff.file_version
    );
    let fields = gff
        .read_struct_fields(0)
        .expect("Failed to read root struct");
    Some(
        fields
            .into_iter()
            .map(|(k, v)| (k, v.into_owned()))
            .collect(),
    )
}

fn dump_equip_list(fields: &indexmap::IndexMap<String, GffValue<'static>>) {
    let equip_list = match fields.get("Equip_ItemList") {
        Some(GffValue::List(lazy_structs)) => {
            println!(
                "  Equip_ItemList: List with {} lazy structs",
                lazy_structs.len()
            );
            lazy_structs
                .iter()
                .map(|ls| {
                    println!(
                        "    LazyStruct: struct_index={}, struct_id={}",
                        ls.struct_index, ls.struct_id
                    );
                    ls.force_load().expect("load struct")
                })
                .collect::<Vec<_>>()
        }
        Some(GffValue::ListOwned(maps)) => {
            println!("  Equip_ItemList: ListOwned with {} entries", maps.len());
            maps.clone()
        }
        other => {
            println!(
                "  Equip_ItemList: {:?}",
                other.map(|v| std::mem::discriminant(v))
            );
            return;
        }
    };

    for (i, item) in equip_list.iter().enumerate() {
        let struct_id = item.get("__struct_id__");
        let base_item = item.get("BaseItem");
        let tag = item.get("Tag");

        let tag_str = match tag {
            Some(GffValue::String(s)) => s.to_string(),
            _ => "-".to_string(),
        };

        println!(
            "  [{}] struct_id={:?}, BaseItem={:?}, Tag={}",
            i, struct_id, base_item, tag_str
        );

        // Check Tintable deeply
        let tintable = item.get("Tintable");
        match tintable {
            Some(GffValue::StructOwned(s)) => {
                if let Some(GffValue::StructOwned(tint)) = s.get("Tint") {
                    for ch_key in ["1", "2", "3"] {
                        if let Some(GffValue::StructOwned(ch)) = tint.get(ch_key) {
                            let r = ch.get("r");
                            let g = ch.get("g");
                            let b = ch.get("b");
                            let a = ch.get("a");
                            println!(
                                "       Tint.{}: r={:?} g={:?} b={:?} a={:?}",
                                ch_key, r, g, b, a
                            );
                        }
                    }
                } else {
                    println!("       Tintable present but no Tint substruct");
                    println!("       Tintable keys: {:?}", s.keys().collect::<Vec<_>>());
                }
            }
            Some(GffValue::Struct(lazy)) => {
                let loaded = lazy.force_load().expect("load struct");
                println!(
                    "       Tintable (lazy): {:?}",
                    loaded.keys().collect::<Vec<_>>()
                );
            }
            None => println!("       No Tintable"),
            _ => println!("       Tintable: unexpected type"),
        }
    }
}

fn force_own_all(
    fields: indexmap::IndexMap<String, GffValue<'static>>,
) -> indexmap::IndexMap<String, GffValue<'static>> {
    fields
        .into_iter()
        .map(|(k, v)| (k, v.force_owned().expect("force owned")))
        .collect()
}

fn dump_value(value: &GffValue<'static>, indent: usize) {
    let pad = " ".repeat(indent);
    match value {
        GffValue::StructOwned(map) => {
            println!("{pad}Struct {{");
            for (k, v) in map.iter() {
                print!("{pad}  {k}: ");
                dump_value(v, indent + 2);
            }
            println!("{pad}}}");
        }
        GffValue::ListOwned(items) => {
            println!("{pad}List [{} items]", items.len());
            for (i, item) in items.iter().enumerate() {
                println!("{pad}  [{i}] {{");
                for (k, v) in item {
                    print!("{pad}    {k}: ");
                    dump_value(v, indent + 4);
                }
                println!("{pad}  }}");
            }
        }
        GffValue::Struct(lazy) => {
            let loaded = lazy.force_load().expect("load struct");
            print!("LazyStruct -> ");
            let owned = GffValue::StructOwned(Box::new(loaded));
            dump_value(&owned, indent);
        }
        GffValue::List(lazys) => {
            let owned: Vec<indexmap::IndexMap<String, GffValue<'static>>> =
                lazys.iter().map(|l| l.force_load().expect("load struct")).collect();
            dump_value(&GffValue::ListOwned(owned), indent);
        }
        other => println!("{other:?}"),
    }
}

#[test]
fn diagnose_chest_armor_full_dump() {
    let path = PathBuf::from(
        r"C:\Users\01tee\Documents\Neverwinter Nights 2\saves\000049 - 01-08-2025-11-19_backup_20250815_010644",
    );
    if !path.exists() {
        println!("Save not found, skipping");
        return;
    }

    let fields = load_via_bic(&path).expect("No player.bic");
    let owned = force_own_all(fields);

    let equip_list = match owned.get("Equip_ItemList") {
        Some(GffValue::ListOwned(maps)) => maps,
        _ => {
            println!("No Equip_ItemList");
            return;
        }
    };

    // Find chest armor (struct_id=2)
    for (i, item) in equip_list.iter().enumerate() {
        let struct_id = match item.get("__struct_id__") {
            Some(GffValue::Dword(v)) => *v,
            _ => 0,
        };
        if struct_id != 2 {
            continue;
        }
        println!("\n=== Chest Armor (item {i}, struct_id={struct_id}) - FULL DUMP ===");
        for (k, v) in item {
            print!("  {k}: ");
            dump_value(v, 2);
        }
    }
}

#[test]
fn diagnose_buggy_save_both_paths() {
    let path = PathBuf::from(
        r"C:\Users\01tee\Documents\Neverwinter Nights 2\saves\000049 - 01-08-2025-11-19_backup_20250815_010644",
    );
    if !path.exists() {
        println!("Buggy save not found, skipping");
        return;
    }

    println!("\n=== Path 1: playerlist.ifo (what the app uses) ===");
    let ifo_fields = load_via_playerlist_ifo(&path);
    println!("--- Raw (before force_owned) ---");
    dump_equip_list(&ifo_fields);
    println!("\n--- After force_owned ---");
    let ifo_owned = force_own_all(ifo_fields);
    dump_equip_list(&ifo_owned);

    println!("\n\n=== Path 2: player.bic (what previous test used) ===");
    if let Some(bic_fields) = load_via_bic(&path) {
        println!("--- Raw (before force_owned) ---");
        dump_equip_list(&bic_fields);
        println!("\n--- After force_owned ---");
        let bic_owned = force_own_all(bic_fields);
        dump_equip_list(&bic_owned);
    } else {
        println!("  No player.bic in this save");
    }
}
