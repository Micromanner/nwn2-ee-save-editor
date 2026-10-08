mod common;

use app_lib::character::{AbilityIndex, Character};
use app_lib::parsers::gff::GffParser;
use app_lib::services::savegame_handler::SaveGameHandler;
use common::{create_test_context, fixtures_path};

fn load_storm_player() -> Character {
    let save_path = fixtures_path().join("saves/STORM_Campaign");
    let handler =
        SaveGameHandler::new(&save_path).expect("Failed to open STORM_Campaign save");
    let player_data = handler
        .extract_player_bic()
        .expect("Failed to extract player.bic")
        .expect("player.bic not found in save");
    let parser = GffParser::from_bytes(player_data).expect("Failed to parse player.bic GFF");
    let root = parser
        .read_struct_fields(0)
        .expect("Failed to read root struct");
    Character::from_gff(root).expect("character from gff")
}

#[tokio::test]
async fn extract_storm_campaign_character() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;
    let decoder = &ctx.decoder;
    let character = load_storm_player();

    println!("================================================================");
    println!("  STORM CAMPAIGN - CHARACTER DATA (using command-level functions)");
    println!("================================================================");

    // ── Overview (same as get_overview_state) ──
    let overview = character.get_overview_state(game_data, decoder);
    println!("\n--- OVERVIEW (get_overview_state) ---");
    println!("Name:        {}", overview.full_name);
    println!("Race:        {}", overview.race_name);
    println!("Gender:      {}", overview.gender);
    println!("Age:         {}", overview.age);
    println!("Deity:       {}", overview.deity);
    println!(
        "Alignment:   {} (LC={}, GE={})",
        overview.alignment_string, overview.alignment.law_chaos, overview.alignment.good_evil
    );
    println!(
        "Experience:  {} / {}",
        overview.experience, overview.xp_progress.xp_for_next_level
    );
    println!("Total Level: {}", overview.total_level);
    println!(
        "HP:          {} / {}",
        overview.hit_points.current, overview.hit_points.max
    );
    println!("AC:          {}", overview.armor_class);
    println!("BAB:         {}", overview.base_attack_bonus);
    println!(
        "Saves:       Fort {} / Ref {} / Will {}",
        overview.saving_throws.fortitude,
        overview.saving_throws.reflex,
        overview.saving_throws.will
    );
    println!("Gold:        {}", overview.gold);
    println!("Background:  {:?}", overview.background);
    for cls in &overview.classes {
        println!("  Class: {} - Level {}", cls.name, cls.level);
    }

    // ── Abilities (same as get_abilities_state) ──
    let abilities = character.get_abilities_state(game_data, decoder);
    println!("\n--- ABILITIES (get_abilities_state) ---");
    println!("         BASE  EFF  TOTAL  MOD");
    for ability in AbilityIndex::all() {
        println!(
            "  {:3}:   {:>3}  {:>3}    {:>3}  {:+}",
            ability.short_name(),
            abilities.base_scores.get(ability),
            abilities.effective_scores.get(ability),
            abilities.base_scores.get(ability) + abilities.equipment_modifiers.get(ability),
            abilities.modifiers.get(ability),
        );
    }
    println!(
        "  Equipment bonuses: STR {:+} DEX {:+} CON {:+} INT {:+} WIS {:+} CHA {:+}",
        abilities.equipment_modifiers.str_mod,
        abilities.equipment_modifiers.dex_mod,
        abilities.equipment_modifiers.con_mod,
        abilities.equipment_modifiers.int_mod,
        abilities.equipment_modifiers.wis_mod,
        abilities.equipment_modifiers.cha_mod
    );

    // ── Combat Summary (same as get_combat_summary) ──
    let combat = character.get_combat_summary(game_data, decoder);
    println!("\n--- COMBAT (get_combat_summary) ---");
    println!("  AC Total:      {}", combat.armor_class.total);
    println!("  AC Touch:      {}", combat.armor_class.touch);
    println!("  AC Flat-foot:  {}", combat.armor_class.flat_footed);
    let bd = &combat.armor_class.breakdown;
    println!(
        "  AC Breakdown:  base={} armor={} shield={} dex={} natural={} dodge={} deflection={} size={} misc={}",
        bd.base, bd.armor, bd.shield, bd.dex, bd.natural, bd.dodge, bd.deflection, bd.size, bd.misc
    );
    println!("  BAB:           {}", combat.bab);
    println!("  Attack Seq:    {:?}", combat.attack_sequence);
    println!(
        "  Melee Attack:  {} (BAB {} + ability {} + size {} + equip {} + misc {})",
        combat.attack_bonuses.melee,
        combat.attack_bonuses.melee_breakdown.base,
        combat.attack_bonuses.melee_breakdown.ability,
        combat.attack_bonuses.melee_breakdown.size,
        combat.attack_bonuses.melee_breakdown.equipment,
        combat.attack_bonuses.melee_breakdown.misc
    );
    println!(
        "  Ranged Attack: {} (BAB {} + ability {} + size {} + equip {} + misc {})",
        combat.attack_bonuses.ranged,
        combat.attack_bonuses.ranged_breakdown.base,
        combat.attack_bonuses.ranged_breakdown.ability,
        combat.attack_bonuses.ranged_breakdown.size,
        combat.attack_bonuses.ranged_breakdown.equipment,
        combat.attack_bonuses.ranged_breakdown.misc
    );
    println!(
        "  Initiative:    {} (dex {} + feat {} + misc {})",
        combat.initiative.total,
        combat.initiative.dex,
        combat.initiative.feat,
        combat.initiative.misc
    );
    println!(
        "  Movement:      base={} current={} armor_penalty={}",
        combat.movement.base, combat.movement.current, combat.movement.armor_penalty
    );
    for dr in &combat.damage_reductions {
        println!(
            "  DR:            {}/{}  ({})",
            dr.amount, dr.bypass, dr.source
        );
    }

    // ── Saving Throws (same as get_saving_throws) ──
    let saves = character.get_saving_throws(game_data, decoder);
    println!("\n--- SAVING THROWS (get_saving_throws) ---");
    for (name, s) in [
        ("Fort", &saves.fortitude),
        ("Ref ", &saves.reflex),
        ("Will", &saves.will),
    ] {
        println!(
            "  {}: {:>3} = base {} + ability {} + equip {} + feat {} + class {} + misc {}",
            name, s.total, s.base, s.ability, s.equipment, s.feat, s.class_bonus, s.misc
        );
    }

    // ── Skills (same as get_skill_summary) ──
    let skills = character.get_skill_summary(game_data, Some(decoder));
    println!("\n--- SKILLS (get_skill_summary) ---");
    for s in &skills {
        if s.ranks > 0 || s.total != 0 {
            println!(
                "  {:25} Total: {:>3}  (rank {} + ability_mod {} + item {} + feat {}) untrained={}",
                s.name, s.total, s.ranks, s.modifier, s.item_bonus, s.feat_bonus, s.untrained
            );
        }
    }

    // ── Feats (same as get_feats_state) ──
    let feats = character.get_feats_state(game_data);
    println!("\n--- FEATS (get_feats_state) ---");
    println!("  Total: {}", feats.summary.total);
    for f in &feats.summary.general_feats {
        println!("  [general {:>4}] {}", f.id.0, f.name);
    }
    for f in &feats.summary.class_feats {
        println!("  [class   {:>4}] {}", f.id.0, f.name);
    }
    for f in &feats.summary.custom_feats {
        println!("  [custom  {:>4}] {}", f.id.0, f.name);
    }

    // ── Equipment Bonuses (get_equipment_bonuses) ──
    let eq_bonuses = character.get_equipment_bonuses(game_data, decoder);
    println!("\n--- EQUIPMENT BONUSES ---");
    println!(
        "  STR: +{}, DEX: +{}, CON: +{}, INT: +{}, WIS: +{}, CHA: +{}",
        eq_bonuses.str_bonus,
        eq_bonuses.dex_bonus,
        eq_bonuses.con_bonus,
        eq_bonuses.int_bonus,
        eq_bonuses.wis_bonus,
        eq_bonuses.cha_bonus
    );
    println!(
        "  Fort: +{}, Ref: +{}, Will: +{}",
        eq_bonuses.fortitude_bonus, eq_bonuses.reflex_bonus, eq_bonuses.will_bonus
    );
    println!(
        "  AC armor: +{}, shield: +{}, natural: +{}, dodge: +{}, deflection: +{}",
        eq_bonuses.ac_armor_bonus,
        eq_bonuses.ac_shield_bonus,
        eq_bonuses.ac_natural_bonus,
        eq_bonuses.ac_dodge_bonus,
        eq_bonuses.ac_deflection_bonus
    );
    println!("  Attack: +{}", eq_bonuses.attack_bonus);

    // ── Equipped Items (raw GFF struct_ids) ──
    println!("\n--- EQUIPPED ITEMS (raw Equip_ItemList) ---");
    if let Some(equip_list) = character.get_list_owned("Equip_ItemList") {
        for item_struct in &equip_list {
            let struct_id = item_struct
                .get("__struct_id__")
                .map(|v| format!("{:?}", v))
                .unwrap_or_else(|| "MISSING".to_string());
            let tag = item_struct
                .get("Tag")
                .map(|v| format!("{:?}", v))
                .unwrap_or_else(|| "NO TAG".to_string());
            let base_item = item_struct
                .get("BaseItem")
                .map(|v| format!("{:?}", v))
                .unwrap_or_else(|| "NONE".to_string());
            println!(
                "  struct_id={:20} tag={:40} base={}",
                struct_id, tag, base_item
            );
        }
    }

    let eq_summary = character.get_full_inventory_summary(game_data, decoder, None);
    println!("\n--- EQUIPPED (get_full_inventory_summary) ---");
    println!("  Equipped count: {}", eq_summary.equipped.len());
    for item in &eq_summary.equipped {
        println!(
            "  slot={:15} name={:30} base_item={}",
            item.slot, item.name, item.base_item
        );
    }

    // ── Inventory ──
    println!("\n--- INVENTORY ---");
    println!("  Gold: {}", character.gold());
    let inventory = character.inventory_items();
    for (idx, item) in inventory.iter().enumerate() {
        println!("  [{}] {} (stack: {})", idx, item.tag, item.stack_size);
    }

    println!("\n================================================================");
    println!("  END OF EXTRACTION");
    println!("================================================================");
}
