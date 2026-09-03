use crate::common::create_test_context;
use app_lib::character::{ArmorAccessories, ItemAppearance, TintChannels};
use app_lib::loaders::GameData;
use app_lib::utils::parsing::row_str;

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
