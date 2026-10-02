use app_lib::parsers::gff::parser::GffParser;
use app_lib::parsers::gff::types::GffValue;
use app_lib::services::resource_manager::ResourceManager;
use app_lib::services::savegame_handler::SaveGameHandler;
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

fn dump_value(out: &mut String, key: &str, val: &GffValue, depth: usize) {
    let pad = "  ".repeat(depth);
    match val {
        GffValue::Byte(v) => writeln!(out, "{pad}{key}: Byte({v})").unwrap(),
        GffValue::Char(v) => writeln!(out, "{pad}{key}: Char({v})").unwrap(),
        GffValue::Word(v) => writeln!(out, "{pad}{key}: Word({v})").unwrap(),
        GffValue::Short(v) => writeln!(out, "{pad}{key}: Short({v})").unwrap(),
        GffValue::Dword(v) => writeln!(out, "{pad}{key}: Dword({v})").unwrap(),
        GffValue::Int(v) => writeln!(out, "{pad}{key}: Int({v})").unwrap(),
        GffValue::Float(v) => writeln!(out, "{pad}{key}: Float({v})").unwrap(),
        GffValue::Dword64(v) => writeln!(out, "{pad}{key}: Dword64({v})").unwrap(),
        GffValue::Int64(v) => writeln!(out, "{pad}{key}: Int64({v})").unwrap(),
        GffValue::Double(v) => writeln!(out, "{pad}{key}: Double({v})").unwrap(),
        GffValue::String(s) => writeln!(out, "{pad}{key}: \"{s}\"").unwrap(),
        GffValue::ResRef(s) => writeln!(out, "{pad}{key}: resref(\"{s}\")").unwrap(),
        GffValue::LocString(ls) => writeln!(
            out,
            "{pad}{key}: LocString(ref={}, n={})",
            ls.string_ref,
            ls.substrings.len()
        )
        .unwrap(),
        GffValue::Void(bytes) => writeln!(out, "{pad}{key}: Void({} bytes)", bytes.len()).unwrap(),
        GffValue::StructOwned(s) => {
            writeln!(out, "{pad}{key}: Struct {{").unwrap();
            for (k, v) in s.as_ref() {
                dump_value(out, k, v, depth + 1);
            }
            writeln!(out, "{pad}}}").unwrap();
        }
        GffValue::Struct(lazy) => {
            let loaded = lazy.force_load();
            writeln!(out, "{pad}{key}: Struct {{").unwrap();
            for (k, v) in &loaded {
                dump_value(out, k, v, depth + 1);
            }
            writeln!(out, "{pad}}}").unwrap();
        }
        GffValue::ListOwned(items) => {
            writeln!(out, "{pad}{key}: List[{}] [", items.len()).unwrap();
            for (i, item) in items.iter().enumerate() {
                writeln!(out, "{pad}  [{i}] {{").unwrap();
                for (k, v) in item {
                    dump_value(out, k, v, depth + 2);
                }
                writeln!(out, "{pad}  }}").unwrap();
            }
            writeln!(out, "{pad}]").unwrap();
        }
        GffValue::List(lazy_list) => {
            writeln!(out, "{pad}{key}: List[{}] [", lazy_list.len()).unwrap();
            for (i, lazy) in lazy_list.iter().enumerate() {
                let loaded = lazy.force_load();
                writeln!(out, "{pad}  [{i}] {{").unwrap();
                for (k, v) in &loaded {
                    dump_value(out, k, v, depth + 2);
                }
                writeln!(out, "{pad}  }}").unwrap();
            }
            writeln!(out, "{pad}]").unwrap();
        }
        GffValue::StructRef(idx) => writeln!(out, "{pad}{key}: StructRef({idx})").unwrap(),
        GffValue::ListRef(v) => writeln!(out, "{pad}{key}: ListRef({v:?})").unwrap(),
    }
}

#[tokio::test]
#[ignore = "diagnostic — run with cargo test dump_full_armor_item -- --ignored --nocapture"]
async fn dump_full_armor_item() {
    let nwn2_paths = Arc::new(RwLock::new(app_lib::config::NWN2Paths::new()));
    let rm = Arc::new(RwLock::new(ResourceManager::new(nwn2_paths)));
    {
        let mut rm_guard = rm.write().await;
        let _ = rm_guard.initialize().await;
    }
    let rm = rm.read().await;

    let mut out = String::new();

    // 1. Dump darksteel plate UTI template (pristine item definition)
    let template = std::env::var("ARMOR_DUMP_TEMPLATE").unwrap_or_else(|_| "mwa_hvfp_drk_3".into());
    writeln!(out, "=== UTI template: {template} ===\n").unwrap();
    match rm.get_resource_bytes(&template, "uti") {
        Ok(bytes) => match GffParser::from_bytes(bytes) {
            Ok(gff) => match gff.read_struct_fields(0) {
                Ok(fields) => {
                    for (k, v) in &fields {
                        dump_value(&mut out, k, v, 0);
                    }
                }
                Err(e) => writeln!(out, "read failed: {e}").unwrap(),
            },
            Err(e) => writeln!(out, "parse failed: {e}").unwrap(),
        },
        Err(e) => writeln!(out, "not found: {e}").unwrap(),
    }

    // 2. Dump equipped darksteel plate from the live save
    writeln!(out, "\n\n=== EQUIPPED armor from save ===\n").unwrap();
    let save_path = std::env::var_os("ARMOR_DUMP_SAVE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                r"C:\Users\01tee\Documents\Neverwinter Nights 2\saves\000064 - 17-04-2026-09-09",
            )
        });
    if save_path.exists() {
        let handler = SaveGameHandler::new(&save_path, false).expect("handler");
        let bic_data = handler
            .extract_player_bic()
            .expect("extract")
            .expect("no bic");
        let gff = GffParser::from_bytes(bic_data).expect("parse");
        let fields: indexmap::IndexMap<_, _> = gff
            .read_struct_fields(0)
            .expect("read")
            .into_iter()
            .map(|(key, value)| (key, value.force_owned()))
            .collect();
        if let Some(GffValue::ListOwned(equip)) = fields.get("Equip_ItemList") {
            for item in equip {
                let base_item = match item.get("BaseItem") {
                    Some(GffValue::Byte(v)) => i32::from(*v),
                    Some(GffValue::Word(v)) => i32::from(*v),
                    Some(GffValue::Dword(v)) => *v as i32,
                    Some(GffValue::Int(v)) => *v,
                    _ => -1,
                };
                if base_item == 16 {
                    for (k, v) in item {
                        dump_value(&mut out, k, v, 0);
                    }
                    break;
                }
            }
        }
    } else {
        writeln!(out, "save not found").unwrap();
    }

    let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target_test")
        .join("full_armor_item.txt");
    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
    std::fs::write(&out_path, &out).unwrap();
    eprintln!("Output: {}", out_path.display());
}
