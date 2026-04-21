use app_lib::parsers::gff::parser::GffParser;
use app_lib::parsers::gff::types::GffValue;
use app_lib::parsers::gff::writer::GffWriter;
use app_lib::services::savegame_handler::{PlayerOutputs, SaveGameHandler};
use std::path::PathBuf;

fn set_tint_channel(
    tint_struct: &mut indexmap::IndexMap<String, GffValue<'static>>,
    channel_key: &str,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
) {
    let mut channel = indexmap::IndexMap::new();
    channel.insert("r".to_string(), GffValue::Byte(r));
    channel.insert("g".to_string(), GffValue::Byte(g));
    channel.insert("b".to_string(), GffValue::Byte(b));
    channel.insert("a".to_string(), GffValue::Byte(a));
    channel.insert("__struct_id__".to_string(), GffValue::Dword(0));
    tint_struct.insert(
        channel_key.to_string(),
        GffValue::StructOwned(Box::new(channel)),
    );
}

fn build_tintable() -> GffValue<'static> {
    let mut tint = indexmap::IndexMap::new();
    set_tint_channel(&mut tint, "1", 255, 0, 0, 255); // Channel 1 = pure RED
    set_tint_channel(&mut tint, "2", 0, 255, 0, 255); // Channel 2 = pure GREEN
    set_tint_channel(&mut tint, "3", 0, 0, 255, 255); // Channel 3 = pure BLUE
    tint.insert("__struct_id__".to_string(), GffValue::Dword(0));

    let mut tintable = indexmap::IndexMap::new();
    tintable.insert("Tint".to_string(), GffValue::StructOwned(Box::new(tint)));
    tintable.insert("__struct_id__".to_string(), GffValue::Dword(0));
    GffValue::StructOwned(Box::new(tintable))
}

fn set_chest_tint(fields: &mut indexmap::IndexMap<String, GffValue<'static>>) -> bool {
    let equip_list = match fields.get_mut("Equip_ItemList") {
        Some(GffValue::ListOwned(list)) => list,
        _ => return false,
    };

    let chest = equip_list
        .iter_mut()
        .find(|item| matches!(item.get("__struct_id__"), Some(GffValue::Dword(2))));

    if let Some(chest) = chest {
        chest.insert("Tintable".to_string(), build_tintable());
        true
    } else {
        false
    }
}

fn parse_and_own(
    data: Vec<u8>,
) -> (
    indexmap::IndexMap<String, GffValue<'static>>,
    String,
    String,
) {
    let gff = GffParser::from_bytes(data).expect("parse GFF");
    let file_type = gff.file_type.clone();
    let file_version = gff.file_version.clone();
    let fields = gff.read_struct_fields(0).expect("read root");
    let owned: indexmap::IndexMap<String, GffValue<'static>> = fields
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();
    (owned, file_type, file_version)
}

fn write_gff(
    fields: indexmap::IndexMap<String, GffValue<'static>>,
    file_type: &str,
    file_version: &str,
) -> Vec<u8> {
    let root_struct_id = match fields.get("__struct_id__") {
        Some(GffValue::Dword(id)) => *id,
        _ => 0xFFFFFFFF,
    };
    let mut writer = GffWriter::new(file_type, file_version);
    writer
        .write_with_struct_id(fields, root_struct_id)
        .expect("write GFF")
}

#[test]
fn set_armor_tint_test_colors() {
    let save_path = PathBuf::from(
        r"C:\Users\01tee\Documents\Neverwinter Nights 2\saves\000053 - 14-04-2026-12-26",
    );
    if !save_path.exists() {
        println!("Save not found");
        return;
    }

    // Create test copy
    let test_save = save_path.parent().unwrap().join("000053 - tint-test");
    if test_save.exists() {
        std::fs::remove_dir_all(&test_save).expect("cleanup");
    }
    fn copy_dir(src: &std::path::Path, dst: &std::path::Path) {
        std::fs::create_dir_all(dst).unwrap();
        for entry in std::fs::read_dir(src).unwrap() {
            let entry = entry.unwrap();
            let dst_path = dst.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_dir(&entry.path(), &dst_path);
            } else {
                std::fs::copy(entry.path(), dst_path).unwrap();
            }
        }
    }
    copy_dir(&save_path, &test_save);

    let handler = SaveGameHandler::new(&test_save, false, false).expect("handler");

    // Modify playerlist.ifo (the file the game actually reads)
    let ifo_data = handler.extract_file("playerlist.ifo").expect("extract ifo");
    let (mut ifo_fields, ifo_type, ifo_ver) = parse_and_own(ifo_data);

    // playerlist.ifo has character data nested differently - find it
    // The player data is usually in a list, let's check structure
    println!("IFO top-level keys:");
    for (k, v) in &ifo_fields {
        println!("  {k}: {:?}", std::mem::discriminant(v));
    }

    // Try to set tint in IFO
    let ifo_modified = set_chest_tint(&mut ifo_fields);
    println!("IFO chest tint set: {ifo_modified}");

    // If IFO has player data in a sub-list, check that too
    if !ifo_modified {
        // Try PlayerList or similar
        if let Some(GffValue::ListOwned(player_list)) = ifo_fields.get_mut("PlayerList") {
            for player in player_list.iter_mut() {
                if set_chest_tint(player) {
                    println!("Set tint in PlayerList entry");
                }
            }
        }
    }

    // Modify player.bic too
    let bic_data = handler
        .extract_player_bic()
        .expect("extract")
        .expect("no bic");
    let (mut bic_fields, bic_type, bic_ver) = parse_and_own(bic_data);
    let bic_modified = set_chest_tint(&mut bic_fields);
    println!("BIC chest tint set: {bic_modified}");

    // Write both back
    let new_ifo = write_gff(ifo_fields, &ifo_type, &ifo_ver);
    let new_bic = write_gff(bic_fields, &bic_type, &bic_ver);

    let mut handler = SaveGameHandler::new(&test_save, false, false).expect("handler");
    handler
        .rewrite_player_files(|_src| Ok(PlayerOutputs {
            playerlist: new_ifo,
            player_bic: Some(new_bic),
        }))
        .expect("update");

    println!("Test save written to: {}", test_save.display());
    println!("Tints: Ch1=RED(255,0,0), Ch2=GREEN(0,255,0), Ch3=BLUE(0,0,255)");
}
