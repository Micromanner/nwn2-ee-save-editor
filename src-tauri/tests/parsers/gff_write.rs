use super::super::common::load_test_gff;
use app_lib::parsers::gff::parser::GffParser;
use app_lib::parsers::gff::types::{GffValue, LocalizedString, LocalizedSubstring};
use app_lib::parsers::gff::writer::GffWriter;
use std::borrow::Cow;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_temp_bic(fixture_name: &str) -> (TempDir, PathBuf, Vec<u8>) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let bytes = load_test_gff(fixture_name);
    let temp_path = temp_dir.path().join("test_character.bic");
    std::fs::write(&temp_path, &bytes).expect("Failed to write temp file");
    (temp_dir, temp_path, bytes)
}

// =============================================================================
// FILE-BASED WRITE TESTS
// Copy fixture → modify → save to disk → reload from disk → verify → cleanup
// =============================================================================

#[tokio::test]
async fn test_file_modify_experience_and_save() {
    let (_temp_dir, temp_path, bytes) = create_temp_bic("occidiooctavon/occidiooctavon1.bic");

    let parser = GffParser::from_bytes(bytes).expect("Parse");
    let root = parser.read_struct_fields(0).expect("Read root");

    let original_exp = match root.get("Experience") {
        Some(GffValue::Dword(v)) => *v,
        _ => panic!("Experience should be Dword"),
    };
    println!("Original experience: {original_exp}");

    let mut root_owned = indexmap::IndexMap::new();
    for (k, v) in root {
        if k == "Experience" {
            root_owned.insert(k, GffValue::Dword(original_exp + 50000));
        } else {
            root_owned.insert(k, v.force_owned());
        }
    }

    let mut writer = GffWriter::new(&parser.file_type, &parser.file_version);
    let new_bytes = writer.write(root_owned).expect("Write");
    std::fs::write(&temp_path, &new_bytes).expect("Save to disk");

    let saved_bytes = std::fs::read(&temp_path).expect("Read from disk");
    let parser2 = GffParser::from_bytes(saved_bytes).expect("Re-parse");
    let root2 = parser2.read_struct_fields(0).expect("Read root 2");

    let new_exp = match root2.get("Experience") {
        Some(GffValue::Dword(v)) => *v,
        _ => panic!("Experience should still be Dword"),
    };

    println!("Saved experience: {new_exp}");
    assert_eq!(
        new_exp,
        original_exp + 50000,
        "Experience should be increased by 50000"
    );
}

#[tokio::test]
async fn test_file_modify_all_abilities_and_save() {
    let (_temp_dir, temp_path, bytes) = create_temp_bic("occidiooctavon/occidiooctavon1.bic");

    let parser = GffParser::from_bytes(bytes).expect("Parse");
    let root = parser.read_struct_fields(0).expect("Read root");

    let ability_targets = [
        ("Str", 25u8),
        ("Dex", 22u8),
        ("Con", 20u8),
        ("Int", 18u8),
        ("Wis", 16u8),
        ("Cha", 14u8),
    ];

    let mut root_owned = indexmap::IndexMap::new();
    for (k, v) in root {
        let new_value = ability_targets
            .iter()
            .find(|(name, _)| *name == k.as_str())
            .map(|(_, val)| GffValue::Byte(*val));

        root_owned.insert(k, new_value.unwrap_or_else(|| v.force_owned()));
    }

    let mut writer = GffWriter::new(&parser.file_type, &parser.file_version);
    let new_bytes = writer.write(root_owned).expect("Write");
    std::fs::write(&temp_path, &new_bytes).expect("Save to disk");

    let saved_bytes = std::fs::read(&temp_path).expect("Read from disk");
    let parser2 = GffParser::from_bytes(saved_bytes).expect("Re-parse");
    let root2 = parser2.read_struct_fields(0).expect("Read root 2");

    for (name, expected) in ability_targets {
        let actual = match root2.get(name) {
            Some(GffValue::Byte(v)) => *v,
            _ => panic!("{name} should be Byte"),
        };
        assert_eq!(actual, expected, "{name} should be {expected}");
        println!("{name}: {actual}");
    }
}

#[tokio::test]
async fn test_file_modify_name_and_save() {
    let (_temp_dir, temp_path, bytes) = create_temp_bic("occidiooctavon/occidiooctavon1.bic");

    let parser = GffParser::from_bytes(bytes).expect("Parse");
    let root = parser.read_struct_fields(0).expect("Read root");

    let new_name = "ModifiedTestCharacter";

    let mut root_owned = indexmap::IndexMap::new();
    for (k, v) in root {
        if k == "FirstName" {
            let new_locstring = LocalizedString {
                string_ref: -1,
                substrings: vec![LocalizedSubstring {
                    string: Cow::Owned(new_name.to_string()),
                    language: 0,
                    gender: 0,
                }],
            };
            root_owned.insert(k, GffValue::LocString(new_locstring));
        } else {
            root_owned.insert(k, v.force_owned());
        }
    }

    let mut writer = GffWriter::new(&parser.file_type, &parser.file_version);
    let new_bytes = writer.write(root_owned).expect("Write");
    std::fs::write(&temp_path, &new_bytes).expect("Save to disk");

    let saved_bytes = std::fs::read(&temp_path).expect("Read from disk");
    let parser2 = GffParser::from_bytes(saved_bytes).expect("Re-parse");
    let root2 = parser2.read_struct_fields(0).expect("Read root 2");

    let saved_name = match root2.get("FirstName") {
        Some(GffValue::LocString(ls)) => ls
            .substrings
            .first()
            .map(|s| s.string.to_string())
            .unwrap_or_default(),
        _ => panic!("FirstName should still be LocString"),
    };

    println!("Saved name: {saved_name}");
    assert_eq!(saved_name, new_name, "Name should be updated");
}

#[tokio::test]
async fn test_file_save_preserves_lists() {
    let (_temp_dir, temp_path, bytes) = create_temp_bic("occidiooctavon/occidiooctavon1.bic");

    let parser = GffParser::from_bytes(bytes).expect("Parse");
    let root = parser.read_struct_fields(0).expect("Read root");

    let original_class_count = match root.get("ClassList") {
        Some(GffValue::List(list)) => list.len(),
        _ => 0,
    };
    let original_feat_count = match root.get("FeatList") {
        Some(GffValue::List(list)) => list.len(),
        _ => 0,
    };
    let original_skill_count = match root.get("SkillList") {
        Some(GffValue::List(list)) => list.len(),
        _ => 0,
    };

    println!(
        "Original: {original_class_count} classes, {original_feat_count} feats, {original_skill_count} skills"
    );

    let mut root_owned = indexmap::IndexMap::new();
    for (k, v) in root {
        if k == "Gold" {
            root_owned.insert(k, GffValue::Dword(999999));
        } else {
            root_owned.insert(k, v.force_owned());
        }
    }

    let mut writer = GffWriter::new(&parser.file_type, &parser.file_version);
    let new_bytes = writer.write(root_owned).expect("Write");
    std::fs::write(&temp_path, &new_bytes).expect("Save to disk");

    let saved_bytes = std::fs::read(&temp_path).expect("Read from disk");
    let parser2 = GffParser::from_bytes(saved_bytes).expect("Re-parse");
    let root2 = parser2.read_struct_fields(0).expect("Read root 2");

    let saved_class_count = match root2.get("ClassList") {
        Some(GffValue::List(list)) => list.len(),
        _ => 0,
    };
    let saved_feat_count = match root2.get("FeatList") {
        Some(GffValue::List(list)) => list.len(),
        _ => 0,
    };
    let saved_skill_count = match root2.get("SkillList") {
        Some(GffValue::List(list)) => list.len(),
        _ => 0,
    };

    println!(
        "After save: {saved_class_count} classes, {saved_feat_count} feats, {saved_skill_count} skills"
    );

    assert_eq!(
        original_class_count, saved_class_count,
        "Class count preserved"
    );
    assert_eq!(
        original_feat_count, saved_feat_count,
        "Feat count preserved"
    );
    assert_eq!(
        original_skill_count, saved_skill_count,
        "Skill count preserved"
    );
}

#[tokio::test]
async fn test_file_save_all_fixtures() {
    let fixtures = [
        "occidiooctavon/occidiooctavon1.bic",
        "oneofmany/oneofmany1.bic",
        "okkugodofbears/okkugodofbears1.bic",
        "qaraofblacklake/qaraofblacklake1.bic",
        "ryathstrongarm/ryathstrongarm1.bic",
        "sagemelchior/sagemelchior1.bic",
        "theconstruct/theconstruct1.bic",
    ];

    for fixture in fixtures {
        println!("\n=== Testing file save for {fixture} ===");

        let (_temp_dir, temp_path, bytes) = create_temp_bic(fixture);

        let parser = GffParser::from_bytes(bytes).expect("Parse");
        let root = parser.read_struct_fields(0).expect("Read root");
        let original_field_count = root.len();

        let mut root_owned = indexmap::IndexMap::new();
        for (k, v) in root {
            root_owned.insert(k, v.force_owned());
        }

        let mut writer = GffWriter::new(&parser.file_type, &parser.file_version);
        let new_bytes = writer.write(root_owned).expect("Write failed");
        std::fs::write(&temp_path, &new_bytes).expect("Save failed");

        let saved_bytes = std::fs::read(&temp_path).expect("Read failed");
        let parser2 = GffParser::from_bytes(saved_bytes).expect("Re-parse failed");
        let root2 = parser2.read_struct_fields(0).expect("Read root 2 failed");

        assert_eq!(
            original_field_count,
            root2.len(),
            "Field count mismatch for {fixture}"
        );

        println!("  {} fields preserved after file save", root2.len());
    }
}

#[tokio::test]
async fn test_file_multiple_saves() {
    let (_temp_dir, temp_path, initial_bytes) =
        create_temp_bic("occidiooctavon/occidiooctavon1.bic");

    let mut current_bytes = initial_bytes;

    for i in 1..=5 {
        println!("Save iteration {i}");

        let parser = GffParser::from_bytes(current_bytes).expect("Parse");
        let root = parser.read_struct_fields(0).expect("Read root");

        let mut root_owned = indexmap::IndexMap::new();
        for (k, v) in root {
            if k == "Experience" {
                if let GffValue::Dword(exp) = v {
                    root_owned.insert(k, GffValue::Dword(exp + 1000));
                } else {
                    root_owned.insert(k, v.force_owned());
                }
            } else {
                root_owned.insert(k, v.force_owned());
            }
        }

        let mut writer = GffWriter::new(&parser.file_type, &parser.file_version);
        let new_bytes = writer.write(root_owned).expect("Write");
        std::fs::write(&temp_path, &new_bytes).expect("Save to disk");

        current_bytes = std::fs::read(&temp_path).expect("Read back");
    }

    let final_parser = GffParser::from_bytes(current_bytes).expect("Final parse");
    let final_root = final_parser.read_struct_fields(0).expect("Final read");

    if let Some(GffValue::Dword(final_exp)) = final_root.get("Experience") {
        println!("Final experience after 5 saves: {final_exp}");
    }

    assert!(
        !final_root.is_empty(),
        "File should still be valid after 5 saves"
    );
}

/// Round-trip non-ASCII content through parse → write → re-parse and verify
/// byte-for-byte stability. Guards against UTF-8/Windows-1252 confusion in the
/// writer: GFF stores Windows-1252, so writing raw UTF-8 bytes would cause
/// every save to double the byte count of non-ASCII localized strings.
#[tokio::test]
async fn test_roundtrip_non_ascii_locstring_stable() {
    let (_temp_dir, temp_path, bytes) = create_temp_bic("occidiooctavon/occidiooctavon1.bic");

    let samples: &[(u32, &str)] = &[
        (0, "English: Adaur lives in West Harbor."),
        (2, "German: Es macht mir Ärger - übel, schön, größer, weiß."),
        (4, "Spanish: ¡Qué día! Niño piñata año corazón."),
        (1, "French: Peut-être être élève à côté du café."),
        (3, "Italian: perché città università pietà più."),
        (5, "Polish with transliteration: opowiesci zlych drow."),
        (6, "Symbols: © ® ± ¼ ½ ¾ « » ¡ ¿"),
    ];

    let parser = GffParser::from_bytes(bytes).expect("Parse");
    let root = parser.read_struct_fields(0).expect("Read root");

    let mut root_owned: indexmap::IndexMap<String, GffValue<'static>> = indexmap::IndexMap::new();
    for (k, v) in root {
        if k == "Description" {
            let substrings = samples
                .iter()
                .map(|(lang, text)| LocalizedSubstring {
                    string: Cow::Owned((*text).to_string()),
                    language: *lang,
                    gender: 0,
                })
                .collect();
            root_owned.insert(
                k,
                GffValue::LocString(LocalizedString {
                    string_ref: -1,
                    substrings,
                }),
            );
        } else {
            root_owned.insert(k, v.force_owned());
        }
    }

    let mut current_bytes: Vec<u8> = {
        let mut writer = GffWriter::new(&parser.file_type, &parser.file_version);
        writer.write(root_owned).expect("Write #1")
    };

    let mut byte_lens_by_pass: Vec<Vec<usize>> = Vec::new();

    for pass in 1..=3 {
        std::fs::write(&temp_path, &current_bytes).expect("Save to disk");
        let reread = std::fs::read(&temp_path).expect("Re-read");

        let p = GffParser::from_bytes(reread).expect("Re-parse");
        let r = p.read_struct_fields(0).expect("Read root");

        let desc = r.get("Description").expect("Description present");
        let ls = match desc {
            GffValue::LocString(ls) => ls.clone(),
            _ => panic!("Description should be LocString"),
        };

        let lens: Vec<usize> = ls.substrings.iter().map(|s| s.string.len()).collect();
        println!("Pass {pass}: substring lens = {lens:?}");

        for (i, (_, original)) in samples.iter().enumerate() {
            let actual = &ls.substrings[i].string;
            assert_eq!(
                actual.as_ref(),
                *original,
                "Pass {pass} substring[{i}] corrupted: expected {original:?}, got {actual:?}"
            );
        }

        byte_lens_by_pass.push(lens);

        let mut next_root: indexmap::IndexMap<String, GffValue<'static>> =
            indexmap::IndexMap::new();
        for (k, v) in r {
            next_root.insert(k, v.force_owned());
        }
        let mut writer = GffWriter::new(&p.file_type, &p.file_version);
        current_bytes = writer.write(next_root).expect("Write");
    }

    let first = &byte_lens_by_pass[0];
    for (i, lens) in byte_lens_by_pass.iter().enumerate() {
        assert_eq!(
            lens,
            first,
            "Pass {} substring lengths diverged from pass 1 - encoding is not round-trip stable",
            i + 1
        );
    }
}

#[tokio::test]
async fn test_file_verify_bytes_on_disk() {
    let (_temp_dir, temp_path, bytes) = create_temp_bic("occidiooctavon/occidiooctavon1.bic");

    let parser = GffParser::from_bytes(bytes).expect("Parse");
    let root = parser.read_struct_fields(0).expect("Read root");

    let mut root_owned = indexmap::IndexMap::new();
    for (k, v) in root {
        root_owned.insert(k, v.force_owned());
    }

    let mut writer = GffWriter::new(&parser.file_type, &parser.file_version);
    let written_bytes = writer.write(root_owned).expect("Write");
    std::fs::write(&temp_path, &written_bytes).expect("Save to disk");

    let disk_bytes = std::fs::read(&temp_path).expect("Read from disk");

    assert_eq!(written_bytes.len(), disk_bytes.len(), "Byte count matches");
    assert_eq!(
        written_bytes, disk_bytes,
        "Bytes on disk match written bytes"
    );

    println!(
        "Verified {} bytes written correctly to disk",
        disk_bytes.len()
    );
}
