use super::super::common::{create_test_context, fixtures_path, load_test_gff};
use app_lib::character::{Character, ClassId};
use app_lib::parsers::gff::GffParser;

#[tokio::test]
async fn test_wizard_progression() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    // Load Sage Melchior (Wizard 5 / Cleric 7 / Warlock 18)
    let path = fixtures_path().join("gff/sagemelchior/sagemelchior4.bic");
    let data = std::fs::read(&path).expect("Failed to read fixture");
    let parser = GffParser::from_bytes(data).expect("Failed to parse GFF");
    let gff = parser
        .read_struct_fields(0)
        .expect("Failed to read root struct");
    let character = Character::from_gff(gff).expect("character from gff");

    // Find Wizard Class (ID 10)
    let wizard_id = ClassId(10);
    assert!(character.has_class(wizard_id));

    // Verify Class Properties
    assert!(character.is_spellcaster(wizard_id, game_data));
    assert!(character.is_prepared_caster(wizard_id, game_data));
    assert!(!character.uses_all_spells_known(wizard_id, game_data));

    // Wizard 5 -> Caster Level 5
    let caster_level = character.get_caster_level(wizard_id, game_data);
    assert_eq!(caster_level, 5, "Wizard 5 should have Caster Level 5");

    // Check Spell Slots (Base)
    // Level 5 Wizard: L0=4, L1=3, L2=2, L3=1
    let slots = character.calculate_spell_slots(wizard_id, game_data);
    println!("Wizard 5 Base Slots: {slots:?}");
    assert!(slots[0] >= Some(4));
    assert!(slots[1] >= Some(2));
    assert!(slots[2] >= Some(1));
    assert!(slots[3] >= Some(1));
    assert_eq!(slots[4], None); // No L4 spells yet (needs L7)

    // Check Known Spells
    let known_l1 = character.known_spells(wizard_id, 1);
    println!("Wizard Known L1: {known_l1:?}");
    assert!(!known_l1.is_empty(), "Wizard should have known L1 spells");

    // Check Memorized Spells (if any)
    let memory_l1 = character.memorized_spells(wizard_id, 1);
    println!("Wizard Memorized L1: {memory_l1:?}");
    // Note: Fixture might not have memorized spells set if it's a fresh save or cleaned
}

#[tokio::test]
async fn test_cleric_domains() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    // Load Sage Melchior (Wizard 5 / Cleric 7 / Warlock 18)
    let path = fixtures_path().join("gff/sagemelchior/sagemelchior4.bic");
    let data = std::fs::read(&path).expect("Failed to read fixture");
    let parser = GffParser::from_bytes(data).expect("Failed to parse GFF");
    let gff = parser
        .read_struct_fields(0)
        .expect("Failed to read root struct");
    let character = Character::from_gff(gff).expect("character from gff");

    // Find Cleric Class (ID 2)
    let cleric_id = ClassId(2);
    assert!(character.has_class(cleric_id));

    // Verify Class Properties
    assert!(character.is_spellcaster(cleric_id, game_data));
    assert!(character.is_divine_caster(cleric_id, game_data));
    assert!(character.uses_all_spells_known(cleric_id, game_data));

    // Cleric 7 -> Caster Level 7
    let caster_level = character.get_caster_level(cleric_id, game_data);
    assert_eq!(caster_level, 7);

    // Domain Spells
    let domain_spells = character.get_domain_spells(cleric_id, game_data);
    println!("Cleric Domain Spells: {domain_spells:?}");
    // Cleric 7 should have domain spells for levels 1, 2, 3, 4
    assert!(domain_spells.contains_key(&1));
    assert!(domain_spells.contains_key(&2));
    assert!(domain_spells.contains_key(&3));
    assert!(domain_spells.contains_key(&4));

    // All Known Spells
    // Should return essentially all spells in spells.2da that are Cleric-capable
    let known_l1 = character.get_all_known_spells(cleric_id, 1, game_data);
    assert!(!known_l1.is_empty());
    assert!(known_l1.len() > 10, "Cleric should know many L1 spells");
}

#[tokio::test]
async fn test_sorcerer_spells() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    // Load Qara (Sorcerer 10 / Arcane Scholar 10 / Dragon Disciple 10)
    let path = fixtures_path().join("gff/qaraofblacklake/qaraofblacklake4.bic");
    let data = std::fs::read(&path).expect("Failed to read fixture");
    let parser = GffParser::from_bytes(data).expect("Failed to parse GFF");
    let gff = parser
        .read_struct_fields(0)
        .expect("Failed to read root struct");
    let character = Character::from_gff(gff).expect("character from gff");

    // Find Sorcerer Class (ID 9)
    let sorcerer_id = ClassId(9);
    assert!(character.has_class(sorcerer_id));

    // Verify Properties
    assert!(character.is_spellcaster(sorcerer_id, game_data));
    assert!(!character.is_prepared_caster(sorcerer_id, game_data)); // Spontaneous
    assert!(!character.uses_all_spells_known(sorcerer_id, game_data));

    // Caster Level: Sorcerer 10 + Arcane Scholar 10 (full progression); Dragon
    // Disciple adds none.
    let cl = character.get_caster_level(sorcerer_id, game_data);
    assert_eq!(cl, 20);

    // Check Spontaneous Casting Slots
    let slots = character.calculate_spell_slots(sorcerer_id, game_data);
    println!("Qara Sorcerer Slots: {slots:?}");
    assert!(slots[1] > Some(0));
    assert!(slots[9] > Some(0));

    // Known Spells
    let known_l5 = character.known_spells(sorcerer_id, 5);
    println!("Qara Known L5: {known_l5:?}");
    assert!(!known_l5.is_empty());
}

#[tokio::test]
async fn test_paladin_spells() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    // Load Occidio (Paladin 20 / Divine Champion 10)
    let path = fixtures_path().join("gff/occidiooctavon/occidiooctavon4.bic");
    let data = std::fs::read(&path).expect("Failed to read fixture");
    let parser = GffParser::from_bytes(data).expect("Failed to parse GFF");
    let gff = parser
        .read_struct_fields(0)
        .expect("Failed to read root struct");
    let character = Character::from_gff(gff).expect("character from gff");

    // Find Paladin Class (ID 6)
    let paladin_id = ClassId(6);
    assert!(character.has_class(paladin_id));

    // Properties
    assert!(character.is_spellcaster(paladin_id, game_data));
    assert!(character.is_prepared_caster(paladin_id, game_data)); // Paladins prepare
    let all_known = character.uses_all_spells_known(paladin_id, game_data);
    println!("Paladin uses all spells known: {all_known}");

    // Caster Level: Paladin 2DA lookup uses class level directly for slot table.
    let cl = character.get_caster_level(paladin_id, game_data);
    assert_eq!(
        cl, 20,
        "Paladin 20 should have Lookup Level 20 for spell slots"
    );

    // Slots
    let slots = character.calculate_spell_slots(paladin_id, game_data);
    println!("Paladin Slots: {slots:?}");
    assert!(slots[1] > Some(0));
    assert!(slots[3] > Some(0));
    assert_eq!(slots[5], None); // Pal only goes up to L4
}

#[tokio::test]
async fn test_wizard_slots_modification() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    // Load Sage Melchior (Wizard 5 / Cleric 7 / Warlock 18)
    // We use a real fixture to ensure valid GFF structure foundation
    let path = fixtures_path().join("gff/sagemelchior/sagemelchior4.bic");
    let data = std::fs::read(&path).expect("Failed to read fixture");
    let parser = GffParser::from_bytes(data).expect("Failed to parse GFF");
    let gff = parser
        .read_struct_fields(0)
        .expect("Failed to read root struct");
    let mut character = Character::from_gff(gff).expect("character from gff");

    let wizard_id = ClassId(10); // Wizard

    // Verify initial state (Wizard 5)
    let initial_level = character.class_level(wizard_id);
    assert_eq!(initial_level, 5, "Melchior should start as Wizard 5");

    let initial_slots = character.calculate_spell_slots(wizard_id, game_data);
    println!("Initial Wizard 5 Slots: {initial_slots:?}");
    assert_eq!(initial_slots[5], None, "Wizard 5 should not have L5 slots");

    // Modify to Wizard 10
    // This tests that our slot calculations respond dynamically to level changes in-memory
    character
        .set_class_level(wizard_id, 10)
        .expect("Failed to set Wizard level");

    let new_level = character.class_level(wizard_id);
    assert_eq!(new_level, 10);

    // Recalculate slots
    // Wizard 10 should have L5 slots (Base progression)
    // L0:4, L1:4, L2:4, L3:3, L4:3, L5:2
    let new_slots = character.calculate_spell_slots(wizard_id, game_data);
    println!("Modified Wizard 10 Slots: {new_slots:?}");

    assert!(
        new_slots[5] >= Some(2),
        "Wizard 10 should have at least 2 L5 slots"
    );
    assert_eq!(new_slots[6], None, "Wizard 10 should not have L6 slots");
}

#[tokio::test]
async fn test_aasimar_ability_spells() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    // Load player.bic (Aasimar - Planetouched race with subrace index 13)
    let data = load_test_gff("player.bic");
    let parser = GffParser::from_bytes(data).expect("Failed to parse GFF");
    let gff = parser
        .read_struct_fields(0)
        .expect("Failed to read root struct");
    let character = Character::from_gff(gff).expect("character from gff");

    // Verify this is an Aasimar
    let race_name = character.race_display_name(game_data);
    println!("Race: {race_name}");
    assert_eq!(race_name, "Aasimar", "Fixture should be an Aasimar");

    // Get ability spells - Aasimar should have racial spell-like abilities
    let ability_spells = character.get_ability_spells(game_data);
    println!(
        "Ability spells ({}): {:#?}",
        ability_spells.len(),
        ability_spells
    );
    assert!(
        !ability_spells.is_empty(),
        "Aasimar should have at least one racial spell-like ability"
    );

    // Aasimar racial spell: Light (spell 942, granted via feat 1086 FEAT_AASIMAR_LIGHT)
    // This tests the feat.2da -> spellid reverse lookup path
    assert!(
        ability_spells.iter().any(|s| s.spell_id == 942),
        "Aasimar should have Racial Spell (Light) - spell ID 942"
    );

    // Verify ability spell entries have required fields populated
    for spell in &ability_spells {
        assert!(!spell.name.is_empty(), "Ability spell should have a name");
        assert!(
            !spell.icon.is_empty(),
            "Ability spell '{}' should have an icon",
            spell.name
        );
        assert!(
            spell.innate_level >= 0,
            "Ability spell '{}' should have non-negative innate level",
            spell.name
        );
    }
}

#[tokio::test]
async fn test_ability_spells_via_spells_state() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    // Load player.bic (Aasimar)
    let data = load_test_gff("player.bic");
    let parser = GffParser::from_bytes(data).expect("Failed to parse GFF");
    let gff = parser
        .read_struct_fields(0)
        .expect("Failed to read root struct");
    let character = Character::from_gff(gff).expect("character from gff");

    // Verify ability spells come through the full SpellsState pipeline
    let spells_state = character.get_spells_state(game_data, &ctx.decoder);
    assert!(
        !spells_state.ability_spells.is_empty(),
        "SpellsState should include ability spells for Aasimar"
    );

    // Should match direct call
    let direct = character.get_ability_spells(game_data);
    assert_eq!(
        spells_state.ability_spells.len(),
        direct.len(),
        "SpellsState ability_spells should match direct get_ability_spells()"
    );
}

#[tokio::test]
async fn test_non_caster_has_no_class_ability_spells_overlap() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    // Load Ryath Strongarm (Human) - should have no racial spell-like abilities
    // but may have feat-granted abilities from class features
    let path = fixtures_path().join("gff/ryathstrongarm/ryathstrongarm4.bic");
    let data = std::fs::read(&path).expect("Failed to read fixture");
    let parser = GffParser::from_bytes(data).expect("Failed to parse GFF");
    let gff = parser
        .read_struct_fields(0)
        .expect("Failed to read root struct");
    let character = Character::from_gff(gff).expect("character from gff");

    let race_name = character.race_display_name(game_data);
    println!("Race: {race_name}");

    let ability_spells = character.get_ability_spells(game_data);
    println!(
        "Ability spells for {} ({}): {:?}",
        race_name,
        ability_spells.len(),
        ability_spells.iter().map(|s| &s.name).collect::<Vec<_>>()
    );

    // Human has no racial spell-like abilities, but may have class-granted ones
    // Just verify the function doesn't crash and returns valid data
    for spell in &ability_spells {
        assert!(!spell.name.is_empty());
    }
}

/// Level-up spell picks: Sorcerer tracks the SpellKnownTable per level and clears
/// as spells are learned; Warlock invocations are not offered by the editor and the
/// game stores them in one KnownList regardless of grade, so no picks are pending.
#[tokio::test]
async fn test_pending_spell_picks_sorcerer_and_warlock() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    let load = |name: &str| {
        let parser = GffParser::from_bytes(load_test_gff(name)).expect("Failed to parse GFF");
        Character::from_gff(parser.read_struct_fields(0).expect("root"))
            .expect("character from gff")
    };

    let mut qara = load("qaraofblacklake/qaraofblacklake1.bic");
    let sorcerer = ClassId(9);
    assert!(
        qara.get_spells_state(game_data, &ctx.decoder)
            .pending_spell_learning
            .is_empty()
    );
    qara.level_up(sorcerer, game_data).expect("level up");
    let pending = qara
        .get_spells_state(game_data, &ctx.decoder)
        .pending_spell_learning;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].by_level.get(&0), Some(&1));
    let slot = qara
        .get_spells_state(game_data, &ctx.decoder)
        .spellcasting_classes
        .iter()
        .find(|c| c.class_id == sorcerer)
        .map(|c| c.index as usize)
        .expect("sorcerer caster slot");
    qara.add_known_spell(slot, 0, app_lib::character::SpellId(424), game_data)
        .expect("learn cantrip");
    assert!(
        qara.get_spells_state(game_data, &ctx.decoder)
            .pending_spell_learning
            .is_empty()
    );

    let mut melchior = load("sagemelchior/sagemelchior1.bic");
    let warlock = ClassId(39);
    melchior.level_up(warlock, game_data).expect("level up");
    melchior.level_up(warlock, game_data).expect("level up");
    assert!(
        melchior
            .get_spells_state(game_data, &ctx.decoder)
            .pending_spell_learning
            .is_empty()
    );
}

/// The 2DA-derived learnable columns must cover exactly the spell-level columns the
/// loaded `spells.2da` actually has (vanilla carries no FavSoul/SpiritShm columns:
/// Favored Soul resolves to `cleric`, Spirit Shaman to `druid`), with Warlock
/// excluded (invocations live in `KnownList4` and the editor neither lists nor
/// counts them).
#[tokio::test]
async fn test_learnable_spell_columns_derived_from_2da() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    let columns = Character::learnable_spell_columns(game_data);
    let mut names: Vec<&str> = columns.keys().map(String::as_str).collect();
    names.sort_unstable();

    assert_eq!(
        names,
        vec!["bard", "cleric", "druid", "paladin", "ranger", "wiz_sorc"],
    );
    assert!(!columns.contains_key("warlock"));

    let ids = |col: &str| -> Vec<i32> {
        let mut v: Vec<i32> = columns[col].iter().map(|c| c.0).collect();
        v.sort_unstable();
        v
    };
    assert_eq!(ids("wiz_sorc"), vec![9, 10], "Sorcerer and Wizard share");
    assert_eq!(ids("cleric"), vec![2, 58], "Favored Soul maps to cleric");
    assert_eq!(ids("druid"), vec![3, 55], "Spirit Shaman maps to druid");
}

/// A `0` in a spell gain table unlocks the spell level with bonus slots only, `****`
/// means no access; the cleric table already carries the domain slot.
#[tokio::test]
async fn test_spell_gain_table_access_and_domain_slots() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    let load = |name: &str| {
        let parser = GffParser::from_bytes(load_test_gff(name)).expect("Failed to parse GFF");
        Character::from_gff(parser.read_struct_fields(0).expect("root"))
            .expect("character from gff")
    };

    let mut occidio = load("occidiooctavon/occidiooctavon4.bic");
    let paladin = ClassId(6);
    occidio
        .set_class_level(paladin, 4)
        .expect("Failed to set Paladin level");
    let slots = occidio.calculate_spell_slots(paladin, game_data);
    assert_eq!(slots[1], Some(0), "Paladin 4 unlocks level 1 spells");
    assert_eq!(slots[2], None);
    assert_eq!(occidio.max_spell_level(paladin, game_data), Some(1));

    let melchior = load("sagemelchior/sagemelchior4.bic");
    let cleric = ClassId(2);
    assert_eq!(melchior.get_caster_level(cleric, game_data), 7);
    let slots = melchior.calculate_spell_slots(cleric, game_data);
    let table = game_data
        .get_table("cls_spgn_cler")
        .expect("cleric gain table");
    for (level, slot) in slots.iter().enumerate() {
        let cell = table
            .get_cell(6, &format!("SpellLevel{level}"))
            .expect("cell")
            .and_then(|v| v.parse::<i32>().ok());
        assert_eq!(*slot, cell, "Cleric 7 spell level {level}");
    }
}

/// Classes that do not cast have no caster level, whatever their class level.
#[tokio::test]
async fn test_non_caster_has_no_caster_level() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;

    let parser = GffParser::from_bytes(load_test_gff("occidiooctavon/occidiooctavon4.bic"))
        .expect("Failed to parse GFF");
    let occidio = Character::from_gff(parser.read_struct_fields(0).expect("root"))
        .expect("character from gff");
    let divine_champion = ClassId(32);
    assert!(occidio.has_class(divine_champion));
    assert_eq!(occidio.get_caster_level(divine_champion, game_data), 0);
}
