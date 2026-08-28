use app_lib::parsers::gff::GffParser;
use app_lib::services::item_property_decoder::{EditorContext, ItemPropertyDecoder};
use serde_json::Value;
use std::collections::HashMap;

#[path = "../common/mod.rs"]
mod common;

/// Decoder wired up against the real game 2DAs/TLK, as the app does at runtime.
async fn init_decoder(context: &common::TestContext) -> ItemPropertyDecoder {
    let mut decoder = ItemPropertyDecoder::new(context.resource_manager.clone());
    let rm = context.resource_manager.read().await;
    decoder.initialize_with_rm(&rm);
    drop(rm);
    decoder
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
            assert!(!d.label.is_empty());
        }
    }
}

/// `iprp_damagecost.2da` row 6 is 1d4 and row 10 is 2d6. A hardcoded map used to
/// claim row 1 was 1d4, so every damage bonus displayed the wrong dice.
#[tokio::test]
async fn damage_bonus_dice_come_from_iprp_damagecost() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;

    let one_d_four = decoder
        .decode_property(&property(16, 0, 6))
        .expect("damage bonus decodes");
    assert_eq!(one_d_four.damage_dice.as_deref(), Some("1d4"));

    let two_d_six = decoder
        .decode_property(&property(16, 0, 10))
        .expect("damage bonus decodes");
    assert_eq!(two_d_six.damage_dice.as_deref(), Some("2d6"));

    // Rows 1-5 are flat bonuses, not dice.
    let flat = decoder
        .decode_property(&property(16, 0, 1))
        .expect("damage bonus decodes");
    assert_eq!(flat.damage_dice.as_deref(), Some("+1"));
}

/// `iprp_damvulcost.2da` row 1 is 5%, not the 25% a hardcoded map assumed.
#[tokio::test]
async fn damage_vulnerability_percent_comes_from_iprp_damvulcost() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;

    let decoded = decoder
        .decode_property(&property(24, 0, 1))
        .expect("vulnerability decodes");
    assert_eq!(decoded.vulnerability_percent, Some(5));

    let decoded = decoder
        .decode_property(&property(24, 0, 5))
        .expect("vulnerability decodes");
    assert_eq!(decoded.vulnerability_percent, Some(75));
}

/// AC Bonus has no subtype in `itempropdef.2da`; the engine derives the AC type
/// from the item. We used to index `iprp_acmodtype` with the always-0 subtype and
/// label every AC bonus "AC Dodge Modifier".
#[tokio::test]
async fn ac_bonus_is_not_labelled_with_an_ac_modifier_type() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;

    let decoded = decoder
        .decode_property(&property(1, 0, 3))
        .expect("AC bonus decodes");

    assert_eq!(decoded.label, "AC Bonus +3");
    assert_eq!(decoded.ac_type, None);
    assert_eq!(decoded.bonus_value, Some(3));
}

/// Property names come from the `Name` strref, which is what the game shows.
#[tokio::test]
async fn property_labels_use_the_game_names() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;

    let label = |id: u32| {
        decoder
            .get_property_definition(id)
            .unwrap_or_else(|| panic!("property {id} defined"))
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
    let rm = context.resource_manager.read().await;

    let metadata = decoder.get_editor_property_metadata_with_rm(&EditorContext::default(), &rm);
    assert!(!metadata.is_empty(), "editor metadata should not be empty");

    for meta in &metadata {
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
    let rm = context.resource_manager.read().await;

    let metadata = decoder.get_editor_property_metadata_with_rm(&EditorContext::default(), &rm);
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
    assert_eq!(
        on_cloak[0].description,
        "Armor Bonus vs. Slashing + 3 (Deflection)"
    );
    assert_eq!(on_cloak[0].ac_type.as_deref(), Some("Deflection"));

    // AC Bonus vs. Alignment Group, subtype Evil (iprp_aligngrp row 5), +2, on armor.
    let on_armor = decoder.decode_item_properties(&[property(2, 5, 2)], 16);
    assert_eq!(on_armor[0].description, "Armor Bonus vs. Evil + 2 (Armor)");

    // AC Bonus vs. Specific Alignment, Lawful Good (iprp_alignment row 0), +1, on a shield.
    let on_shield = decoder.decode_item_properties(&[property(5, 0, 1)], 56);
    assert_eq!(
        on_shield[0].description,
        "Armor Bonus vs. Lawful Good + 1 (Shield)"
    );

    // Plain AC Bonus keeps its own text, only gaining the AC type.
    let plain = decoder.decode_item_properties(&[property(1, 0, 3)], 80);
    assert_eq!(plain[0].description, "Armor Class Bonus: +3 (Deflection)");
}

#[tokio::test]
async fn cast_spell_reads_like_the_game() {
    let context = common::create_test_context().await;
    let mut decoder = init_decoder(&context).await;
    let spells = {
        let rm = context.resource_manager.read().await;
        app_lib::services::item_property_decoder::load_2da_options_from_rm(&rm, "iprp_spells")
    };
    decoder.set_lookup_tables(
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        spells,
        HashMap::new(),
    );

    // Acid Fog (iprp_spells row 0, caster level 11), 4 Uses/Day (iprp_chargecost row 11).
    let decoded = decoder
        .decode_property(&property(15, 0, 11))
        .expect("cast spell decodes");
    assert_eq!(decoded.description, "Use: Acid Fog (11) 4 Uses/Day");
}

/// `itemprops.2da` decides which properties a base item honours: a cloak
/// (PropColumn 16) takes AC Bonus but neither Keen nor Attack Bonus.
#[tokio::test]
async fn metadata_carries_the_itemprops_allow_list() {
    let context = common::create_test_context().await;
    let decoder = init_decoder(&context).await;
    let rm = context.resource_manager.read().await;
    let metadata = decoder.get_editor_property_metadata_with_rm(&EditorContext::default(), &rm);
    let allowed = |id: u32| {
        metadata
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("property {id} in metadata"))
            .allowed_columns
            .clone()
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
