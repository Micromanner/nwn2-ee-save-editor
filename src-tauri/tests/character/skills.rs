use app_lib::character::{Character, SkillId};

mod common {
    include!("../common/mod.rs");
}

use app_lib::parsers::gff::parser::GffParser;

#[tokio::test]
async fn test_skills_progression() {
    let ctx = common::create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    // Helper to parse character from bytes since Character::from_bytes doesn't exist
    let parse_char = |bytes: Vec<u8>| {
        let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
        let fields = parser
            .read_struct_fields(0)
            .expect("Failed to read root fields");
        Character::from_gff(fields)
    };

    // Load Level 1 Character (Qara - Sorcerer)
    let data_l1 = common::load_test_gff("qaraofblacklake/qaraofblacklake1.bic");
    let char_l1 = parse_char(data_l1);

    // Load Level 30 Character (Qara - Sorcerer)
    let data_l30 = common::load_test_gff("qaraofblacklake/qaraofblacklake4.bic");
    let char_l30 = parse_char(data_l30);

    // 1. Resolve Skill IDs dynamically
    let skills_table = game_data.get_table("skills").expect("skills.2da missing");
    let mut conc_id = SkillId(0); // Default fallback
    let mut spellcraft_id = SkillId(17); // Default fallback

    for i in 0..skills_table.row_count() {
        if let Some(row) = skills_table.get_by_id(i as i32) {
            let label = row
                .get("Label")
                .or_else(|| row.get("label"))
                .and_then(|v| v.as_deref());
            if let Some(lbl) = label {
                if lbl == "Concentration" {
                    conc_id = SkillId(i as i32);
                } else if lbl == "Spellcraft" {
                    spellcraft_id = SkillId(i as i32);
                }
            }
        }
    }

    // L1: Class Skill Max = 1 + 3 = 4
    // Using dynamic IDs
    let max_conc = char_l1.get_max_skill_ranks(conc_id, game_data);
    let max_spell = char_l1.get_max_skill_ranks(spellcraft_id, game_data);

    assert_eq!(max_conc, 4, "Max ranks for Concentration (L1)");
    assert_eq!(max_spell, 4, "Max ranks for Spellcraft (L1)");

    // L30: Class Skill Max = 30 + 3 = 33
    assert_eq!(char_l30.get_max_skill_ranks(conc_id, game_data), 33);
    assert_eq!(char_l30.get_max_skill_ranks(spellcraft_id, game_data), 33);

    // 2. Check Actual Ranks increasing
    let l1_conc = char_l1.skill_rank(conc_id);
    let l1_spell = char_l1.skill_rank(spellcraft_id);
    let l30_conc = char_l30.skill_rank(conc_id);
    let l30_spell = char_l30.skill_rank(spellcraft_id);

    assert!(
        l30_conc > l1_conc,
        "Concentration rank should increase (L1: {l1_conc}, L30: {l30_conc})"
    );
    assert!(
        l30_spell > l1_spell,
        "Spellcraft rank should increase (L1: {l1_spell}, L30: {l30_spell})"
    );

    // 3. Check Modifiers
    // Modifiers should also increase significantly due to ability bumps + items + ranks
    let conc_total = |character: &Character| {
        character
            .get_skill_summary(game_data, Some(&ctx.decoder))
            .into_iter()
            .find(|entry| entry.skill_id == conc_id)
            .map_or(0, |entry| entry.total)
    };
    let l1_conc_mod = conc_total(&char_l1);
    let l30_conc_mod = conc_total(&char_l30);

    assert!(
        l30_conc_mod > l1_conc_mod,
        "Concentration modifier should increase (L1: {l1_conc_mod}, L30: {l30_conc_mod})"
    );
}

#[tokio::test]
async fn test_skill_summary_parity() {
    let ctx = common::create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Failed to load game data");

    let parse_char = |bytes: Vec<u8>| {
        let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
        let fields = parser
            .read_struct_fields(0)
            .expect("Failed to read root fields");
        Character::from_gff(fields)
    };

    let data = common::load_test_gff("occidiooctavon/occidiooctavon4.bic");
    let character = parse_char(data);

    let summary = character.get_skill_summary(game_data, Some(&ctx.decoder));

    // Ensure we get a populated list
    assert!(!summary.is_empty(), "Skill summary should not be empty");

    for entry in &summary {
        let expected_max = character.get_max_skill_ranks(entry.skill_id, game_data);
        assert_eq!(
            entry.max_ranks, expected_max,
            "Max ranks mismatch for skill {}",
            entry.name
        );

        if entry.untrained || entry.ranks > 0 {
            let expected_total = entry.ranks + entry.modifier + entry.item_bonus + entry.feat_bonus;
            assert_eq!(
                entry.total, expected_total,
                "Total mismatch for skill {}",
                entry.name
            );
        } else {
            assert_eq!(
                entry.total, 0,
                "Untrained skill with 0 ranks should have total 0: {}",
                entry.name
            );
        }
    }
}

#[tokio::test]
async fn test_skill_costs_and_points() {
    let ctx = common::create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Failed to load game data");

    let parse_char = |bytes: Vec<u8>| {
        let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
        let fields = parser
            .read_struct_fields(0)
            .expect("Failed to read root fields");
        Character::from_gff(fields)
    };

    // Load L1 Character
    let data = common::load_test_gff("qaraofblacklake/qaraofblacklake1.bic");
    let mut character = parse_char(data);

    // Qara is Sorcerer -> Spellcraft is Class Skill, Stealth likely Cross-Class
    let mut spellcraft = SkillId(17);
    let mut stealth = SkillId(18); // Default

    // Resolve dynamically
    let skills_table = game_data.get_table("skills").unwrap();
    for i in 0..skills_table.row_count() {
        if let Some(row) = skills_table.get_by_id(i as i32) {
            let label = row
                .get("Label")
                .or_else(|| row.get("label"))
                .and_then(|v| v.as_deref());
            if let Some(lbl) = label {
                if lbl == "Spellcraft" {
                    spellcraft = SkillId(i as i32);
                }
                // NWN2 has Hide and Move Silently as separate skills; Hide is cross-class for Sorcerer.
                if lbl == "Hide" {
                    stealth = SkillId(i as i32);
                }
            }
        }
    }

    // Verify Class Skill Status
    let is_spell_class = character.is_class_skill(spellcraft, game_data);

    assert!(
        is_spell_class,
        "Spellcraft should be class skill for Sorcerer"
    );
    let is_stealth_class = character.is_class_skill(stealth, game_data);

    // Verify Costs
    // Class Skill: 1 pt
    assert_eq!(
        character.calculate_skill_cost(spellcraft, 1, false, game_data),
        1
    );

    // Cross Class: 2 pts (if it is indeed cross class)
    if !is_stealth_class {
        assert_eq!(
            character.calculate_skill_cost(stealth, 1, false, game_data),
            2
        );
    }

    // Verify Ability Learner Feat interaction (Feat 406)
    // We can simulate having the feat by passing true
    assert_eq!(
        character.calculate_skill_cost(stealth, 1, true, game_data),
        1,
        "Able Learner should reduce cross-class cost to 1"
    );

    // Test Point Expenditure
    let initial_points = character.get_available_skill_points();

    // Attempt to buy 1 rank of Spellcraft
    let current_rank = character.skill_rank(spellcraft);
    let new_rank = current_rank + 1;

    // Guard against exceeding max ranks
    let max_rank = character.get_max_skill_ranks(spellcraft, game_data);
    if new_rank <= max_rank {
        // Mock setting points if too low
        if initial_points < 10 {
            character.set_available_skill_points(10);
        }

        let cost = character
            .change_skill_rank(spellcraft, new_rank, game_data)
            .expect("Failed to set rank");
        assert_eq!(cost, 1);

        let new_points = character.get_available_skill_points();
        if initial_points < 10 {
            assert_eq!(new_points, 9, "Points should be 9 after spending 1 from 10");
        } else {
            assert_eq!(
                new_points,
                initial_points - 1,
                "Points should decrease by 1"
            );
        }
    }
}

/// Ground truth for the level-history skill point fields, checked over every
/// fixture character with real game data.
///
/// `LvlStatList[n].SkillPoints` is the pool remainder after that level-up,
/// so between consecutive levels `field[n] - field[n-1] == granted[n] -
/// cost(ranks recorded at n)`, with `granted` recomputed from class/feats/INT
/// and ranks costed against the class taken at that level.
/// Deltas start at level 3: toolset-built companions carry their creation
/// ranks in `SkillList` only and their level 1 field is uninitialised
/// garbage (5376, 24430, 12391). The last remainder must equal the
/// character's `SkillPoints` pool.
///
/// Excluded fixtures: `qaraofblacklake2-4` (INT raised 12 -> 14 without a
/// `LvlStatAbility` record, so the grant cannot be replayed; every level is
/// consistent with INT 12) and `sagemelchior2-4` (spends more per level than
/// any grant allows - ranks were injected). Proven by the rest: NWN2 has no
/// race skill-point column, humans get +1 via `FEAT_SKILLED` (1773), and a
/// rank is costed against the class being levelled (Occidio pays 2 for Heal
/// during Divine Champion levels although Paladin lists it).
#[tokio::test]
async fn level_history_skill_points_are_pool_remainders() {
    let ctx = common::create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    let fixtures = [
        "player.bic",
        "occidiooctavon/occidiooctavon1.bic",
        "occidiooctavon/occidiooctavon2.bic",
        "occidiooctavon/occidiooctavon3.bic",
        "occidiooctavon/occidiooctavon4.bic",
        "qaraofblacklake/qaraofblacklake1.bic",
        "ryathstrongarm/ryathstrongarm1.bic",
        "ryathstrongarm/ryathstrongarm2.bic",
        "ryathstrongarm/ryathstrongarm3.bic",
        "ryathstrongarm/ryathstrongarm4.bic",
        "sagemelchior/sagemelchior1.bic",
        "oneofmany/oneofmany1.bic",
        "oneofmany/oneofmany2.bic",
        "oneofmany/oneofmany3.bic",
        "oneofmany/oneofmany4.bic",
        "okkugodofbears/okkugodofbears1.bic",
        "okkugodofbears/okkugodofbears4.bic",
        "theconstruct/theconstruct1.bic",
        "theconstruct/theconstruct4.bic",
    ];

    let mut mismatches = Vec::new();
    let mut levels_checked = 0;

    for path in fixtures {
        let parser = GffParser::from_bytes(common::load_test_gff(path)).expect("parse GFF");
        let character = Character::from_gff(parser.read_struct_fields(0).expect("root"));

        let granted = character.skill_points_granted_per_level(game_data);
        let history = character.level_history();
        assert_eq!(granted.len(), history.len(), "{path}: one grant per level");

        let able_learner = character.has_feat(app_lib::character::FeatId(406));
        let spent_at = |entry: &app_lib::character::LevelHistoryEntry| -> i32 {
            entry
                .skills_gained
                .iter()
                .map(|s| {
                    character.skill_cost_for_class(
                        entry.class_id,
                        s.skill_id,
                        s.ranks,
                        able_learner,
                        game_data,
                    )
                })
                .sum()
        };

        for (n, (entry, granted)) in history.iter().zip(&granted).enumerate().skip(2) {
            let delta = entry.skill_points_remaining - history[n - 1].skill_points_remaining;
            let spent = spent_at(entry);
            levels_checked += 1;
            if delta != granted - spent {
                mismatches.push(format!(
                    "{path} level {}: field delta {delta} vs granted {granted} - spent {spent}",
                    entry.character_level
                ));
            }
        }

        if let Some(last) = history.last()
            && character.get_available_skill_points() != last.skill_points_remaining
        {
            mismatches.push(format!(
                "{path}: pool {} vs last remainder {}",
                character.get_available_skill_points(),
                last.skill_points_remaining
            ));
        }
    }

    println!("levels checked: {levels_checked}");
    assert!(
        mismatches.is_empty(),
        "{} mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
