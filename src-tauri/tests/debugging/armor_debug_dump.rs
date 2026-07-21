use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use app_lib::parsers::gff::parser::GffParser;
use app_lib::parsers::gff::types::GffValue;
use app_lib::parsers::tda::types::TDAParser;
use indexmap::IndexMap;
use serde_json::{Value, json};

// ── GFF helpers ──────────────────────────────────────────────────────────────

fn gff_i32(fields: &IndexMap<String, GffValue<'_>>, key: &str) -> Option<i32> {
    match fields.get(key)? {
        GffValue::Byte(v) => Some(i32::from(*v)),
        GffValue::Char(v) => Some(*v as i32),
        GffValue::Word(v) => Some(i32::from(*v)),
        GffValue::Short(v) => Some(i32::from(*v)),
        GffValue::Dword(v) => Some(*v as i32),
        GffValue::Int(v) => Some(*v),
        _ => None,
    }
}

fn gff_str(fields: &IndexMap<String, GffValue<'_>>, key: &str) -> Option<String> {
    match fields.get(key)? {
        GffValue::String(s) => Some(s.to_string()),
        GffValue::ResRef(s) => Some(s.to_string()),
        _ => None,
    }
}

fn locstring_first(fields: &IndexMap<String, GffValue<'_>>, key: &str) -> String {
    match fields.get(key) {
        Some(GffValue::LocString(ls)) => ls
            .substrings
            .first()
            .map(|s| s.string.to_string())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn force_struct(v: &GffValue<'_>) -> Option<IndexMap<String, GffValue<'static>>> {
    match v {
        GffValue::Struct(lazy) => Some(lazy.force_load()),
        GffValue::StructOwned(map) => Some(
            map.iter()
                .map(|(k, v)| (k.clone(), v.clone().into_owned()))
                .collect(),
        ),
        _ => None,
    }
}

// ── 2DA helpers ──────────────────────────────────────────────────────────────

fn row_dict_to_json(parser: &TDAParser, row: usize) -> Value {
    match parser.get_row_dict(row) {
        Ok(map) => {
            let obj: serde_json::Map<String, Value> = map
                .into_iter()
                .map(|(k, v)| (k, v.map(Value::String).unwrap_or(Value::Null)))
                .collect();
            Value::Object(obj)
        }
        Err(_) => Value::Null,
    }
}

fn parse_equip_slots(raw: &str) -> u32 {
    let s = raw.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).unwrap_or(0)
    } else {
        s.parse::<u32>().unwrap_or(0)
    }
}

// ── Zip helpers ──────────────────────────────────────────────────────────────

fn read_zip_entry(zip_path: &Path, entry_name_suffix: &str) -> Option<Vec<u8>> {
    let file = std::fs::File::open(zip_path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let suffix_lower = entry_name_suffix.to_lowercase();
    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
        .collect();
    let matched = names
        .iter()
        .find(|n| n.to_lowercase().ends_with(&suffix_lower))?
        .clone();
    let mut entry = archive.by_name(&matched).ok()?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).ok()?;
    Some(buf)
}

/// Load a named 2DA from the three game Data zips (x2 > x1 > base, first hit wins).
fn load_2da_from_zips(game_dir: &Path, name: &str) -> Option<TDAParser> {
    let suffix = format!("{}.2da", name.to_lowercase());
    for zip_name in &["2da_x2.zip", "2da_x1.zip", "2da.zip"] {
        let zip_path = game_dir.join("Data").join(zip_name);
        if let Some(bytes) = read_zip_entry(&zip_path, &suffix) {
            let mut parser = TDAParser::new();
            if parser.parse_from_bytes(&bytes).is_ok() {
                return Some(parser);
            }
        }
    }
    None
}

/// Build a lowercase-name → zip-basename map for every `.mdb` entry in
/// `game_dir/Data/*.zip` plus loose files under `game_dir/enhanced/data/` (recursive).
fn build_mdb_index(game_dir: &Path) -> HashMap<String, String> {
    let mut index: HashMap<String, String> = HashMap::new();

    let data_dir = game_dir.join("Data");
    if let Ok(entries) = std::fs::read_dir(&data_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("zip") {
                continue;
            }
            let zip_source = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let Ok(file) = std::fs::File::open(&path) else {
                continue;
            };
            let Ok(mut archive) = zip::ZipArchive::new(file) else {
                continue;
            };
            for i in 0..archive.len() {
                let Ok(entry) = archive.by_index(i) else {
                    continue;
                };
                let name_lower = entry.name().to_lowercase();
                if name_lower.ends_with(".mdb") {
                    let basename = name_lower
                        .rsplit('/')
                        .next()
                        .unwrap_or(&name_lower)
                        .trim_end_matches(".mdb")
                        .to_string();
                    index.entry(basename).or_insert_with(|| zip_source.clone());
                }
            }
        }
    }

    let enhanced_dir = game_dir.join("enhanced").join("data");
    if enhanced_dir.exists() {
        for entry in walkdir::WalkDir::new(&enhanced_dir)
            .into_iter()
            .flatten()
            .filter(|e| {
                e.file_type().is_file()
                    && e.path()
                        .extension()
                        .and_then(|x| x.to_str())
                        .map(|x| x.to_lowercase() == "mdb")
                        .unwrap_or(false)
            })
        {
            let rel = entry
                .path()
                .strip_prefix(game_dir)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .to_string();
            let basename = entry
                .path()
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            index.entry(basename).or_insert(rel);
        }
    }

    index
}

// ── Tint extraction ───────────────────────────────────────────────────────────

fn read_tint_channels(fields: &IndexMap<String, GffValue<'_>>) -> Value {
    let tint_struct = fields
        .get("Tintable")
        .and_then(force_struct)
        .and_then(|t| t.get("Tint").and_then(force_struct));

    let read_channel = |parent: &Option<IndexMap<String, GffValue<'static>>>, ch: &str| -> Value {
        let Some(ts) = parent else {
            return Value::Null;
        };
        let Some(ch_val) = ts.get(ch) else {
            return Value::Null;
        };
        let Some(ch_map) = force_struct(ch_val) else {
            return Value::Null;
        };
        json!({
            "r": gff_i32(&ch_map, "r"),
            "g": gff_i32(&ch_map, "g"),
            "b": gff_i32(&ch_map, "b"),
            "a": gff_i32(&ch_map, "a"),
        })
    };

    json!({
        "channel1": read_channel(&tint_struct, "1"),
        "channel2": read_channel(&tint_struct, "2"),
        "channel3": read_channel(&tint_struct, "3"),
    })
}

fn read_accessory_tint(fields: &IndexMap<String, GffValue<'static>>) -> Value {
    let tint_struct = fields
        .get("Tintable")
        .and_then(force_struct)
        .and_then(|t| t.get("Tint").and_then(force_struct));

    let read_channel = |parent: &Option<IndexMap<String, GffValue<'static>>>, ch: &str| -> Value {
        let Some(ts) = parent else {
            return Value::Null;
        };
        let Some(ch_val) = ts.get(ch) else {
            return Value::Null;
        };
        let Some(ch_map) = force_struct(ch_val) else {
            return Value::Null;
        };
        json!({
            "r": gff_i32(&ch_map, "r"),
            "g": gff_i32(&ch_map, "g"),
            "b": gff_i32(&ch_map, "b"),
            "a": gff_i32(&ch_map, "a"),
        })
    };

    json!({
        "channel1": read_channel(&tint_struct, "1"),
        "channel2": read_channel(&tint_struct, "2"),
        "channel3": read_channel(&tint_struct, "3"),
    })
}

// ── Item extraction ───────────────────────────────────────────────────────────

const ACCESSORY_FIELDS: &[&str] = &[
    "ACLtShoulder",
    "ACRtShoulder",
    "ACLtArm",
    "ACRtArm",
    "ACLtElbow",
    "ACRtElbow",
    "ACLtBracer",
    "ACRtBracer",
    "ACLtLeg",
    "ACRtLeg",
    "ACLtKnee",
    "ACRtKnee",
    "ACLtShin",
    "ACRtShin",
    "ACLtHip",
    "ACRtHip",
    "ACLtAnkle",
    "ACRtAnkle",
    "ACLtFoot",
    "ACRtFoot",
    "ACBkHip",
    "ACFtHip",
];

fn extract_item(
    fields: &IndexMap<String, GffValue<'static>>,
    baseitems: Option<&TDAParser>,
) -> Value {
    let base_item = gff_i32(fields, "BaseItem").unwrap_or(-1);
    let equip_slots = baseitems
        .and_then(|t| {
            if base_item >= 0 {
                t.get_cell_by_name(base_item as usize, "equipableslots")
                    .ok()
                    .flatten()
                    .map(parse_equip_slots)
            } else {
                None
            }
        })
        .unwrap_or(0);

    let slot = detect_slot(equip_slots);

    let mut obj = json!({
        "slot": slot,
        "BaseItem": base_item,
        "equipableslots": format!("0x{equip_slots:08X}"),
        "Tag": gff_str(fields, "Tag"),
        "TemplateResRef": gff_str(fields, "TemplateResRef"),
        "ArmorVisualType": gff_i32(fields, "ArmorVisualType"),
        "ArmorRulesType": gff_i32(fields, "ArmorRulesType"),
        "Variation": gff_i32(fields, "Variation"),
        "ModelPart1": gff_i32(fields, "ModelPart1"),
        "ModelPart2": gff_i32(fields, "ModelPart2"),
        "ModelPart3": gff_i32(fields, "ModelPart3"),
        "LocalizedName": locstring_first(fields, "LocalizedName"),
        "tints": read_tint_channels(fields),
    });

    // Nested boots / gloves
    for nested_key in ["Boots", "Gloves"] {
        if let Some(ns) = fields.get(nested_key).and_then(force_struct) {
            obj[nested_key] = json!({
                "ArmorVisualType": gff_i32(&ns, "ArmorVisualType"),
                "Variation": gff_i32(&ns, "Variation"),
            });
        }
    }

    // Accessories (chest only)
    if slot.as_deref() == Some("chest") {
        let mut accessories = serde_json::Map::new();
        for &ac_field in ACCESSORY_FIELDS {
            let v = fields
                .get(ac_field)
                .and_then(force_struct)
                .map(|ac_fields| {
                    json!({
                        "id": gff_i32(&ac_fields, "id"),
                        "tints": read_accessory_tint(&ac_fields),
                    })
                })
                .unwrap_or(Value::Null);
            accessories.insert(ac_field.to_string(), v);
        }
        obj["accessories"] = Value::Object(accessories);
    }

    obj
}

fn detect_slot(equip_slots: u32) -> Option<String> {
    // Bitmasks match EquipmentSlot::to_bitmask in src/character/inventory.rs.
    // Head checked before chest — matches detect_armor_slot ordering in item_appearance.rs
    if equip_slots & 0x0001 != 0 {
        Some("head".into())
    } else if equip_slots & 0x0002 != 0 {
        Some("chest".into())
    } else if equip_slots & 0x0004 != 0 {
        Some("boots".into())
    } else if equip_slots & 0x0008 != 0 {
        Some("gloves".into())
    } else if equip_slots & 0x0040 != 0 {
        Some("cloak".into())
    } else {
        None
    }
}

// ── Main test ─────────────────────────────────────────────────────────────────

#[test]
#[ignore = "requires NWN2 EE install; run with --ignored --nocapture"]
fn dump_armor_debug_data() -> Result<()> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = manifest.join("tests").join("fixtures");
    let save_dir = std::env::var("ARMOR_DEBUG_SAVE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            fixtures
                .join("armor_debug")
                .join("000064 - 18-04-2026-17-23")
        });
    let game_dir = std::env::var("NWN2_GAME_FOLDER")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common\NWN2 Enhanced Edition")
        });

    if !save_dir.exists() {
        eprintln!("Save dir not found: {}", save_dir.display());
        return Ok(());
    }
    if !game_dir.exists() {
        eprintln!("Game dir not found: {}", game_dir.display());
        return Ok(());
    }

    // ── Parse player.bic ─────────────────────────────────────────────────────
    let resgff_zip = save_dir.join("resgff.zip");
    let bic_bytes = read_zip_entry(&resgff_zip, "player.bic")
        .with_context(|| format!("player.bic not found in {}", resgff_zip.display()))?;

    let parser = GffParser::from_bytes(bic_bytes).context("GFF parse failed")?;
    let root: IndexMap<String, GffValue<'static>> = parser
        .read_struct_fields(0)
        .context("read root struct")?
        .into_iter()
        .map(|(k, v)| (k, v.into_owned()))
        .collect();

    // ── Character identity fields ─────────────────────────────────────────────
    let race = gff_i32(&root, "Race").unwrap_or(-1) as usize;
    let gender = gff_i32(&root, "Gender").unwrap_or(-1) as usize;
    let appearance_type = gff_i32(&root, "Appearance_Type").unwrap_or(-1) as usize;
    let first_name = locstring_first(&root, "FirstName");
    let last_name = locstring_first(&root, "LastName");

    // ── Load 2DAs ─────────────────────────────────────────────────────────────
    let tda_names = [
        "appearance",
        "gender",
        "armor",
        "armorvisualdata",
        "armorrulestats",
        "baseitems",
        "parts_chest",
        "parts_helmet",
        "parts_robe",
    ];
    let mut tdas: HashMap<String, TDAParser> = HashMap::new();
    for name in &tda_names {
        match load_2da_from_zips(&game_dir, name) {
            Some(p) => {
                tdas.insert(name.to_string(), p);
            }
            None => eprintln!("2DA not found: {name}"),
        }
    }

    // ── Compute body_prefix ───────────────────────────────────────────────────
    let nwn2_model_body = tdas
        .get("appearance")
        .and_then(|t| {
            t.get_cell_by_name(appearance_type, "nwn2_model_body")
                .ok()
                .flatten()
                .map(str::to_string)
        })
        .unwrap_or_default();

    let gender_letter = tdas
        .get("gender")
        .and_then(|t| {
            t.get_cell_by_name(gender, "gender")
                .ok()
                .flatten()
                .map(|s| {
                    // gender.2da gender column: "Male"/"Female" → first letter uppercase
                    s.chars().next().unwrap_or('M').to_uppercase().to_string()
                })
        })
        .unwrap_or_else(|| "M".to_string());

    let body_prefix = nwn2_model_body.replace('?', &gender_letter);

    // ── Walk Equip_ItemList ───────────────────────────────────────────────────
    let equip_list: Vec<IndexMap<String, GffValue<'static>>> = match root.get("Equip_ItemList") {
        Some(GffValue::List(lazy_list)) => lazy_list.iter().map(|lazy| lazy.force_load()).collect(),
        _ => Vec::new(),
    };

    let baseitems = tdas.get("baseitems");
    let mut chest_item: Option<Value> = None;
    let mut helm_item: Option<Value> = None;
    let mut boots_item: Option<Value> = None;
    let mut gloves_item: Option<Value> = None;
    let mut all_items: Vec<Value> = Vec::new();

    for item_fields in &equip_list {
        let base_item = gff_i32(item_fields, "BaseItem").unwrap_or(-1);
        let equip_slots = baseitems
            .and_then(|t| {
                if base_item >= 0 {
                    t.get_cell_by_name(base_item as usize, "equipableslots")
                        .ok()
                        .flatten()
                        .map(parse_equip_slots)
                } else {
                    None
                }
            })
            .unwrap_or(0);
        let slot = detect_slot(equip_slots);
        let item_json = extract_item(item_fields, baseitems);
        all_items.push(item_json.clone());
        match slot.as_deref() {
            Some("head") if helm_item.is_none() => helm_item = Some(item_json),
            Some("chest") if chest_item.is_none() => chest_item = Some(item_json),
            Some("boots") if boots_item.is_none() => boots_item = Some(item_json),
            Some("gloves") if gloves_item.is_none() => gloves_item = Some(item_json),
            _ => {}
        }
    }

    let root_field_names: Vec<String> = root.keys().cloned().collect();

    // ── 2DA snapshots ────────────────────────────────────────────────────────
    let chest_avt = chest_item
        .as_ref()
        .and_then(|v| v["ArmorVisualType"].as_i64())
        .unwrap_or(-1) as usize;
    let chest_art = chest_item
        .as_ref()
        .and_then(|v| v["ArmorRulesType"].as_i64())
        .unwrap_or(-1) as usize;
    let chest_base = chest_item
        .as_ref()
        .and_then(|v| v["BaseItem"].as_i64())
        .unwrap_or(-1) as usize;
    let helm_avt = helm_item
        .as_ref()
        .and_then(|v| v["ArmorVisualType"].as_i64())
        .unwrap_or(-1) as usize;
    let helm_base = helm_item
        .as_ref()
        .and_then(|v| v["BaseItem"].as_i64())
        .unwrap_or(-1) as usize;

    let mut snapshots = serde_json::Map::new();

    let snap = |table: Option<&TDAParser>, row: usize, label: &str| -> (String, Value) {
        let v = table
            .map(|t| row_dict_to_json(t, row))
            .unwrap_or(Value::Null);
        (label.to_string(), v)
    };

    for (label, val) in [
        snap(
            tdas.get("appearance"),
            appearance_type,
            &format!("appearance.2da[{appearance_type}]"),
        ),
        snap(tdas.get("gender"), gender, &format!("gender.2da[{gender}]")),
        snap(
            tdas.get("baseitems"),
            chest_base,
            &format!("baseitems.2da[chest={chest_base}]"),
        ),
        snap(
            tdas.get("baseitems"),
            helm_base,
            &format!("baseitems.2da[helm={helm_base}]"),
        ),
        snap(
            tdas.get("armor"),
            chest_avt,
            &format!("armor.2da[chest_avt={chest_avt}]"),
        ),
        snap(
            tdas.get("armor"),
            chest_avt.saturating_sub(1),
            &format!("armor.2da[chest_avt-1={}]", chest_avt.saturating_sub(1)),
        ),
        snap(
            tdas.get("armor"),
            helm_avt,
            &format!("armor.2da[helm_avt={helm_avt}]"),
        ),
        snap(
            tdas.get("armor"),
            helm_avt.saturating_sub(1),
            &format!("armor.2da[helm_avt-1={}]", helm_avt.saturating_sub(1)),
        ),
        snap(
            tdas.get("armorvisualdata"),
            chest_avt,
            &format!("armorvisualdata.2da[chest_avt={chest_avt}]"),
        ),
        snap(
            tdas.get("armorvisualdata"),
            chest_avt.saturating_sub(1),
            &format!(
                "armorvisualdata.2da[chest_avt-1={}]",
                chest_avt.saturating_sub(1)
            ),
        ),
        snap(
            tdas.get("armorrulestats"),
            chest_art,
            &format!("armorrulestats.2da[chest_art={chest_art}]"),
        ),
        snap(
            tdas.get("armorrulestats"),
            chest_art.saturating_sub(1),
            &format!(
                "armorrulestats.2da[chest_art-1={}]",
                chest_art.saturating_sub(1)
            ),
        ),
    ] {
        snapshots.insert(label, val);
    }

    // Parts lookups from armorvisualdata
    let avd_columns = [
        "part_chest",
        "part_helm",
        "part_robe",
        "ChestModelVar",
        "HelmModelVar",
    ];
    if let Some(avd) = tdas.get("armorvisualdata") {
        for row_idx in [chest_avt, chest_avt.saturating_sub(1)] {
            if let Ok(dict) = avd.get_row_dict(row_idx) {
                for col in &avd_columns {
                    if let Some(Some(val_str)) = dict.get(*col)
                        && let Ok(n) = val_str.parse::<usize>()
                    {
                        for (tbl_name, col_label) in
                            [("parts_chest", "part_chest"), ("parts_helmet", "part_helm")]
                        {
                            let (label, v) = snap(
                                tdas.get(tbl_name),
                                n,
                                &format!("{tbl_name}.2da[{col_label}={n}] (from avd[{row_idx}])"),
                            );
                            snapshots.entry(label).or_insert(v);
                        }
                    }
                }
            }
        }
    }

    // ── MDB index & candidates ───────────────────────────────────────────────
    eprintln!("Building MDB index (scanning Data/*.zip)...");
    let mdb_index = build_mdb_index(&game_dir);
    eprintln!("MDB index: {} entries", mdb_index.len());

    // race/gender 3-letter chunk: strip leading "P_" from body_prefix → EEM
    let bp_upper = body_prefix.to_uppercase();
    let body_chunk = bp_upper.strip_prefix("P_").unwrap_or(&bp_upper);

    let body_re_prefix = format!("p_{}_", body_chunk.to_lowercase());
    let mut body_candidates: Vec<Value> = mdb_index
        .iter()
        .filter(|(name, _)| name.starts_with(&body_re_prefix) && name.contains("_body"))
        .map(|(name, source)| json!({"name": name, "source": source}))
        .collect();
    body_candidates.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });

    let mut helm_candidates_all: Vec<Value> = mdb_index
        .iter()
        .filter(|(name, _)| name.starts_with(&body_re_prefix) && name.contains("_helm"))
        .map(|(name, source)| json!({"name": name, "source": source}))
        .collect();
    helm_candidates_all.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });

    // ── Resolver analysis ────────────────────────────────────────────────────
    let chest_variation_effective = chest_item
        .as_ref()
        .and_then(|v| v["Variation"].as_i64())
        .map(|v| (v + 1).max(1))
        .unwrap_or(1);

    let armor_prefix_for = |avt: usize| -> Vec<String> {
        let mut prefixes = Vec::new();
        if let Some(t) = tdas.get("armor") {
            if let Some(p) = t
                .get_cell_by_name(avt, "prefix")
                .ok()
                .flatten()
                .filter(|s| !s.is_empty())
            {
                prefixes.push(p.to_string());
            }
            // -1 fallback
            if avt > 0
                && let Some(p) = t
                    .get_cell_by_name(avt - 1, "prefix")
                    .ok()
                    .flatten()
                    .filter(|s| !s.is_empty())
                && !prefixes.contains(&p.to_string())
            {
                prefixes.push(format!("{p} (avt-1 fallback)"));
            }
        }
        prefixes
    };

    let chest_prefixes = armor_prefix_for(chest_avt);

    let mdb_lookup = |name: &str| -> Value {
        let lower = name.to_lowercase();
        match mdb_index.get(&lower) {
            Some(src) => json!({"name": lower, "exists": true, "source": src}),
            None => json!({"name": lower, "exists": false, "source": null}),
        }
    };

    let mut body_resolver_candidates: Vec<Value> = Vec::new();
    let mut helm_resolver_candidates: Vec<Value> = Vec::new();

    for pfx_raw in &chest_prefixes {
        let pfx = pfx_raw.split(' ').next().unwrap_or(pfx_raw);
        // primary variation
        let body_name = format!("{body_prefix}_{pfx}_Body{chest_variation_effective:02}");
        body_resolver_candidates.push(mdb_lookup(&body_name));
        // variation=1 fallback
        let body_fb = format!("{body_prefix}_{pfx}_Body01");
        if chest_variation_effective != 1 {
            body_resolver_candidates.push(mdb_lookup(&body_fb));
        }
        // helm uses chest prefix + helm AVT
        let helm_name = format!("{body_prefix}_{pfx}_Helm{helm_avt:02}");
        helm_resolver_candidates.push(mdb_lookup(&helm_name));
    }

    let resolver_analysis = json!({
        "body_prefix": body_prefix,
        "chest_avt": chest_avt,
        "chest_variation_raw": chest_item.as_ref().and_then(|v| v["Variation"].as_i64()),
        "chest_variation_effective": chest_variation_effective,
        "chest_armor_prefixes": chest_prefixes,
        "helm_avt": helm_avt,
        "current_code_output": {
            "body_candidates": body_resolver_candidates,
            "helm_candidates_current": helm_resolver_candidates.clone(),
        },
        "alternative_helmet_hypothesis": {
            "note": "Current code ALSO uses chest prefixes for helmet — formula is identical. If helm_avt differs from chest_avt this determines the variant number only.",
            "helm_candidates_alt": helm_resolver_candidates,
        },
    });

    // ── Assemble output ──────────────────────────────────────────────────────
    let output = json!({
        "character": {
            "first_name": first_name,
            "last_name": last_name,
            "race": race,
            "gender": gender,
            "appearance_type": appearance_type,
            "body_prefix": body_prefix,
            "nwn2_model_body_raw": nwn2_model_body,
            "gender_letter": gender_letter,
        },
        "equipped": {
            "chest": chest_item,
            "helm": helm_item,
            "boots": boots_item,
            "gloves": gloves_item,
            "all_items": all_items,
            "total_items": equip_list.len(),
        },
        "root_fields": root_field_names,
        "2da_snapshots": snapshots,
        "mdb_index": {
            "total_entries": mdb_index.len(),
            "body_candidates": body_candidates,
            "helm_candidates": helm_candidates_all,
        },
        "resolver_analysis": resolver_analysis,
    });

    // ── Print human summary ───────────────────────────────────────────────────
    eprintln!(
        "Character: {first_name} {last_name} — appearance {appearance_type}, body_prefix {body_prefix}"
    );
    if let Some(ci) = output["equipped"]["chest"].as_object() {
        eprintln!(
            "Chest: tag={} — base_item={}, AVT={}, Variation={}→effective {}, prefixes={:?}",
            ci.get("Tag").and_then(Value::as_str).unwrap_or("?"),
            ci.get("BaseItem").and_then(Value::as_i64).unwrap_or(-1),
            ci.get("ArmorVisualType")
                .and_then(Value::as_i64)
                .unwrap_or(-1),
            ci.get("Variation").and_then(Value::as_i64).unwrap_or(-1),
            chest_variation_effective,
            chest_prefixes,
        );
    }
    if let Some(hi) = output["equipped"]["helm"].as_object() {
        eprintln!(
            "Helm:  tag={} — base_item={}, AVT={}",
            hi.get("Tag").and_then(Value::as_str).unwrap_or("?"),
            hi.get("BaseItem").and_then(Value::as_i64).unwrap_or(-1),
            hi.get("ArmorVisualType")
                .and_then(Value::as_i64)
                .unwrap_or(-1),
        );
    }
    eprintln!("\nCurrent resolver body candidates (exists yes/no):");
    for c in output["resolver_analysis"]["current_code_output"]["body_candidates"]
        .as_array()
        .into_iter()
        .flatten()
    {
        eprintln!(
            "  {} — {}",
            c["name"].as_str().unwrap_or("?"),
            if c["exists"].as_bool().unwrap_or(false) {
                "YES"
            } else {
                "no"
            }
        );
    }
    eprintln!("\nAll existing {body_chunk} *_Body*.mdb files:");
    for c in output["mdb_index"]["body_candidates"]
        .as_array()
        .into_iter()
        .flatten()
    {
        eprintln!(
            "  {} ({})",
            c["name"].as_str().unwrap_or("?"),
            c["source"].as_str().unwrap_or("?")
        );
    }
    eprintln!("\nAll existing {body_chunk} *_Helm*.mdb files:");
    for c in output["mdb_index"]["helm_candidates"]
        .as_array()
        .into_iter()
        .flatten()
    {
        eprintln!(
            "  {} ({})",
            c["name"].as_str().unwrap_or("?"),
            c["source"].as_str().unwrap_or("?")
        );
    }

    let out_path = save_dir.join("armor_dump.json");
    let json_str = serde_json::to_string_pretty(&output)?;
    std::fs::write(&out_path, &json_str)
        .with_context(|| format!("writing {}", out_path.display()))?;
    eprintln!("\nJSON written to: {}", out_path.display());

    Ok(())
}
