use std::sync::Arc;

use super::super::common::load_test_gff;

use app_lib::parsers::gff::parser::GffParser;
use app_lib::parsers::gff::types::GffValue;

fn parse_character(name: &str) -> Arc<GffParser> {
    let bytes = load_test_gff(name);
    GffParser::from_bytes(bytes).expect("Failed to parse character GFF")
}

fn fixtures_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gff")
}

#[test]
fn test_list_available_character_fixtures() {
    let gff_path = fixtures_path();

    println!("\n=== Available Character Fixtures ===");
    println!(
        "{:<25} {:<15} {:<15} {:<6}",
        "Character", "Race", "Class", "Level"
    );
    println!("{}", "-".repeat(65));

    for entry in std::fs::read_dir(&gff_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_dir() {
            let dir_name = path.file_name().unwrap().to_string_lossy();

            for bic_entry in std::fs::read_dir(&path).unwrap() {
                let bic_entry = bic_entry.unwrap();
                let bic_path = bic_entry.path();

                if bic_path.extension().map(|e| e == "bic").unwrap_or(false) {
                    let file_name = bic_path.file_name().unwrap().to_string_lossy();
                    let rel_path = format!("{dir_name}/{file_name}");

                    if let Ok(parser) = GffParser::from_bytes(load_test_gff(&rel_path))
                        && let Ok(root) = parser.read_struct_fields(0)
                    {
                        let name = match root.get("FirstName") {
                            Some(GffValue::LocString(ls)) => ls
                                .substrings
                                .first()
                                .map(|s| s.string.to_string())
                                .unwrap_or_default(),
                            _ => String::new(),
                        };

                        let race_id = match root.get("Race") {
                            Some(GffValue::Byte(r)) => *r,
                            _ => 0,
                        };

                        let (class_id, total_levels) = match root.get("ClassList") {
                            Some(GffValue::List(classes)) => {
                                let first_class = classes
                                    .first()
                                    .map(|c| {
                                        let fields = c.force_load().expect("load struct");
                                        match fields.get("Class") {
                                            Some(GffValue::Int(id)) => *id,
                                            _ => 0,
                                        }
                                    })
                                    .unwrap_or(0);

                                let lvls = classes
                                    .iter()
                                    .map(|c| {
                                        let fields = c.force_load().expect("load struct");
                                        match fields.get("ClassLevel") {
                                            Some(GffValue::Short(l)) => i32::from(*l),
                                            _ => 0,
                                        }
                                    })
                                    .sum::<i32>();

                                (first_class, lvls)
                            }
                            _ => (0, 0),
                        };

                        println!(
                            "{name:<25} Race {race_id:>2}       Class {class_id:>2}       {total_levels:>3}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn test_character_root_structure() {
    let parser = parse_character("occidiooctavon/occidiooctavon1.bic");

    let root = parser.read_struct_fields(0).expect("Failed to read root");

    let expected_fields = [
        "FirstName",
        "LastName",
        "ClassList",
        "FeatList",
        "SkillList",
    ];
    for field in expected_fields {
        assert!(root.contains_key(field), "Missing expected field: {field}");
    }

    println!("\n=== Character Root Fields ===");
    for key in root.keys().take(20) {
        println!("  {key}");
    }
    println!("  ... ({} total fields)", root.len());
}

#[test]
fn test_character_identity() {
    let parser = parse_character("occidiooctavon/occidiooctavon1.bic");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    if let Some(GffValue::LocString(ls)) = root.get("FirstName") {
        println!("FirstName: {:?}", ls.substrings);
        assert!(
            !ls.substrings.is_empty(),
            "FirstName should have substrings"
        );
    }

    if let Some(GffValue::Byte(gender)) = root.get("Gender") {
        println!("Gender: {gender}");
    }

    if let Some(GffValue::Byte(race)) = root.get("Race") {
        println!("Race ID: {race}");
    }

    if let Some(GffValue::Word(subrace)) = root.get("Subrace") {
        println!("Subrace: {subrace}");
    }
}

#[test]
fn test_class_list_structure() {
    let parser = parse_character("occidiooctavon/occidiooctavon1.bic");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    let class_list = match root.get("ClassList") {
        Some(GffValue::List(list)) => list,
        _ => panic!("ClassList should be a List"),
    };

    println!("\n=== ClassList ===");
    println!("Classes: {}", class_list.len());

    for (idx, lazy_struct) in class_list.iter().enumerate() {
        let class_entry = lazy_struct.force_load().expect("load struct");

        let class_id = match class_entry.get("Class") {
            Some(GffValue::Int(id)) => *id,
            _ => 0,
        };
        let class_level = match class_entry.get("ClassLevel") {
            Some(GffValue::Short(lvl)) => *lvl,
            _ => 0,
        };

        println!("  Class {idx}: ID={class_id}, Level={class_level}");
    }
}

#[test]
fn test_lvlstatlist_structure() {
    let parser = parse_character("occidiooctavon/occidiooctavon1.bic");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    let lvlstat_list = match root.get("LvlStatList") {
        Some(GffValue::List(list)) => list,
        None => {
            println!("No LvlStatList found (character may be level 1)");
            return;
        }
        _ => panic!("LvlStatList should be a List"),
    };

    println!("\n=== LvlStatList (Level History) ===");
    println!("Levels: {}", lvlstat_list.len());

    for (idx, lazy_struct) in lvlstat_list.iter().take(5).enumerate() {
        let level_entry = lazy_struct.force_load().expect("load struct");

        let class_id = match level_entry.get("LvlStatClass") {
            Some(GffValue::Byte(id)) => i32::from(*id),
            _ => -1,
        };
        let hp_roll = match level_entry.get("LvlStatHitDie") {
            Some(GffValue::Byte(hp)) => *hp,
            _ => 0,
        };

        println!(
            "  Level {}: ClassID={}, HPRoll={}",
            idx + 1,
            class_id,
            hp_roll
        );

        if let Some(GffValue::List(feats)) = level_entry.get("FeatList") {
            println!("    Feats gained: {}", feats.len());
        }
    }
}

#[test]
fn test_feat_list_structure() {
    let parser = parse_character("occidiooctavon/occidiooctavon1.bic");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    let feat_list = match root.get("FeatList") {
        Some(GffValue::List(list)) => list,
        _ => panic!("FeatList should be a List"),
    };

    println!("\n=== FeatList ===");
    println!("Total feats: {}", feat_list.len());

    let mut feat_ids = Vec::new();
    for lazy_struct in feat_list {
        let feat_entry = lazy_struct.force_load().expect("load struct");

        if let Some(GffValue::Word(id)) = feat_entry.get("Feat") {
            feat_ids.push(*id);
        }
    }

    println!(
        "First 10 feat IDs: {:?}",
        &feat_ids[..feat_ids.len().min(10)]
    );
}

#[test]
fn test_skill_list_structure() {
    let parser = parse_character("occidiooctavon/occidiooctavon1.bic");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    let skill_list = match root.get("SkillList") {
        Some(GffValue::List(list)) => list,
        _ => panic!("SkillList should be a List"),
    };

    println!("\n=== SkillList ===");
    println!("Skills: {}", skill_list.len());

    for (idx, lazy_struct) in skill_list.iter().take(10).enumerate() {
        let skill_entry = lazy_struct.force_load().expect("load struct");

        let rank = match skill_entry.get("Rank") {
            Some(GffValue::Byte(r)) => *r,
            _ => 0,
        };

        if rank > 0 {
            println!("  Skill {idx}: {rank} ranks");
        }
    }
}

#[test]
fn test_ability_scores() {
    let parser = parse_character("occidiooctavon/occidiooctavon1.bic");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    println!("\n=== Ability Scores ===");

    for ability in ["Str", "Dex", "Con", "Int", "Wis", "Cha"] {
        let value = match root.get(ability) {
            Some(GffValue::Byte(v)) => *v,
            _ => 0,
        };
        println!("  {ability}: {value}");
    }
}

#[test]
fn test_equipment_slots() {
    let parser = parse_character("occidiooctavon/occidiooctavon4.bic");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    let slot_names: [(u16, &str); 14] = [
        (0x0001, "Head"),
        (0x0002, "Chest"),
        (0x0004, "Boots"),
        (0x0008, "Gloves"),
        (0x0010, "Right Hand"),
        (0x0020, "Left Hand"),
        (0x0040, "Cloak"),
        (0x0080, "Left Ring"),
        (0x0100, "Right Ring"),
        (0x0200, "Neck"),
        (0x0400, "Belt"),
        (0x0800, "Arrows"),
        (0x1000, "Bullets"),
        (0x2000, "Bolts"),
    ];

    let mut equipped_slots: std::collections::HashMap<u16, (String, i32)> =
        std::collections::HashMap::new();

    if let Some(GffValue::List(equip_list)) = root.get("Equip_ItemList") {
        for lazy_struct in equip_list {
            let item = lazy_struct.force_load().expect("load struct");

            let struct_id = match item.get("__struct_id__") {
                Some(GffValue::Dword(id)) => *id as u16,
                Some(GffValue::Word(id)) => *id,
                _ => 0,
            };

            if struct_id > 0 {
                let tag = match item.get("Tag") {
                    Some(GffValue::String(s)) => s.to_string(),
                    _ => "Unknown".to_string(),
                };
                let base_item = match item.get("BaseItem") {
                    Some(GffValue::Int(id)) => *id,
                    _ => 0,
                };
                equipped_slots.insert(struct_id, (tag, base_item));
            }
        }
    }

    println!("\n=== Equipment Slots ===");
    for (bitmask, slot_name) in slot_names {
        match equipped_slots.get(&bitmask) {
            Some((tag, base_item)) => {
                println!("  {slot_name:<12}: {tag} (BaseItem {base_item})");
            }
            None => {
                println!("  {slot_name:<12}: [empty]");
            }
        }
    }

    if let Some(GffValue::List(items)) = root.get("ItemList") {
        println!("\nInventory items: {}", items.len());
    }

    if let Some(GffValue::List(equip_list)) = root.get("Equip_ItemList")
        && !equip_list.is_empty()
        && equipped_slots.is_empty()
    {
        println!(
            "\nDebug: Equip_ItemList has {} entries but no items detected",
            equip_list.len()
        );

        let first = equip_list
            .first()
            .unwrap()
            .force_load()
            .expect("load struct");
        println!("First entry fields: {:?}", first.keys().collect::<Vec<_>>());
    }
}

#[test]
fn test_equipment_across_characters() {
    println!("\n=== Equipment Across All Characters ===");

    let characters = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("qaraofblacklake/qaraofblacklake4.bic", "Qara L30"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
    ];

    for (path, name) in characters {
        let parser = parse_character(path);
        let root = parser.read_struct_fields(0).expect("Failed to read root");

        let mut equipped_count = 0;

        if let Some(GffValue::List(equip_list)) = root.get("Equip_ItemList") {
            for lazy_struct in equip_list {
                let item = lazy_struct.force_load().expect("load struct");

                let struct_id = match item.get("__struct_id__") {
                    Some(GffValue::Dword(id)) => *id as u16,
                    Some(GffValue::Word(id)) => *id,
                    _ => 0,
                };

                if struct_id > 0 {
                    equipped_count += 1;
                }
            }
        }

        let inventory_count = match root.get("ItemList") {
            Some(GffValue::List(items)) => items.len(),
            _ => 0,
        };

        println!("  {name}: {equipped_count} equipped, {inventory_count} in inventory");
    }
}

#[test]
fn test_high_level_character() {
    let parser = parse_character("occidiooctavon/occidiooctavon4.bic");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    let lvlstat_list = match root.get("LvlStatList") {
        Some(GffValue::List(list)) => list,
        _ => return,
    };

    println!("\n=== High Level Character ===");
    println!("Total levels: {}", lvlstat_list.len());

    let class_list = match root.get("ClassList") {
        Some(GffValue::List(list)) => list,
        _ => return,
    };

    println!("Classes taken: {}", class_list.len());

    for lazy_struct in class_list {
        let class_entry = lazy_struct.force_load().expect("load struct");

        let class_id = match class_entry.get("Class") {
            Some(GffValue::Int(id)) => *id,
            _ => 0,
        };
        let class_level = match class_entry.get("ClassLevel") {
            Some(GffValue::Short(lvl)) => *lvl,
            _ => 0,
        };

        println!("  Class ID {class_id}: {class_level} levels");
    }
}

#[test]
fn test_spell_memorization() {
    let parser = parse_character("occidiooctavon/occidiooctavon1.bic");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    println!("\n=== Spell Data ===");

    if let Some(GffValue::List(class_list)) = root.get("ClassList") {
        for lazy_struct in class_list {
            let class_entry = lazy_struct.force_load().expect("load struct");

            let class_id = match class_entry.get("Class") {
                Some(GffValue::Int(id)) => *id,
                _ => continue,
            };

            if let Some(GffValue::List(known)) = class_entry.get("KnownList0") {
                println!("Class {} known cantrips: {}", class_id, known.len());
            }

            if let Some(GffValue::List(mem)) = class_entry.get("MemorizedList0") {
                println!("Class {} memorized level 0: {}", class_id, mem.len());
            }
        }
    }
}
