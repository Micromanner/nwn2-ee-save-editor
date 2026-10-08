use app_lib::character::ItemAppearance;
use app_lib::parsers::gff::types::GffValue;
use app_lib::services::resource_manager::ResourceManager;
use app_lib::services::savegame_handler::SaveGameHandler;
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[path = "../common/mod.rs"]
mod common;

fn i32_of(fields: &indexmap::IndexMap<String, GffValue<'_>>, key: &str) -> Option<i32> {
    match fields.get(key)? {
        GffValue::Byte(b) => Some(i32::from(*b)),
        GffValue::Char(c) => Some(*c as i32),
        GffValue::Word(w) => Some(i32::from(*w)),
        GffValue::Short(s) => Some(i32::from(*s)),
        GffValue::Dword(d) => Some(*d as i32),
        GffValue::Int(i) => Some(*i),
        _ => None,
    }
}

fn resolve_struct<'a>(val: &'a GffValue<'a>) -> Option<indexmap::IndexMap<String, GffValue<'a>>> {
    match val {
        GffValue::StructOwned(s) => Some(s.as_ref().clone()),
        GffValue::Struct(lazy) => Some(lazy.force_load().expect("load struct")),
        _ => None,
    }
}

fn dump_item(out: &mut String, item: &indexmap::IndexMap<String, GffValue<'_>>, label: &str) {
    writeln!(out, "\n--- {label} ---").unwrap();
    for key in [
        "TemplateResRef",
        "BaseItem",
        "Variation",
        "ModelPart1",
        "ModelPart2",
        "ModelPart3",
        "ArmorVisualType",
    ] {
        if let Some(val) = item.get(key) {
            writeln!(
                out,
                "  {key}: {:?}",
                i32_of(item, key)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| format!("{val:?}"))
            )
            .unwrap();
        } else {
            writeln!(out, "  {key}: MISSING").unwrap();
        }
    }
    // Nested boots/gloves
    for nested_key in ["Boots", "Gloves"] {
        if let Some(val) = item.get(nested_key)
            && let Some(nested) = resolve_struct(val)
        {
            writeln!(out, "  Nested {nested_key}:").unwrap();
            for k in ["ArmorVisualType", "Variation"] {
                writeln!(
                    out,
                    "    {k}: {:?}",
                    i32_of(&nested, k)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "?".into())
                )
                .unwrap();
            }
        }
    }
}

#[tokio::test]
async fn diagnose_item_models() {
    let mut out = String::new();

    // Load resource manager to access baseitems.2da
    let nwn2_paths = Arc::new(RwLock::new(app_lib::config::NWN2Paths::new()));
    let rm = Arc::new(RwLock::new(ResourceManager::new(nwn2_paths)));
    {
        let mut rm_guard = rm.write().await;
        let _ = rm_guard.initialize().await;
    }
    let rm_read = rm.read().await;

    // Load baseitems.2da to cross-reference BaseItem → modeltype/label
    writeln!(out, "=== baseitems.2da lookup ===").unwrap();
    let baseitems_bytes = rm_read.get_resource_bytes("baseitems", "2da").ok();
    let baseitems_text = baseitems_bytes
        .as_ref()
        .map(|b| String::from_utf8_lossy(b).to_string());

    let save_path = PathBuf::from(
        r"C:\Users\01tee\Documents\Neverwinter Nights 2\saves\000064 - 17-04-2026-09-09",
    );
    if !save_path.exists() {
        writeln!(out, "Save not found: {}", save_path.display()).unwrap();
        let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target_test")
            .join("item_models_diag.txt");
        std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
        std::fs::write(&out_path, &out).unwrap();
        eprintln!("Output: {}", out_path.display());
        return;
    }

    let handler = SaveGameHandler::new(&save_path).expect("handler");
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

    // Full GameData to run resolve_model_resrefs against.
    let ctx = common::create_test_context().await;
    let game_data = &ctx.game_data;

    // Dump armor.2da rows 0-20 so we can see which prefix each VT maps to.
    writeln!(out, "\n=== armor.2da (rows 0..20) ===").unwrap();
    if let Some(armor_table) = game_data.get_table("armor") {
        for row_id in 0..20 {
            if let Some(row) = armor_table.get_by_id(row_id) {
                let prefix = app_lib::utils::parsing::row_str(&row, "prefix").unwrap_or_default();
                let label = app_lib::utils::parsing::row_str(&row, "label").unwrap_or_default();
                writeln!(out, "  armor[{row_id}]: prefix='{prefix}', label='{label}'").unwrap();
            }
        }
    }

    // Compare: UTI template Variation vs what's in the save.
    // If template `mwa_hvfp_ada_4.uti` has Variation=3, then the suffix "_4"
    // means filename Body04 and GFF Variation=3 is 0-indexed (needs +1).
    // If it has Variation=4, the GFF is 1-indexed (maps directly).
    writeln!(out, "\n=== Template UTI probe: mwa_hvfp_ada_4 ===").unwrap();
    if let Ok(uti_bytes) = rm_read.get_resource_bytes("mwa_hvfp_ada_4", "uti") {
        let uti = app_lib::parsers::gff::parser::GffParser::from_bytes(uti_bytes).expect("parse");
        let uti_fields = uti.read_struct_fields(0).expect("read");
        let owned: indexmap::IndexMap<String, GffValue<'static>> = uti_fields
            .into_iter()
            .map(|(k, v)| (k, v.force_owned().expect("force owned")))
            .collect();
        for key in ["Variation", "ArmorVisualType", "BaseItem"] {
            writeln!(out, "  {key}: {:?}", owned.get(key)).unwrap();
        }
    } else {
        writeln!(out, "  UTI not found").unwrap();
    }
    // Same for mwa_hvfp_mth_4
    writeln!(out, "\n=== Template UTI probe: mwa_hvfp_mth_4 ===").unwrap();
    if let Ok(uti_bytes) = rm_read.get_resource_bytes("mwa_hvfp_mth_4", "uti") {
        let uti = app_lib::parsers::gff::parser::GffParser::from_bytes(uti_bytes).expect("parse");
        let uti_fields = uti.read_struct_fields(0).expect("read");
        let owned: indexmap::IndexMap<String, GffValue<'static>> = uti_fields
            .into_iter()
            .map(|(k, v)| (k, v.force_owned().expect("force owned")))
            .collect();
        for key in ["Variation", "ArmorVisualType", "BaseItem"] {
            writeln!(out, "  {key}: {:?}", owned.get(key)).unwrap();
        }
    } else {
        writeln!(out, "  UTI not found").unwrap();
    }

    // Campaign x2 breastplates with sequential suffixes 00..04 — perfect for
    // confirming whether Variation is 0-indexed (suffix matches Variation) or
    // 1-indexed (suffix = Variation - 1).
    // Inspect the 3 Full-Plate Body meshes to see what textures each uses.
    // The user says Darksteel plate loads "chainmail-looking" — if Body02
    // textures match a chainmail style while Body03 looks like plate, the
    // GFF Variation=2 needs +1 (i.e. it's 0-indexed).
    writeln!(out, "\n=== PF Body mesh texture probe ===").unwrap();
    for name in ["p_hhm_pf_body01", "p_hhm_pf_body02", "p_hhm_pf_body03"] {
        writeln!(out, "  {name}:").unwrap();
        if let Ok(mdb_bytes) = rm_read.get_resource_bytes(name, "mdb") {
            let mdb = app_lib::parsers::mdb::parser::MdbParser::parse(&mdb_bytes).expect("parse");
            for mesh in &mdb.skin_meshes {
                if mesh.name.contains("_L0") {
                    continue;
                }
                writeln!(
                    out,
                    "    mesh={}, diffuse='{}', normal='{}', tint='{}'",
                    mesh.name,
                    mesh.material.diffuse_map_name,
                    mesh.material.normal_map_name,
                    mesh.material.tint_map_name
                )
                .unwrap();
            }
        }
    }

    for tmpl in [
        "nx2_bplate_00",
        "nx2_bplate_01",
        "nx2_bplate_02",
        "nx2_bplate_04",
    ] {
        writeln!(out, "\n=== Template UTI probe: {tmpl} ===").unwrap();
        if let Ok(uti_bytes) = rm_read.get_resource_bytes(tmpl, "uti") {
            let uti =
                app_lib::parsers::gff::parser::GffParser::from_bytes(uti_bytes).expect("parse");
            let uti_fields = uti.read_struct_fields(0).expect("read");
            let owned: indexmap::IndexMap<String, GffValue<'static>> = uti_fields
                .into_iter()
                .map(|(k, v)| (k, v.force_owned().expect("force owned")))
                .collect();
            for key in ["Variation", "ArmorVisualType", "BaseItem"] {
                writeln!(out, "  {key}: {:?}", owned.get(key)).unwrap();
            }
        } else {
            writeln!(out, "  UTI not found").unwrap();
        }
    }

    writeln!(out, "\n=== Equipped Items ===").unwrap();
    if let Some(GffValue::ListOwned(equip_list)) = owned.get("Equip_ItemList") {
        for (idx, item) in equip_list.iter().enumerate() {
            let base_item = i32_of(item, "BaseItem").unwrap_or(-1);
            let struct_id = match item.get("__struct_id__") {
                Some(GffValue::Dword(d)) => *d as i32,
                _ => -1,
            };
            let label = format!("Equip[{idx}] struct_id={struct_id} BaseItem={base_item}");
            dump_item(&mut out, item, &label);

            // Show what ItemAppearance::from_gff decodes it to
            let appearance = ItemAppearance::from_gff(item);
            writeln!(
                out,
                "  decoded: variation={}, model_parts={:?}, armor_visual_type={:?}",
                appearance.variation, appearance.model_parts, appearance.armor_visual_type
            )
            .unwrap();

            // Show the actual resref groups the resolver produces
            let groups = appearance.resolve_model_resrefs(base_item, game_data, None);
            writeln!(out, "  resolved groups:").unwrap();
            for (gi, g) in groups.iter().enumerate() {
                writeln!(out, "    group[{gi}]: {g:?}").unwrap();
            }

            // Look up baseitems row
            if let Some(ref text) = baseitems_text {
                for (i, line) in text.lines().enumerate() {
                    if i < 3 {
                        continue;
                    }
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.is_empty() {
                        continue;
                    }
                    if let Ok(row_id) = parts[0].parse::<i32>()
                        && row_id == base_item
                    {
                        writeln!(out, "  baseitems row {row_id}: {line}").unwrap();
                        break;
                    }
                }
            }
        }
    } else {
        writeln!(out, "No Equip_ItemList").unwrap();
    }

    writeln!(out, "\n=== Inventory (first 20) ===").unwrap();
    if let Some(GffValue::ListOwned(inv_list)) = owned.get("ItemList") {
        for (idx, item) in inv_list.iter().take(20).enumerate() {
            let base_item = i32_of(item, "BaseItem").unwrap_or(-1);
            let label = format!("Inv[{idx}] BaseItem={base_item}");
            dump_item(&mut out, item, &label);
        }
    }

    let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target_test")
        .join("item_models_diag.txt");
    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
    std::fs::write(&out_path, &out).unwrap();
    eprintln!("Output: {}", out_path.display());
}
