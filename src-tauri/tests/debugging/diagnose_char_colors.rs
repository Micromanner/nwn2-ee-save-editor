use app_lib::parsers::gff::types::GffValue;
use app_lib::services::savegame_handler::SaveGameHandler;
use std::fmt::Write;
use std::path::PathBuf;

#[test]
fn dump_char_color_fields() {
    let save_path = PathBuf::from(
        r"C:\Users\01tee\Documents\Neverwinter Nights 2\saves\000053 - 14-04-2026-12-26",
    );
    if !save_path.exists() {
        println!("Save not found");
        return;
    }

    let handler = SaveGameHandler::new(&save_path).expect("handler");
    let bic_data = handler
        .extract_player_bic()
        .expect("extract")
        .expect("no bic");
    let gff = app_lib::parsers::gff::parser::GffParser::from_bytes(bic_data).expect("parse");
    let fields = gff.read_struct_fields(0).expect("read root");

    let mut out = String::new();

    // Dump ALL top-level field names and types, flagging anything color/tint/cloth related
    writeln!(out, "=== ALL character fields ===").unwrap();
    for (k, v) in &fields {
        let k_lower = k.to_lowercase();
        let interesting = k_lower.contains("color")
            || k_lower.contains("tint")
            || k_lower.contains("cloth")
            || k_lower.contains("skin")
            || k_lower.contains("appearance")
            || k_lower.contains("body")
            || k_lower.contains("visual");

        if interesting {
            writeln!(out, "  ** {k}: {v:?}").unwrap();
        }
    }

    // Dump ArmorTint struct
    writeln!(out, "\n=== ArmorTint struct ===").unwrap();
    if let Some(val) = fields.get("ArmorTint") {
        let at = match val {
            GffValue::Struct(lazy) => Some(lazy.force_load().expect("load struct")),
            GffValue::StructOwned(s) => Some(s.as_ref().clone()),
            _ => None,
        };
        if let Some(at) = at {
            for (k, v) in &at {
                writeln!(out, "  {k}: {v:?}").unwrap();
                // If it's a struct, dig one level deeper
                let inner = match v {
                    GffValue::Struct(lazy) => Some(lazy.force_load().expect("load struct")),
                    GffValue::StructOwned(s) => Some(s.as_ref().clone()),
                    _ => None,
                };
                if let Some(inner) = inner {
                    for (ik, iv) in &inner {
                        writeln!(out, "    {ik}: {iv:?}").unwrap();
                        let deep = match iv {
                            GffValue::Struct(lazy) => Some(lazy.force_load().expect("load struct")),
                            GffValue::StructOwned(s) => Some(s.as_ref().clone()),
                            _ => None,
                        };
                        if let Some(deep) = deep {
                            for (dk, dv) in &deep {
                                writeln!(out, "      {dk}: {dv:?}").unwrap();
                            }
                        }
                    }
                }
            }
        }
    }

    // Also list ALL field names for reference
    writeln!(out, "\n=== ALL field names ===").unwrap();
    let mut names: Vec<&String> = fields.keys().collect();
    names.sort();
    for name in &names {
        let disc = std::mem::discriminant(fields.get(*name).unwrap());
        writeln!(out, "  {name}: {disc:?}").unwrap();
    }

    let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target_test")
        .join("char_color_fields.txt");
    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
    std::fs::write(&out_path, &out).unwrap();
    eprintln!("Output: {}", out_path.display());
}
