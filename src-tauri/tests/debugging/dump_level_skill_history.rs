//! Print per-level class and skill purchases of multiclass characters across all fixture saves.
//! Run: `cargo test --features integration-tests --test debugging dump_level_skill_history -- --ignored --nocapture`

use std::path::{Path, PathBuf};

use app_lib::parsers::gff::{GffParser, GffValue};
use app_lib::services::savegame_handler::SaveGameHandler;
use indexmap::IndexMap;

fn i32_of(v: Option<&GffValue>) -> Option<i32> {
    match v? {
        GffValue::Byte(b) => Some(i32::from(*b)),
        GffValue::Word(w) => Some(i32::from(*w)),
        GffValue::Short(s) => Some(i32::from(*s)),
        GffValue::Dword(d) => Some(*d as i32),
        GffValue::Int(i) => Some(*i),
        _ => None,
    }
}

fn list_of(v: Option<&GffValue>) -> Vec<IndexMap<String, GffValue<'static>>> {
    match v.map(|v| v.clone().force_owned().expect("force owned")) {
        Some(GffValue::ListOwned(l)) => l,
        _ => Vec::new(),
    }
}

fn report(label: &str, bytes: Vec<u8>) {
    let Ok(gff) = GffParser::from_bytes(bytes) else {
        return;
    };
    let Ok(root) = gff.read_struct_fields(0) else {
        return;
    };
    let levels = list_of(root.get("LvlStatList"));
    let classes: std::collections::BTreeSet<i32> = levels
        .iter()
        .filter_map(|e| i32_of(e.get("LvlStatClass")))
        .collect();
    if classes.len() < 2 {
        return;
    }
    let name = root
        .get("FirstName")
        .map(|v| format!("{v:?}"))
        .unwrap_or_default();
    let name: String = name.chars().take(40).collect();
    println!(
        "\n== {label} {name} classes={classes:?} SkillPoints={:?}",
        i32_of(root.get("SkillPoints"))
    );
    let totals = list_of(root.get("SkillList"));
    let totals: Vec<(usize, i32)> = totals
        .iter()
        .enumerate()
        .filter_map(|(i, s)| i32_of(s.get("Rank")).filter(|r| *r > 0).map(|r| (i, r)))
        .collect();
    println!("   total ranks {totals:?}");
    for (idx, e) in levels.iter().enumerate() {
        let bought: Vec<(usize, i32)> = list_of(e.get("SkillList"))
            .iter()
            .enumerate()
            .filter_map(|(i, s)| i32_of(s.get("Rank")).filter(|r| *r != 0).map(|r| (i, r)))
            .collect();
        println!(
            "   L{:>2} class={:?} pool={:?} bought={bought:?}",
            idx + 1,
            i32_of(e.get("LvlStatClass")),
            i32_of(e.get("SkillPoints"))
        );
    }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.file_name().is_some_and(|n| n == "resgff.zip")
            || p.extension().is_some_and(|e| e == "bic" || e == "ros")
        {
            out.push(p);
        }
    }
}

#[test]
#[ignore = "diagnostic"]
fn dump_level_skill_history() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut files = Vec::new();
    walk(&root, &mut files);
    for f in files {
        let label = f.strip_prefix(&root).unwrap_or(&f).display().to_string();
        if f.file_name().is_some_and(|n| n == "resgff.zip") {
            let Ok(h) = SaveGameHandler::new(&f) else {
                continue;
            };
            if let Ok(b) = h.extract_file("player.bic") {
                report(&format!("{label}/player.bic"), b);
            }
            for c in h.list_companions().unwrap_or_default() {
                if let Ok(b) = h.extract_companion(&c) {
                    report(&format!("{label}/{c}"), b);
                }
            }
        } else if let Ok(b) = std::fs::read(&f) {
            report(&label, b);
        }
    }
}
