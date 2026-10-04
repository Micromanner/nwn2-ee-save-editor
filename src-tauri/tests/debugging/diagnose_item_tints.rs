use app_lib::parsers::gff::types::GffValue;
use app_lib::services::resource_manager::ResourceManager;
use app_lib::services::savegame_handler::SaveGameHandler;
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

fn get_byte(fields: &indexmap::IndexMap<String, GffValue<'_>>, key: &str) -> u8 {
    match fields.get(key) {
        Some(GffValue::Byte(b)) => *b,
        _ => 0,
    }
}

fn resolve_struct<'a>(val: &'a GffValue<'a>) -> Option<indexmap::IndexMap<String, GffValue<'a>>> {
    match val {
        GffValue::StructOwned(s) => Some(s.as_ref().clone()),
        GffValue::Struct(lazy) => Some(lazy.force_load().expect("load struct")),
        _ => None,
    }
}

fn dump_tintable(
    out: &mut String,
    tintable: &indexmap::IndexMap<String, GffValue<'_>>,
    indent: &str,
) {
    let tint = match tintable.get("Tint") {
        Some(val) => resolve_struct(val),
        None => {
            writeln!(out, "{indent}No Tint field").unwrap();
            return;
        }
    };
    let Some(tint) = tint else {
        writeln!(out, "{indent}Tint is not a struct").unwrap();
        return;
    };
    for ch_key in ["1", "2", "3"] {
        if let Some(ch_val) = tint.get(ch_key) {
            if let Some(ch) = resolve_struct(ch_val) {
                let r = get_byte(&ch, "r");
                let g = get_byte(&ch, "g");
                let b = get_byte(&ch, "b");
                let a = get_byte(&ch, "a");
                writeln!(out, "{indent}Channel {ch_key}: r={r}, g={g}, b={b}, a={a}").unwrap();
            } else {
                writeln!(out, "{indent}Channel {ch_key}: not a struct").unwrap();
            }
        } else {
            writeln!(out, "{indent}Channel {ch_key}: missing").unwrap();
        }
    }
}

fn dump_gff_tintable(
    out: &mut String,
    fields: &indexmap::IndexMap<String, GffValue<'_>>,
    label: &str,
) {
    writeln!(out, "\n{label}:").unwrap();
    match fields.get("Tintable") {
        Some(GffValue::StructOwned(s)) => dump_tintable(out, s, "  "),
        Some(GffValue::Struct(lazy)) => dump_tintable(out, &lazy.force_load().expect("load struct"), "  "),
        _ => {
            writeln!(out, "  No Tintable field").unwrap();
        }
    }
}

#[tokio::test]
async fn diagnose_item_tints() {
    let mut out = String::new();

    // 1. Load item TEMPLATE from game data
    let nwn2_paths = Arc::new(RwLock::new(app_lib::config::NWN2Paths::new()));
    let rm = Arc::new(RwLock::new(ResourceManager::new(nwn2_paths)));
    {
        let mut rm_guard = rm.write().await;
        let _ = rm_guard.initialize().await;
    }
    let rm_read = rm.read().await;

    writeln!(out, "=== Item Template: nw_maarcl002.uti ===").unwrap();
    if let Ok(uti_bytes) = rm_read.get_resource_bytes("nw_maarcl002", "uti") {
        let gff =
            app_lib::parsers::gff::parser::GffParser::from_bytes(uti_bytes).expect("parse uti");
        let fields = gff.read_struct_fields(0).expect("read root");
        let owned: indexmap::IndexMap<String, GffValue<'static>> = fields
            .into_iter()
            .map(|(k, v)| (k, v.force_owned().expect("force owned")))
            .collect();

        // Dump ALL fields that might be relevant
        for key in [
            "ArmorVisualType",
            "Variation",
            "BaseItem",
            "TemplateResRef",
            "LocalizedName",
        ] {
            if let Some(val) = owned.get(key) {
                writeln!(out, "  {key}: {val:?}").unwrap();
            }
        }
        dump_gff_tintable(&mut out, &owned, "Template Tintable");

        // Check for AC* fields on template
        let ac_fields = ["ACLtShoulder", "ACRtShoulder", "ACLtBracer", "ACRtBracer"];
        for ac in &ac_fields {
            if let Some(val) = owned.get(*ac) {
                writeln!(out, "  {ac}: present ({:?})", std::mem::discriminant(val)).unwrap();
            }
        }
    } else {
        writeln!(out, "  NOT FOUND").unwrap();
    }

    // 2. Load SAME item from save file's Equip_ItemList
    let save_path = PathBuf::from(
        r"C:\Users\01tee\Documents\Neverwinter Nights 2\saves\000053 - 10-04-2026-12-34",
    );
    if !save_path.exists() {
        writeln!(out, "\nSave not found, skipping save comparison").unwrap();
    } else {
        let handler = SaveGameHandler::new(&save_path, false).expect("handler");
        let bic_data = handler
            .extract_player_bic()
            .expect("extract")
            .expect("no bic");
        let gff = app_lib::parsers::gff::parser::GffParser::from_bytes(bic_data).expect("parse");
        let fields = gff.read_struct_fields(0).expect("read root");
        let owned: indexmap::IndexMap<String, GffValue<'static>> = fields
            .into_iter()
            .map(|(k, v)| (k, v.force_owned().expect("force owned")))
            .collect();

        // Find chest armor in Equip_ItemList
        if let Some(GffValue::ListOwned(equip_list)) = owned.get("Equip_ItemList") {
            let chest = equip_list
                .iter()
                .find(|item| matches!(item.get("__struct_id__"), Some(GffValue::Dword(2))));

            if let Some(chest_item) = chest {
                writeln!(out, "\n=== Equipped Chest Item (save) ===").unwrap();
                for key in ["TemplateResRef", "BaseItem", "ArmorVisualType", "Variation"] {
                    if let Some(val) = chest_item.get(key) {
                        writeln!(out, "  {key}: {val:?}").unwrap();
                    }
                }
                dump_gff_tintable(&mut out, chest_item, "Equipped Item Tintable");

                // Also dump ALL AC* tintables
                let ac_fields = [
                    "ACLtShoulder",
                    "ACRtShoulder",
                    "ACLtBracer",
                    "ACRtBracer",
                    "ACLtElbow",
                    "ACRtElbow",
                    "ACLtArm",
                    "ACRtArm",
                    "ACFtHip",
                    "ACBkHip",
                    "ACLtHip",
                    "ACRtHip",
                    "ACLtLeg",
                    "ACRtLeg",
                    "ACLtShin",
                    "ACRtShin",
                    "ACLtKnee",
                    "ACRtKnee",
                    "ACLtFoot",
                    "ACRtFoot",
                    "ACLtAnkle",
                    "ACRtAnkle",
                ];
                for ac in &ac_fields {
                    if let Some(ac_val) = chest_item.get(*ac) {
                        if let Some(s) = resolve_struct(ac_val) {
                            let accessory = get_byte(&s, "Accessory");
                            if accessory > 0 || s.contains_key("Tintable") {
                                writeln!(out, "\n  {ac}: Accessory={accessory}").unwrap();
                                if let Some(tintable_val) = s.get("Tintable") {
                                    if let Some(t) = resolve_struct(tintable_val) {
                                        dump_tintable(&mut out, &t, "    ");
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                writeln!(out, "\nNo chest armor equipped").unwrap();
            }
        }

        // Also dump character-level head/hair tints for comparison
        writeln!(out, "\n=== Character Tintable ===").unwrap();
        if let Some(val) = owned.get("Tintable") {
            if let Some(tintable) = resolve_struct(val) {
                dump_tintable(&mut out, &tintable, "  ");
            }
        }

        for field in ["Tint_Head", "Tint_Hair"] {
            writeln!(out, "\n=== {field} ===").unwrap();
            match owned.get(field) {
                Some(val) => {
                    if let Some(outer) = resolve_struct(val) {
                        if let Some(tintable_val) = outer.get("Tintable") {
                            if let Some(tintable) = resolve_struct(tintable_val) {
                                dump_tintable(&mut out, &tintable, "  ");
                            }
                        } else {
                            writeln!(
                                out,
                                "  No Tintable sub-key, keys: {:?}",
                                outer.keys().collect::<Vec<_>>()
                            )
                            .unwrap();
                        }
                    }
                }
                None => writeln!(out, "  NOT PRESENT").unwrap(),
            }
        }
    }

    let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target_test")
        .join("item_tint_comparison.txt");
    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
    std::fs::write(&out_path, &out).unwrap();
    eprintln!("Output: {}", out_path.display());
}
