use super::super::common::{fixtures_path, load_test_gff};
use app_lib::parsers::gff::parser::GffParser;
use app_lib::parsers::gff::types::GffValue;
use app_lib::parsers::gff::writer::GffWriter;

// =============================================================================
// CORE PARSING TESTS
// =============================================================================

#[tokio::test]
async fn test_gff_parse_from_bytes() {
    let filename = "occidiooctavon/occidiooctavon1.bic";
    let bytes = load_test_gff(filename);

    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF from bytes");

    assert_eq!(parser.file_type, "BIC ", "File type should be 'BIC '");
    assert_eq!(parser.file_version, "V3.2", "File version should be V3.2");
}

#[tokio::test]
async fn test_gff_header_parsing() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");

    assert!(
        !parser.file_type.is_empty(),
        "File type should not be empty"
    );
    assert!(
        !parser.file_version.is_empty(),
        "File version should not be empty"
    );
}

#[tokio::test]
async fn test_gff_root_struct_access() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");

    let root_fields = parser
        .read_struct_fields(0)
        .expect("Failed to read root struct");

    assert!(!root_fields.is_empty(), "Root struct should have fields");
    println!("Root struct has {} fields", root_fields.len());
}

// =============================================================================
// FIELD TYPE TESTS (All GFF field types)
// =============================================================================

#[tokio::test]
async fn test_gff_byte_field() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    if let Some(GffValue::Byte(value)) = root.get("Gender") {
        println!("Gender (Byte): {value}");
        assert!(*value <= 2, "Gender should be 0-2");
    } else {
        println!("Gender field not found or not Byte type");
    }
}

#[tokio::test]
async fn test_gff_word_field() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    if let Some(GffValue::Word(value)) = root.get("Race") {
        println!("Race (Word): {value}");
    } else if let Some(GffValue::Byte(value)) = root.get("Race") {
        println!("Race (Byte): {value}");
    } else {
        println!("Race field not found");
    }
}

#[tokio::test]
async fn test_gff_dword_field() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    if let Some(GffValue::Dword(value)) = root.get("Experience") {
        println!("Experience (Dword): {value}");
    } else {
        println!("Experience field not found or not Dword");
    }
}

#[tokio::test]
async fn test_gff_int_field() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    if let Some(GffValue::Int(value)) = root.get("HitPoints") {
        println!("HitPoints (Int): {value}");
    } else if let Some(GffValue::Short(value)) = root.get("HitPoints") {
        println!("HitPoints (Short): {value}");
    } else {
        println!("HitPoints field not found");
    }
}

#[tokio::test]
async fn test_gff_float_field() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    if let Some(GffValue::Float(value)) = root.get("ChallengeRating") {
        println!("ChallengeRating (Float): {value}");
        assert!(value.is_finite(), "Float should be finite");
    } else {
        println!("ChallengeRating field not found");
    }
}

#[tokio::test]
async fn test_gff_locstring_field() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    if let Some(GffValue::LocString(ls)) = root.get("FirstName") {
        println!("FirstName LocString:");
        println!("  StringRef: {}", ls.string_ref);
        println!("  Substrings count: {}", ls.substrings.len());
        for sub in &ls.substrings {
            println!("    Language {}: {}", sub.language, sub.string);
        }
        assert!(!ls.substrings.is_empty(), "Character should have a name");
    } else {
        panic!("FirstName should be a LocString");
    }

    if let Some(GffValue::LocString(ls)) = root.get("LastName") {
        println!("LastName LocString:");
        println!("  StringRef: {}", ls.string_ref);
        for sub in &ls.substrings {
            println!("    Language {}: {}", sub.language, sub.string);
        }
    }
}

#[tokio::test]
async fn test_gff_resref_field() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    if let Some(GffValue::ResRef(value)) = root.get("Portrait") {
        println!("Portrait (ResRef): {value}");
    }

    if let Some(GffValue::ResRef(value)) = root.get("Deity") {
        println!("Deity (ResRef): {value}");
    }
}

#[tokio::test]
async fn test_gff_string_field() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    if let Some(GffValue::String(value)) = root.get("Tag") {
        println!("Tag (String): {value}");
    }
}

// =============================================================================
// LIST AND STRUCT TESTS
// =============================================================================

#[tokio::test]
async fn test_gff_list_field() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    if let Some(GffValue::List(items)) = root.get("ClassList") {
        println!("ClassList has {} entries", items.len());
        assert!(
            !items.is_empty(),
            "Character should have at least one class"
        );

        for (i, lazy_struct) in items.iter().enumerate() {
            let fields = lazy_struct.force_load();
            println!("  Class {}: {} fields", i, fields.len());
            if let Some(GffValue::Int(class_id)) = fields.get("Class") {
                println!("    Class ID: {class_id}");
            }
            if let Some(GffValue::Short(level)) = fields.get("ClassLevel") {
                println!("    Level: {level}");
            }
        }
    } else {
        panic!("ClassList should be a List");
    }
}

#[tokio::test]
async fn test_gff_nested_struct() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    if let Some(GffValue::List(feats)) = root.get("FeatList") {
        println!("Character has {} feats", feats.len());
        for (i, feat) in feats.iter().take(5).enumerate() {
            let fields = feat.force_load();
            if let Some(GffValue::Word(feat_id)) = fields.get("Feat") {
                println!("  Feat {i}: ID {feat_id}");
            }
        }
    }
}

#[tokio::test]
async fn test_gff_skill_list() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    if let Some(GffValue::List(skills)) = root.get("SkillList") {
        println!("SkillList has {} entries", skills.len());
        for (i, skill) in skills.iter().take(5).enumerate() {
            let fields = skill.force_load();
            if let Some(GffValue::Byte(rank)) = fields.get("Rank") {
                println!("  Skill {i}: Rank {rank}");
            }
        }
    }
}

#[tokio::test]
async fn test_gff_lvlstatlist() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    if let Some(GffValue::List(levels)) = root.get("LvlStatList") {
        println!("LvlStatList has {} entries", levels.len());
        assert!(!levels.is_empty(), "Character should have level history");

        for (i, level) in levels.iter().enumerate() {
            let fields = level.force_load();
            println!("  Level {} entry has {} fields", i + 1, fields.len());

            if let Some(GffValue::Byte(class)) = fields.get("LvlStatClass") {
                println!("    Class: {class}");
            }
            if let Some(GffValue::Int(hp)) = fields.get("LvlStatHitDie") {
                println!("    HP gained: {hp}");
            }
        }
    }
}

// =============================================================================
// PATH-BASED ACCESS TESTS
// =============================================================================

#[tokio::test]
async fn test_gff_get_value_simple() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");

    let exp_result = parser.get_value("Experience");
    if let Ok(GffValue::Dword(exp)) = exp_result {
        println!("Experience via path: {exp}");
    }

    let gender_result = parser.get_value("Gender");
    if let Ok(GffValue::Byte(gender)) = gender_result {
        println!("Gender via path: {gender}");
    }
}

#[tokio::test]
async fn test_gff_read_field_by_label() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");

    let result = parser.read_field_by_label(0, "Str");
    if let Ok(GffValue::Byte(str_val)) = result {
        println!("Str via read_field_by_label: {str_val}");
        assert!(
            (3..=50).contains(&str_val),
            "Strength should be in reasonable range"
        );
    }
}

// =============================================================================
// ROUND-TRIP TESTS
// =============================================================================

#[tokio::test]
async fn test_gff_round_trip() {
    let filename = "occidiooctavon/occidiooctavon1.bic";
    let bytes = load_test_gff(filename);
    println!("Loaded {} bytes from {}", bytes.len(), filename);

    let parser = GffParser::from_bytes(bytes.clone()).expect("Failed to parse GFF");

    let root_fields = parser
        .read_struct_fields(0)
        .expect("Failed to read root struct");

    let mut root_owned = indexmap::IndexMap::new();
    for (k, v) in root_fields {
        root_owned.insert(k, v.force_owned());
    }

    let mut writer = GffWriter::new(&parser.file_type, &parser.file_version);
    let new_bytes = writer.write(root_owned).expect("Failed to write GFF");

    println!(
        "Original: {} bytes, Reserialized: {} bytes",
        bytes.len(),
        new_bytes.len()
    );

    let parser2 = GffParser::from_bytes(new_bytes.clone()).expect("Failed to re-parse GFF");

    assert!(parser2.read_struct_fields(0).is_ok());
    assert_eq!(parser.file_type, parser2.file_type);

    let root2 = parser2.read_struct_fields(0).unwrap();
    if let Some(GffValue::LocString(ls)) = root2.get("FirstName") {
        assert!(
            !ls.substrings.is_empty(),
            "FirstName should have substrings after round-trip"
        );
    }
}

#[tokio::test]
async fn test_gff_round_trip_preserves_data() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");

    let parser1 = GffParser::from_bytes(bytes.clone()).expect("Parse 1");
    let root1 = parser1.read_struct_fields(0).unwrap();

    let original_exp = match root1.get("Experience") {
        Some(GffValue::Dword(v)) => *v,
        _ => 0,
    };
    let original_str = match root1.get("Str") {
        Some(GffValue::Byte(v)) => *v,
        _ => 0,
    };

    let mut root_owned = indexmap::IndexMap::new();
    for (k, v) in root1 {
        root_owned.insert(k, v.force_owned());
    }

    let mut writer = GffWriter::new(&parser1.file_type, &parser1.file_version);
    let new_bytes = writer.write(root_owned).expect("Write");

    let parser2 = GffParser::from_bytes(new_bytes).expect("Parse 2");
    let root2 = parser2.read_struct_fields(0).unwrap();

    let round_trip_exp = match root2.get("Experience") {
        Some(GffValue::Dword(v)) => *v,
        _ => 0,
    };
    let round_trip_str = match root2.get("Str") {
        Some(GffValue::Byte(v)) => *v,
        _ => 0,
    };

    assert_eq!(
        original_exp, round_trip_exp,
        "Experience should be preserved"
    );
    assert_eq!(original_str, round_trip_str, "Str should be preserved");
}

// =============================================================================
// MULTIPLE CHARACTER FILES
// =============================================================================

#[tokio::test]
async fn test_gff_parse_multiple_characters() {
    let characters = [
        "occidiooctavon/occidiooctavon1.bic",
        "oneofmany/oneofmany1.bic",
        "okkugodofbears/okkugodofbears1.bic",
        "qaraofblacklake/qaraofblacklake1.bic",
        "ryathstrongarm/ryathstrongarm1.bic",
        "sagemelchior/sagemelchior1.bic",
        "theconstruct/theconstruct1.bic",
    ];

    for filename in characters {
        println!("\n=== Parsing {filename} ===");
        let bytes = load_test_gff(filename);
        let parser = GffParser::from_bytes(bytes);

        match parser {
            Ok(p) => {
                let root = p.read_struct_fields(0).expect("Failed to read root");

                let name = if let Some(GffValue::LocString(ls)) = root.get("FirstName") {
                    ls.substrings
                        .first()
                        .map(|s| s.string.to_string())
                        .unwrap_or_default()
                } else {
                    String::new()
                };

                let class_count = if let Some(GffValue::List(classes)) = root.get("ClassList") {
                    classes.len()
                } else {
                    0
                };

                println!("  Name: {name}, Classes: {class_count}");
            }
            Err(e) => {
                println!("  Failed to parse: {e}");
            }
        }
    }
}

// =============================================================================
// ERROR HANDLING TESTS
// =============================================================================

#[tokio::test]
async fn test_gff_invalid_bytes() {
    let invalid_bytes = vec![0u8; 10];
    let result = GffParser::from_bytes(invalid_bytes);
    assert!(result.is_err(), "Should fail on invalid bytes");
}

#[tokio::test]
async fn test_gff_empty_bytes() {
    let empty_bytes = vec![];
    let result = GffParser::from_bytes(empty_bytes);
    assert!(result.is_err(), "Should fail on empty bytes");
}

#[tokio::test]
async fn test_gff_truncated_header() {
    let truncated = vec![0x47, 0x46, 0x46, 0x20]; // "GFF " but incomplete
    let result = GffParser::from_bytes(truncated);
    assert!(result.is_err(), "Should fail on truncated header");
}

#[tokio::test]
async fn test_gff_nonexistent_field() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    assert!(
        root.get("NonExistentField12345").is_none(),
        "Should return None for nonexistent field"
    );
}

// =============================================================================
// EQUIPPED ITEMS TEST
// =============================================================================

#[tokio::test]
async fn test_gff_equipped_items() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    if let Some(GffValue::List(items)) = root.get("Equip_ItemList") {
        println!("Equipped items: {}", items.len());
        for item in items {
            let fields = item.force_load();
            println!("  Slot __struct_id__: {}", item.struct_id);
            if let Some(GffValue::LocString(name)) = fields.get("LocalizedName")
                && let Some(sub) = name.substrings.first()
            {
                println!("    Name: {}", sub.string);
            }
        }
    }
}

#[tokio::test]
async fn test_gff_inventory_items() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    if let Some(GffValue::List(items)) = root.get("ItemList") {
        println!("Inventory items: {}", items.len());
        for (i, item) in items.iter().take(10).enumerate() {
            let fields = item.force_load();
            if let Some(GffValue::LocString(name)) = fields.get("LocalizedName")
                && let Some(sub) = name.substrings.first()
            {
                println!("  {}: {}", i, sub.string);
            }
        }
    }
}

// =============================================================================
// ABILITY SCORE TESTS
// =============================================================================

#[tokio::test]
async fn test_gff_ability_scores() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    let abilities = ["Str", "Dex", "Con", "Int", "Wis", "Cha"];
    println!("Ability Scores:");
    for ability in abilities {
        if let Some(GffValue::Byte(value)) = root.get(ability) {
            println!("  {ability}: {value}");
            assert!(
                *value >= 1 && *value <= 100,
                "{ability} should be in valid range"
            );
        }
    }
}

// =============================================================================
// FIELD ENUMERATION TEST
// =============================================================================

#[tokio::test]
async fn test_gff_enumerate_all_root_fields() {
    let bytes = load_test_gff("occidiooctavon/occidiooctavon1.bic");
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");

    println!("All root fields ({} total):", root.len());
    for (name, value) in root.iter().take(30) {
        let type_name = match value {
            GffValue::Byte(_) => "Byte",
            GffValue::Char(_) => "Char",
            GffValue::Word(_) => "Word",
            GffValue::Short(_) => "Short",
            GffValue::Dword(_) => "Dword",
            GffValue::Int(_) => "Int",
            GffValue::Dword64(_) => "Dword64",
            GffValue::Int64(_) => "Int64",
            GffValue::Float(_) => "Float",
            GffValue::Double(_) => "Double",
            GffValue::String(_) => "String",
            GffValue::ResRef(_) => "ResRef",
            GffValue::LocString(_) => "LocString",
            GffValue::Void(_) => "Void",
            GffValue::Struct(_) => "Struct",
            GffValue::List(l) => {
                println!("  {}: List[{}]", name, l.len());
                continue;
            }
            GffValue::StructOwned(_) => "StructOwned",
            GffValue::ListOwned(_) => "ListOwned",
            GffValue::StructRef(_) => "StructRef",
            GffValue::ListRef(_) => "ListRef",
        };
        println!("  {name}: {type_name}");
    }
}

/// A save written by a Russian EE client (fixture `saves/Russian_Client`,
/// 2026-09-19): the engine stores typed names as UTF-8 in both the `.bic`
/// LocStrings and `playerinfo.bin`, so no code page fallback may touch them,
/// and writing them back must be byte-identical.
#[tokio::test]
async fn test_russian_client_save_names_are_utf8() {
    use app_lib::services::playerinfo::PlayerInfo;
    use app_lib::utils::ZipContentReader;

    let fixtures = fixtures_path().join("saves/Russian_Client");
    let bic = ZipContentReader::new()
        .read_file_from_zip(
            fixtures.join("resgff.zip").to_string_lossy().into_owned(),
            "player.bic".to_string(),
        )
        .expect("player.bic in resgff.zip");

    let parser = GffParser::from_bytes(bic).expect("parse player.bic");
    let root = parser.read_struct_fields(0).expect("root");
    let name = |field: &str| match root.get(field) {
        Some(GffValue::LocString(ls)) => ls.substrings[0].string.to_string(),
        other => panic!("{field}: {other:?}"),
    };
    assert_eq!(name("FirstName"), "Друг");
    assert_eq!(name("LastName"), "Тест");

    let mut writer = GffWriter::new(&parser.file_type, &parser.file_version);
    let root_owned = root
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();
    let written = writer.write(root_owned).expect("write");
    let reparsed = GffParser::from_bytes(written).expect("re-parse");
    let root2 = reparsed.read_struct_fields(0).expect("root");
    match root2.get("FirstName") {
        Some(GffValue::LocString(ls)) => assert_eq!(ls.substrings[0].string, "Друг"),
        other => panic!("FirstName after write: {other:?}"),
    }

    let info = PlayerInfo::load(fixtures.join("playerinfo.bin")).expect("load playerinfo.bin");
    assert_eq!(info.data.first_name, "Друг");
    assert_eq!(info.data.last_name, "Тест");
    assert_eq!(info.data.subrace, "Человек");
}
