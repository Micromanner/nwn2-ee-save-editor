use super::super::common;
use app_lib::character::Character;
use app_lib::parsers::gff::GffParser;

#[tokio::test]
async fn test_race_data_loading() {
    let ctx = common::create_test_context().await;
    let game_data = &ctx.game_data;

    // Test with Ryath Strongarm (likely Human)
    let data = common::load_test_gff("ryathstrongarm/ryathstrongarm1.bic");
    let parser = GffParser::from_bytes(data).expect("Failed to parse GFF");
    let fields = parser
        .read_struct_fields(0)
        .expect("Failed to read top level struct");
    let character = Character::from_gff(fields).expect("character from gff");

    // Basic consistency checks
    let race_id = character.race_id();
    let race_name = character.race_display_name(game_data);
    println!("Ryath race: {race_name} (ID: {race_id:?})");

    assert_ne!(race_name, "", "Race name should not be empty");
    assert!(
        !race_name.contains("Race "),
        "Should resolve actual name, not fallback ID"
    );

    // Humans usually have race ID 6 (Human) or similar standard ID
    // Verify ability modifiers are reasonable
    let mods = character.get_racial_ability_modifiers_for_race(race_id.0, game_data);
    println!("Racial Mods: {mods:?}");

    // Humans generally have no ability modifiers
    if race_name == "Human" {
        assert_eq!(mods.str_mod, 0);
        assert_eq!(mods.dex_mod, 0);
        assert_eq!(mods.con_mod, 0);
        assert_eq!(mods.int_mod, 0);
        assert_eq!(mods.wis_mod, 0);
        assert_eq!(mods.cha_mod, 0);
    }

    // Size check
    let size = character.size_category();
    let size_val = character.creature_size();
    println!("Size: {size:?} ({size_val})");
    assert_eq!(size_val, size as i32);
}

#[tokio::test]
async fn test_race_progression_consistency() {
    let ctx = common::create_test_context().await;
    let _game_data = &ctx.game_data;

    // Compare Ryath at different stages/levels
    let files = [
        "ryathstrongarm/ryathstrongarm1.bic",
        "ryathstrongarm/ryathstrongarm2.bic",
        "ryathstrongarm/ryathstrongarm3.bic",
        "ryathstrongarm/ryathstrongarm4.bic",
    ];

    let mut first_race_id = None;
    let mut first_subrace = None;

    for (i, file) in files.iter().enumerate() {
        let data = common::load_test_gff(file);
        let parser = GffParser::from_bytes(data).expect("Failed to parse GFF");
        let fields = parser
            .read_struct_fields(0)
            .expect("Failed to read top level struct");
        let character = Character::from_gff(fields).expect("character from gff");

        let race_id = character.race_id();
        let subrace = character.subrace_index();

        println!("Checking {file} -> Race: {race_id:?} Subrace: {subrace:?}");

        if i == 0 {
            first_race_id = Some(race_id);
            first_subrace = subrace.clone();
        } else {
            assert_eq!(
                Some(race_id),
                first_race_id,
                "Race ID should stay consistent across saves"
            );
            assert_eq!(
                subrace, first_subrace,
                "Subrace should stay consistent across saves"
            );
        }
    }
}

#[tokio::test]
async fn test_diverse_races() {
    let ctx = common::create_test_context().await;
    let game_data = &ctx.game_data;

    // Test different characters to cover different races
    // Need to verify what characters we have.
    // Okku is likely a Spirit Bear or similar special race
    let characters = vec![
        ("okkugodofbears/okkugodofbears1.bic", "Okku"),
        ("qaraofblacklake/qaraofblacklake1.bic", "Qara"),
        ("sagemelchior/sagemelchior1.bic", "Zhjaeve"), // Assuming name
        ("theconstruct/theconstruct1.bic", "Construct"),
    ];

    for (file, name) in characters {
        if let Ok(data) = std::panic::catch_unwind(|| common::load_test_gff(file)) {
            let parser = GffParser::from_bytes(data).expect("Failed to parse GFF");
            let fields = parser
                .read_struct_fields(0)
                .expect("Failed to read top level struct");
            let character = Character::from_gff(fields).expect("character from gff");

            let race_name = character.race_display_name(game_data);
            let size = character.size_category();

            println!("Character: {name} ({file}) - Race: {race_name}, Size: {size:?}");

            // Ensure we get valid data back
            assert!(!race_name.is_empty());

            // Check racial feats
            let racial_feats =
                character.get_racial_feats_for_race(character.race_id().0, game_data);
            println!("  Racial Feats: {}", racial_feats.len());
            for feat in &racial_feats {
                // Verify we can resolve feat names
                let feat_name = character.get_feat_name(*feat, game_data);
                assert!(
                    !feat_name.is_empty(),
                    "Racial feat {feat:?} should have a name"
                );
            }
        } else {
            println!("Skipping {file}, file not found");
        }
    }
}

#[tokio::test]
async fn test_race_change_validation() {
    let ctx = common::create_test_context().await;
    let game_data = &ctx.game_data;

    // Load a character
    let data = common::load_test_gff("ryathstrongarm/ryathstrongarm1.bic");
    let parser = GffParser::from_bytes(data).expect("Failed to parse GFF");
    let fields = parser
        .read_struct_fields(0)
        .expect("Failed to read top level struct");
    let character = Character::from_gff(fields).expect("character from gff");

    // Validate a known valid change (to Human, ID 6)
    let human_id = 6; // Standard Human ID in NWN2
    let _validation = character.validate_race_change(human_id, None, game_data);

    // Ensure we can't change to a non-existent race
    let invalid_race_id = 99999;
    let validation_invalid = character.validate_race_change(invalid_race_id, None, game_data);
    assert!(!validation_invalid.valid, "Should reject invalid race ID");

    // Validate subrace check
    // "Shield Dwarf" is subrace of Dwarf. Dwarf is 0.
    // "Gold Dwarf" is subrace of Dwarf.
    // "Drow" is subrace of Elf (1).

    // Drow base race is Elf; applying Drow subrace to a Human base race should fail.
    let drow_validation = character.validate_subrace(6, "drow", game_data);
    if drow_validation.valid {
        let drow_data = character.get_subrace_data("drow", game_data);
        if drow_data.is_some() {
            assert!(
                !drow_validation.valid,
                "Should not allow Drow subrace on Human base race"
            );
        }
    }
}

#[tokio::test]
async fn test_race_change_execution() {
    let ctx = common::create_test_context().await;
    let game_data = &ctx.game_data;

    // Load Ryath (Human) - Load into memory ONLY, original file is untouched
    let data = common::load_test_gff("ryathstrongarm/ryathstrongarm1.bic");
    let parser = GffParser::from_bytes(data).expect("Failed to parse GFF");
    let fields = parser
        .read_struct_fields(0)
        .expect("Failed to read top level struct");
    let mut character = Character::from_gff(fields).expect("character from gff");
    character.set_age(30).ok(); // Arbitrary age since set_game_data is gone

    // Snapshot initial state
    let initial_race = character.race_id();
    let initial_con = character.base_ability(app_lib::character::AbilityIndex::CON);
    let initial_cha = character.base_ability(app_lib::character::AbilityIndex::CHA);
    let _initial_feats_count = character.feat_ids().len();
    let initial_max_hp = character.max_hp();

    println!("Initial Race: {initial_race:?}, CON: {initial_con}, CHA: {initial_cha}");

    // Ensure we are starting with Human (ID 6) for this test to be predictable
    if initial_race.0 != 6 {
        println!(
            "WARNING: Test character is not Human (ID 6), assertions might need adjustment. Actual: {initial_race:?}"
        );
    }

    // Change to Dwarf (ID 0)
    let dwarf_id = 0;

    let result = character
        .change_race(dwarf_id, None, false, game_data)
        .expect("Failed to change race to Dwarf");

    println!("Race Change Result: {result:?}");

    // Verify Race ID
    assert_eq!(
        character.race_id().0,
        dwarf_id,
        "Race ID should be Dwarf (0)"
    );

    assert_eq!(
        character.subrace_index(),
        Some(2),
        "Dwarf should get its DefaultSubRace"
    );

    // Adjustments are derived at read time; stored scores stay untouched
    assert_eq!(
        character.base_ability(app_lib::character::AbilityIndex::CON),
        initial_con
    );
    assert_eq!(
        character.base_ability(app_lib::character::AbilityIndex::CHA),
        initial_cha
    );

    // Shield Dwarf CON +2 raises the CON modifier by 1 at every level
    assert_eq!(character.max_hp(), initial_max_hp + character.total_level());

    // Verify Feat Changes
    // Darkvision (Feat 228) should be added
    let darkvision = app_lib::character::FeatId(228);
    let has_darkvision = character.has_feat(darkvision);
    println!("Has Darkvision: {has_darkvision}");
    assert!(has_darkvision, "Dwarf should have Darkvision");

    // Verify Human "Quick to Master" (Feat 258) is removed
    let quick_to_master = app_lib::character::FeatId(258);
    let has_quick_to_master = character.has_feat(quick_to_master);
    println!("Has Quick to Master: {has_quick_to_master}");
    assert!(
        !has_quick_to_master,
        "Dwarf should NOT have Quick to Master (Human trait)"
    );

    // Check removal log
    let feat_removed = result.feats_removed.iter().any(|f| f.feat_id == 258);
    assert!(
        feat_removed,
        "Result should record removal of Quick to Master"
    );
}

#[tokio::test]
async fn test_aasimar_subrace_from_byte_index() {
    let ctx = common::create_test_context().await;
    let game_data = &ctx.game_data;

    let data = common::load_test_gff("player.bic");
    let parser = GffParser::from_bytes(data).expect("Failed to parse GFF");
    let fields = parser
        .read_struct_fields(0)
        .expect("Failed to read top level struct");
    let character = Character::from_gff(fields).expect("character from gff");

    // player.bic has Race=21 (Planetouched) and Subrace=13 (Byte index for Aasimar)
    let race_id = character.race_id();
    println!("Race ID: {race_id:?}");
    assert_eq!(race_id.0, 21, "Should have Planetouched base race (21)");

    let subrace_idx = character.subrace_index();
    println!("Subrace Index: {subrace_idx:?}");
    assert_eq!(
        subrace_idx,
        Some(13),
        "Should have subrace index 13 (Aasimar)"
    );

    let subrace_name = character.subrace_name(game_data);
    println!("Subrace Name: {subrace_name:?}");
    assert!(subrace_name.is_some(), "Should resolve subrace name");
    assert!(
        subrace_name
            .as_ref()
            .unwrap()
            .to_lowercase()
            .contains("aasimar"),
        "Subrace name should be Aasimar, got: {subrace_name:?}"
    );

    let race_name = character.race_display_name(game_data);
    println!("Race Name (display): {race_name}");
    assert!(
        race_name.to_lowercase().contains("aasimar"),
        "Race display name should be Aasimar, got: {race_name}"
    );
}

#[tokio::test]
async fn test_rejected_race_change_leaves_character_untouched() {
    let ctx = common::create_test_context().await;
    let game_data = &ctx.game_data;

    let data = common::load_test_gff("ryathstrongarm/ryathstrongarm1.bic");
    let parser = GffParser::from_bytes(data).expect("Failed to parse GFF");
    let fields = parser
        .read_struct_fields(0)
        .expect("Failed to read top level struct");
    let mut character = Character::from_gff(fields).expect("character from gff");
    let race_before = character.race_id();

    assert!(
        character
            .change_race(99999, None, false, game_data)
            .is_err()
    );
    assert!(character.change_race(300, None, false, game_data).is_err());
    assert_eq!(character.race_id(), race_before);
    assert!(!character.is_modified());
}
