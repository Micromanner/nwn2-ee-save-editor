use app_lib::parsers::gff::GffParser;
use app_lib::services::item_property_decoder::{ItemBonuses, ItemPropertyDecoder};
use serde_json::Value;
use std::collections::HashMap;

#[path = "../common/mod.rs"]
mod common;

/// Decoder wired up against the real game 2DAs/TLK, as the app does at runtime.
async fn init_decoder(context: &common::TestContext) -> ItemPropertyDecoder {
    let rm = context.resource_manager.read().await;
    ItemPropertyDecoder::load(&rm)
}

fn property(property_id: u32, subtype: u32, cost_value: u32) -> HashMap<String, Value> {
    HashMap::from([
        ("PropertyName".to_string(), Value::from(property_id)),
        ("Subtype".to_string(), Value::from(subtype)),
        ("CostValue".to_string(), Value::from(cost_value)),
    ])
}

#[tokio::test]
async fn test_item_property_decoding_from_fixture() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;

    // Load a character fixture
    let data = common::load_test_gff("ryathstrongarm/ryathstrongarm4.bic");
    let parser = GffParser::from_bytes(data).expect("Failed to parse GFF");

    let root_struct = parser
        .read_struct_fields(0)
        .expect("Failed to read root fields");

    let mut item_with_props = None;

    if let Some(app_lib::parsers::gff::GffValue::List(items)) = root_struct.get("ItemList") {
        for lazy_item in items {
            let item_struct_fields = lazy_item.force_load();
            if let Some(app_lib::parsers::gff::GffValue::List(props)) =
                item_struct_fields.get("PropertiesList")
                && !props.is_empty()
            {
                item_with_props = Some(props.clone());
                break;
            }
        }
    }

    if item_with_props.is_none() {
        // Try Equip_ItemList
        if let Some(app_lib::parsers::gff::GffValue::List(items)) =
            root_struct.get("Equip_ItemList")
        {
            for lazy_item in items {
                let item_struct_fields = lazy_item.force_load();
                if let Some(app_lib::parsers::gff::GffValue::List(props)) =
                    item_struct_fields.get("PropertiesList")
                    && !props.is_empty()
                {
                    item_with_props = Some(props.clone());
                    break;
                }
            }
        }
    }

    let properties_list =
        item_with_props.expect("Could not find any item with properties in fixture");

    // Convert first property to JSON-like map for decoder
    // Property structure: PropertyName (word), Subtype (word), CostValue (word), Param1Value (word) usually.
    // GffParser values are explicitly typed.

    for lazy_prop in properties_list {
        let prop_fields = lazy_prop.force_load();
        let mut prop_map = HashMap::new();

        if let Some(app_lib::parsers::gff::GffValue::Word(v)) = prop_fields.get("PropertyName") {
            prop_map.insert("PropertyName".to_string(), Value::from(*v));
        }
        if let Some(app_lib::parsers::gff::GffValue::Word(v)) = prop_fields.get("Subtype") {
            prop_map.insert("Subtype".to_string(), Value::from(*v));
        }
        if let Some(app_lib::parsers::gff::GffValue::Word(v)) = prop_fields.get("CostValue") {
            prop_map.insert("CostValue".to_string(), Value::from(*v));
        }
        if let Some(app_lib::parsers::gff::GffValue::Word(v)) = prop_fields.get("Param1Value") {
            prop_map.insert("Param1Value".to_string(), Value::from(*v));
        } else if let Some(app_lib::parsers::gff::GffValue::Byte(v)) =
            prop_fields.get("Param1Value")
        {
            prop_map.insert("Param1Value".to_string(), Value::from(*v));
        }

        let decoded = decoder.decode_property(&prop_map);
        if let Some(d) = decoded {
            println!("Decoded Property: {d:?}");
            assert!(!d.is_empty());
        }
    }
}

fn bonuses_of(decoder: &ItemPropertyDecoder, properties: &[HashMap<String, Value>]) -> ItemBonuses {
    let mut bonuses = ItemBonuses::default();
    decoder.add_item_bonuses(&mut bonuses, properties, 80);
    bonuses
}

fn describe(
    decoder: &ItemPropertyDecoder,
    property_id: u32,
    subtype: u32,
    cost_value: u32,
) -> String {
    decoder
        .decode_property(&property(property_id, subtype, cost_value))
        .unwrap_or_else(|| panic!("property {property_id} decodes"))
}

/// `iprp_damagecost.2da` row 6 is 1d4 and row 10 is 2d6. A hardcoded map used to
/// claim row 1 was 1d4, so every damage bonus displayed the wrong dice.
#[tokio::test]
async fn damage_bonus_dice_come_from_iprp_damagecost() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;

    assert_eq!(
        describe(&decoder, 16, 0, 6),
        "Damage Bonus: Bludgeoning 1d4"
    );
    assert_eq!(
        describe(&decoder, 16, 0, 10),
        "Damage Bonus: Bludgeoning 2d6"
    );
    // Rows 1-5 are flat bonuses, not dice.
    assert_eq!(describe(&decoder, 16, 0, 1), "Damage Bonus: Bludgeoning 1");
}

/// `iprp_damvulcost.2da` row 1 is 5%, not the 25% a hardcoded map assumed.
#[tokio::test]
async fn damage_vulnerability_percent_comes_from_iprp_damvulcost() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;

    assert!(describe(&decoder, 24, 0, 1).ends_with(" 5% Damage Weakness"));
    assert!(describe(&decoder, 24, 0, 5).ends_with(" 75% Damage Weakness"));
}

/// AC Bonus has no subtype in `itempropdef.2da`; the engine derives the AC type
/// from the item. We used to index `iprp_acmodtype` with the always-0 subtype and
/// label every AC bonus "AC Dodge Modifier".
#[tokio::test]
async fn ac_bonus_is_not_labelled_with_an_ac_modifier_type() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;

    assert_eq!(describe(&decoder, 1, 0, 3), "AC Bonus: + 3");
}

/// `iprp_srcost.2da` row 3 is SR 16; the row index used to be shown and applied.
#[tokio::test]
async fn spell_resistance_uses_the_cost_table_value() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;

    assert_eq!(describe(&decoder, 39, 0, 3), "Spell Resistance: 16");
    let bonuses = bonuses_of(&decoder, &[property(39, 0, 3)]);
    assert_eq!(bonuses.spell_resistance, 16);
}

/// Bonus Feat's subtype is an `iprp_feats` row (row 2 = Cleave, feat 6), not a
/// `feat.2da` id.
#[tokio::test]
async fn bonus_feat_reads_the_iprp_feats_row() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;

    assert_eq!(describe(&decoder, 12, 2, 0), "Bonus Feat: Cleave");
}

/// Damage Reduction (row 90) has no cost table; its amount is the subtype.
#[tokio::test]
async fn damage_reduction_reads_its_subtype() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;

    assert_eq!(describe(&decoder, 90, 5, 0), "Damage Reduction: +5");
}

/// Bonuses and penalties come from the cost table `Value` column, matched on
/// property and subtype ids, so they count in every UI language.
#[tokio::test]
async fn item_bonuses_apply_bonuses_and_penalties() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;

    let bonuses = bonuses_of(
        &decoder,
        &[
            property(0, 0, 2),  // Strength +2
            property(27, 1, 3), // Decreased Dexterity -3
            property(41, 1, 2), // Fortitude +2
            property(50, 2, 3), // Reduced Will -3
            property(49, 0, 1), // Reduced saves vs Universal -1
            property(60, 0, 2), // Attack Penalty -2
            property(28, 4, 1), // Decreased AC (Deflection) -1
            property(29, 0, 5), // Decreased skill row 0 -5
        ],
    );

    assert_eq!(bonuses.str_bonus, 2);
    assert_eq!(bonuses.dex_bonus, -3);
    assert_eq!(bonuses.fortitude_bonus, 1);
    assert_eq!(bonuses.will_bonus, -4);
    assert_eq!(bonuses.reflex_bonus, -1);
    assert_eq!(bonuses.attack_bonus, -2);
    assert_eq!(bonuses.ac_deflection_bonus, -1);
    assert_eq!(bonuses.skill_bonuses.get(&0), Some(&-5));
}

/// Property names come from the `Name` strref, which is what the game shows.
#[tokio::test]
async fn property_labels_use_the_game_names() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;

    let label = |id: u32| {
        decoder
            .editor_metadata()
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("property {id} in metadata"))
            .label
            .clone()
    };

    assert_eq!(label(1), "AC Bonus");
    assert_eq!(label(3), "AC Bonus vs. Damage Type");
    assert_eq!(label(52), "Skill Bonus");
    // Row 85's Name strref points at "Turn Resistance" in vanilla data, so this
    // one has to fall back to its label column.
    assert_eq!(label(85), "Arrow Catching");
}

/// Row 0 of every cost table is "Random" with no Name strref. The engine cannot
/// resolve a description for it, so an item carrying it reads "Bad StrRef".
#[tokio::test]
async fn cost_table_options_never_offer_random() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;

    let metadata = decoder.editor_metadata();
    assert!(!metadata.is_empty(), "editor metadata should not be empty");

    for meta in metadata {
        for (row, label) in &meta.cost_table_options {
            assert!(
                !label.to_lowercase().contains("random"),
                "property {} offers a Random cost value at row {row}",
                meta.id
            );
        }
    }

    let options = |id: u32| {
        metadata
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("property {id} in metadata"))
            .cost_table_options
            .clone()
    };

    // Tables whose row 0 is the strref-less "Random" placeholder.
    for id in [1, 16, 52] {
        let opts = options(id);
        assert!(!opts.is_empty(), "property {id} should still offer values");
        assert!(
            !opts.contains_key(&0),
            "property {id} still offers cost table row 0"
        );
    }

    // ...but a row 0 that does carry a strref is a real value and must survive
    // (iprp_arcspell row 0 = -50% arcane spell failure).
    assert!(
        options(84).contains_key(&0),
        "arcane spell failure lost its first real value"
    );
}

/// The editor writes `CostTable`/`Param1` from this metadata; when it was missing
/// the frontend fell back to 0/255 and stamped every property with IPRP_BASE1.
#[tokio::test]
async fn metadata_exposes_the_cost_table_index() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;

    let metadata = decoder.editor_metadata();
    let idx = |id: u32| {
        metadata
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("property {id} in metadata"))
            .cost_table_idx
    };

    assert_eq!(idx(1), Some(2)); // AC Bonus -> IPRP_BONUSCOST
    assert_eq!(idx(16), Some(4)); // Damage Bonus -> IPRP_DAMAGECOST
    assert_eq!(idx(52), Some(25)); // Skill Bonus -> IPRP_SKILLCOST
}

#[tokio::test]
async fn ac_bonus_family_reads_like_the_game_and_carries_the_item_ac_type() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;

    // AC Bonus vs. Damage Type, subtype Slashing (iprp_combatdam row 2), +3, on a cloak.
    let on_cloak = decoder.decode_item_properties(&[property(3, 2, 3)], 80);
    assert_eq!(on_cloak[0], "Armor Bonus vs. Slashing + 3 (Deflection)");

    // AC Bonus vs. Alignment Group, subtype Evil (iprp_aligngrp row 5), +2, on armor.
    let on_armor = decoder.decode_item_properties(&[property(2, 5, 2)], 16);
    assert_eq!(on_armor[0], "Armor Bonus vs. Evil + 2 (Armor)");

    // AC Bonus vs. Specific Alignment, Lawful Good (iprp_alignment row 0), +1, on a shield.
    let on_shield = decoder.decode_item_properties(&[property(5, 0, 1)], 56);
    assert_eq!(on_shield[0], "Armor Bonus vs. Lawful Good + 1 (Shield)");

    // Plain AC Bonus keeps its own text, only gaining the AC type.
    let plain = decoder.decode_item_properties(&[property(1, 0, 3)], 80);
    assert_eq!(plain[0], "AC Bonus: + 3 (Deflection)");
}

#[tokio::test]
async fn cast_spell_reads_like_the_game() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;

    // Acid Fog (iprp_spells row 0, caster level 11), 4 Uses/Day (iprp_chargecost row 11).
    assert_eq!(
        describe(&decoder, 15, 0, 11),
        "Use: Acid Fog (11) 4 Uses/Day"
    );
}

/// `itemprops.2da` decides which properties a base item honours: a cloak
/// (PropColumn 16) takes AC Bonus but neither Keen nor Attack Bonus.
#[tokio::test]
async fn metadata_carries_the_itemprops_allow_list() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;
    let metadata = decoder.editor_metadata();
    let allowed = |id: u32| {
        metadata
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("property {id} in metadata"))
            .allowed_columns
            .clone()
            .expect("itemprops.2da should load in the test fixture")
    };

    let cloak_column = 16;
    assert!(allowed(1).contains(&cloak_column), "AC Bonus on cloak");
    assert!(!allowed(43).contains(&cloak_column), "Keen on cloak");
    assert!(
        !allowed(56).contains(&cloak_column),
        "Attack Bonus on cloak"
    );
    assert!(allowed(43).contains(&0), "Keen on melee weapons");
}
