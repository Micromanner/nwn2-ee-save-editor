use crate::common::create_test_context;
use app_lib::character::Character;
use app_lib::character::{ArmorAccessories, ItemAppearance, TintChannels};
use app_lib::loaders::GameData;
use app_lib::parsers::gff::GffValue;
use app_lib::utils::parsing::row_str;
use indexmap::IndexMap;

/// Companion creature rows (construct = `Golem_Blade`, modeltype L, no head;
/// Zhjaeve = `NPC_Zhjaeve`, modeltype F) ship `{body}_CL_Body01` meshes like
/// player bodies, not a bare `{body}` mesh, and may have no head at all.
#[tokio::test]
async fn creature_appearances_resolve_body_and_optional_head() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().unwrap();
    let rm = ctx.resource_manager.read().await;
    let appearance_table = game_data.get_table("appearance").unwrap();
    let find_row = |label: &str| {
        (0..appearance_table.row_count() as i32)
            .find(|id| {
                appearance_table.get_by_id(*id).is_some_and(|row| {
                    row_str(&row, "label").is_some_and(|s| s.eq_ignore_ascii_case(label))
                })
            })
            .unwrap()
    };
    let make_character = |appearance: i32, gender: u8| {
        Character::from_gff(IndexMap::from([
            ("Appearance_Type".into(), GffValue::Word(appearance as u16)),
            ("Gender".into(), GffValue::Byte(gender)),
            ("Appearance_Head".into(), GffValue::Byte(1)),
            ("Equip_ItemList".into(), GffValue::ListOwned(vec![])),
        ]))
    };

    let parts = make_character(find_row("Golem_Blade"), 4)
        .resolve_model_parts(game_data, &rm)
        .unwrap();
    assert!(parts.head_resref.is_none());
    assert!(parts.body_parts.iter().any(|b| rm.has_resource(b, "mdb")));
    assert!(rm.has_resource(&parts.naked_body_resref, "mdb"));

    let parts = make_character(find_row("NPC_Zhjaeve"), 1)
        .resolve_model_parts(game_data, &rm)
        .unwrap();
    assert!(rm.has_resource(parts.head_resref.as_deref().unwrap(), "mdb"));
    assert!(parts.body_parts.iter().any(|b| rm.has_resource(b, "mdb")));
    assert!(rm.has_resource(&parts.naked_body_resref, "mdb"));
}

#[tokio::test]
async fn equipped_part_precedence_and_nested_tints_match_item_preview() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().unwrap();
    let rm = ctx.resource_manager.read().await;
    let appearance_table = game_data.get_table("appearance").unwrap();
    let human = (0..appearance_table.row_count() as i32)
        .find(|id| {
            appearance_table.get_by_id(*id).is_some_and(|row| {
                row_str(&row, "nwn2_model_body").is_some_and(|s| s.eq_ignore_ascii_case("P_HH?"))
            })
        })
        .unwrap();
    let make_character = |items| {
        Character::from_gff(IndexMap::from([
            ("Appearance_Type".into(), GffValue::Word(human as u16)),
            ("Gender".into(), GffValue::Byte(0)),
            ("Appearance_Head".into(), GffValue::Byte(1)),
            ("Equip_ItemList".into(), GffValue::ListOwned(items)),
        ]))
    };
    let part = |avt, variation| {
        IndexMap::from([
            ("ArmorVisualType".into(), GffValue::Byte(avt)),
            ("Variation".into(), GffValue::Byte(variation)),
        ])
    };
    let mut nested_boots = part(0, 0);
    let tint = IndexMap::from([
        ("r".into(), GffValue::Byte(31)),
        ("g".into(), GffValue::Byte(97)),
        ("b".into(), GffValue::Byte(173)),
        ("a".into(), GffValue::Byte(255)),
    ]);
    nested_boots.insert(
        "ArmorTint".into(),
        GffValue::StructOwned(Box::new(IndexMap::from([
            ("1".into(), GffValue::StructOwned(Box::new(tint.clone()))),
            ("2".into(), GffValue::StructOwned(Box::new(tint.clone()))),
            ("3".into(), GffValue::StructOwned(Box::new(tint))),
        ]))),
    );
    let mut chest = part(8, 0);
    chest.insert(
        "BaseItem".into(),
        GffValue::Int(find_base_item_by_label(game_data, "Armor")),
    );
    chest.insert(
        "Boots".into(),
        GffValue::StructOwned(Box::new(nested_boots)),
    );
    let mut boots = part(2, 0);
    boots.insert(
        "BaseItem".into(),
        GffValue::Int(find_base_item_by_label(game_data, "Boots")),
    );

    for items in [
        vec![chest.clone(), boots.clone()],
        vec![boots, chest.clone()],
    ] {
        let character = make_character(items);
        let parts = character.resolve_model_parts(game_data, &rm).unwrap();
        assert_eq!(parts.boots_candidates[0], "P_HHM_LE_Boots01");
        assert!(parts.has_standalone_boots);
        assert!(parts.boots_candidates.iter().all(|s| s.contains("_LE_")));
    }

    let character = make_character(vec![chest.clone()]);
    let parts = character.resolve_model_parts(game_data, &rm).unwrap();
    assert_eq!(parts.boots_candidates[0], "P_HHM_CL_Boots01");
    assert!(!parts.has_standalone_boots);
    assert_eq!(parts.boots_tint.as_ref().unwrap().channel1.r, 31);
    let appearance = ItemAppearance::from_gff(&chest);
    assert_eq!(appearance.boots.as_ref().unwrap().variation, 0);
    assert_eq!(
        appearance
            .boots
            .as_ref()
            .unwrap()
            .tints
            .as_ref()
            .unwrap()
            .channel1
            .b,
        173
    );
    let model = app_lib::commands::item_appearance::load_item_model_impl(
        find_base_item_by_label(game_data, "Armor"),
        &appearance,
        Some("P_HHM"),
        game_data,
        &rm,
    )
    .unwrap();
    let boot_mesh = model
        .meshes
        .iter()
        .find(|m| m.name.to_lowercase().contains("_boots"))
        .unwrap();
    assert_eq!(boot_mesh.override_tints.as_ref().unwrap().channel1.g, 97);

    let mut bracer = part(2, 4);
    bracer.insert(
        "BaseItem".into(),
        GffValue::Int(find_base_item_by_label(game_data, "Bracer")),
    );
    let parts = make_character(vec![bracer])
        .resolve_model_parts(game_data, &rm)
        .unwrap();
    assert_eq!(parts.gloves_candidates[0], "P_HHM_LE_Gloves01");
}

fn appearance_with_variation(variation: i32) -> ItemAppearance {
    ItemAppearance {
        variation,
        model_parts: [0, 0, 0],
        tints: TintChannels::default(),
        armor_visual_type: None,
        boots: None,
        gloves: None,
        accessories: ArmorAccessories::default(),
    }
}

#[tokio::test]
async fn accessory_palettes_cancel_the_viewer_swap_for_every_slot() {
    use app_lib::character::{AccessorySlot, AccessorySlotData, TintChannel};

    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().unwrap();
    let rm = ctx.resource_manager.read().await;
    let tints = TintChannels {
        channel1: TintChannel {
            r: 74,
            g: 97,
            b: 61,
            a: 255,
        },
        channel2: TintChannel {
            r: 100,
            g: 135,
            b: 80,
            a: 255,
        },
        channel3: TintChannel {
            r: 71,
            g: 50,
            b: 50,
            a: 255,
        },
    };
    let mut appearance = appearance_with_variation(0);
    appearance.armor_visual_type = Some(2);
    for (slot, id) in [
        (AccessorySlot::LtShoulder, 26),
        (AccessorySlot::RtShoulder, 7),
        (AccessorySlot::LtArm, 8),
        (AccessorySlot::RtArm, 5),
        (AccessorySlot::LtBracer, 5),
        (AccessorySlot::RtBracer, 6),
    ] {
        appearance.accessories.set_data(
            slot,
            Some(AccessorySlotData {
                id,
                tints: tints.clone(),
            }),
        );
    }
    let model = app_lib::commands::item_appearance::load_item_model_impl(
        find_base_item_by_label(game_data, "Armor"),
        &appearance,
        Some("P_HHM"),
        game_data,
        &rm,
    )
    .unwrap();
    for accessory in appearance.accessories.iter_renderable("P_HHM") {
        let meshes: Vec<_> = model
            .meshes
            .iter()
            .filter(|mesh| {
                mesh.name
                    .to_ascii_lowercase()
                    .starts_with(&accessory.resref.to_ascii_lowercase())
            })
            .collect();
        assert!(!meshes.is_empty(), "missing {}", accessory.resref);
        for mesh in meshes {
            let tint = mesh
                .override_tints
                .as_ref()
                .expect("independent accessory palette");
            assert_eq!(
                (tint.channel1.r, tint.channel1.g, tint.channel1.b),
                (74, 97, 61)
            );
            assert_eq!(
                (tint.channel2.r, tint.channel2.g, tint.channel2.b),
                (71, 50, 50)
            );
            assert_eq!(
                (tint.channel3.r, tint.channel3.g, tint.channel3.b),
                (100, 135, 80)
            );
        }
    }
}

/// baseitems.2da row with the given `label` (e.g. `Boots`).
fn find_base_item_by_label(game_data: &GameData, label: &str) -> i32 {
    let table = game_data.get_table("baseitems").expect("baseitems.2da");
    for id in 0..table.row_count() as i32 {
        if let Some(row) = table.get_by_id(id)
            && row_str(&row, "label")
                .unwrap_or_default()
                .eq_ignore_ascii_case(label)
        {
            return id;
        }
    }
    panic!("no baseitems row labelled {label}");
}

/// First baseitems.2da row whose `modeltype` matches `want`.
fn find_base_item_with_modeltype(game_data: &GameData, want: &str) -> i32 {
    let table = game_data.get_table("baseitems").expect("baseitems.2da");
    for id in 0..table.row_count() as i32 {
        if let Some(row) = table.get_by_id(id)
            && row_str(&row, "modeltype").unwrap_or_default().trim() == want
        {
            return id;
        }
    }
    panic!("no baseitems row with modeltype {want}");
}

#[tokio::test]
async fn prepare_for_write_routes_single_part_variation_to_model_part1() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().unwrap();
    let base_item_id = find_base_item_with_modeltype(game_data, "0");

    let mut appearance = appearance_with_variation(3);
    let ui_only = appearance.prepare_for_write(base_item_id, game_data);

    assert!(
        ui_only,
        "SinglePart items store the variant in ModelPart1, Variation is UI-only"
    );
    assert_eq!(appearance.model_parts[0], 3);
}

#[tokio::test]
async fn options_report_no_model_for_rings_and_a_model_for_shields() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().unwrap();
    let rm = ctx.resource_manager.read().await;

    // Ring (row 52): modeltype=0 like shields, but no itemclass and no MDB
    // family on disk — the UI must not offer tints or a variant stepper.
    let ring = ItemAppearance::get_options(52, None, None, game_data, &rm);
    assert!(!ring.has_model, "rings have no in-world model");
    assert!(ring.available_variations.is_empty());

    // Small shield (row 14): same modeltype, but real w_she_small_* meshes.
    let shield = ItemAppearance::get_options(14, None, None, game_data, &rm);
    assert!(shield.has_model, "shields render an in-world model");
    assert!(
        !shield.available_variations.is_empty(),
        "shield variants should be discovered on disk"
    );
}

#[tokio::test]
async fn options_list_only_materials_with_meshes_for_the_slot() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().unwrap();
    let rm = ctx.resource_manager.read().await;

    let boots = find_base_item_by_label(game_data, "Boots");
    let opts = ItemAppearance::get_options(boots, Some(2), None, game_data, &rm);
    assert!(
        opts.available_materials
            .iter()
            .any(|m| m.id == 2 && m.name == "Leather"),
        "boots should offer Leather (armor.2da row 2): {:?}",
        opts.available_materials
    );
    for material in &opts.available_materials {
        let per_material =
            ItemAppearance::get_options(boots, Some(material.id), None, game_data, &rm);
        assert!(
            !per_material.available_variations.is_empty(),
            "material {} listed for boots but has no variants",
            material.name
        );
    }

    let cloak = find_base_item_by_label(game_data, "Cloak");
    let cloak_opts = ItemAppearance::get_options(cloak, None, None, game_data, &rm);
    assert!(
        cloak_opts.available_materials.is_empty(),
        "cloaks are hardcoded to CL and must not offer a material"
    );

    let shield = ItemAppearance::get_options(14, None, None, game_data, &rm);
    assert!(shield.available_materials.is_empty());
}

#[tokio::test]
async fn prepare_for_write_keeps_variation_field_for_weapons_and_armor() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().unwrap();

    for modeltype in ["2", "3"] {
        let base_item_id = find_base_item_with_modeltype(game_data, modeltype);
        let mut appearance = appearance_with_variation(3);
        let ui_only = appearance.prepare_for_write(base_item_id, game_data);
        assert!(
            !ui_only,
            "modeltype {modeltype} items keep their Variation field"
        );
        assert_eq!(appearance.model_parts[0], 0);
    }
}

#[tokio::test]
async fn options_list_accessory_meshes_only_for_chest_armor() {
    use app_lib::character::AccessorySlot;

    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().unwrap();
    let rm = ctx.resource_manager.read().await;

    let armor = find_base_item_by_label(game_data, "Armor");
    let opts = ItemAppearance::get_options(armor, Some(2), None, game_data, &rm);
    let shoulder = opts
        .available_accessories
        .iter()
        .find(|o| o.slot == AccessorySlot::LtShoulder)
        .expect("chest armor offers left shoulder accessories");
    assert!(
        shoulder.ids.iter().all(|id| *id > 0),
        "accessory ids are the 1-based mesh numbers: {:?}",
        shoulder.ids
    );
    assert!(
        rm.has_resource(&format!("A_HHM_LShoulder{}", shoulder.ids[0]), "mdb"),
        "listed id must resolve to the mesh the loader will render (files are not zero-padded)"
    );
    for slot in [
        AccessorySlot::LtAnkle,
        AccessorySlot::RtFoot,
        AccessorySlot::BkHip,
        AccessorySlot::FtHip,
    ] {
        assert!(
            !opts.available_accessories.iter().any(|o| o.slot == slot),
            "vestigial slot {slot:?} has no meshes and must not be offered"
        );
    }

    let boots = find_base_item_by_label(game_data, "Boots");
    let boots_opts = ItemAppearance::get_options(boots, Some(2), None, game_data, &rm);
    assert!(boots_opts.available_accessories.is_empty());
}
