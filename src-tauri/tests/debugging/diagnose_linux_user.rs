use app_lib::character::Character;
use app_lib::parsers::gff::GffParser;
use app_lib::services::savegame_handler::SaveGameHandler;
use std::path::PathBuf;

#[path = "../common/mod.rs"]
mod common;

fn linux_save_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/saves/user_save_linux")
}

fn load_character_from_save() -> Character {
    let handler =
        SaveGameHandler::new(linux_save_dir(), false, false).expect("Failed to open save");

    let bic_data = handler
        .extract_player_bic()
        .expect("Failed to extract player.bic")
        .expect("No player.bic in save");

    let parser = GffParser::from_bytes(bic_data).expect("Failed to parse BIC");
    let root = parser.read_struct_fields(0).expect("Failed to read root");
    Character::from_gff(root)
}

#[test]
#[ignore]
fn diagnose_linux_save_skills() {
    let character = load_character_from_save();
    let ctx = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(common::create_test_context());
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Linux User Save: Character ===");
    println!("Name: {}", character.full_name());
    println!("Race: {:?}", character.race_id());
    println!("Total level: {}", character.total_level());
    println!(
        "Classes: {:?}",
        character
            .class_entries()
            .iter()
            .map(|e| format!("class_id={} level={}", e.class_id.0, e.level))
            .collect::<Vec<_>>()
    );

    println!("\n=== Skills Table Check ===");
    let skills_table = game_data.get_table("skills");
    println!("skills.2da loaded: {}", skills_table.is_some());
    if let Some(table) = &skills_table {
        println!("skills.2da row count: {}", table.row_count());
        for row_idx in 0..table.row_count().min(5) {
            if let Some(row) = table.get_by_id(row_idx as i32) {
                let label = row
                    .get("label")
                    .and_then(|opt| opt.as_deref())
                    .unwrap_or("(none)");
                let removed = row
                    .get("removed")
                    .and_then(|opt| opt.as_deref())
                    .unwrap_or("(none)");
                println!("  row {row_idx}: label={label}, removed={removed}");
            }
        }
    }

    println!("\n=== Skill Summary ===");
    let summary = character.get_skill_summary(&game_data, None);
    println!("Skill entries returned: {}", summary.len());
    for skill in &summary {
        println!(
            "  {} (id={}): ranks={}, total={}, class_skill={}",
            skill.name, skill.skill_id.0, skill.ranks, skill.total, skill.is_class_skill
        );
    }

    assert!(!summary.is_empty(), "Skills should not be empty!");
}

#[test]
#[ignore]
fn diagnose_linux_save_appearance() {
    let character = load_character_from_save();
    let ctx = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(common::create_test_context());
    let game_data = ctx.loader.game_data().expect("Game data not loaded");
    let rm = ctx.resource_manager.blocking_read();

    println!("\n=== Linux User Save: Appearance ===");
    println!("Appearance type: {}", character.appearance_type());
    println!("Gender: {}", character.gender());
    println!("Head: {}", character.appearance_head());

    let appearance_table = game_data.get_table("appearance");
    println!("appearance.2da loaded: {}", appearance_table.is_some());
    if let Some(table) = &appearance_table {
        let app_id = character.appearance_type();
        let row = table.get_by_id(app_id);
        println!("Row for appearance_id {app_id}: {}", row.is_some());
        if let Some(r) = &row {
            for key in [
                "label",
                "nwn2_model_body",
                "nwn2_model_head",
                "nwn2_skeleton_file",
                "modeltype",
            ] {
                let val = r
                    .get(key)
                    .and_then(|opt| opt.as_deref())
                    .unwrap_or("(missing)");
                println!("  {key} = {val}");
            }
        }
    }

    let gender_table = game_data.get_table("gender");
    println!("gender.2da loaded: {}", gender_table.is_some());

    let model_parts = character.resolve_model_parts(&game_data, &rm);
    println!(
        "resolve_model_parts: {}",
        if model_parts.is_some() {
            "OK"
        } else {
            "NONE (this is the bug!)"
        }
    );

    if let Some(parts) = &model_parts {
        println!("  skeleton: {}", parts.skeleton_resref);
        println!("  body_parts: {:?}", parts.body_parts);
        println!("  head: {}", parts.head_resref);
    }
}

#[test]
#[ignore]
fn diagnose_linux_save_spells() {
    let character = load_character_from_save();
    let ctx = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(common::create_test_context());
    let game_data = ctx.loader.game_data().expect("Game data not loaded");

    println!("\n=== Linux User Save: Spells ===");

    let spells_table = game_data.get_table("spells");
    println!("spells.2da loaded: {}", spells_table.is_some());
    if let Some(table) = &spells_table {
        println!("spells.2da row count: {}", table.row_count());
    }

    println!("\nClass entries:");
    for entry in character.class_entries() {
        let class_id = entry.class_id;
        let is_caster = character.is_spellcaster(class_id, &game_data);
        println!(
            "  class_id={}: level={}, is_spellcaster={}",
            class_id.0, entry.level, is_caster
        );

        if is_caster {
            for spell_level in 0..=9 {
                let known = character.known_spells(class_id, spell_level);
                if !known.is_empty() {
                    println!(
                        "    Level {spell_level}: {} known spells {:?}",
                        known.len(),
                        known.iter().map(|s| s.0).collect::<Vec<_>>()
                    );
                }
            }
        }
    }

    let ability_spells = character.get_ability_spells(&game_data);
    println!("\nAbility spells: {}", ability_spells.len());
    for s in &ability_spells {
        println!(
            "  {} (id={}): innate_level={}",
            s.name, s.spell_id, s.innate_level
        );
    }
}
