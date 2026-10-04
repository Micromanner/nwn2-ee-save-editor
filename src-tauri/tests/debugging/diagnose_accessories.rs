use app_lib::parsers::gff::types::GffValue;
use app_lib::services::resource_manager::ResourceManager;
use app_lib::services::savegame_handler::SaveGameHandler;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[path = "common/mod.rs"]
mod common;

fn load_character_bic(
    save_path: &std::path::Path,
) -> indexmap::IndexMap<String, GffValue<'static>> {
    let handler = SaveGameHandler::new(save_path, false).expect("Failed to create handler");
    let bic_data = handler
        .extract_player_bic()
        .expect("extract")
        .expect("no bic");
    let gff =
        app_lib::parsers::gff::parser::GffParser::from_bytes(bic_data).expect("Failed to parse");
    let fields = gff
        .read_struct_fields(0)
        .expect("Failed to read root struct");
    let owned: indexmap::IndexMap<String, GffValue<'static>> = fields
        .into_iter()
        .map(|(k, v)| (k, v.force_owned().expect("force owned")))
        .collect();
    owned
}

#[tokio::test]
async fn diagnose_accessory_mesh_names() {
    let save_path = PathBuf::from(
        r"C:\Users\01tee\Documents\Neverwinter Nights 2\saves\000049 - 01-08-2025-11-19_backup_20250815_010644",
    );
    if !save_path.exists() {
        println!("Save not found, skipping");
        return;
    }

    let fields = load_character_bic(&save_path);

    // Dump character-level tint fields
    println!("\n=== Character-level tint fields ===");
    for field_name in ["Tintable", "Color_Skin", "Tint_Head", "Tint_Hair"] {
        if let Some(val) = fields.get(field_name) {
            println!("  {field_name}: {val:?}");
        } else {
            println!("  {field_name}: NOT PRESENT");
        }
    }

    // Find chest armor
    let equip_list = match fields.get("Equip_ItemList") {
        Some(GffValue::ListOwned(maps)) => maps,
        _ => {
            println!("No Equip_ItemList");
            return;
        }
    };

    let chest = equip_list
        .iter()
        .find(|item| matches!(item.get("__struct_id__"), Some(GffValue::Dword(2))))
        .expect("No chest armor");

    let armor_visual_type = match chest.get("ArmorVisualType") {
        Some(GffValue::Byte(v)) => *v as i32,
        Some(GffValue::Int(v)) => *v,
        _ => 0,
    };
    println!("ArmorVisualType: {armor_visual_type}");

    // List all AC* fields and their Accessory values
    let ac_fields = [
        ("ACLtShoulder", "ShoulderL"),
        ("ACRtShoulder", "ShoulderR"),
        ("ACLtBracer", "BracerL"),
        ("ACRtBracer", "BracerR"),
        ("ACLtElbow", "ElbowL"),
        ("ACRtElbow", "ElbowR"),
        ("ACLtArm", "ArmL"),
        ("ACRtArm", "ArmR"),
        ("ACLtHip", "HipL"),
        ("ACRtHip", "HipR"),
        ("ACFtHip", "FtHip"),
        ("ACBkHip", "BkHip"),
        ("ACLtLeg", "LegL"),
        ("ACRtLeg", "LegR"),
        ("ACLtShin", "ShinL"),
        ("ACRtShin", "ShinR"),
        ("ACLtKnee", "KneeL"),
        ("ACRtKnee", "KneeR"),
        ("ACLtFoot", "FootL"),
        ("ACRtFoot", "FootR"),
        ("ACLtAnkle", "AnkleL"),
        ("ACRtAnkle", "AnkleR"),
    ];

    println!("\n=== AC* Accessory Fields ===");
    for (gff_field, mesh_suffix) in &ac_fields {
        let item = match chest.get(*gff_field) {
            Some(GffValue::StructOwned(s)) => s,
            _ => {
                println!("  {gff_field}: NOT PRESENT");
                continue;
            }
        };
        let accessory = match item.get("Accessory") {
            Some(GffValue::Byte(v)) => *v,
            _ => 0,
        };
        if accessory == 0 {
            println!("  {gff_field}: Accessory=0 (none)");
            continue;
        }
        println!(
            "  {gff_field}: Accessory={accessory} -> expected mesh suffix: {mesh_suffix}{accessory:02}"
        );
    }

    // Now try to find actual mesh files in the game data
    println!("\n=== Searching game data for accessory meshes ===");
    let nwn2_paths = Arc::new(RwLock::new(app_lib::config::NWN2Paths::new()));
    let rm = Arc::new(RwLock::new(ResourceManager::new(nwn2_paths)));
    {
        let mut rm_guard = rm.write().await;
        let _ = rm_guard.initialize().await;
    }
    let rm = rm.read().await;

    // The body prefix for this character (human male = P_HHM)
    // ArmorVisualType -> armor.2da prefix
    let prefixes_to_try = ["p_hhm_ch", "p_hhm_pl", "p_hhm_cl"];
    let suffixes_to_try = [
        "shoulder", "bracer", "elbow", "arm", "hip", "knee", "shin", "leg", "foot", "ankle",
    ];

    for prefix in &prefixes_to_try {
        for suffix in &suffixes_to_try {
            let search = format!("{prefix}_{suffix}");
            let found = rm.list_resources_by_prefix(&search, "mdb");
            if !found.is_empty() {
                println!("  {search}*: {:?}", &found[..found.len().min(5)]);
            }
        }
    }

    // Also try without left/right distinction
    for prefix in &prefixes_to_try {
        let search = format!("{prefix}_");
        let found = rm.list_resources_by_prefix(&search, "mdb");
        if !found.is_empty() {
            println!("\n  All {search}* MDBs ({} total):", found.len());
            for f in &found {
                println!("    {f}");
            }
        }
    }

    // Search for accessory meshes with broader patterns
    println!("\n=== Broader accessory mesh search ===");
    let broad_searches = [
        "p_hhm_shoulder",
        "p_hhm_bracer",
        "p_hhm_knee",
        "p_hhm_elbow",
        "p_hhm_arm",
        "p_hhm_hip",
        "p_hhm_shin",
        "p_hhm_leg",
        "p_hhm_foot",
        "p_hhm_ankle",
        "p_hhm_ac",
        "p_hhm_a_",
        // Maybe accessories are just numbered without prefix?
        "shoulder",
        "bracer",
    ];
    for search in &broad_searches {
        let found = rm.list_resources_by_prefix(search, "mdb");
        if !found.is_empty() {
            println!("  {search}*: {} matches", found.len());
            for f in found.iter().take(8) {
                println!("    {f}");
            }
            if found.len() > 8 {
                println!("    ... and {} more", found.len() - 8);
            }
        }
    }

    // Load the item TEMPLATE (nw_maarcl002.uti) and check its tints
    println!("\n=== Item template tints (nw_maarcl002.uti) ===");
    if let Ok(uti_bytes) = rm.get_resource_bytes("nw_maarcl002", "uti") {
        println!("  Template found: {} bytes", uti_bytes.len());
        let gff =
            app_lib::parsers::gff::parser::GffParser::from_bytes(uti_bytes).expect("parse uti");
        let fields = gff.read_struct_fields(0).expect("read root");
        for (k, v) in &fields {
            let k_lower = k.to_lowercase();
            if k_lower.contains("tint")
                || k_lower.contains("armor")
                || k_lower == "variation"
                || k_lower == "armorvisualtype"
            {
                let owned = v.clone().force_owned().expect("force owned");
                println!("  {k}: {owned:?}");
            }
        }
    } else {
        println!("  nw_maarcl002.uti NOT FOUND in game data");
    }

    // Check armoraccessory 2DA
    println!("\n=== Checking for armoraccessory-related 2DA ===");
    for table in [
        "armoraccessory",
        "armor_accessory",
        "accessory",
        "itemaccessory",
        "capart",
    ] {
        let found = rm.get_resource_bytes(table, "2da");
        println!(
            "  {table}.2da: {}",
            if found.is_ok() { "FOUND" } else { "not found" }
        );
    }

    // List ALL body meshes for P_HHM across all armor prefixes
    println!("\n=== ALL P_HHM body meshes ===");
    let all_prefixes = ["cl", "cp", "le", "ls", "ch", "sc", "ba", "ph", "pf"];
    for pfx in &all_prefixes {
        let search = format!("p_hhm_{pfx}_body");
        let found = rm.list_resources_by_prefix(&search, "mdb");
        if !found.is_empty() {
            println!("  {pfx} ({} bodies): {:?}", found.len(), found);
        }
    }

    // Dump ALL CH body variants - verts + textures
    println!("\n=== ALL CH body variants ===");
    for i in 1..=6 {
        let name = format!("p_hhm_ch_body{i:02}");
        if let Ok(mdb_bytes) = rm.get_resource_bytes(&name, "mdb") {
            let mdb =
                app_lib::parsers::mdb::parser::MdbParser::parse(&mdb_bytes).expect("parse mdb");
            for mesh in &mdb.skin_meshes {
                if mesh.name.contains("_L0") {
                    continue;
                }
                println!(
                    "  {}: verts={}, diffuse='{}', tint='{}'",
                    mesh.name,
                    mesh.vertices.len(),
                    mesh.material.diffuse_map_name,
                    mesh.material.tint_map_name,
                );
            }
        }
    }
    // Also check naked body for comparison
    if let Ok(mdb_bytes) = rm.get_resource_bytes("p_hhm_nk_body01", "mdb") {
        let mdb = app_lib::parsers::mdb::parser::MdbParser::parse(&mdb_bytes).expect("parse");
        for mesh in &mdb.skin_meshes {
            if mesh.name.contains("_L0") {
                continue;
            }
            println!(
                "  {}: verts={}, diffuse='{}', tint='{}'",
                mesh.name,
                mesh.vertices.len(),
                mesh.material.diffuse_map_name,
                mesh.material.tint_map_name,
            );
        }
    }

    // Check what armor.2da says for visual type 4
    println!("\n=== armor.2da lookup ===");
    let ctx = common::create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data");
    if let Some(armor_table) = game_data.get_table("armor") {
        for row_id in 0..10 {
            if let Some(row) = armor_table.get_by_id(row_id) {
                let prefix = app_lib::utils::parsing::row_str(&row, "prefix").unwrap_or_default();
                let label = app_lib::utils::parsing::row_str(&row, "label").unwrap_or_default();
                println!("  armor[{row_id}]: prefix='{prefix}', label='{label}'");
            }
        }
    }

    // Dump material data (textures) from body meshes
    println!("\n=== Body mesh material/texture info ===");
    for body in ["p_hhm_ch_body01", "p_hhm_ch_body04", "p_hhm_cl_body04"] {
        if let Ok(mdb_bytes) = rm.get_resource_bytes(body, "mdb") {
            let mdb = app_lib::parsers::mdb::parser::MdbParser::parse(&mdb_bytes).expect("parse");
            for mesh in &mdb.skin_meshes {
                if mesh.name.contains("_L0") {
                    continue;
                }
                println!(
                    "  {}: diffuse='{}', normal='{}', tint='{}', glow='{}'",
                    mesh.name,
                    mesh.material.diffuse_map_name,
                    mesh.material.normal_map_name,
                    mesh.material.tint_map_name,
                    mesh.material.glow_map_name,
                );
            }
        } else {
            println!("  {body}: NOT FOUND");
        }
    }
}
