use super::super::common::{create_test_context, fixtures_path, load_test_gff};
use app_lib::character::Character;
use app_lib::parsers::gff::GffParser;
use app_lib::services::savegame_handler::SaveGameHandler;

fn load_character(fixture_path: &str) -> Character {
    let bytes = load_test_gff(fixture_path);
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");
    Character::from_gff(root)
}

// ============================================================================
// Name Tests
// ============================================================================

#[tokio::test]
async fn test_character_names_across_fixtures() {
    println!("\n=== Character Names Across Fixtures ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon1.bic", "Occidio"),
        ("qaraofblacklake/qaraofblacklake1.bic", "Qara"),
        ("ryathstrongarm/ryathstrongarm1.bic", "Ryath"),
        ("sagemelchior/sagemelchior1.bic", "Melchior"),
        ("oneofmany/oneofmany1.bic", "OneOfMany"),
        ("okkugodofbears/okkugodofbears1.bic", "Okku"),
        ("theconstruct/theconstruct1.bic", "Construct"),
    ];

    for (path, expected_contains) in fixtures {
        let character = load_character(path);
        let first_name = character.first_name();
        let last_name = character.last_name();
        let full_name = character.full_name();

        println!(
            "{expected_contains}: first='{first_name}', last='{last_name}', full='{full_name}'"
        );

        assert!(
            !first_name.is_empty() || !last_name.is_empty(),
            "{expected_contains} should have at least a first or last name"
        );

        assert!(
            full_name.len() >= first_name.len(),
            "Full name should be at least as long as first name"
        );
    }
}

#[tokio::test]
async fn test_name_consistency_across_levels() {
    println!("\n=== Name Consistency Across Levels ===");

    let characters = [
        (
            "occidiooctavon/occidiooctavon1.bic",
            "occidiooctavon/occidiooctavon4.bic",
            "Occidio",
        ),
        (
            "qaraofblacklake/qaraofblacklake1.bic",
            "qaraofblacklake/qaraofblacklake4.bic",
            "Qara",
        ),
        (
            "sagemelchior/sagemelchior1.bic",
            "sagemelchior/sagemelchior4.bic",
            "Melchior",
        ),
    ];

    for (l1_path, l30_path, name) in characters {
        let char_l1 = load_character(l1_path);
        let char_l30 = load_character(l30_path);

        println!(
            "{}: L1='{}' vs L30='{}'",
            name,
            char_l1.full_name(),
            char_l30.full_name()
        );

        assert_eq!(
            char_l1.first_name(),
            char_l30.first_name(),
            "{name} first name should remain unchanged between L1 and L30"
        );

        assert_eq!(
            char_l1.last_name(),
            char_l30.last_name(),
            "{name} last name should remain unchanged between L1 and L30"
        );
    }
}

#[tokio::test]
async fn test_set_names_on_real_character() {
    println!("\n=== Set Names on Real Character ===");

    let mut character = load_character("ryathstrongarm/ryathstrongarm1.bic");
    let original_first = character.first_name();
    let original_last = character.last_name();

    println!("Original: '{original_first}' '{original_last}'");

    character.set_first_name("TestFirst".to_string());
    character.set_last_name("TestLast".to_string());

    assert_eq!(character.first_name(), "TestFirst");
    assert_eq!(character.last_name(), "TestLast");
    assert_eq!(character.full_name(), "TestFirst TestLast");
    assert!(character.is_modified());

    println!(
        "Modified: '{}' '{}' (is_modified={})",
        character.first_name(),
        character.last_name(),
        character.is_modified()
    );
}

// ============================================================================
// Classic Campaign Tests
// ============================================================================

#[tokio::test]
async fn test_classic_campaign_identity() {
    println!("\n=== Classic Campaign Identity ===");

    let save_path = fixtures_path().join("saves/Classic_Campaign");
    println!("Loading save from: {save_path:?}");

    let handler =
        SaveGameHandler::new(&save_path, false).expect("Failed to create SaveGameHandler");

    let player_data = handler
        .extract_player_bic()
        .expect("Failed to extract player.bic")
        .expect("player.bic not found in save");

    let parser = GffParser::from_bytes(player_data).expect("Failed to parse player.bic GFF");

    let root = parser
        .read_struct_fields(0)
        .expect("Failed to read root struct");

    let character = Character::from_gff(root);

    println!(
        "Classic Campaign Character: '{}' (Class count: {})",
        character.full_name(),
        character.class_count()
    );

    // Verify some Identity properties
    assert!(
        !character.first_name().is_empty(),
        "First name should not be empty"
    );
    // Michael has no last name in this save
    // assert!(!character.last_name().is_empty(), "Last name should not be empty");
    assert!(
        !character.full_name().is_empty(),
        "Full name should not be empty"
    );
    assert!(character.age() >= 0, "Age should be non-negative");
    assert!(character.experience() >= 0, "XP should be non-negative");

    let alignment = character.alignment();
    println!("Alignment: {}", alignment.alignment_string());
    println!("Deity: '{}'", character.deity());
    println!("Description: '{}'", character.description());

    assert_eq!(
        character.deity(),
        "Shaundakul",
        "Deity should be Shaundakul"
    );
    assert!(
        character
            .description()
            .contains("Not much is known about your history"),
        "Description should match default text"
    );

    assert!(alignment.law_chaos >= 0 && alignment.law_chaos <= 100);
    assert!(alignment.good_evil >= 0 && alignment.good_evil <= 100);
}

// ============================================================================
// Age Tests
// ============================================================================

#[tokio::test]
async fn test_age_across_fixtures() {
    println!("\n=== Age Across Fixtures ===");

    let fixtures = [
        "occidiooctavon/occidiooctavon1.bic",
        "qaraofblacklake/qaraofblacklake1.bic",
        "ryathstrongarm/ryathstrongarm1.bic",
        "sagemelchior/sagemelchior1.bic",
        "oneofmany/oneofmany1.bic",
    ];

    for path in fixtures {
        let character = load_character(path);
        let age = character.age();

        println!("{}: Age = {}", character.first_name(), age);

        assert!(
            age >= 0,
            "{} should have non-negative age",
            character.first_name()
        );
        assert!(
            age <= 10000,
            "{} should have reasonable age",
            character.first_name()
        );
    }
}

#[tokio::test]
async fn test_set_age_validation() {
    println!("\n=== Set Age Validation ===");

    let mut character = load_character("ryathstrongarm/ryathstrongarm1.bic");
    let original_age = character.age();

    println!("Original age: {original_age}");

    let valid_result = character.set_age(50);
    assert!(valid_result.is_ok());
    assert_eq!(character.age(), 50);
    assert!(character.is_modified());

    let invalid_result = character.set_age(-5);
    assert!(invalid_result.is_err());
    assert_eq!(
        character.age(),
        50,
        "Age should remain unchanged after error"
    );

    println!("Final age after tests: {}", character.age());
}

// ============================================================================
// Experience Tests
// ============================================================================

#[tokio::test]
async fn test_experience_progression() {
    println!("\n=== Experience Progression ===");

    let characters = [
        (
            "occidiooctavon/occidiooctavon1.bic",
            "occidiooctavon/occidiooctavon4.bic",
            "Occidio",
        ),
        (
            "qaraofblacklake/qaraofblacklake1.bic",
            "qaraofblacklake/qaraofblacklake4.bic",
            "Qara",
        ),
        (
            "ryathstrongarm/ryathstrongarm1.bic",
            "ryathstrongarm/ryathstrongarm4.bic",
            "Ryath",
        ),
    ];

    for (l1_path, l30_path, name) in characters {
        let char_l1 = load_character(l1_path);
        let char_l30 = load_character(l30_path);

        let xp_l1 = char_l1.experience();
        let xp_l30 = char_l30.experience();

        println!(
            "{}: L1 XP = {}, L30 XP = {} (gained: {})",
            name,
            xp_l1,
            xp_l30,
            xp_l30 - xp_l1
        );

        assert!(xp_l1 >= 0, "{name} L1 XP should be non-negative");
        assert!(xp_l30 >= xp_l1, "{name} L30 XP should be >= L1 XP");

        if char_l30.total_level() > char_l1.total_level() {
            assert!(
                xp_l30 > xp_l1,
                "{name} L30 XP should be greater than L1 XP for higher level character"
            );
        }
    }
}

#[tokio::test]
async fn test_experience_matches_level() {
    println!("\n=== Experience Matches Level ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon1.bic", 1),
        ("occidiooctavon/occidiooctavon4.bic", 30),
    ];

    for (path, expected_min_level) in fixtures {
        let character = load_character(path);
        let xp = character.experience();
        let level = character.total_level();

        println!("{}: Level {} with {} XP", character.first_name(), level, xp);

        assert!(
            level >= expected_min_level,
            "Level should match expectation"
        );

        if level == 1 {
            assert!(xp < 1000, "Level 1 should have < 1000 XP");
        } else if level >= 20 {
            assert!(
                xp >= 190000,
                "Level 20+ should have significant XP (>=190k)"
            );
        }
    }
}

#[tokio::test]
async fn test_set_experience_validation() {
    println!("\n=== Set Experience Validation ===");

    let mut character = load_character("ryathstrongarm/ryathstrongarm1.bic");
    let original_xp = character.experience();

    println!("Original XP: {original_xp}");

    let valid_result = character.set_experience(50000);
    assert!(valid_result.is_ok());
    assert_eq!(character.experience(), 50000);
    assert!(character.is_modified());

    let invalid_result = character.set_experience(-100);
    assert!(invalid_result.is_err());
    assert_eq!(
        character.experience(),
        50000,
        "XP should remain unchanged after error"
    );

    println!("Final XP: {}", character.experience());
}

// ============================================================================
// Alignment Tests
// ============================================================================

#[tokio::test]
async fn test_alignment_across_fixtures() {
    println!("\n=== Alignment Across Fixtures ===");

    let fixtures = [
        "occidiooctavon/occidiooctavon1.bic",
        "qaraofblacklake/qaraofblacklake1.bic",
        "ryathstrongarm/ryathstrongarm1.bic",
        "sagemelchior/sagemelchior1.bic",
        "oneofmany/oneofmany1.bic",
        "okkugodofbears/okkugodofbears1.bic",
        "theconstruct/theconstruct1.bic",
    ];

    for path in fixtures {
        let character = load_character(path);
        let alignment = character.alignment();

        println!(
            "{}: LC={}, GE={} -> '{}' (lawful={}, chaotic={}, good={}, evil={})",
            character.first_name(),
            alignment.law_chaos,
            alignment.good_evil,
            alignment.alignment_string(),
            alignment.is_lawful(),
            alignment.is_chaotic(),
            alignment.is_good(),
            alignment.is_evil()
        );

        assert!(
            alignment.law_chaos >= 0 && alignment.law_chaos <= 100,
            "{} law_chaos should be 0-100",
            character.first_name()
        );
        assert!(
            alignment.good_evil >= 0 && alignment.good_evil <= 100,
            "{} good_evil should be 0-100",
            character.first_name()
        );

        assert!(
            !alignment.alignment_string().is_empty(),
            "Alignment string should not be empty"
        );

        if alignment.is_lawful() {
            assert!(!alignment.is_chaotic(), "Cannot be both lawful and chaotic");
        }
        if alignment.is_good() {
            assert!(!alignment.is_evil(), "Cannot be both good and evil");
        }
    }
}

#[tokio::test]
async fn test_alignment_string_coverage() {
    println!("\n=== Alignment String Coverage ===");

    let fixtures = [
        "occidiooctavon/occidiooctavon1.bic",
        "qaraofblacklake/qaraofblacklake1.bic",
        "ryathstrongarm/ryathstrongarm1.bic",
        "sagemelchior/sagemelchior1.bic",
        "oneofmany/oneofmany1.bic",
        "okkugodofbears/okkugodofbears1.bic",
        "theconstruct/theconstruct1.bic",
    ];

    let valid_alignments = [
        "Lawful Good",
        "Lawful Neutral",
        "Lawful Evil",
        "Neutral Good",
        "True Neutral",
        "Neutral Evil",
        "Chaotic Good",
        "Chaotic Neutral",
        "Chaotic Evil",
    ];

    for path in fixtures {
        let character = load_character(path);
        let alignment_str = character.alignment().alignment_string();

        assert!(
            valid_alignments.contains(&alignment_str.as_str()),
            "{} has invalid alignment string: '{}'",
            character.first_name(),
            alignment_str
        );
    }
}

#[tokio::test]
async fn test_set_alignment_on_real_character() {
    println!("\n=== Set Alignment on Real Character ===");

    let mut character = load_character("ryathstrongarm/ryathstrongarm1.bic");
    let original = character.alignment();

    println!(
        "Original: LC={}, GE={} -> '{}'",
        original.law_chaos,
        original.good_evil,
        original.alignment_string()
    );

    let result = character.set_alignment(Some(0), Some(100));
    assert!(result.is_ok());

    let updated = character.alignment();
    assert_eq!(updated.law_chaos, 0);
    assert_eq!(updated.good_evil, 100);
    assert_eq!(updated.alignment_string(), "Chaotic Good");
    assert!(character.is_modified());

    println!(
        "Updated: LC={}, GE={} -> '{}'",
        updated.law_chaos,
        updated.good_evil,
        updated.alignment_string()
    );
}

#[tokio::test]
async fn test_alignment_clamping() {
    println!("\n=== Alignment Clamping ===");

    let mut character = load_character("ryathstrongarm/ryathstrongarm1.bic");

    let result = character.set_alignment(Some(150), Some(-50));
    assert!(result.is_ok());

    let alignment = character.alignment();
    assert_eq!(alignment.law_chaos, 100, "Law/chaos should clamp to 100");
    assert_eq!(alignment.good_evil, 0, "Good/evil should clamp to 0");

    println!(
        "Clamped to: LC={}, GE={} -> '{}'",
        alignment.law_chaos,
        alignment.good_evil,
        alignment.alignment_string()
    );
}

#[tokio::test]
async fn test_alignment_partial_update() {
    println!("\n=== Alignment Partial Update ===");

    let mut character = load_character("ryathstrongarm/ryathstrongarm1.bic");
    let original = character.alignment();

    let result = character.set_alignment(Some(25), None);
    assert!(result.is_ok());

    let updated = character.alignment();
    assert_eq!(updated.law_chaos, 25);
    assert_eq!(
        updated.good_evil, original.good_evil,
        "Good/evil should remain unchanged"
    );

    println!(
        "Partial update: LC {} -> {}, GE unchanged at {}",
        original.law_chaos, updated.law_chaos, updated.good_evil
    );
}

// ============================================================================
// Deity Tests
// ============================================================================

#[tokio::test]
async fn test_deity_across_fixtures() {
    println!("\n=== Deity Across Fixtures ===");

    let fixtures = [
        "occidiooctavon/occidiooctavon1.bic",
        "qaraofblacklake/qaraofblacklake1.bic",
        "ryathstrongarm/ryathstrongarm1.bic",
        "sagemelchior/sagemelchior1.bic",
    ];

    for path in fixtures {
        let character = load_character(path);
        let deity = character.deity();

        println!("{}: Deity = '{}'", character.first_name(), deity);
    }
}

#[tokio::test]
async fn test_set_deity_on_real_character() {
    println!("\n=== Set Deity on Real Character ===");

    let mut character = load_character("ryathstrongarm/ryathstrongarm1.bic");
    let original_deity = character.deity();

    println!("Original deity: '{original_deity}'");

    character.set_deity("Tempus".to_string());
    assert_eq!(character.deity(), "Tempus");
    assert!(character.is_modified());

    character.set_deity(String::new());
    assert_eq!(character.deity(), "");

    println!("Final deity: '{}'", character.deity());
}

// ============================================================================
// Gender Tests
// ============================================================================

#[tokio::test]
async fn test_gender_across_fixtures() {
    println!("\n=== Gender Across Fixtures ===");

    let fixtures = [
        "occidiooctavon/occidiooctavon1.bic",
        "qaraofblacklake/qaraofblacklake1.bic",
        "ryathstrongarm/ryathstrongarm1.bic",
        "sagemelchior/sagemelchior1.bic",
        "oneofmany/oneofmany1.bic",
    ];

    for path in fixtures {
        let character = load_character(path);
        let gender = character.gender();
        let gender_str = if gender == 0 { "Male" } else { "Female" };

        println!(
            "{}: Gender = {} ({})",
            character.first_name(),
            gender,
            gender_str
        );

        assert!(
            gender == 0 || gender == 1,
            "{} should have valid gender (0 or 1)",
            character.first_name()
        );
    }
}

#[tokio::test]
async fn test_set_gender_validation() {
    println!("\n=== Set Gender Validation ===");

    let mut character = load_character("ryathstrongarm/ryathstrongarm1.bic");
    let original_gender = character.gender();

    println!("Original gender: {original_gender}");

    let new_gender = i32::from(original_gender == 0);
    let valid_result = character.set_gender(new_gender);
    assert!(valid_result.is_ok());
    assert_eq!(character.gender(), new_gender);
    assert!(character.is_modified());

    let invalid_result = character.set_gender(2);
    assert!(invalid_result.is_err());
    assert_eq!(
        character.gender(),
        new_gender,
        "Gender should remain unchanged after error"
    );

    let invalid_result = character.set_gender(-1);
    assert!(invalid_result.is_err());

    println!("Final gender: {}", character.gender());
}

// ============================================================================
// Description Tests
// ============================================================================

#[tokio::test]
async fn test_description_across_fixtures() {
    println!("\n=== Description Across Fixtures ===");

    let fixtures = [
        "occidiooctavon/occidiooctavon1.bic",
        "qaraofblacklake/qaraofblacklake1.bic",
        "ryathstrongarm/ryathstrongarm1.bic",
        "sagemelchior/sagemelchior1.bic",
    ];

    for path in fixtures {
        let character = load_character(path);
        let description = character.description();
        let preview = if description.len() > 50 {
            format!("{}...", &description[..50])
        } else {
            description.clone()
        };

        println!(
            "{}: Description ({} chars) = '{}'",
            character.first_name(),
            description.len(),
            preview
        );
    }
}

#[tokio::test]
async fn test_set_description_on_real_character() {
    println!("\n=== Set Description on Real Character ===");

    let mut character = load_character("ryathstrongarm/ryathstrongarm1.bic");
    let original_desc = character.description();

    println!("Original description length: {}", original_desc.len());

    let new_desc = "A mighty warrior from the frozen wastes of the North.".to_string();
    character.set_description(new_desc.clone());
    assert_eq!(character.description(), new_desc);
    assert!(character.is_modified());

    character.set_description(String::new());
    assert_eq!(character.description(), "");

    println!("Description can be set and cleared");
}

// ============================================================================
// Biography Tests
// ============================================================================

#[tokio::test]
async fn test_biography_aggregation() {
    println!("\n=== Biography Aggregation ===");

    let ctx = create_test_context().await;
    let loader = ctx.loader;
    assert!(loader.is_ready());
    let game_data = loader.game_data().expect("Game data should be loaded");

    let fixtures = [
        "occidiooctavon/occidiooctavon1.bic",
        "qaraofblacklake/qaraofblacklake1.bic",
        "ryathstrongarm/ryathstrongarm1.bic",
    ];

    for path in fixtures {
        let character = load_character(path);
        let bio = character.biography(game_data);

        println!(
            "{}: name='{}', age={}, gender={}, xp={}, alignment='{}'",
            bio.first_name,
            bio.name,
            bio.age,
            bio.gender,
            bio.experience,
            bio.alignment.alignment_string()
        );

        assert_eq!(bio.name, character.full_name());
        assert_eq!(bio.first_name, character.first_name());
        assert_eq!(bio.last_name, character.last_name());
        assert_eq!(bio.age, character.age());
        // With full game data loaded, we get resolved TLK names
        if character.first_name() == "Ryath" {
            assert_eq!(bio.background, Some("Militia".to_string()));
        } else if character.first_name() == "Occidio" {
            assert_eq!(bio.background, Some("Natural Leader".to_string()));
        }
        assert_eq!(bio.gender, character.gender());
        assert_eq!(bio.experience, character.experience());
        assert_eq!(bio.deity, character.deity());
        assert_eq!(bio.description, character.description());
        assert_eq!(bio.alignment.law_chaos, character.alignment().law_chaos);
        assert_eq!(bio.alignment.good_evil, character.alignment().good_evil);
    }
}

#[tokio::test]
async fn test_biography_consistency_between_levels() {
    println!("\n=== Biography Consistency (L1 vs L30) ===");

    let ctx = create_test_context().await;
    let loader = ctx.loader;
    assert!(loader.is_ready());
    let game_data = loader.game_data().expect("Game data should be loaded");

    let characters = [
        (
            "occidiooctavon/occidiooctavon1.bic",
            "occidiooctavon/occidiooctavon4.bic",
            "Occidio Octavon",
        ),
        (
            "ryathstrongarm/ryathstrongarm1.bic",
            "ryathstrongarm/ryathstrongarm4.bic",
            "Ryath Strongarm",
        ),
    ];

    for (l1_path, l30_path, name) in characters {
        let char_l1 = load_character(l1_path);
        let char_l30 = load_character(l30_path);

        let bio_l1 = char_l1.biography(game_data);
        let bio_l30 = char_l30.biography(game_data);

        println!("{name} L1 vs L30:");
        println!("  Name: '{}' vs '{}'", bio_l1.name, bio_l30.name);
        println!("  Gender: {} vs {}", bio_l1.gender, bio_l30.gender);
        println!("  XP: {} vs {}", bio_l1.experience, bio_l30.experience);
        println!(
            "  Alignment: '{}' vs '{}'",
            bio_l1.alignment.alignment_string(),
            bio_l30.alignment.alignment_string()
        );

        assert_eq!(
            bio_l1.name, bio_l30.name,
            "{name} name should be consistent"
        );
        assert_eq!(
            bio_l1.gender, bio_l30.gender,
            "{name} gender should be consistent"
        );
    }
}

// ============================================================================
// Comprehensive Identity Comparison
// ============================================================================

#[tokio::test]
async fn test_all_fixtures_identity_sanity() {
    println!("\n=== All Fixtures Identity Sanity ===");

    let all_fixtures = [
        ("occidiooctavon/occidiooctavon1.bic", "Occidio L1"),
        ("occidiooctavon/occidiooctavon2.bic", "Occidio L10"),
        ("occidiooctavon/occidiooctavon3.bic", "Occidio L20"),
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("qaraofblacklake/qaraofblacklake1.bic", "Qara L1"),
        ("qaraofblacklake/qaraofblacklake4.bic", "Qara L30"),
        ("ryathstrongarm/ryathstrongarm1.bic", "Ryath L1"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
        ("sagemelchior/sagemelchior1.bic", "Melchior L1"),
        ("sagemelchior/sagemelchior4.bic", "Melchior L30"),
        ("oneofmany/oneofmany1.bic", "OneOfMany L1"),
        ("oneofmany/oneofmany4.bic", "OneOfMany L30"),
        ("okkugodofbears/okkugodofbears1.bic", "Okku L1"),
        ("okkugodofbears/okkugodofbears4.bic", "Okku L30"),
        ("theconstruct/theconstruct1.bic", "Construct L1"),
        ("theconstruct/theconstruct4.bic", "Construct L30"),
    ];

    for (path, label) in all_fixtures {
        let character = load_character(path);

        assert!(
            !character.full_name().is_empty()
                || character.first_name().is_empty() && character.last_name().is_empty(),
            "{label} should have a name or explicitly empty"
        );

        assert!(character.age() >= 0, "{label} should have non-negative age");
        assert!(
            character.experience() >= 0,
            "{label} should have non-negative XP"
        );
        assert!(
            character.gender() == 0 || character.gender() == 1,
            "{label} should have valid gender"
        );

        let alignment = character.alignment();
        assert!(
            alignment.law_chaos >= 0 && alignment.law_chaos <= 100,
            "{label} should have valid law_chaos"
        );
        assert!(
            alignment.good_evil >= 0 && alignment.good_evil <= 100,
            "{label} should have valid good_evil"
        );

        println!(
            "{}: '{}' age={} xp={} gender={} align='{}'",
            label,
            character.full_name(),
            character.age(),
            character.experience(),
            character.gender(),
            alignment.alignment_string()
        );
    }
}

#[tokio::test]
async fn test_xp_level_correlation() {
    println!("\n=== XP/Level Correlation ===");

    let progression = [
        ("occidiooctavon/occidiooctavon1.bic", 1),
        ("occidiooctavon/occidiooctavon2.bic", 2),
        ("occidiooctavon/occidiooctavon3.bic", 3),
        ("occidiooctavon/occidiooctavon4.bic", 4),
    ];

    let mut prev_xp = 0;
    let mut prev_level = 0;

    for (path, fixture_num) in progression {
        let character = load_character(path);
        let xp = character.experience();
        let level = character.total_level();

        println!(
            "Fixture {}: Level {} with {} XP (delta: +{} xp, +{} levels)",
            fixture_num,
            level,
            xp,
            xp - prev_xp,
            level - prev_level
        );

        if fixture_num > 1 {
            assert!(xp >= prev_xp, "XP should not decrease between fixtures");
            assert!(
                level >= prev_level,
                "Level should not decrease between fixtures"
            );
        }

        prev_xp = xp;
        prev_level = level;
    }
}

// ============================================================================
// Modification Tests
// ============================================================================

#[tokio::test]
async fn test_is_modified_tracking() {
    println!("\n=== Is Modified Tracking ===");

    let mut character = load_character("ryathstrongarm/ryathstrongarm1.bic");

    assert!(
        !character.is_modified(),
        "Fresh character should not be modified"
    );

    character.set_first_name("ModifiedName".to_string());
    assert!(
        character.is_modified(),
        "Should be modified after name change"
    );

    println!("Modified tracking works correctly");
}

#[tokio::test]
async fn test_multiple_modifications() {
    println!("\n=== Multiple Modifications ===");

    let mut character = load_character("ryathstrongarm/ryathstrongarm1.bic");

    character.set_first_name("NewFirst".to_string());
    character.set_last_name("NewLast".to_string());
    character.set_age(100).unwrap();
    character.set_experience(999999).unwrap();
    character.set_alignment(Some(0), Some(0)).unwrap();
    character.set_deity("Shar".to_string());
    character.set_description("Custom description".to_string());
    character.set_gender(1).unwrap();

    assert_eq!(character.first_name(), "NewFirst");
    assert_eq!(character.last_name(), "NewLast");
    assert_eq!(character.full_name(), "NewFirst NewLast");
    assert_eq!(character.age(), 100);
    assert_eq!(character.experience(), 999999);
    assert_eq!(character.alignment().law_chaos, 0);
    assert_eq!(character.alignment().good_evil, 0);
    assert_eq!(character.alignment().alignment_string(), "Chaotic Evil");
    assert_eq!(character.deity(), "Shar");
    assert_eq!(character.description(), "Custom description");
    assert_eq!(character.gender(), 1);
    assert!(character.is_modified());

    println!("All identity fields successfully modified");
}

#[tokio::test]
async fn test_background_retrieval() {
    use app_lib::character::BackgroundId;

    let ctx = create_test_context().await;
    let loader = ctx.loader;
    assert!(loader.is_ready());
    let game_data = loader.game_data().expect("Game data should be loaded");
    let bg_table = loader
        .get_table("backgrounds")
        .expect("backgrounds.2da should be loaded");

    // Pick a selectable row and confirm background() resolves to its Name/Label.
    let target_row = (1..bg_table.row_count())
        .find(|&r| bg_table.get_cell(r, "REMOVED").ok().flatten().as_deref() != Some("1"))
        .expect("No selectable background in 2DA");

    let mut character = load_character("occidiooctavon/occidiooctavon1.bic");
    character
        .add_background(BackgroundId(target_row as i32), game_data)
        .expect("add_background should succeed");

    let resolved = character
        .background(game_data)
        .expect("background() must resolve once CharBackground is set");
    assert!(
        !resolved.is_empty(),
        "Resolved background name must be non-empty"
    );
    println!("Row {target_row}: background() = {resolved:?}");

    character
        .remove_background(game_data)
        .expect("remove_background should succeed");
    assert_eq!(
        character.background(game_data),
        None,
        "background() must be None after remove"
    );
}
