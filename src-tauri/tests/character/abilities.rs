use super::super::common::{create_test_context, load_test_gff};
use app_lib::character::{AbilityIndex, Character};
use app_lib::parsers::gff::GffParser;
use app_lib::services::item_property_decoder::ItemPropertyDecoder;
use std::sync::Arc;
use tokio::sync::RwLock;

fn load_character(fixture_path: &str) -> Character {
    let bytes = load_test_gff(fixture_path);
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");
    Character::from_gff(root)
}

#[tokio::test]
async fn test_real_character_base_abilities() {
    println!("\n=== Base Ability Scores Across Characters ===");

    let characters = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("qaraofblacklake/qaraofblacklake4.bic", "Qara L30"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
        ("sagemelchior/sagemelchior4.bic", "Melchior L30"),
        ("oneofmany/oneofmany4.bic", "OneOfMany L30"),
    ];

    for (path, name) in characters {
        let character = load_character(path);

        let scores = character.base_scores();
        let total_level = character.total_level();

        println!(
            "{:<15}: STR={:>2} DEX={:>2} CON={:>2} INT={:>2} WIS={:>2} CHA={:>2}  (Level {})",
            name,
            scores.str_,
            scores.dex,
            scores.con,
            scores.int,
            scores.wis,
            scores.cha,
            total_level
        );

        // Base abilities should be within valid D&D ranges
        for ability in AbilityIndex::all() {
            let score = scores.get(ability);
            assert!(
                (3..=50).contains(&score),
                "{} has invalid {} score: {}",
                name,
                ability.gff_field(),
                score
            );
        }
    }
}

#[tokio::test]
async fn test_ability_modifiers_formula() {
    println!("\n=== Ability Modifier Formula Verification ===");

    let character = load_character("occidiooctavon/occidiooctavon4.bic");

    let scores = character.base_scores();
    let mods = character.ability_modifiers();

    // Verify modifier formula: (score - 10) / 2, rounded down
    let expected_str_mod = (scores.str_ - 10) / 2;
    let expected_dex_mod = (scores.dex - 10) / 2;
    let expected_con_mod = (scores.con - 10) / 2;
    let expected_int_mod = (scores.int - 10) / 2;
    let expected_wis_mod = (scores.wis - 10) / 2;
    let expected_cha_mod = (scores.cha - 10) / 2;

    println!(
        "STR: {} -> mod {} (expected {})",
        scores.str_, mods.str_mod, expected_str_mod
    );
    println!(
        "DEX: {} -> mod {} (expected {})",
        scores.dex, mods.dex_mod, expected_dex_mod
    );
    println!(
        "CON: {} -> mod {} (expected {})",
        scores.con, mods.con_mod, expected_con_mod
    );
    println!(
        "INT: {} -> mod {} (expected {})",
        scores.int, mods.int_mod, expected_int_mod
    );
    println!(
        "WIS: {} -> mod {} (expected {})",
        scores.wis, mods.wis_mod, expected_wis_mod
    );
    println!(
        "CHA: {} -> mod {} (expected {})",
        scores.cha, mods.cha_mod, expected_cha_mod
    );

    assert_eq!(mods.str_mod, expected_str_mod);
    assert_eq!(mods.dex_mod, expected_dex_mod);
    assert_eq!(mods.con_mod, expected_con_mod);
    assert_eq!(mods.int_mod, expected_int_mod);
    assert_eq!(mods.wis_mod, expected_wis_mod);
    assert_eq!(mods.cha_mod, expected_cha_mod);
}

#[tokio::test]
async fn test_racial_ability_modifiers() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Racial Ability Modifiers ===");

    // Load characters and check their racial modifiers
    let characters = [
        "occidiooctavon/occidiooctavon1.bic",
        "qaraofblacklake/qaraofblacklake4.bic",
        "ryathstrongarm/ryathstrongarm4.bic",
    ];

    for path in characters {
        let character = load_character(path);

        let race_id = character.race_id();
        let racial_mods = character.get_racial_ability_modifiers(game_data);

        // Get race name from racialtypes table
        let race_name = if let Some(racialtypes) = game_data.get_table("racialtypes") {
            if let Some(row) = racialtypes.get_by_id(race_id.0) {
                row.get("Label")
                    .or_else(|| row.get("label"))
                    .and_then(|v| v.clone())
                    .unwrap_or_else(|| format!("Race {}", race_id.0))
            } else {
                format!("Race {}", race_id.0)
            }
        } else {
            format!("Race {}", race_id.0)
        };

        println!(
            "{}: {} - STR {:+} DEX {:+} CON {:+} INT {:+} WIS {:+} CHA {:+}",
            character.first_name(),
            race_name,
            racial_mods.str_mod,
            racial_mods.dex_mod,
            racial_mods.con_mod,
            racial_mods.int_mod,
            racial_mods.wis_mod,
            racial_mods.cha_mod
        );

        // Racial modifiers should be within reasonable bounds (-4 to +4 typically)
        for ability in AbilityIndex::all() {
            let mod_val = racial_mods.get(ability);
            assert!(
                (-10..=10).contains(&mod_val),
                "{} has unusual racial {} modifier: {}",
                character.first_name(),
                ability.gff_field(),
                mod_val
            );
        }
    }
}

#[tokio::test]
async fn test_effective_abilities_with_racial_bonuses() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Effective Abilities (Base + Racial) ===");

    let character = load_character("ryathstrongarm/ryathstrongarm4.bic");

    let base = character.base_scores();
    let racial_mods = character.get_racial_ability_modifiers(game_data);
    let effective = character.get_effective_abilities(game_data);

    println!("Character: {}", character.first_name());
    println!(
        "Base:      STR={:>2} DEX={:>2} CON={:>2} INT={:>2} WIS={:>2} CHA={:>2}",
        base.str_, base.dex, base.con, base.int, base.wis, base.cha
    );
    println!(
        "Racial:    STR={:+} DEX={:+} CON={:+} INT={:+} WIS={:+} CHA={:+}",
        racial_mods.str_mod,
        racial_mods.dex_mod,
        racial_mods.con_mod,
        racial_mods.int_mod,
        racial_mods.wis_mod,
        racial_mods.cha_mod
    );
    println!(
        "Effective: STR={:>2} DEX={:>2} CON={:>2} INT={:>2} WIS={:>2} CHA={:>2}",
        effective.str_, effective.dex, effective.con, effective.int, effective.wis, effective.cha
    );

    // Verify effective = base + racial
    assert_eq!(effective.str_, base.str_ + racial_mods.str_mod);
    assert_eq!(effective.dex, base.dex + racial_mods.dex_mod);
    assert_eq!(effective.con, base.con + racial_mods.con_mod);
    assert_eq!(effective.int, base.int + racial_mods.int_mod);
    assert_eq!(effective.wis, base.wis + racial_mods.wis_mod);
    assert_eq!(effective.cha, base.cha + racial_mods.cha_mod);
}

#[tokio::test]
async fn test_total_abilities_with_equipment() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    let paths = Arc::new(RwLock::new(app_lib::config::NWN2Paths::new()));
    let rm = Arc::new(RwLock::new(
        app_lib::services::resource_manager::ResourceManager::new(paths),
    ));

    {
        let mut rm_guard = rm.write().await;
        rm_guard.initialize().await.expect("Failed to init RM");
    }

    let decoder = ItemPropertyDecoder::new(rm);

    println!("\n=== Total Abilities (Base + Racial + Equipment) ===");

    // Use a high-level character that likely has magical equipment
    let character = load_character("occidiooctavon/occidiooctavon4.bic");

    let base = character.base_scores();
    let effective = character.get_effective_abilities(game_data);
    let total = character.get_total_abilities(game_data, &decoder);
    let equip_bonuses = character.get_equipment_bonuses(game_data, &decoder);

    println!(
        "Character: {} (Level {})",
        character.first_name(),
        character.total_level()
    );
    println!(
        "Base:      STR={:>2} DEX={:>2} CON={:>2} INT={:>2} WIS={:>2} CHA={:>2}",
        base.str_, base.dex, base.con, base.int, base.wis, base.cha
    );
    println!(
        "Effective: STR={:>2} DEX={:>2} CON={:>2} INT={:>2} WIS={:>2} CHA={:>2}",
        effective.str_, effective.dex, effective.con, effective.int, effective.wis, effective.cha
    );
    println!(
        "Equipment: STR={:+} DEX={:+} CON={:+} INT={:+} WIS={:+} CHA={:+}",
        equip_bonuses.str_bonus,
        equip_bonuses.dex_bonus,
        equip_bonuses.con_bonus,
        equip_bonuses.int_bonus,
        equip_bonuses.wis_bonus,
        equip_bonuses.cha_bonus
    );
    println!(
        "Total:     STR={:>2} DEX={:>2} CON={:>2} INT={:>2} WIS={:>2} CHA={:>2}",
        total.str_, total.dex, total.con, total.int, total.wis, total.cha
    );

    // Verify total = effective + equipment
    assert_eq!(total.str_, effective.str_ + equip_bonuses.str_bonus);
    assert_eq!(total.dex, effective.dex + equip_bonuses.dex_bonus);
    assert_eq!(total.con, effective.con + equip_bonuses.con_bonus);
    assert_eq!(total.int, effective.int + equip_bonuses.int_bonus);
    assert_eq!(total.wis, effective.wis + equip_bonuses.wis_bonus);
    assert_eq!(total.cha, effective.cha + equip_bonuses.cha_bonus);
}

#[tokio::test]
async fn test_ability_increase_history() {
    println!("\n=== Ability Increase History (Level-Up Choices) ===");

    // High-level character should have multiple ability increases
    let character = load_character("occidiooctavon/occidiooctavon4.bic");

    let history = character.get_level_up_ability_history();
    let total_level = character.total_level();
    let expected_increases = total_level / 4; // One increase every 4 levels

    println!(
        "Character: {} (Level {})",
        character.first_name(),
        total_level
    );
    println!("Expected ability increases: {expected_increases}");
    println!("Actual ability increases: {}", history.len());

    for increase in &history {
        println!(
            "  Level {:>2}: {} (+1)",
            increase.level,
            increase.ability.gff_field()
        );
    }

    // Verify increases happen at correct levels (4, 8, 12, 16, 20, 24, 28)
    for increase in &history {
        assert_eq!(
            increase.level % 4,
            0,
            "Ability increase at level {} should be at a multiple of 4",
            increase.level
        );
    }

    // May not have all expected increases if LvlStatList is incomplete
    // Just verify we have reasonable data
    assert!(
        history.len() <= expected_increases as usize,
        "Should not have more increases than expected"
    );
}

#[tokio::test]
async fn test_ability_points_summary() {
    println!("\n=== Ability Points Summary ===");

    let characters = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("qaraofblacklake/qaraofblacklake4.bic", "Qara L30"),
    ];

    for (path, name) in characters {
        let character = load_character(path);

        let summary = character.get_ability_points_summary();
        let total_level = character.total_level();

        println!("{name} (Level {total_level}):");
        println!(
            "  Base Scores: STR={} DEX={} CON={} INT={} WIS={} CHA={}",
            summary.base_scores.str_,
            summary.base_scores.dex,
            summary.base_scores.con,
            summary.base_scores.int,
            summary.base_scores.wis,
            summary.base_scores.cha
        );
        println!(
            "  Expected Increases: {}, Actual: {}",
            summary.expected_increases, summary.actual_increases
        );

        // Expected increases should match level / 4
        assert_eq!(
            summary.expected_increases,
            total_level / 4,
            "Expected increases mismatch for {name}"
        );
    }
}

#[tokio::test]
async fn test_constitution_cascade_real_character() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Constitution Cascade (HP Recalculation) ===");

    // Load a higher-level character for more meaningful HP cascade test
    let mut character = load_character("occidiooctavon/occidiooctavon4.bic");

    let initial_con = character.base_ability(AbilityIndex::CON);
    let initial_max_hp = character.max_hp();
    let initial_current_hp = character.current_hp();
    let total_level = character.total_level();

    println!("Initial State:");
    println!("  CON: {} (mod: {})", initial_con, (initial_con - 10) / 2);
    println!("  Max HP: {initial_max_hp}");
    println!("  Current HP: {initial_current_hp}");
    println!("  Level: {total_level}");

    // Increase CON by 2 (should increase HP by level per +1 mod)
    let new_con = initial_con + 2;
    let result = character.set_ability_with_cascades(AbilityIndex::CON, new_con, game_data);
    assert!(result.is_ok(), "Failed to set CON: {result:?}");

    let new_max_hp = character.max_hp();
    let hp_change = new_max_hp - initial_max_hp;

    println!("\nAfter CON {initial_con} -> {new_con}:");
    println!("  Max HP: {new_max_hp} (was {initial_max_hp})");
    println!("  HP Change: {hp_change:+}");

    // If modifier changed, HP should have changed
    let old_mod = (initial_con - 10) / 2;
    let new_mod = (new_con - 10) / 2;

    if old_mod != new_mod {
        let expected_hp_change = (new_mod - old_mod) * total_level;
        assert_eq!(
            hp_change,
            expected_hp_change,
            "HP should change by (mod_diff * level) = {} * {} = {}",
            new_mod - old_mod,
            total_level,
            expected_hp_change
        );
    }
}

#[tokio::test]
async fn test_hit_points_consistency() {
    println!("\n=== Hit Points Consistency ===");

    let characters = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("qaraofblacklake/qaraofblacklake4.bic", "Qara L30"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
    ];

    for (path, name) in characters {
        let character = load_character(path);

        let hp = character.hit_points();

        println!(
            "{}: Current={} Max={} Temp={} Effective={}",
            name,
            hp.current,
            hp.max,
            hp.temp,
            hp.effective_current()
        );

        // Current HP should never exceed max
        assert!(
            hp.current <= hp.max,
            "{} has current HP ({}) > max HP ({})",
            name,
            hp.current,
            hp.max
        );

        // Effective current = current + temp
        assert_eq!(hp.effective_current(), hp.current + hp.temp);

        // Max HP should be positive for any character
        assert!(hp.max > 0, "{name} should have positive max HP");
    }
}

#[tokio::test]
async fn test_effective_ability_modifier() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Effective Ability Modifiers ===");

    let character = load_character("occidiooctavon/occidiooctavon4.bic");

    println!("Character: {}", character.first_name());

    for ability in AbilityIndex::all() {
        let base_score = character.base_ability(ability);
        let base_mod = character.ability_modifier(ability);
        let effective_mod = character.get_effective_ability_modifier(ability, game_data);
        let effective_scores = character.get_effective_abilities(game_data);
        let expected_effective_mod = (effective_scores.get(ability) - 10) / 2;

        println!(
            "  {}: base={} (mod {:+}), effective (mod {:+})",
            ability.gff_field(),
            base_score,
            base_mod,
            effective_mod
        );

        // Effective modifier should be calculated from effective score
        assert_eq!(
            effective_mod,
            expected_effective_mod,
            "Effective modifier for {} mismatch",
            ability.gff_field()
        );
    }
}
