use app_lib::parsers::gff::types::GffValue;
use app_lib::services::savegame_handler::SaveGameHandler;
use std::fmt::Write;
use std::path::PathBuf;

fn get_byte(fields: &indexmap::IndexMap<String, GffValue<'_>>, key: &str) -> u8 {
    match fields.get(key) {
        Some(GffValue::Byte(b)) => *b,
        _ => 0,
    }
}

fn resolve_struct<'a>(val: &'a GffValue<'a>) -> Option<indexmap::IndexMap<String, GffValue<'a>>> {
    match val {
        GffValue::StructOwned(s) => Some(s.as_ref().clone()),
        GffValue::Struct(lazy) => Some(lazy.force_load()),
        _ => None,
    }
}

#[test]
fn verify_tint_write() {
    let save_path = PathBuf::from(
        r"C:\Users\01tee\Documents\Neverwinter Nights 2\saves\000053 - 14-04-2026-14-02",
    );
    if !save_path.exists() {
        println!("Test save not found");
        return;
    }

    let handler = SaveGameHandler::new(&save_path, false).expect("handler");
    let bic_data = handler
        .extract_player_bic()
        .expect("extract")
        .expect("no bic");
    let gff = app_lib::parsers::gff::parser::GffParser::from_bytes(bic_data).expect("parse");
    let fields = gff.read_struct_fields(0).expect("read root");
    let owned: indexmap::IndexMap<String, GffValue<'static>> = fields
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();

    let mut out = String::new();

    if let Some(GffValue::ListOwned(equip_list)) = owned.get("Equip_ItemList") {
        let chest = equip_list
            .iter()
            .find(|item| matches!(item.get("__struct_id__"), Some(GffValue::Dword(2))));

        if let Some(chest_item) = chest {
            writeln!(out, "=== Chest item in tint-test save ===").unwrap();
            if let Some(tintable_val) = chest_item.get("Tintable") {
                if let Some(tintable) = resolve_struct(tintable_val) {
                    if let Some(tint_val) = tintable.get("Tint") {
                        if let Some(tint) = resolve_struct(tint_val) {
                            for ch_key in ["1", "2", "3"] {
                                if let Some(ch_val) = tint.get(ch_key) {
                                    if let Some(ch) = resolve_struct(ch_val) {
                                        let r = get_byte(&ch, "r");
                                        let g = get_byte(&ch, "g");
                                        let b = get_byte(&ch, "b");
                                        let a = get_byte(&ch, "a");
                                        writeln!(
                                            out,
                                            "  Channel {ch_key}: r={r}, g={g}, b={b}, a={a}"
                                        )
                                        .unwrap();
                                    }
                                }
                            }
                        } else {
                            writeln!(out, "  Tint is not a struct").unwrap();
                        }
                    } else {
                        writeln!(out, "  No Tint key in Tintable").unwrap();
                    }
                } else {
                    writeln!(out, "  Tintable is not a struct").unwrap();
                }
            } else {
                writeln!(out, "  No Tintable on chest item").unwrap();
            }
        } else {
            writeln!(out, "No chest armor found").unwrap();
        }
    }

    let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target_test")
        .join("verify_tint_write.txt");
    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
    std::fs::write(&out_path, &out).unwrap();
    eprintln!("Output: {}", out_path.display());
}
