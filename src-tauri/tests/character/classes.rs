use super::super::common::{create_test_context, load_test_gff};
use app_lib::character::gff_helpers::gff_value_to_i32;
use app_lib::character::{AbilityIndex, Character, ClassId, FeatId};
use app_lib::parsers::gff::GffParser;

fn load_character(fixture_path: &str) -> Character {
    let bytes = load_test_gff(fixture_path);
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");
    Character::from_gff(root).expect("character from gff")
}

#[tokio::test]
async fn test_total_level_across_fixtures() {
    println!("\n=== Total Level Across Fixtures ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon1.bic", "Occidio L1"),
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("qaraofblacklake/qaraofblacklake1.bic", "Qara L1"),
        ("qaraofblacklake/qaraofblacklake4.bic", "Qara L30"),
        ("ryathstrongarm/ryathstrongarm1.bic", "Ryath L1"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
        ("sagemelchior/sagemelchior1.bic", "Melchior L1"),
        ("sagemelchior/sagemelchior4.bic", "Melchior L30"),
        ("oneofmany/oneofmany1.bic", "OneOfMany L1"),
        ("oneofmany/oneofmany4.bic", "OneOfMany L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let level = character.total_level();

        println!("{name:<15}: Level {level}");

        assert!(level >= 1, "{name} should have level >= 1");
        assert!(level <= 60, "{name} should have level <= 60");

        if name.contains("L30") {
            assert!(
                level >= 20,
                "{name} should be high level (>=20), got {level}"
            );
        }
    }
}

#[tokio::test]
async fn test_class_entries_structure() {
    println!("\n=== Class Entries Structure ===");

    let fixtures = [
        "occidiooctavon/occidiooctavon4.bic",
        "qaraofblacklake/qaraofblacklake4.bic",
        "ryathstrongarm/ryathstrongarm4.bic",
    ];

    for path in fixtures {
        let character = load_character(path);
        let entries = character.class_entries();

        println!(
            "{}: {} class entries",
            character.first_name(),
            entries.len()
        );

        assert!(
            !entries.is_empty(),
            "{} should have at least one class",
            character.first_name()
        );
        // Note: Standard NWN2 allows 3 classes, but modded/EE saves may have more
        assert!(
            entries.len() <= 10,
            "{} has unreasonable class count",
            character.first_name()
        );

        let mut total_from_entries = 0;
        for entry in &entries {
            println!("  ClassId({}) - Level {}", entry.class_id.0, entry.level);

            assert!(entry.class_id.0 >= 0, "Class ID should be non-negative");
            assert!(entry.level >= 1, "Class level should be >= 1");
            assert!(entry.level <= 60, "Class level should be <= 60");

            total_from_entries += entry.level;
        }

        assert_eq!(
            total_from_entries,
            character.total_level(),
            "Sum of class levels should equal total level"
        );
    }
}

#[tokio::test]
async fn test_class_level_per_class() {
    println!("\n=== Class Level Per Class ===");

    let character = load_character("ryathstrongarm/ryathstrongarm4.bic");

    println!(
        "{}: Level {}",
        character.first_name(),
        character.total_level()
    );

    let entries = character.class_entries();
    for entry in &entries {
        let level_by_method = character.class_level(entry.class_id);

        println!(
            "  ClassId({}) - via entries: {}, via class_level(): {}",
            entry.class_id.0, entry.level, level_by_method
        );

        assert_eq!(
            entry.level, level_by_method,
            "class_level() should match class_entries() level"
        );
    }

    let nonexistent_class = ClassId(255);
    assert_eq!(
        character.class_level(nonexistent_class),
        0,
        "Non-existent class should return level 0"
    );
}

#[tokio::test]
async fn test_level_history_parsing() {
    println!("\n=== Level History Parsing ===");

    let character = load_character("occidiooctavon/occidiooctavon4.bic");

    let history = character.level_history();
    let total_level = character.total_level();

    println!(
        "{}: {} history entries for {} total levels",
        character.first_name(),
        history.len(),
        total_level
    );

    if !history.is_empty() {
        for entry in &history[..std::cmp::min(5, history.len())] {
            println!(
                "  Level {}: ClassId({}) class_lvl={} hp={} ability={:?}",
                entry.character_level,
                entry.class_id.0,
                entry.class_level,
                entry.hp_gained,
                entry.ability_increase
            );
        }

        if history.len() > 5 {
            println!("  ... ({} more entries)", history.len() - 5);
        }

        for (idx, entry) in history.iter().enumerate() {
            assert_eq!(
                entry.character_level as usize,
                idx + 1,
                "Character level should be sequential"
            );
            assert!(entry.class_level >= 1, "Class level should be >= 1");
            assert!(entry.hp_gained >= 0, "HP gained should be non-negative");
        }

        let ability_increases: Vec<_> = history
            .iter()
            .filter(|e| e.ability_increase.is_some())
            .collect();

        println!("  Ability increases: {}", ability_increases.len());

        for increase in &ability_increases {
            assert_eq!(
                increase.character_level % 4,
                0,
                "Ability increase should be at level divisible by 4, got {}",
                increase.character_level
            );
        }
    }
}

#[tokio::test]
async fn test_multiclass_characters() {
    println!("\n=== Multiclass Characters ===");

    let fixtures = [
        "occidiooctavon/occidiooctavon4.bic",
        "sagemelchior/sagemelchior4.bic",
        "theconstruct/theconstruct4.bic",
    ];

    for path in fixtures {
        let character = load_character(path);
        let class_count = character.class_count();
        let entries = character.class_entries();

        println!(
            "{}: {} classes (multiclass={})",
            character.first_name(),
            class_count,
            class_count > 1
        );

        for entry in &entries {
            let has = character.has_class(entry.class_id);
            assert!(has, "has_class() should return true for existing class");

            println!("  ClassId({}) - has_class: {}", entry.class_id.0, has);
        }

        assert!(
            !character.has_class(ClassId(255)),
            "Should not have non-existent class"
        );
    }
}

#[tokio::test]
async fn test_class_info_resolution() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    println!("\n=== Class Info Resolution ===");

    let character = load_character("ryathstrongarm/ryathstrongarm4.bic");

    let entries = character.class_entries();

    for entry in &entries {
        if let Some(info) = character.get_class_info(entry.class_id, game_data) {
            println!(
                "  ClassId({}): {} (HD: d{}, Caster: {}, BAB: {:?})",
                info.id.0, info.name, info.hit_die, info.is_spellcaster, info.bab_type
            );

            assert!(!info.name.is_empty(), "Class name should not be empty");
            assert!(info.hit_die >= 4, "Hit die should be at least d4");
            assert!(info.hit_die <= 12, "Hit die should be at most d12");
        } else {
            println!("  ClassId({}) - No info found", entry.class_id.0);
        }
    }
}

#[tokio::test]
async fn test_hit_die_lookup() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    println!("\n=== Hit Die Lookup ===");

    let character = load_character("qaraofblacklake/qaraofblacklake4.bic");

    let entries = character.class_entries();

    for entry in &entries {
        let hit_die = character.get_hit_die(entry.class_id, game_data);
        let class_name = character.get_class_name(entry.class_id, game_data);

        println!(
            "  {} (ClassId {}): d{}",
            class_name, entry.class_id.0, hit_die
        );

        assert!(hit_die >= 4, "Hit die should be at least d4");
        assert!(hit_die <= 12, "Hit die should be at most d12");
    }

    let unknown_hd = character.get_hit_die(ClassId(255), game_data);
    assert_eq!(unknown_hd, 6, "Unknown class should default to d6");
}

#[tokio::test]
async fn test_xp_progress_calculation() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    println!("\n=== XP Progress Calculation ===");

    let fixtures = [
        "occidiooctavon/occidiooctavon1.bic",
        "occidiooctavon/occidiooctavon4.bic",
    ];

    for path in fixtures {
        let character = load_character(path);
        let xp_progress = character.get_xp_progress(game_data);

        println!(
            "{}: Level {} - XP {} ({}% to next)",
            character.first_name(),
            xp_progress.current_level,
            xp_progress.current_xp,
            xp_progress.progress_percent as i32
        );
        println!(
            "  Current level XP: {}, Next level XP: {}, Remaining: {}",
            xp_progress.xp_for_current_level,
            xp_progress.xp_for_next_level,
            xp_progress.xp_remaining
        );

        assert_eq!(
            xp_progress.current_level,
            character.total_level(),
            "XP progress level should match total level"
        );
        assert!(
            xp_progress.progress_percent >= 0.0 && xp_progress.progress_percent <= 100.0,
            "Progress percent should be 0-100"
        );
        assert!(
            xp_progress.xp_for_next_level >= xp_progress.xp_for_current_level,
            "Next level XP should be >= current level XP"
        );
    }
}

#[tokio::test]
async fn test_class_summary() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    println!("\n=== Class Summary ===");

    let character = load_character("occidiooctavon/occidiooctavon4.bic");

    let summary = character.get_class_summary(game_data);

    println!("{}: {} class(es)", character.first_name(), summary.len());

    for entry in &summary {
        println!(
            "  {} (ClassId {}) - Level {} (d{})",
            entry.name, entry.class_id.0, entry.level, entry.hit_die
        );

        assert!(!entry.name.is_empty(), "Class name should not be empty");
        assert!(entry.level >= 1, "Level should be >= 1");
        assert!(
            entry.hit_die >= 4 && entry.hit_die <= 12,
            "Hit die should be d4-d12"
        );
    }

    let total_from_summary: i32 = summary.iter().map(|e| e.level).sum();
    assert_eq!(
        total_from_summary,
        character.total_level(),
        "Summary levels should sum to total level"
    );
}

#[tokio::test]
async fn test_level_progression_comparison() {
    println!("\n=== Level Progression Comparison (L1 vs L30) ===");

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

        let level_l1 = char_l1.total_level();
        let level_l30 = char_l30.total_level();
        let classes_l1 = char_l1.class_count();
        let classes_l30 = char_l30.class_count();

        println!(
            "{name}: L1 = {level_l1} levels ({classes_l1} class), L30 = {level_l30} levels ({classes_l30} classes)"
        );

        assert!(level_l30 > level_l1, "{name} L30 should be higher than L1");
        assert!(
            classes_l30 >= classes_l1,
            "{name} should have >= classes at L30"
        );

        let l1_entries = char_l1.class_entries();
        let l30_entries = char_l30.class_entries();

        for l1_entry in &l1_entries {
            if let Some(l30_entry) = l30_entries.iter().find(|e| e.class_id == l1_entry.class_id) {
                assert!(
                    l30_entry.level >= l1_entry.level,
                    "{} class {} should have same or higher level at L30",
                    name,
                    l1_entry.class_id.0
                );
            }
        }
    }
}

#[tokio::test]
async fn test_prestige_class_detection() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    println!("\n=== Prestige Class Detection ===");

    let character = load_character("occidiooctavon/occidiooctavon4.bic");

    let entries = character.class_entries();

    for entry in &entries {
        let is_prestige = character.is_prestige_class(entry.class_id, game_data);
        let class_name = character.get_class_name(entry.class_id, game_data);

        println!(
            "  {} (ClassId {}): prestige = {}",
            class_name, entry.class_id.0, is_prestige
        );
    }

    let well_known_base = [ClassId(0), ClassId(1), ClassId(2)];
    for class_id in well_known_base {
        let is_prestige = character.is_prestige_class(class_id, game_data);
        let class_name = character.get_class_name(class_id, game_data);
        println!(
            "  Testing {} (ClassId {}): prestige = {}",
            class_name, class_id.0, is_prestige
        );
    }
}

#[tokio::test]
async fn test_xp_for_level_calculation() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    println!("\n=== XP for Level Calculation ===");

    let character = load_character("occidiooctavon/occidiooctavon4.bic");

    let xp_values: Vec<i32> = (1..=10)
        .map(|lvl| character.calculate_xp_for_level(lvl, game_data))
        .collect();

    println!("XP requirements for levels 1-10:");
    for (i, xp) in xp_values.iter().enumerate() {
        let level = i + 1;
        println!("  Level {level:>2}: {xp:>7} XP");
    }

    for i in 1..xp_values.len() {
        assert!(
            xp_values[i] >= xp_values[i - 1],
            "XP for level {} ({}) should be >= level {} ({})",
            i + 1,
            xp_values[i],
            i,
            xp_values[i - 1]
        );
    }

    assert_eq!(
        character.calculate_xp_for_level(1, game_data),
        0,
        "Level 1 should require 0 XP"
    );
}

#[tokio::test]
async fn test_class_count_consistency() {
    println!("\n=== Class Count Consistency ===");

    let fixtures = [
        "occidiooctavon/occidiooctavon4.bic",
        "qaraofblacklake/qaraofblacklake4.bic",
        "ryathstrongarm/ryathstrongarm4.bic",
        "sagemelchior/sagemelchior4.bic",
        "oneofmany/oneofmany4.bic",
        "theconstruct/theconstruct4.bic",
        "okkugodofbears/okkugodofbears4.bic",
    ];

    for path in fixtures {
        let character = load_character(path);
        let class_count = character.class_count();
        let entries = character.class_entries();

        println!(
            "{}: class_count()={}, entries.len()={}",
            character.first_name(),
            class_count,
            entries.len()
        );

        assert_eq!(
            class_count,
            entries.len(),
            "class_count() should equal class_entries().len()"
        );
    }
}

#[tokio::test]
async fn test_level_up_grants_only_automatic_class_feats() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;
    let mut character = load_character("occidiooctavon/occidiooctavon1.bic");
    let cleric = ClassId(2);
    let epic_spell_feats = [FeatId(874), FeatId(875), FeatId(878)];
    assert!(epic_spell_feats.iter().all(|f| !character.has_feat(*f)));

    for _ in 0..21 {
        character.level_up(cleric, game_data).expect("level up");
    }

    assert_eq!(character.class_level(cleric), 21);
    for feat in epic_spell_feats {
        assert!(
            !character.has_feat(feat),
            "cls_feat_cler lists feat {} as List 0 (selectable), not granted",
            feat.0
        );
    }
}

#[tokio::test]
async fn test_level_up_then_level_down_restores_character() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;
    let original = load_character("occidiooctavon/occidiooctavon1.bic");
    let mut character = load_character("occidiooctavon/occidiooctavon1.bic");
    let fighter = ClassId(4);

    for _ in 0..3 {
        character.level_up(fighter, game_data).expect("level up");
    }
    for _ in 0..3 {
        character
            .level_down(fighter, game_data)
            .expect("level down");
    }

    let feats = |c: &Character| {
        let mut ids: Vec<i32> = c.feat_entries().iter().map(|f| f.feat_id.0).collect();
        ids.sort_unstable();
        ids
    };
    assert_eq!(feats(&character), feats(&original));
    assert_eq!(character.base_hp(), original.base_hp());
    assert_eq!(character.max_hp(), original.max_hp());
    assert_eq!(character.base_attack_bonus(), original.base_attack_bonus());
    for field in ["fortbonus", "refbonus", "willbonus"] {
        assert_eq!(character.get_i32(field), original.get_i32(field), "{field}");
    }
    assert!(!character.has_field("BAB"));
}

#[tokio::test]
async fn test_remove_class_keeps_feats_picked_at_other_class_levels() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;
    let mut character = load_character("occidiooctavon/occidiooctavon1.bic");
    let (rogue, barbarian, deflect_arrows) = (ClassId(8), ClassId(0), FeatId(8));
    assert!(!character.has_feat(deflect_arrows));

    for _ in 0..5 {
        character.level_up(rogue, game_data).expect("level up");
    }
    character
        .add_feat(deflect_arrows)
        .expect("pick feat at a Rogue level");
    character.level_up(barbarian, game_data).expect("level up");

    character
        .remove_class(barbarian, game_data)
        .expect("remove class");

    assert!(
        character.has_feat(deflect_arrows),
        "a feat picked at a Rogue level must survive removing Barbarian"
    );
}

#[tokio::test]
async fn test_remove_middle_class_realigns_ability_increases() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;
    let mut character = load_character("occidiooctavon/occidiooctavon1.bic");
    let (fighter, wizard) = (ClassId(4), ClassId(10));
    let str_before = character.base_ability(AbilityIndex::STR);
    let int_before = character.base_ability(AbilityIndex::INT);

    character.level_up(fighter, game_data).expect("level up");
    for _ in 0..6 {
        character.level_up(wizard, game_data).expect("level up");
        match character.total_level() {
            4 => character
                .set_ability_with_cascades(AbilityIndex::STR, str_before + 1, game_data)
                .expect("STR increase"),
            8 => character
                .set_ability_with_cascades(AbilityIndex::INT, int_before + 1, game_data)
                .expect("INT increase"),
            _ => {}
        }
    }

    character
        .remove_class(fighter, game_data)
        .expect("remove class");

    assert_eq!(character.total_level(), 7);
    let increases: Vec<(usize, i32)> = character
        .get_list("LvlStatList")
        .expect("level history")
        .iter()
        .enumerate()
        .filter_map(|(idx, entry)| {
            entry
                .get("LvlStatAbility")
                .and_then(gff_value_to_i32)
                .filter(|a| (0..=5).contains(a))
                .map(|a| (idx, a))
        })
        .collect();
    assert_eq!(increases, vec![(3, 0)], "one increase left, on level 4");
    assert_eq!(character.base_ability(AbilityIndex::STR), str_before + 1);
    assert_eq!(character.base_ability(AbilityIndex::INT), int_before);
    assert!(character.get_available_skill_points() >= 0);
}
