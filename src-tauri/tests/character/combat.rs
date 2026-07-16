use super::super::common::{create_test_context, load_test_gff};
use app_lib::character::Character;
use app_lib::parsers::gff::GffParser;

fn load_character(fixture_path: &str) -> Character {
    let bytes = load_test_gff(fixture_path);
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");
    Character::from_gff(root)
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
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let bab = character.calculate_bab(game_data);
        let level = character.total_level();

        println!("{name:<15}: Level {level:>2}, BAB {bab:>2}");

        assert!(bab >= 0, "{name} should have non-negative BAB");

        if name.contains("L30") {
            assert!(
                bab >= 10,
                "{name} should have BAB >= 10 at high level, got {bab}"
            );
        }
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
async fn test_melee_attack_bonus() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Melee Attack Bonus ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
        ("okkugodofbears/okkugodofbears4.bic", "Okku L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let melee_ab = character.get_melee_attack_bonus(game_data);
        let bab = character.calculate_bab(game_data);
        let str_mod = character.ability_modifier(app_lib::character::types::AbilityIndex::STR);
        let size_mod = character.size_modifier();

        println!("{name:<15}: MeleeAB={melee_ab:>2} (BAB={bab} + STR={str_mod} + Size={size_mod})");

        assert_eq!(
            melee_ab,
            bab + str_mod + size_mod,
            "{name} melee AB should equal BAB + STR mod + size mod"
        );
    }
}

#[tokio::test]
async fn test_ranged_attack_bonus() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Ranged Attack Bonus ===");

    let fixtures = [
        ("qaraofblacklake/qaraofblacklake4.bic", "Qara L30"),
        ("sagemelchior/sagemelchior4.bic", "Melchior L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let ranged_ab = character.get_ranged_attack_bonus(game_data);
        let bab = character.calculate_bab(game_data);
        let dex_mod = character.ability_modifier(app_lib::character::types::AbilityIndex::DEX);
        let size_mod = character.size_modifier();

        println!(
            "{name:<15}: RangedAB={ranged_ab:>2} (BAB={bab} + DEX={dex_mod} + Size={size_mod})"
        );

        assert_eq!(
            ranged_ab,
            bab + dex_mod + size_mod,
            "{name} ranged AB should equal BAB + DEX mod + size mod"
        );
    }
}

#[tokio::test]
async fn test_damage_bonuses() {
    println!("\n=== Damage Bonuses ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
        ("okkugodofbears/okkugodofbears4.bic", "Okku L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let damage = character.get_damage_bonuses();
        let str_mod = character.ability_modifier(app_lib::character::types::AbilityIndex::STR);

        println!(
            "{:<15}: STR mod={:>2}, Melee={:>2}, 2H={:>2}, Off-hand={:>2}, Ranged={:>2}",
            name, str_mod, damage.melee, damage.two_handed, damage.off_hand, damage.ranged
        );

        assert_eq!(
            damage.melee, str_mod,
            "{name} melee damage should equal STR mod"
        );
        assert_eq!(
            damage.two_handed,
            (str_mod * 3) / 2,
            "{name} two-handed damage should be 1.5x STR mod"
        );
        assert_eq!(
            damage.off_hand,
            str_mod / 2,
            "{name} off-hand damage should be 0.5x STR mod"
        );
        assert_eq!(damage.ranged, 0, "{name} ranged damage bonus should be 0");
    }
}

#[tokio::test]
async fn test_initiative_calculation() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Initiative Calculation ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("qaraofblacklake/qaraofblacklake4.bic", "Qara L30"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let initiative = character.calculate_initiative(game_data);
        let dex_mod = character.ability_modifier(app_lib::character::types::AbilityIndex::DEX);

        println!("{name:<15}: Initiative={initiative:>2} (DEX mod={dex_mod})");

        assert_eq!(
            initiative, dex_mod,
            "{name} initiative should equal DEX mod"
        );
    }
}

#[tokio::test]
async fn test_base_ac_calculation() {
    println!("\n=== Base AC Calculation ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("qaraofblacklake/qaraofblacklake4.bic", "Qara L30"),
        ("theconstruct/theconstruct4.bic", "Construct L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let base_ac = character.calculate_base_ac();
        let natural_ac = character.natural_ac();

        println!("{name:<15}: BaseAC={base_ac:>2} (10 + NaturalAC={natural_ac})");

        assert_eq!(
            base_ac,
            10 + natural_ac,
            "{name} base AC should be 10 + natural AC"
        );
        assert!(natural_ac >= 0, "{name} natural AC should be non-negative");
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
        let current_hp = character.current_hit_points();
        let max_hp = character.max_hit_points();
        let temp_hp = character.temp_hit_points();

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

        let hp_l1 = char_l1.max_hit_points();
        let hp_l30 = char_l30.max_hit_points();

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

#[tokio::test]
async fn test_combat_stats_aggregate() {
    println!("\n=== Combat Stats Aggregate ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("theconstruct/theconstruct4.bic", "Construct L30"),
        ("okkugodofbears/okkugodofbears4.bic", "Okku L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let stats = character.combat_stats();

        println!(
            "{:<15}: NaturalAC={}, DR={}, SR={}",
            name, stats.natural_ac, stats.damage_reduction, stats.spell_resistance
        );

        assert_eq!(
            stats.natural_ac,
            character.natural_ac(),
            "{name} combat_stats natural_ac should match method"
        );
        assert_eq!(
            stats.damage_reduction,
            character.damage_reduction(),
            "{name} combat_stats DR should match method"
        );
        assert_eq!(
            stats.spell_resistance,
            character.spell_resistance(),
            "{name} combat_stats SR should match method"
        );
    }
}

#[tokio::test]
async fn test_barbarian_damage_reduction() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Barbarian Damage Reduction ===");

    let character = load_character("okkugodofbears/okkugodofbears4.bic");
    let barb_dr = character.get_barbarian_damage_reduction(game_data);
    let barb_level = character.get_class_level(app_lib::character::ClassId(0));

    println!("Okku: Barbarian Level={barb_level}, Barbarian DR={barb_dr}");

    if barb_level >= 7 {
        let expected_dr = 1 + (barb_level - 7) / 3;
        assert_eq!(
            barb_dr, expected_dr,
            "Barbarian DR at level {barb_level} should be {expected_dr}"
        );
    } else {
        assert_eq!(barb_dr, 0, "Barbarian DR should be 0 below level 7");
    }

    let non_barb = load_character("qaraofblacklake/qaraofblacklake4.bic");
    let non_barb_dr = non_barb.get_barbarian_damage_reduction(game_data);
    println!("Qara: Barbarian DR={non_barb_dr} (no barbarian levels)");
    assert_eq!(non_barb_dr, 0, "Non-barbarian should have 0 barbarian DR");
}

#[tokio::test]
async fn test_spell_resistance() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Spell Resistance ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("qaraofblacklake/qaraofblacklake4.bic", "Qara L30"),
        ("theconstruct/theconstruct4.bic", "Construct L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let base_sr = character.spell_resistance();
        let racial_sr = character.get_racial_spell_resistance(game_data);
        let total_sr = character.get_total_spell_resistance(game_data);

        println!("{name:<15}: BaseSR={base_sr}, RacialSR={racial_sr}, TotalSR={total_sr}");

        assert!(base_sr >= 0, "{name} base SR should be non-negative");
        assert!(racial_sr >= 0, "{name} racial SR should be non-negative");
        assert_eq!(
            total_sr,
            base_sr.max(racial_sr),
            "{name} total SR should be max(base, racial)"
        );
    }
}

// ============================================================
// Combat Summary Integration Tests
// ============================================================

async fn create_decoder() -> app_lib::services::item_property_decoder::ItemPropertyDecoder {
    use app_lib::config::nwn2_paths::NWN2Paths;
    use app_lib::services::resource_manager::ResourceManager;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    let paths = Arc::new(RwLock::new(NWN2Paths::new()));
    let rm = Arc::new(RwLock::new(ResourceManager::new(paths.clone())));
    {
        let mut rm_guard = rm.write().await;
        let _ = rm_guard.initialize().await;
    }
    app_lib::services::item_property_decoder::ItemPropertyDecoder::new(rm)
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
            "{:<15}: BAB={}, AC={}, MeleeAB={}, RangedAB={}",
            name,
            summary.bab,
            summary.armor_class.total,
            summary.attack_bonuses.melee,
            summary.attack_bonuses.ranged
        );

        assert_eq!(summary.bab, character.calculate_bab(game_data));
        assert!(!summary.attack_sequence.is_empty());
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
async fn test_combat_maneuver_bonus() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Combat Maneuver Bonus ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
        ("okkugodofbears/okkugodofbears4.bic", "Okku L30"),
    ];

    let decoder = create_decoder().await;

    for (path, name) in fixtures {
        let character = load_character(path);
        let cmb = character.get_combat_maneuver_bonus(game_data, &decoder);

        println!(
            "{:<15}: CMB={:>2} (BAB={}, STR={}, Size={})",
            name, cmb.total, cmb.bab, cmb.str_mod, cmb.size_mod
        );

        let expected = cmb.bab + cmb.str_mod - cmb.size_mod;
        assert_eq!(cmb.total, expected, "{name} CMB should be BAB + STR - size");
        assert_eq!(cmb.bab, character.calculate_bab(game_data));
    }
}

#[tokio::test]
async fn test_movement_speed() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Movement Speed ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("okkugodofbears/okkugodofbears4.bic", "Okku L30"),
        ("qaraofblacklake/qaraofblacklake4.bic", "Qara L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let speed = character.get_movement_speed(game_data);

        println!(
            "{:<15}: Base={}, Current={}, ArmorPenalty={}",
            name, speed.base, speed.current, speed.armor_penalty
        );

        // Note: MovementRate may not exist in fixtures, defaults to 0 which becomes 30 with class bonuses
        // or stays 0 if not set. Just verify the struct is populated correctly.
        assert!(speed.base >= 0, "{name} base speed should be non-negative");
        assert!(
            speed.current >= 0,
            "{name} current speed should be non-negative"
        );
    }
}

#[tokio::test]
async fn test_damage_reductions_list() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");
    let decoder = create_decoder().await;

    println!("\n=== Damage Reductions List ===");

    let character = load_character("okkugodofbears/okkugodofbears4.bic");
    let reductions = character.get_damage_reductions(game_data, &decoder);

    println!("Okku L30: {} damage reduction entries", reductions.len());
    for dr in &reductions {
        println!("  DR {}/{}  (Source: {})", dr.amount, dr.bypass, dr.source);
    }

    let character_no_dr = load_character("qaraofblacklake/qaraofblacklake4.bic");
    let no_reductions = character_no_dr.get_damage_reductions(game_data, &decoder);
    println!("Qara L30: {} damage reduction entries", no_reductions.len());
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
            "{:<10}: L1: BAB={}, AC={}, Melee={} | L30: BAB={}, AC={}, Melee={}",
            name,
            summary_l1.bab,
            summary_l1.armor_class.total,
            summary_l1.attack_bonuses.melee,
            summary_l30.bab,
            summary_l30.armor_class.total,
            summary_l30.attack_bonuses.melee
        );

        assert!(
            summary_l30.bab > summary_l1.bab,
            "{name} L30 BAB should exceed L1"
        );
        assert!(
            summary_l30.attack_sequence.len() >= summary_l1.attack_sequence.len(),
            "{name} L30 should have >= attacks than L1"
        );
    }
}
