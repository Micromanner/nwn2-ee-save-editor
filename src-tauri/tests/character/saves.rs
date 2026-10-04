use super::super::common::{create_test_context, load_test_gff};
use app_lib::character::Character;
use app_lib::character::save_summary::SaveType;
use app_lib::parsers::gff::GffParser;

fn load_character(fixture_path: &str) -> Character {
    let bytes = load_test_gff(fixture_path);
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");
    Character::from_gff(root).expect("character from gff")
}

#[tokio::test]
async fn test_base_saves_calculation_sanity() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Base Saves Calculation Sanity ===");

    let fixtures = [
        "occidiooctavon/occidiooctavon4.bic",
        "qaraofblacklake/qaraofblacklake4.bic",
        "ryathstrongarm/ryathstrongarm4.bic",
        "sagemelchior/sagemelchior4.bic",
        "oneofmany/oneofmany4.bic",
    ];

    for path in fixtures {
        let character = load_character(path);

        // GFF-stored values are misc bonuses, not class-derived base saves.
        let misc_fort = character.base_fortitude();
        let misc_ref = character.base_reflex();
        let misc_will = character.base_will();

        // Values calculated from class levels
        let calculated = character.calculate_base_saves(game_data);

        println!(
            "{}: MiscBonuses[{}/{}/{}] vs CalcBase[{}/{}/{}]",
            character.first_name(),
            misc_fort,
            misc_ref,
            misc_will,
            calculated.fortitude,
            calculated.reflex,
            calculated.will
        );

        // We can't compare misc_bonus to base_save.
        // Instead, we verify that calculated base saves are reasonable for high level characters.
        // All these fixtures are level ~30 or at least mid-level.

        assert!(
            calculated.fortitude > 0,
            "Calculated Base Fortitude should be positive for {}",
            character.first_name()
        );
        assert!(
            calculated.reflex > 0,
            "Calculated Base Reflex should be positive for {}",
            character.first_name()
        );
        assert!(
            calculated.will > 0,
            "Calculated Base Will should be positive for {}",
            character.first_name()
        );

        // For L30 characters, base saves should be substantial (at least ~10 usually, mostly higher)
        if character.total_level() >= 20 {
            assert!(
                calculated.fortitude + calculated.reflex + calculated.will > 20,
                "Total base saves should be significant for high level char"
            );
        }
    }
}

#[tokio::test]
async fn test_total_saves_sanity() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");
    let decoder = &ctx.decoder;

    println!("\n=== Total Saves Sanity ===");

    let fixtures = [
        "occidiooctavon/occidiooctavon1.bic",
        "occidiooctavon/occidiooctavon4.bic",
    ];

    for path in fixtures {
        let character = load_character(path);
        let summary = character.get_save_summary(game_data, decoder);

        println!(
            "{}: Fort {}, Ref {}, Will {}",
            character.first_name(),
            summary.fortitude,
            summary.reflex,
            summary.will
        );

        // Basic sanity checks
        assert!(summary.fortitude > -10 && summary.fortitude < 100);
        assert!(summary.reflex > -10 && summary.reflex < 100);
        assert!(summary.will > -10 && summary.will < 100);

        // Check consistency with breakdown
        assert_eq!(summary.fortitude, summary.saves.fortitude.total);
        assert_eq!(summary.reflex, summary.saves.reflex.total);
        assert_eq!(summary.will, summary.saves.will.total);
    }
}

#[tokio::test]
async fn test_save_breakdown_correctness() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");
    let decoder = &ctx.decoder;

    println!("\n=== Save Breakdown Correctness ===");

    // Ryath might have good equipment/stats
    let character = load_character("ryathstrongarm/ryathstrongarm4.bic");
    let breakdown = character.get_saving_throws(game_data, decoder).fortitude;

    println!(
        "Ryath Fortitude: Total {} (Base {} + Abil {} + Equip {} + Feat {} + Class {} + Misc {})",
        breakdown.total,
        breakdown.base,
        breakdown.ability,
        breakdown.equipment,
        breakdown.feat,
        breakdown.class_bonus,
        breakdown.misc
    );

    let calculated_total = breakdown.base
        + breakdown.ability
        + breakdown.equipment
        + breakdown.feat
        + breakdown.class_bonus
        + breakdown.misc;

    assert_eq!(
        breakdown.total, calculated_total,
        "Breakdown total sum mismatch"
    );
    assert!(
        breakdown.base > 0,
        "L30 character should have base fortitude > 0"
    );
    // Ryath is a fighter/weapons master likely, should have high base fort
    assert!(breakdown.base >= 10);
}

#[tokio::test]
async fn test_level_progression_saves() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");
    let decoder = &ctx.decoder;

    println!("\n=== Level Progression Saves (L1 vs L30) ===");

    let fixture_pairs = [
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

    for (l1_path, l30_path, name) in fixture_pairs {
        let char_l1 = load_character(l1_path);
        let char_l30 = load_character(l30_path);

        let saves_l1 = char_l1.get_save_summary(game_data, decoder);
        let saves_l30 = char_l30.get_save_summary(game_data, decoder);

        println!(
            "{} L1 : Fort {}, Ref {}, Will {}",
            name, saves_l1.fortitude, saves_l1.reflex, saves_l1.will
        );
        println!(
            "{} L30: Fort {}, Ref {}, Will {}",
            name, saves_l30.fortitude, saves_l30.reflex, saves_l30.will
        );

        // Saves should generally increase significantly
        assert!(
            saves_l30.fortitude > saves_l1.fortitude,
            "{name} L30 Fortitude should be higher"
        );
        assert!(
            saves_l30.reflex > saves_l1.reflex,
            "{name} L30 Reflex should be higher"
        );
        assert!(
            saves_l30.will > saves_l1.will,
            "{name} L30 Will should be higher"
        );

        // Base saves specifically should definitely increase
        let base_l1 = char_l1.calculate_base_saves(game_data);
        let base_l30 = char_l30.calculate_base_saves(game_data);

        assert!(base_l30.fortitude > base_l1.fortitude);
        assert!(base_l30.reflex > base_l1.reflex);
        assert!(base_l30.will > base_l1.will);
    }
}

#[tokio::test]
async fn test_epic_save_progression() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    // Epic saves come straight from the 60-row class tables, with no separate epic bonus
    // (verified in-game 2026-09-26: Cleric 21 sheet = cls_savthr_cler row 21 + bonuses).
    let character = load_character("theconstruct/theconstruct4.bic");
    assert_eq!(character.total_level(), 30);
    let table = game_data
        .get_table("cls_savthr_fight")
        .expect("fighter save table");
    let row = |column| {
        table
            .get_cell(29, column)
            .ok()
            .flatten()
            .and_then(|v| v.parse::<i32>().ok())
            .expect("level 30 row")
    };

    let base_saves = character.calculate_base_saves(game_data);

    assert_eq!(base_saves.fortitude, row("FortSave"));
    assert_eq!(base_saves.reflex, row("RefSave"));
    assert_eq!(base_saves.will, row("WillSave"));
}

/// Divine Grace (feat 217) adds the positive CHA modifier of the total score
/// (stored + racial + items) to every save.
#[tokio::test]
async fn test_divine_grace_adds_total_cha_modifier() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");
    let decoder = &ctx.decoder;

    let occidio = load_character("occidiooctavon/occidiooctavon4.bic");
    assert!(occidio.has_feat(app_lib::character::FeatId(217)));
    let cha = occidio.get_total_abilities(game_data, decoder).cha;
    let expected = ((cha - 10).div_euclid(2)).max(0);
    let saves = occidio.get_saving_throws(game_data, decoder);
    assert_eq!(saves.fortitude.class_bonus, expected);
    assert_eq!(saves.reflex.class_bonus, expected);
    assert_eq!(saves.will.class_bonus, expected);
}
