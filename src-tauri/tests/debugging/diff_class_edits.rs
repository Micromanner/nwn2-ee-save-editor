//! Diff dump for class-edit crash investigation.
//!
//! Compares the post-edit `.bic` from two save folders side-by-side, focusing
//! on class-related fields. The `LevelDown` save is the no-crash baseline; the
//! `RemovedClass` save is the crashing one. Whatever differs structurally
//! between them is a strong candidate for what `remove_class` corrupts.
//!
//! Run with:
//!   cargo test --test debugging --features integration-tests \
//!     dump_class_edit_diff -- --ignored --nocapture

use app_lib::character::gff_helpers::gff_value_to_i32;
use app_lib::parsers::gff::helpers::variant_name;
use app_lib::parsers::gff::parser::GffParser;
use app_lib::parsers::gff::types::GffValue;
use app_lib::services::savegame_handler::SaveGameHandler;
use indexmap::IndexMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/saves/memoryerror")
}

fn load_bic(save_path: &Path) -> IndexMap<String, GffValue<'static>> {
    let handler = SaveGameHandler::new(save_path, false, false).expect("handler");
    let data = handler
        .extract_player_bic()
        .expect("extract_player_bic")
        .expect("missing player.bic");
    let gff = GffParser::from_bytes(data).expect("parse bic");
    let fields = gff.read_struct_fields(0).expect("root");
    fields
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect()
}

fn as_i64(v: &GffValue) -> Option<i64> {
    gff_value_to_i32(v).map(i64::from)
}

fn list_owned<'a>(v: &'a GffValue<'static>) -> Option<&'a [IndexMap<String, GffValue<'static>>]> {
    if let GffValue::ListOwned(items) = v {
        Some(items.as_slice())
    } else {
        None
    }
}

fn fmt_class_list(list: &[IndexMap<String, GffValue<'static>>]) -> String {
    let mut s = String::new();
    for (i, entry) in list.iter().enumerate() {
        let class = entry.get("Class").and_then(as_i64).unwrap_or(-1);
        let level = entry.get("ClassLevel").and_then(as_i64).unwrap_or(-1);
        let scl = entry.get("SpellCasterLevel").and_then(as_i64);
        let known_total: usize = (0..10)
            .filter_map(|k| {
                entry
                    .get(&format!("KnownList{k}"))
                    .and_then(list_owned)
                    .map(|l| l.len())
            })
            .sum();
        let mem_total: usize = (0..10)
            .filter_map(|k| {
                entry
                    .get(&format!("MemorizedList{k}"))
                    .and_then(list_owned)
                    .map(|l| l.len())
            })
            .sum();
        writeln!(
            s,
            "    [{i}] Class={class} Level={level} SCL={scl:?} Known={known_total} Mem={mem_total}"
        )
        .unwrap();
    }
    s
}

fn fmt_lvl_stat_list(list: &[IndexMap<String, GffValue<'static>>]) -> String {
    let mut s = format!("  ({} entries)\n", list.len());
    for (i, entry) in list.iter().enumerate() {
        let cls = entry.get("LvlStatClass").and_then(as_i64).unwrap_or(-1);
        let hd = entry.get("LvlStatHitDie").and_then(as_i64).unwrap_or(-1);
        let abil = entry.get("LvlStatAbility").and_then(as_i64).unwrap_or(-1);
        let sp = entry.get("SkillPoints").and_then(as_i64).unwrap_or(-1);
        let feats = entry
            .get("FeatList")
            .and_then(list_owned)
            .map(|l| l.len())
            .unwrap_or(0);
        let skills = entry
            .get("SkillList")
            .and_then(list_owned)
            .map(|l| l.len())
            .unwrap_or(0);
        writeln!(
            s,
            "    [{i}] Class={cls} HitDie={hd} Ability={abil} SP={sp} Feats={feats} Skills={skills}"
        )
        .unwrap();
    }
    s
}

fn dump(label: &str, bic: &IndexMap<String, GffValue<'static>>) {
    println!("\n{}", "=".repeat(70));
    println!("=== {label} ===");
    println!("{}", "=".repeat(70));

    println!("\n-- Top-level scalars --");
    for key in [
        "Class",
        "Experience",
        "HitPoints",
        "MaxHitPoints",
        "CurrentHitPoints",
        "BaseAttackBonus",
        "BAB",
        "FortbonusSave",
        "RefbonusSave",
        "WillbonusSave",
        "FortSaveThrow",
        "RefSaveThrow",
        "WillSaveThrow",
        "SkillPoints",
        "Str",
        "Dex",
        "Con",
        "Int",
        "Wis",
        "Cha",
    ] {
        if let Some(v) = bic.get(key) {
            let val = as_i64(v)
                .map(|n| n.to_string())
                .unwrap_or_else(|| format!("{v:?}"));
            println!("  {key:<22} = {val}");
        }
    }

    println!("\n-- ClassList --");
    if let Some(list) = bic.get("ClassList").and_then(list_owned) {
        print!("{}", fmt_class_list(list));
    }

    println!("\n-- LvlStatList --");
    if let Some(list) = bic.get("LvlStatList").and_then(list_owned) {
        print!("{}", fmt_lvl_stat_list(list));
    }

    println!("\n-- FeatList count --");
    if let Some(list) = bic.get("FeatList").and_then(list_owned) {
        println!("  {} entries", list.len());
    }

    println!("\n-- SkillList summary --");
    if let Some(list) = bic.get("SkillList").and_then(list_owned) {
        let total_ranks: i64 = list
            .iter()
            .map(|s| s.get("Rank").and_then(as_i64).unwrap_or(0))
            .sum();
        println!("  {} skills, {} total ranks", list.len(), total_ranks);
    }

    println!("\n-- All top-level keys --");
    let keys: Vec<&str> = bic.keys().map(|s| s.as_str()).collect();
    println!("  {keys:?}");
}

fn dump_struct_recursive(label: &str, entry: &IndexMap<String, GffValue<'static>>, indent: usize) {
    let pad = "  ".repeat(indent);
    println!("{pad}-- {label} ({} fields) --", entry.len());
    for (k, v) in entry {
        match v {
            GffValue::ListOwned(items) => {
                println!("{pad}  {k}: List[{}]", items.len());
            }
            other => {
                let val = as_i64(other)
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| format!("{other:?}"));
                println!("{pad}  {k:<22} = {val}");
            }
        }
    }
}

fn dump_full_class_list(label: &str, bic: &IndexMap<String, GffValue<'static>>) {
    println!("\n=== {label} ClassList (full) ===");
    if let Some(list) = bic.get("ClassList").and_then(list_owned) {
        for (i, entry) in list.iter().enumerate() {
            dump_struct_recursive(&format!("ClassList[{i}]"), entry, 0);
        }
    }
}

fn dump_full_lvlstat_list(label: &str, bic: &IndexMap<String, GffValue<'static>>) {
    println!("\n=== {label} LvlStatList (full, scalar fields only) ===");
    if let Some(list) = bic.get("LvlStatList").and_then(list_owned) {
        for (i, entry) in list.iter().enumerate() {
            dump_struct_recursive(&format!("LvlStatList[{i}]"), entry, 0);
            // Only expand sub-lists if non-empty AND not the boilerplate SkillList
            for (sub_key, sub_label) in [
                ("FeatList", "FeatList"),
                ("KnownList0", "KnownList0"),
                ("KnownList1", "KnownList1"),
                ("KnownList2", "KnownList2"),
                ("KnownRemoveList0", "KnownRemoveList0"),
            ] {
                if let Some(sub) = entry.get(sub_key).and_then(list_owned)
                    && !sub.is_empty()
                {
                    for (si, se) in sub.iter().enumerate() {
                        dump_struct_recursive(
                            &format!("LvlStatList[{i}].{sub_label}[{si}]"),
                            se,
                            1,
                        );
                    }
                }
            }
            // SkillList: only ranks > 0
            if let Some(skills) = entry.get("SkillList").and_then(list_owned) {
                let nz: Vec<(usize, i64)> = skills
                    .iter()
                    .enumerate()
                    .filter_map(|(si, s)| {
                        let r = s.get("Rank").and_then(as_i64).unwrap_or(0);
                        (r > 0).then_some((si, r))
                    })
                    .collect();
                if !nz.is_empty() {
                    println!("  LvlStatList[{i}].SkillList nonzero: {nz:?}");
                }
            }
        }
    }
}

fn dump_feats(label: &str, bic: &IndexMap<String, GffValue<'static>>) {
    println!("\n=== {label} FeatList ===");
    if let Some(list) = bic.get("FeatList").and_then(list_owned) {
        let ids: Vec<i64> = list
            .iter()
            .filter_map(|e| e.get("Feat").and_then(as_i64))
            .collect();
        println!("  count={} ids={:?}", list.len(), ids);
    }
}

fn dump_nonzero_skills(label: &str, bic: &IndexMap<String, GffValue<'static>>) {
    println!("\n=== {label} SkillList (non-zero rank entries) ===");
    if let Some(list) = bic.get("SkillList").and_then(list_owned) {
        for (i, e) in list.iter().enumerate() {
            let rank = e.get("Rank").and_then(as_i64).unwrap_or(0);
            if rank > 0 {
                println!("  [{i}] rank={rank}");
            }
        }
    }
}

#[test]
#[ignore = "diagnostic dump for the MClassLevUpIn investigation"]
fn dump_class_edit_diff() {
    let dir = fixtures_dir();
    let level_down = dir.join("LevelDown");
    let removed_class = dir.join("RemovedClass");

    let ld = load_bic(&level_down);
    let rc = load_bic(&removed_class);

    dump("LevelDown (NO CRASH baseline)", &ld);
    dump("RemovedClass (CRASHES)", &rc);

    dump_full_class_list("LevelDown", &ld);
    dump_full_class_list("RemovedClass", &rc);

    dump_full_lvlstat_list("LevelDown", &ld);
    dump_full_lvlstat_list("RemovedClass", &rc);

    dump_feats("LevelDown", &ld);
    dump_feats("RemovedClass", &rc);

    dump_nonzero_skills("LevelDown", &ld);
    dump_nonzero_skills("RemovedClass", &rc);

    println!("\n{}", "=".repeat(70));
    println!("=== KEY-LEVEL DIFF ===");
    println!("{}", "=".repeat(70));

    use std::collections::BTreeSet;
    let ld_keys: BTreeSet<&str> = ld.keys().map(|s| s.as_str()).collect();
    let rc_keys: BTreeSet<&str> = rc.keys().map(|s| s.as_str()).collect();
    let only_ld: Vec<_> = ld_keys.difference(&rc_keys).collect();
    let only_rc: Vec<_> = rc_keys.difference(&ld_keys).collect();
    println!("Only in LevelDown:    {only_ld:?}");
    println!("Only in RemovedClass: {only_rc:?}");

    let common: Vec<&&str> = ld_keys.intersection(&rc_keys).collect();
    println!("\nValue diffs on common scalar keys:");
    for k in common {
        if let Some(lv) = ld.get(*k)
            && let Some(rv) = rc.get(*k)
            && let Some(li) = as_i64(lv)
            && let Some(ri) = as_i64(rv)
            && li != ri
        {
            println!("  {k:<22}  LevelDown={li:>6}  RemovedClass={ri:>6}");
        }
    }

    println!("\n=== Special fields (type + value) ===");
    for k in [
        "MClassLevUpIn",
        "StartingPackage",
        "CharBackground",
        "Class",
    ] {
        let l = ld.get(k);
        let r = rc.get(k);
        let lt = l.map(|v| variant_name(v)).unwrap_or("(missing)");
        let rt = r.map(|v| variant_name(v)).unwrap_or("(missing)");
        let lv = l.and_then(as_i64);
        let rv = r.and_then(as_i64);
        println!("  {k:<22}  LevelDown={lt}({lv:?})  RemovedClass={rt}({rv:?})");
    }

    // Survey MClassLevUpIn in all other save fixtures to understand what it means
    println!("\n=== MClassLevUpIn across all fixture saves ===");
    let saves_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/saves");
    let mut found = Vec::<(String, Option<i64>, usize)>::new();
    if let Ok(entries) = std::fs::read_dir(&saves_root) {
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            // also recurse one level for module-grouped saves
            for cand in std::iter::once(p.clone()).chain(
                std::fs::read_dir(&p)
                    .ok()
                    .into_iter()
                    .flatten()
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| p.is_dir()),
            ) {
                if !cand.join("resgff.zip").exists() {
                    continue;
                }
                let handler = match SaveGameHandler::new(&cand, false, false) {
                    Ok(h) => h,
                    Err(_) => continue,
                };
                let bic = match handler.extract_player_bic() {
                    Ok(Some(b)) => b,
                    _ => continue,
                };
                let gff = match GffParser::from_bytes(bic) {
                    Ok(g) => g,
                    _ => continue,
                };
                let fields = match gff.read_struct_fields(0) {
                    Ok(f) => f,
                    _ => continue,
                };
                let mclass = fields.get("MClassLevUpIn").and_then(as_i64);
                let class_count = fields
                    .get("ClassList")
                    .and_then(|v| {
                        if let GffValue::ListOwned(l) = v {
                            Some(l.len())
                        } else if let GffValue::List(l) = v {
                            Some(l.len())
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);
                found.push((
                    cand.file_name().unwrap().to_string_lossy().to_string(),
                    mclass,
                    class_count,
                ));
            }
        }
    }
    found.sort();
    for (name, mclass, cls) in found {
        println!("  {name:<40} MClassLevUpIn={mclass:?}  ClassList.len={cls}");
    }

    println!("\n=== ItemList (top-level) ===");
    for (label, bic) in [("LevelDown", &ld), ("RemovedClass", &rc)] {
        if let Some(list) = bic.get("ItemList").and_then(list_owned) {
            println!("  {label}: {} items", list.len());
        }
        if let Some(list) = bic.get("Equip_ItemList").and_then(list_owned) {
            println!("  {label} equipped: {} items", list.len());
            for (i, item) in list.iter().enumerate() {
                let resref = item
                    .get("EquippedRes")
                    .or_else(|| item.get("BaseItem"))
                    .map(|v| format!("{v:?}"))
                    .unwrap_or_default();
                let struct_id = item.get("__struct_id__").and_then(as_i64).unwrap_or(-1);
                println!("    [{i}] struct_id={struct_id} {resref}");
            }
        }
    }

    println!("\n=== HotbarList ===");
    for (label, bic) in [("LevelDown", &ld), ("RemovedClass", &rc)] {
        if let Some(list) = bic.get("HotbarList").and_then(list_owned) {
            println!("  {label}: {} hotbar entries", list.len());
            for (i, hb) in list.iter().enumerate() {
                if hb.is_empty() {
                    continue;
                }
                let scalars: Vec<String> = hb
                    .iter()
                    .filter_map(|(k, v)| as_i64(v).map(|n| format!("{k}={n}")))
                    .collect();
                if !scalars.is_empty() {
                    println!("    [{i}] {}", scalars.join(" "));
                }
            }
        } else {
            println!("  {label}: NO HotbarList field");
        }
    }

    println!("\n=== CombatInfo / CombatRoundData ===");
    for (label, bic) in [("LevelDown", &ld), ("RemovedClass", &rc)] {
        for k in ["CombatInfo", "CombatRoundData"] {
            if let Some(v) = bic.get(k) {
                let sample = match v {
                    GffValue::ListOwned(items) => format!("List[{}]", items.len()),
                    GffValue::StructOwned(m) => format!("Struct[{} fields]", m.len()),
                    other => format!("{other:?}"),
                };
                println!("  {label}.{k} = {sample}");
            }
        }
        if let Some(GffValue::StructOwned(m)) = bic.get("CombatInfo") {
            println!("  -- {label}.CombatInfo fields --");
            for (k, v) in m.iter() {
                let val = as_i64(v)
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| format!("{v:?}"));
                println!("    {k:<22} = {val}");
            }
        }
    }

    println!("\n=== HotbarList full dump ===");
    for (label, bic) in [("LevelDown", &ld), ("RemovedClass", &rc)] {
        if let Some(list) = bic.get("HotbarList").and_then(list_owned) {
            println!("  -- {label}: {} entries --", list.len());
            for (i, hb) in list.iter().enumerate() {
                println!("    [{i}] {} fields:", hb.len());
                for (k, v) in hb {
                    let val = as_i64(v)
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| format!("{v:?}"));
                    println!("      {k:<20} = {val}");
                }
            }
        }
    }
}
