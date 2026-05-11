use super::super::common::{create_test_context, load_test_gff};
use app_lib::character::{AbilityIndex, BackgroundId, Character, DomainId, FeatId, FeatSource};
use app_lib::loaders::types::LoadedTable;
use app_lib::parsers::gff::GffParser;

fn load_character(fixture_path: &str) -> Character {
    let bytes = load_test_gff(fixture_path);
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");
    Character::from_gff(root)
}

fn cell_value(table: &LoadedTable, row: usize, col: &str) -> Option<String> {
    table.get_cell(row, col).ok().flatten()
}

fn cell_int(table: &LoadedTable, row: usize, col: &str) -> Option<i32> {
    cell_value(table, row, col).and_then(|s| s.parse().ok())
}

#[tokio::test]
async fn test_feat_count_across_fixtures() {
    println!("\n=== Feat Count Across Fixtures ===");

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
        let feat_count = character.feat_count();
        let total_level = character.total_level();

        println!("{name:<15}: {feat_count} feats (Level {total_level})");

        assert!(feat_count >= 1, "{name} should have at least 1 feat");
        assert!(
            feat_count <= 200,
            "{name} has unreasonable feat count: {feat_count}"
        );

        if name.contains("L30") {
            assert!(
                feat_count >= 10,
                "{name} should have many feats at L30, got {feat_count}"
            );
        }
    }
}

#[tokio::test]
async fn test_feat_entries_structure() {
    println!("\n=== Feat Entries Structure ===");

    let fixtures = [
        "occidiooctavon/occidiooctavon4.bic",
        "qaraofblacklake/qaraofblacklake4.bic",
        "ryathstrongarm/ryathstrongarm4.bic",
    ];

    for path in fixtures {
        let character = load_character(path);
        let entries = character.feat_entries();

        println!("{}: {} feat entries", character.first_name(), entries.len());

        assert_eq!(
            entries.len(),
            character.feat_count(),
            "feat_entries().len() should equal feat_count()"
        );

        for entry in &entries[..std::cmp::min(5, entries.len())] {
            println!(
                "  FeatId({}) - source: {:?}, uses: {:?}",
                entry.feat_id.0, entry.source, entry.uses
            );

            assert!(entry.feat_id.0 >= 0, "Feat ID should be non-negative");
        }
    }
}

#[tokio::test]
async fn test_has_feat_consistency() {
    println!("\n=== has_feat Consistency ===");

    let character = load_character("occidiooctavon/occidiooctavon4.bic");

    let feat_ids = character.feat_ids();
    let entries = character.feat_entries();

    println!(
        "{}: {} feats via feat_ids()",
        character.first_name(),
        feat_ids.len()
    );

    for feat_id in &feat_ids {
        assert!(
            character.has_feat(*feat_id),
            "has_feat() should return true for feat {} in feat_ids()",
            feat_id.0
        );
    }

    for entry in &entries {
        assert!(
            character.has_feat(entry.feat_id),
            "has_feat() should return true for feat {} from entries",
            entry.feat_id.0
        );
    }

    assert!(
        !character.has_feat(FeatId(99999)),
        "has_feat() should return false for non-existent feat"
    );
}

#[tokio::test]
async fn test_feat_progression_comparison() {
    println!("\n=== Feat Progression Comparison (L1 vs L30) ===");

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
        (
            "sagemelchior/sagemelchior1.bic",
            "sagemelchior/sagemelchior4.bic",
            "Melchior",
        ),
    ];

    for (l1_path, l30_path, name) in characters {
        let char_l1 = load_character(l1_path);
        let char_l30 = load_character(l30_path);

        let feats_l1 = char_l1.feat_count();
        let feats_l30 = char_l30.feat_count();
        let level_l1 = char_l1.total_level();
        let level_l30 = char_l30.total_level();

        println!(
            "{}: L1 = {} feats (lvl {}), L30 = {} feats (lvl {}), gained = {}",
            name,
            feats_l1,
            level_l1,
            feats_l30,
            level_l30,
            feats_l30 - feats_l1
        );

        assert!(
            feats_l30 >= feats_l1,
            "{name} L30 should have >= feats than L1"
        );

        let l1_feat_ids = char_l1.feat_ids();
        let l30_feat_ids = char_l30.feat_ids();

        let retained: Vec<_> = l1_feat_ids
            .iter()
            .filter(|id| l30_feat_ids.contains(id))
            .collect();

        println!(
            "  {} of {} L1 feats retained at L30",
            retained.len(),
            l1_feat_ids.len()
        );
    }
}

#[tokio::test]
async fn test_feat_info_resolution() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Feat Info Resolution ===");

    let character = load_character("occidiooctavon/occidiooctavon4.bic");
    let feat_ids = character.feat_ids();

    println!(
        "{}: Resolving info for {} feats",
        character.first_name(),
        feat_ids.len()
    );

    let mut resolved_count = 0;
    for feat_id in feat_ids.iter().take(10) {
        if let Some(info) = character.get_feat_info(*feat_id, game_data) {
            println!(
                "  FeatId({}): {} [category={:?}, protected={}, custom={}]",
                info.id.0, info.label, info.category, info.is_protected, info.is_custom
            );

            assert_eq!(info.id, *feat_id, "FeatInfo id should match requested id");
            assert!(info.has_feat, "has_feat should be true for owned feat");
            resolved_count += 1;
        }
    }

    println!("  Resolved {resolved_count}/10 feats with info");
    assert!(resolved_count > 0, "Should resolve at least some feat info");
}

#[tokio::test]
async fn test_feat_summary() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Feat Summary ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio"),
        ("qaraofblacklake/qaraofblacklake4.bic", "Qara"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let summary = character.get_feat_summary(game_data);

        println!("{}: {} total feats", name, summary.total);
        println!(
            "  Class: {}, General: {}, Custom: {}, Background: {}, Domain: {}",
            summary.class_feats.len(),
            summary.general_feats.len(),
            summary.custom_feats.len(),
            summary.background_feats.len(),
            summary.domain_feats.len()
        );

        assert_eq!(
            summary.total as usize,
            character.feat_count(),
            "Summary total should match feat_count()"
        );

        let categorized_total = summary.class_feats.len()
            + summary.general_feats.len()
            + summary.custom_feats.len()
            + summary.background_feats.len()
            + summary.domain_feats.len();

        assert!(
            categorized_total <= summary.total as usize,
            "Categorized feats should not exceed total"
        );
    }
}

#[tokio::test]
async fn test_feat_slots_calculation() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Feat Slots Calculation (Blueprint Method) ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon1.bic", "Occidio L1"),
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let slots = character.get_feat_slots(game_data);

        println!(
            "{}: general={}, bonus={}, total={}, filled={}, open={}",
            name,
            slots.total_general_slots,
            slots.total_bonus_slots,
            slots.total_slots,
            slots.filled_slots,
            slots.open_slots
        );
        println!(
            "  open_general={}, open_bonus={}",
            slots.open_general_slots, slots.open_bonus_slots
        );

        assert!(slots.total_slots >= 0, "Total slots should be non-negative");
        assert!(
            slots.filled_slots >= 0,
            "Filled slots should be non-negative"
        );
        assert_eq!(
            slots.total_slots,
            slots.total_general_slots + slots.total_bonus_slots,
            "Total should equal general + bonus"
        );

        let expected_open = slots.total_slots - slots.filled_slots;
        if slots.open_slots != expected_open {
            println!(
                "  NOTE: open_slots ({}) differs from total-filled ({}), may be clamped or adjusted",
                slots.open_slots, expected_open
            );
        }
    }
}

#[tokio::test]
async fn test_feat_prerequisite_validation() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Feat Prerequisite Validation ===");

    let mut character = load_character("occidiooctavon/occidiooctavon4.bic");

    println!(
        "Testing with high-level character: {}",
        character.first_name()
    );

    let power_attack_id = FeatId(28);
    let result = character.validate_feat_prerequisites(power_attack_id, game_data);
    println!(
        "Power Attack (Feat 28): can_take={}, missing={:?}",
        result.can_take, result.missing_requirements
    );

    character.set_ability(AbilityIndex::STR, 8).unwrap();
    let result_low_str = character.validate_feat_prerequisites(power_attack_id, game_data);
    println!(
        "Power Attack with STR 8: can_take={}, missing={:?}",
        result_low_str.can_take, result_low_str.missing_requirements
    );

    assert!(
        !result_low_str.can_take,
        "Power Attack should fail with STR 8"
    );
    assert!(
        !result_low_str.missing_requirements.is_empty(),
        "Should have missing requirements listed"
    );
}

#[tokio::test]
async fn test_add_remove_feat_real_character() {
    println!("\n=== Add/Remove Feat on Real Character ===");

    let mut character = load_character("occidiooctavon/occidiooctavon1.bic");
    let initial_count = character.feat_count();

    println!(
        "{}: Starting with {} feats",
        character.first_name(),
        initial_count
    );

    let test_feat = FeatId(389);

    if character.has_feat(test_feat) {
        character
            .remove_feat(test_feat)
            .expect("Failed to remove pre-existing test feat");
        println!("  Removed pre-existing test feat");
    }

    let count_before_add = character.feat_count();
    character.add_feat(test_feat).expect("Failed to add feat");
    assert!(character.has_feat(test_feat), "Should have feat after add");
    assert_eq!(
        character.feat_count(),
        count_before_add + 1,
        "Feat count should increase by 1"
    );
    println!(
        "  Added FeatId({}), count: {} -> {}",
        test_feat.0,
        count_before_add,
        character.feat_count()
    );

    character
        .remove_feat(test_feat)
        .expect("Failed to remove feat");
    assert!(
        !character.has_feat(test_feat),
        "Should not have feat after remove"
    );
    assert_eq!(
        character.feat_count(),
        count_before_add,
        "Feat count should return to original"
    );
    println!(
        "  Removed FeatId({}), count: {}",
        test_feat.0,
        character.feat_count()
    );

    let dup_result = character.add_feat(test_feat);
    assert!(dup_result.is_ok());
    let dup_result2 = character.add_feat(test_feat);
    assert!(dup_result2.is_err(), "Adding duplicate feat should fail");
    println!("  Duplicate add correctly rejected");
}

#[tokio::test]
async fn test_feat_save_bonuses() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Feat Save Bonuses ===");

    let fixtures = [
        "occidiooctavon/occidiooctavon4.bic",
        "qaraofblacklake/qaraofblacklake4.bic",
    ];

    for path in fixtures {
        let character = load_character(path);
        let bonuses = character.get_feat_save_bonuses(game_data);

        println!(
            "{}: Fort {:+}, Reflex {:+}, Will {:+}",
            character.first_name(),
            bonuses.fortitude,
            bonuses.reflex,
            bonuses.will
        );

        assert!(bonuses.fortitude >= 0, "Fort bonus should be non-negative");
        assert!(bonuses.reflex >= 0, "Reflex bonus should be non-negative");
        assert!(bonuses.will >= 0, "Will bonus should be non-negative");
    }
}

#[tokio::test]
async fn test_feat_save_bonuses_after_adding() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    // Use Qara who had 0 save bonuses from feats
    let mut character = load_character("qaraofblacklake/qaraofblacklake4.bic");

    let before = character.get_feat_save_bonuses(game_data);
    println!(
        "Before adding feats: Fort {:+}, Ref {:+}, Will {:+}",
        before.fortitude, before.reflex, before.will
    );

    // Look up Great Fortitude, Iron Will, Lightning Reflexes by label
    let feat_table = game_data.get_table("feat").expect("feat table");
    let mut great_fort_id = None;
    let mut iron_will_id = None;
    let mut lightning_ref_id = None;

    for i in 0..feat_table.row_count() {
        let Some(row) = feat_table.get_by_id(i as i32) else {
            continue;
        };
        let label = row
            .get("label")
            .or_else(|| row.get("Label"))
            .or_else(|| row.get("LABEL"))
            .and_then(|s| s.clone())
            .unwrap_or_default();

        let name = character.get_feat_name(FeatId(i as i32), game_data);
        let name_lower = name.to_lowercase();

        if name_lower == "great fortitude" && great_fort_id.is_none() {
            println!("Found Great Fortitude: id={i}, label='{label}'");
            great_fort_id = Some(i as i32);
        } else if name_lower == "iron will" && iron_will_id.is_none() {
            println!("Found Iron Will: id={i}, label='{label}'");
            iron_will_id = Some(i as i32);
        } else if name_lower == "lightning reflexes" && lightning_ref_id.is_none() {
            println!("Found Lightning Reflexes: id={i}, label='{label}'");
            lightning_ref_id = Some(i as i32);
        }
    }

    let gf = great_fort_id.expect("Great Fortitude not found in feat.2da");
    let iw = iron_will_id.expect("Iron Will not found in feat.2da");
    let lr = lightning_ref_id.expect("Lightning Reflexes not found in feat.2da");
    println!("Feat IDs: Great Fortitude={gf}, Iron Will={iw}, Lightning Reflexes={lr}");

    // Remove them first in case Qara already has them
    let _ = character.remove_feat(FeatId(gf));
    let _ = character.remove_feat(FeatId(iw));
    let _ = character.remove_feat(FeatId(lr));

    let baseline = character.get_feat_save_bonuses(game_data);
    println!(
        "Baseline (after removal): Fort {:+}, Ref {:+}, Will {:+}",
        baseline.fortitude, baseline.reflex, baseline.will
    );

    // Add the three feats
    character.add_feat(FeatId(gf)).expect("add Great Fortitude");
    character.add_feat(FeatId(iw)).expect("add Iron Will");
    character
        .add_feat(FeatId(lr))
        .expect("add Lightning Reflexes");

    let after = character.get_feat_save_bonuses(game_data);
    println!(
        "After adding feats: Fort {:+}, Ref {:+}, Will {:+}",
        after.fortitude, after.reflex, after.will
    );

    // Each should add +2 to its respective save
    assert_eq!(
        after.fortitude,
        baseline.fortitude + 2,
        "Great Fortitude should add +2 to Fort save"
    );
    assert_eq!(
        after.reflex,
        baseline.reflex + 2,
        "Lightning Reflexes should add +2 to Reflex save"
    );
    assert_eq!(
        after.will,
        baseline.will + 2,
        "Iron Will should add +2 to Will save"
    );
}

#[tokio::test]
async fn test_feat_ac_bonuses() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Feat AC Bonuses ===");

    let fixtures = [
        "occidiooctavon/occidiooctavon4.bic",
        "ryathstrongarm/ryathstrongarm4.bic",
    ];

    for path in fixtures {
        let character = load_character(path);
        let ac_bonus = character.get_feat_ac_bonuses(game_data);

        println!("{}: Feat AC bonus = {:+}", character.first_name(), ac_bonus);

        assert!(ac_bonus >= 0, "AC bonus from feats should be non-negative");
    }
}

#[tokio::test]
async fn test_feat_initiative_bonus() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Feat Initiative Bonus ===");

    let fixtures = [
        "occidiooctavon/occidiooctavon4.bic",
        "qaraofblacklake/qaraofblacklake4.bic",
    ];

    for path in fixtures {
        let character = load_character(path);
        let init_bonus = character.get_feat_initiative_bonus(game_data);

        println!(
            "{}: Feat Initiative bonus = {:+}",
            character.first_name(),
            init_bonus
        );

        assert!(
            init_bonus >= 0,
            "Initiative bonus from feats should be non-negative"
        );
    }
}

#[tokio::test]
async fn test_feat_source_tracking() {
    println!("\n=== Feat Source Tracking ===");

    let character = load_character("occidiooctavon/occidiooctavon4.bic");
    let entries = character.feat_entries();

    println!(
        "{}: Analyzing {} feat sources",
        character.first_name(),
        entries.len()
    );

    let mut unknown = 0;
    let mut manual = 0;
    let mut class = 0;
    let mut race = 0;
    let mut domain = 0;
    let mut level = 0;
    let mut background = 0;

    for entry in &entries {
        match entry.source {
            FeatSource::Unknown => unknown += 1,
            FeatSource::Manual => manual += 1,
            FeatSource::Class => class += 1,
            FeatSource::Race => race += 1,
            FeatSource::Domain => domain += 1,
            FeatSource::Level => level += 1,
            FeatSource::Background => background += 1,
        }
    }

    println!(
        "  Unknown: {unknown}, Manual: {manual}, Class: {class}, Race: {race}, Domain: {domain}, Level: {level}, Background: {background}"
    );

    for entry in entries.iter().take(5) {
        let source_via_method = character.feat_source(entry.feat_id);
        assert_eq!(
            source_via_method,
            Some(entry.source),
            "feat_source() should match entry.source for FeatId({})",
            entry.feat_id.0
        );
    }

    let non_existent = character.feat_source(FeatId(99999));
    assert!(
        non_existent.is_none(),
        "Non-existent feat should have no source"
    );
}

#[tokio::test]
async fn test_protected_feats() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Protected Feats ===");

    let character = load_character("occidiooctavon/occidiooctavon4.bic");
    let entries = character.feat_entries();

    println!(
        "{}: Checking protection for {} feats",
        character.first_name(),
        entries.len()
    );

    let mut protected_count = 0;

    for entry in &entries {
        let is_protected = character.is_feat_protected(entry.feat_id, game_data);
        if is_protected {
            protected_count += 1;
        }

        let is_racial_or_background =
            matches!(entry.source, FeatSource::Race | FeatSource::Background);
        if is_racial_or_background {
            assert!(
                is_protected,
                "Racial/Background feat {} should be protected",
                entry.feat_id.0
            );
        }
    }

    println!("  {protected_count} protected feats found");
}

#[tokio::test]
async fn test_domain_feats() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Domain Feats ===");

    let fixtures = [
        "occidiooctavon/occidiooctavon4.bic",
        "sagemelchior/sagemelchior4.bic",
    ];

    for path in fixtures {
        let character = load_character(path);
        let domains = character.get_character_domains();

        println!("{}: {} domains", character.first_name(), domains.len());

        for domain_id in &domains {
            println!("  DomainId({})", domain_id.0);
        }

        let feat_ids = character.feat_ids();
        let mut domain_feat_count = 0;

        for feat_id in &feat_ids {
            if character.is_domain_feat(*feat_id, game_data) {
                domain_feat_count += 1;
            }
        }

        println!("  {domain_feat_count} domain-related feats detected");
    }
}

#[tokio::test]
async fn test_add_domain_and_feats() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Add Domain and Granted Feats ===");

    let mut character = load_character("occidiooctavon/occidiooctavon1.bic");
    let initial_feat_count = character.feat_count();

    println!(
        "{}: {} initial feats",
        character.first_name(),
        initial_feat_count
    );

    if let Some(domains_table) = game_data.get_table("domains")
        && domains_table.row_count() > 1
    {
        let domain_id = DomainId(1);
        match character.add_domain(domain_id, game_data) {
            Ok(added_feats) => {
                println!(
                    "  Added Domain {}, granted {} feats",
                    domain_id.0,
                    added_feats.len()
                );
                for feat_id in &added_feats {
                    println!("    - FeatId({})", feat_id.0);
                    assert!(
                        character.has_feat(*feat_id),
                        "Granted feat should be present"
                    );
                }
                assert!(
                    character.feat_count() >= initial_feat_count,
                    "Feat count should not decrease"
                );
            }
            Err(e) => {
                println!("  Domain add failed (may be invalid ID for this data): {e:?}");
            }
        }
    }
}

/// Stock NWN2 EE 2DA leaves `epithetfeat` blank for several domains (Strength,
/// Death, Evil, Healing, Protection, Sun, Fury) even though `feat.2da` ships
/// matching `FEAT_EPITHET_<NAME>_DOMAIN` rows. Without the label fallback the
/// UI remove-feat path bypasses `remove_domain` and leaves `Domain1`/`Domain2`
/// occupied, which then blocks any subsequent domain add.
#[tokio::test]
async fn test_find_domain_for_unwired_epithet_feat() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    let character = load_character("sagemelchior/sagemelchior4.bic");

    let strength_via_epithet = character.find_domain_for_feat(FeatId(1847), game_data);
    assert_eq!(
        strength_via_epithet,
        Some(DomainId(16)),
        "Strength domain epithet feat 1847 should resolve via label fallback"
    );

    let animal_via_epithet = character.find_domain_for_feat(FeatId(1835), game_data);
    assert_eq!(
        animal_via_epithet,
        Some(DomainId(1)),
        "Animal domain epithet feat 1835 should resolve via 2DA epithetfeat column"
    );
}

fn clear_all_domains(character: &mut Character, game_data: &app_lib::loaders::GameData) {
    let existing = character.get_character_domains();
    for domain_id in existing {
        let _ = character.remove_domain(domain_id, game_data);
    }
}

/// `castablefeat` is a 0/1 flag (passive vs active granted feat), not a feat ID.
/// An earlier version of `add_domain` parsed it as a feat ID and injected
/// `FeatId(1)` (the deleted Ambidexterity feat) on every domain operation.
#[tokio::test]
async fn test_add_domain_does_not_inject_castable_boolean_as_feat() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    let mut character = load_character("sagemelchior/sagemelchior4.bic");
    clear_all_domains(&mut character, game_data);
    let had_feat_one_before = character.has_feat(FeatId(1));
    let had_feat_zero_before = character.has_feat(FeatId(0));

    let strength = DomainId(16);
    let added = character
        .add_domain(strength, game_data)
        .expect("add Strength domain");

    assert!(
        !added.contains(&FeatId(1)),
        "add_domain must not add FeatId(1) (DEL_Ambidex) from castablefeat=1"
    );
    assert!(
        !added.contains(&FeatId(0)),
        "add_domain must not add FeatId(0) from castablefeat=0"
    );
    if !had_feat_one_before {
        assert!(
            !character.has_feat(FeatId(1)),
            "FeatId(1) must not appear in FeatList after add_domain"
        );
    }
    if !had_feat_zero_before {
        assert!(
            !character.has_feat(FeatId(0)),
            "FeatId(0) must not appear in FeatList after add_domain"
        );
    }
}

/// Reproduce the user-reported bug: a Cleric with an unwired-epithet domain
/// (e.g. Strength) in `Domain1`. Removing the epithet feat must clear the slot
/// so the next domain add can succeed.
#[tokio::test]
async fn test_remove_unwired_domain_frees_slot() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    let mut character = load_character("sagemelchior/sagemelchior4.bic");
    clear_all_domains(&mut character, game_data);

    let strength = DomainId(16);
    character
        .add_domain(strength, game_data)
        .expect("add Strength domain");
    assert!(
        character.get_character_domains().contains(&strength),
        "Strength should be in the character's domain list after add"
    );

    let resolved = character.find_domain_for_feat(FeatId(1847), game_data);
    assert_eq!(resolved, Some(strength));

    let removed_feats = character
        .remove_domain(strength, game_data)
        .expect("remove Strength domain");
    assert!(removed_feats.contains(&FeatId(1847)));

    assert!(
        !character.get_character_domains().contains(&strength),
        "Strength must be cleared from Domain1/Domain2 after remove"
    );
}

/// Defense-in-depth: a character file already corrupted by an earlier editor
/// version may carry `Domain1`/`Domain2` values whose epithet feat is no longer
/// in `FeatList`. Adding a new domain must succeed by reclaiming the stale slot.
#[tokio::test]
async fn test_stale_domain_slot_is_reclaimed() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    let mut character = load_character("sagemelchior/sagemelchior4.bic");
    clear_all_domains(&mut character, game_data);

    character
        .add_domain(DomainId(16), game_data)
        .expect("add Strength domain");
    character
        .add_domain(DomainId(1), game_data)
        .expect("add Animal domain");

    character
        .remove_feat(FeatId(1847))
        .expect("remove Strength epithet feat directly to simulate stale state");

    let domains_before = character.get_character_domains();
    assert!(
        domains_before.contains(&DomainId(16)),
        "Domain1=Strength should still be present (slot is stale until reclaimed)"
    );

    let mysticism = DomainId(37);
    character
        .add_domain(mysticism, game_data)
        .expect("adding Mysticism must reclaim the stale Strength slot");

    let domains_after = character.get_character_domains();
    assert!(domains_after.contains(&mysticism));
    assert!(
        !domains_after.contains(&DomainId(16)),
        "Stale Strength slot must be cleared once reclaimed"
    );
}

#[tokio::test]
async fn test_feat_name_resolution() {
    let ctx = create_test_context().await;

    println!("\n=== Feat Name Resolution (via feat.2da + TLK) ===");

    let feat_table = ctx
        .loader
        .get_table("feat")
        .expect("feat.2da should be loaded");
    println!("Feat table loaded: {} rows", feat_table.row_count());

    let character = load_character("occidiooctavon/occidiooctavon4.bic");
    let feat_ids = character.feat_ids();

    println!(
        "{}: Resolving names for {} feats",
        character.first_name(),
        feat_ids.len()
    );

    let mut label_resolved = 0;
    let mut tlk_resolved = 0;

    for feat_id in feat_ids.iter().take(15) {
        let row = feat_id.0 as usize;
        let label =
            cell_value(feat_table, row, "LABEL").unwrap_or_else(|| format!("Feat {}", feat_id.0));
        let strref = cell_int(feat_table, row, "FEAT");

        let tlk_name = strref
            .and_then(|sr| ctx.loader.get_string(sr))
            .unwrap_or_default();

        if !label.is_empty() && !label.starts_with("****") {
            label_resolved += 1;
        }
        if !tlk_name.is_empty() {
            tlk_resolved += 1;
        }

        let display_name = if !tlk_name.is_empty() {
            tlk_name.clone()
        } else {
            label.clone()
        };

        println!(
            "  FeatId({:>4}): {} [label: {}]",
            feat_id.0, display_name, label
        );
    }

    println!("  Labels resolved: {label_resolved}/15, TLK names resolved: {tlk_resolved}/15");
    assert!(
        label_resolved > 0,
        "Should resolve at least some feat labels from 2DA"
    );
}

#[tokio::test]
async fn test_feat_uses() {
    println!("\n=== Feat Uses ===");

    let mut character = load_character("occidiooctavon/occidiooctavon4.bic");
    let entries = character.feat_entries();

    println!(
        "{}: Checking uses for {} feats",
        character.first_name(),
        entries.len()
    );

    let feats_with_uses: Vec<_> = entries.iter().filter(|e| e.uses.is_some()).collect();
    println!("  {} feats have uses defined", feats_with_uses.len());

    for entry in feats_with_uses.iter().take(5) {
        println!(
            "    FeatId({}): {} uses",
            entry.feat_id.0,
            entry.uses.unwrap_or(0)
        );

        let uses_via_method = character.get_feat_uses(entry.feat_id);
        assert_eq!(
            uses_via_method, entry.uses,
            "get_feat_uses should match entry.uses"
        );
    }

    if let Some(first_feat) = entries.first() {
        let new_uses = 5;
        let success = character.set_feat_uses(first_feat.feat_id, new_uses);
        if success {
            let retrieved = character.get_feat_uses(first_feat.feat_id);
            assert_eq!(
                retrieved,
                Some(new_uses),
                "set_feat_uses should update the value"
            );
            println!(
                "  set_feat_uses for FeatId({}) = {} verified",
                first_feat.feat_id.0, new_uses
            );
        }
    }
}

#[tokio::test]
async fn test_general_feat_slots_calculation() {
    println!("\n=== General Feat Slots Calculation ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon1.bic", "Occidio L1"),
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("qaraofblacklake/qaraofblacklake4.bic", "Qara L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let general_slots = character.calculate_general_feat_slots();
        let total_level = character.total_level();

        println!("{name}: Level {total_level} -> {general_slots} general feat slots");

        let base_slots = 1 + (total_level / 3);
        assert!(
            general_slots >= base_slots,
            "{name} should have at least {base_slots} general slots (1 + level/3)"
        );
    }
}

#[tokio::test]
async fn test_bonus_feat_slots_calculation() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Bonus Feat Slots Calculation ===");

    let fixtures = [
        ("occidiooctavon/occidiooctavon4.bic", "Occidio L30"),
        ("ryathstrongarm/ryathstrongarm4.bic", "Ryath L30"),
    ];

    for (path, name) in fixtures {
        let character = load_character(path);
        let bonus_slots = character.calculate_bonus_feat_slots(game_data);

        println!("{name}: {bonus_slots} bonus feat slots from classes");

        assert!(bonus_slots >= 0, "Bonus slots should be non-negative");
    }
}

/// Locate a usable (non-None, non-REMOVED) background row with a DisplayFeat we can exercise.
fn first_selectable_background(table: &LoadedTable) -> Option<(BackgroundId, FeatId)> {
    for row in 1..table.row_count() {
        if cell_value(table, row, "REMOVED").as_deref() == Some("1") {
            continue;
        }
        if let Some(feat_id) = cell_int(table, row, "DisplayFeat") {
            return Some((BackgroundId(row as i32), FeatId(feat_id)));
        }
    }
    None
}

#[tokio::test]
async fn test_find_background_for_feat() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");
    let bg_table = ctx
        .loader
        .get_table("backgrounds")
        .expect("backgrounds.2da should be loaded");

    let (expected_bg, feat_id) =
        first_selectable_background(bg_table).expect("No selectable background in 2DA");

    let character = load_character("occidiooctavon/occidiooctavon1.bic");
    let found = character.find_background_for_feat(feat_id, game_data);

    assert_eq!(
        found,
        Some(expected_bg),
        "find_background_for_feat should map DisplayFeat -> background row"
    );

    let bogus = character.find_background_for_feat(FeatId(-1), game_data);
    assert_eq!(bogus, None, "Unknown feat must not match any background");
}

#[tokio::test]
async fn test_char_background_is_dword_after_add() {
    use app_lib::parsers::gff::GffValue;

    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");
    let bg_table = ctx
        .loader
        .get_table("backgrounds")
        .expect("backgrounds.2da should be loaded");

    let (bg_id, _) = first_selectable_background(bg_table).expect("No selectable background");

    let mut character = load_character("occidiooctavon/occidiooctavon1.bic");
    character
        .add_background(bg_id, game_data)
        .expect("add_background should succeed");

    match character.gff().get("CharBackground") {
        Some(GffValue::Dword(v)) => {
            assert_eq!(
                *v as i32, bg_id.0,
                "CharBackground Dword value must equal background id"
            );
        }
        other => panic!("CharBackground must be GffValue::Dword per NWN2 schema, got {other:?}"),
    }

    character
        .remove_background(game_data)
        .expect("remove_background should succeed");

    match character.gff().get("CharBackground") {
        Some(GffValue::Dword(0)) => (),
        other => panic!("After remove, CharBackground must be Dword(0), got {other:?}"),
    }
}

#[tokio::test]
async fn test_add_background_sets_field_and_feat() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");
    let bg_table = ctx
        .loader
        .get_table("backgrounds")
        .expect("backgrounds.2da should be loaded");

    let (bg_id, feat_id) =
        first_selectable_background(bg_table).expect("No selectable background in 2DA");

    let mut character = load_character("occidiooctavon/occidiooctavon1.bic");
    if character.has_feat(feat_id) {
        character.remove_feat(feat_id).ok();
    }

    let added = character
        .add_background(bg_id, game_data)
        .expect("add_background should succeed");

    assert_eq!(
        character.get_character_background(),
        Some(bg_id),
        "CharBackground must be set to the row id"
    );
    assert!(
        character.has_feat(feat_id),
        "The background's DisplayFeat must be in the feat list"
    );
    assert_eq!(
        character.feat_source(feat_id),
        Some(FeatSource::Background),
        "Added background feat must be tagged with FeatSource::Background"
    );
    assert!(
        added.contains(&feat_id),
        "add_background should report the newly granted feat"
    );
}

#[tokio::test]
async fn test_remove_background_clears_field_and_feat() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");
    let bg_table = ctx
        .loader
        .get_table("backgrounds")
        .expect("backgrounds.2da should be loaded");

    let (bg_id, feat_id) =
        first_selectable_background(bg_table).expect("No selectable background in 2DA");

    let mut character = load_character("occidiooctavon/occidiooctavon1.bic");
    if character.has_feat(feat_id) {
        character.remove_feat(feat_id).ok();
    }

    character
        .add_background(bg_id, game_data)
        .expect("add_background should succeed");

    let removed = character
        .remove_background(game_data)
        .expect("remove_background should succeed");

    assert_eq!(
        character.get_character_background(),
        None,
        "CharBackground must be cleared after remove"
    );
    assert!(
        !character.has_feat(feat_id),
        "The background feat must be gone after remove"
    );
    assert!(
        removed.contains(&feat_id),
        "remove_background should report the removed feat"
    );
}
