use super::super::common::{create_test_context, load_test_gff};
use app_lib::character::Character;
use app_lib::parsers::gff::GffParser;

fn load_character(fixture_path: &str) -> Character {
    let bytes = load_test_gff(fixture_path);
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");
    Character::from_gff(root).expect("character from gff")
}

#[tokio::test]
async fn test_bab_across_fixtures() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== BAB Across Fixtures ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon1.bic", "Occidio L1"),
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("qaraofblacklake/qaraofblacklake1.bic", "Qara L1"),
        ("qaraofblacklake/qaraofblacklake4.bic", "Qara L30"),
        ("ryathstrongarm/ryathstrongarm1.bic", "Ryath L1"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
        ("okkugodofbears/okkugodofbears1.bic", "Okku L1"),
        ("okkugodofbears/okkugodofbears4.bic", "Okku L30"),
        ("theconstruct/theconstruct4.bic", "Fighter L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let bab = character.calculate_bab(game_data);
        let level = character.total_level();

        println!("{name:<15}: Level {level:>2}, BAB {bab:>2}");

        assert_eq!(
            bab,
            character.base_attack_bonus(),
            "{name}: calculated BAB must match the game-written BaseAttackBonus"
        );
    }
}

#[tokio::test]
async fn test_bab_progression_l1_vs_l30() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== BAB Progression L1 vs L30 ===");

    let progressions = [
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
        (
            "okkugodofbears/okkugodofbears1.bic",
            "okkugodofbears/okkugodofbears4.bic",
            "Okku",
        ),
    ];

    for (l1_path, l30_path, name) in progressions {
        let char_l1 = load_character(l1_path);
        let char_l30 = load_character(l30_path);

        let bab_l1 = char_l1.calculate_bab(game_data);
        let bab_l30 = char_l30.calculate_bab(game_data);

        println!(
            "{:<10}: L1 BAB = {:>2}, L30 BAB = {:>2}, Δ = {:>2}",
            name,
            bab_l1,
            bab_l30,
            bab_l30 - bab_l1
        );

        assert!(
            bab_l30 > bab_l1,
            "{name} L30 BAB ({bab_l30}) should exceed L1 BAB ({bab_l1})"
        );
    }
}

#[tokio::test]
async fn test_attack_sequence_generation() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Attack Sequence Generation ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
        ("okkugodofbears/okkugodofbears4.bic", "Okku L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let sequence = character.get_attack_sequence(game_data);
        let bab = character.calculate_bab(game_data);

        println!(
            "{:<15}: BAB={:>2}, Attacks={}, Sequence={:?}",
            name,
            bab,
            sequence.len(),
            sequence
        );

        assert!(
            !sequence.is_empty(),
            "{name} should have at least one attack"
        );
        assert_eq!(sequence[0], bab, "{name} first attack should equal BAB");

        for i in 1..sequence.len() {
            assert_eq!(
                sequence[i],
                sequence[i - 1] - 5,
                "{name} each subsequent attack should be -5"
            );
        }

        if let Some(&last) = sequence.last() {
            assert!(last >= 1, "{name} last attack should be >= 1, got {last}");
        }
    }
}

#[tokio::test]
async fn test_hit_points_consistency() {
    println!("\n=== Hit Points Consistency ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon1.bic", "Occidio L1"),
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("ryathstrongarm/ryathstrongarm1.bic", "Ryath L1"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
        ("okkugodofbears/okkugodofbears4.bic", "Okku L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let current_hp = character.current_hp();
        let max_hp = character.max_hp();
        let temp_hp = character.temp_hp();

        println!("{name:<15}: Current={current_hp:>3}, Max={max_hp:>3}, Temp={temp_hp:>2}");

        assert!(max_hp >= 1, "{name} max HP should be at least 1");
        assert!(
            current_hp <= max_hp + temp_hp,
            "{name} current HP should not exceed max + temp"
        );
        assert!(temp_hp >= 0, "{name} temp HP should be non-negative");
    }
}

#[tokio::test]
async fn test_hp_progression_l1_vs_l30() {
    println!("\n=== HP Progression L1 vs L30 ===");

    let progressions = [
        (
            "occidiooctavon/occidiooctavon1.bic",
            "occidiooctavon/occidiooctavon4.bic",
            "Occidio",
        ),
        (
            "ryathstrongarm/ryathstrongarm1.bic",
            "ryathstrongarm/ryathstrongarm4.bic",
            "Ryath",
        ),
        (
            "qaraofblacklake/qaraofblacklake1.bic",
            "qaraofblacklake/qaraofblacklake4.bic",
            "Qara",
        ),
    ];

    for (l1_path, l30_path, name) in progressions {
        let char_l1 = load_character(l1_path);
        let char_l30 = load_character(l30_path);

        let hp_l1 = char_l1.max_hp();
        let hp_l30 = char_l30.max_hp();

        println!(
            "{:<10}: L1 MaxHP = {:>3}, L30 MaxHP = {:>3}, Δ = {:>3}",
            name,
            hp_l1,
            hp_l30,
            hp_l30 - hp_l1
        );

        assert!(
            hp_l30 > hp_l1,
            "{name} L30 HP ({hp_l30}) should exceed L1 HP ({hp_l1})"
        );
    }
}

#[tokio::test]
async fn test_size_modifier() {
    println!("\n=== Size Modifier ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("qaraofblacklake/qaraofblacklake4.bic", "Qara L30"),
        ("okkugodofbears/okkugodofbears4.bic", "Okku L30"),
        ("theconstruct/theconstruct4.bic", "Construct L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let size = character.creature_size();
        let size_mod = character.size_modifier();

        let expected_mod = match size {
            1 => 2,  // Tiny
            2 => 1,  // Small
            3 => 0,  // Medium
            4 => -1, // Large
            5 => -2, // Huge
            _ => 0,
        };

        println!(
            "{name:<15}: CreatureSize={size}, SizeMod={size_mod:>2} (expected={expected_mod})"
        );

        assert_eq!(
            size_mod, expected_mod,
            "{name} size modifier for size {size} should be {expected_mod}"
        );
    }
}

// ============================================================
// Combat Summary Integration Tests
// ============================================================

async fn create_decoder() -> app_lib::services::item_property_decoder::ItemPropertyDecoder {
    app_lib::services::item_property_decoder::ItemPropertyDecoder::default()
}

#[tokio::test]
async fn test_combat_summary_across_fixtures() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");
    let decoder = create_decoder().await;

    println!("\n=== Combat Summary Across Fixtures ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
        ("okkugodofbears/okkugodofbears4.bic", "Okku L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let summary = character.get_combat_summary(game_data, &decoder);

        println!(
            "{:<15}: BAB={}, AC={}",
            name, summary.bab, summary.armor_class.total
        );

        assert_eq!(summary.bab, character.calculate_bab(game_data));
        assert!(summary.armor_class.total >= 10);
    }
}

#[tokio::test]
async fn test_armor_class_breakdown() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");
    let decoder = create_decoder().await;

    println!("\n=== Armor Class Breakdown ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
        ("theconstruct/theconstruct4.bic", "Construct L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let ac = character.get_armor_class(game_data, &decoder);

        println!(
            "{:<15}: Total={:>2}, Touch={:>2}, FlatFooted={:>2}",
            name, ac.total, ac.touch, ac.flat_footed
        );
        println!(
            "  Breakdown: Base={}, Armor={}, Shield={}, Dex={}, Natural={}, Size={}",
            ac.breakdown.base,
            ac.breakdown.armor,
            ac.breakdown.shield,
            ac.breakdown.dex,
            ac.breakdown.natural,
            ac.breakdown.size
        );

        assert_eq!(ac.breakdown.base, 10, "{name} base AC should be 10");

        let expected_total = ac.breakdown.base
            + ac.breakdown.armor
            + ac.breakdown.shield
            + ac.breakdown.dex
            + ac.breakdown.natural
            + ac.breakdown.natural_item
            + ac.breakdown.dodge
            + ac.breakdown.deflection
            + ac.breakdown.size
            + ac.breakdown.misc;
        assert_eq!(
            ac.total, expected_total,
            "{name} AC total should match breakdown sum"
        );

        assert!(ac.touch <= ac.total, "{name} touch AC should be <= total");
        assert!(
            ac.flat_footed <= ac.total,
            "{name} flat-footed AC should be <= total"
        );
    }
}

#[tokio::test]
async fn test_attack_bonuses_breakdown() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");
    let decoder = create_decoder().await;

    println!("\n=== Attack Bonuses Breakdown ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let attacks = character.get_attack_bonuses(game_data, &decoder);

        println!(
            "{:<15}: Melee={:>2}, Ranged={:>2}, BAB={}",
            name, attacks.melee, attacks.ranged, attacks.bab
        );
        println!(
            "  Melee: Base={}, Ability={}, Size={}, Equip={}",
            attacks.melee_breakdown.base,
            attacks.melee_breakdown.ability,
            attacks.melee_breakdown.size,
            attacks.melee_breakdown.equipment
        );

        assert_eq!(attacks.bab, character.calculate_bab(game_data));

        let expected_melee = attacks.melee_breakdown.base
            + attacks.melee_breakdown.ability
            + attacks.melee_breakdown.size
            + attacks.melee_breakdown.equipment
            + attacks.melee_breakdown.misc;
        assert_eq!(
            attacks.melee, expected_melee,
            "{name} melee should match breakdown"
        );
    }
}

#[tokio::test]
async fn test_initiative_breakdown() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Initiative Breakdown ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("qaraofblacklake/qaraofblacklake4.bic", "Qara L30"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
    ];

    let decoder = create_decoder().await;

    for (path, name) in fixtures {
        let character = load_character(path);
        let init = character.get_initiative_breakdown(game_data, &decoder);

        println!(
            "{:<15}: Total={:>2} (Dex={}, Feat={})",
            name, init.total, init.dex, init.feat
        );

        let expected_total = init.dex + init.feat;
        assert_eq!(
            init.total, expected_total,
            "{name} initiative should match breakdown"
        );
    }
}

#[tokio::test]
async fn test_combat_summary_progression() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");
    let decoder = create_decoder().await;

    println!("\n=== Combat Summary Progression L1 vs L30 ===");

    let progressions = [
        (
            "occidiooctavon/occidiooctavon1.bic",
            "occidiooctavon/occidiooctavon4.bic",
            "Occidio",
        ),
        (
            "ryathstrongarm/ryathstrongarm1.bic",
            "ryathstrongarm/ryathstrongarm4.bic",
            "Ryath",
        ),
    ];

    for (l1_path, l30_path, name) in progressions {
        let char_l1 = load_character(l1_path);
        let char_l30 = load_character(l30_path);

        let summary_l1 = char_l1.get_combat_summary(game_data, &decoder);
        let summary_l30 = char_l30.get_combat_summary(game_data, &decoder);

        println!(
            "{:<10}: L1: BAB={}, AC={} | L30: BAB={}, AC={}",
            name,
            summary_l1.bab,
            summary_l1.armor_class.total,
            summary_l30.bab,
            summary_l30.armor_class.total
        );

        assert!(
            summary_l30.bab > summary_l1.bab,
            "{name} L30 BAB should exceed L1"
        );
        assert!(
            char_l30.get_attack_sequence(game_data).len()
                >= char_l1.get_attack_sequence(game_data).len(),
            "{name} L30 should have >= attacks than L1"
        );
    }
}
