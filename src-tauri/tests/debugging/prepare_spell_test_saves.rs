//! Finish the spells/saves in-game test saves built by `make_test_save` (slots 962-971):
//! equip a +2 INT amulet on 962, add Dark Blessing to 965, then print the editor's slots
//! and saves for every slot to compare with the engine build.
//! Set SKIP_EDITS=1 to only print the values.
//! Run: `cargo test --features integration-tests --test debugging prepare_spell_test_saves -- --ignored --nocapture`

use std::collections::HashMap;
use std::path::PathBuf;

use app_lib::character::{EquipmentSlot, FeatId};
use app_lib::config::NWN2Paths;
use app_lib::services::load_diagnostics::{LoadInput, LoadReport};
use app_lib::state::session_state::SessionState;
use serde_json::json;

#[path = "../common/mod.rs"]
mod common;
use common::create_test_context;

const BASE_ITEM_AMULET: i32 = 19;
const FEAT_DARK_BLESSING: FeatId = FeatId(473);

fn slot_dir(saves: &PathBuf, slot: u32) -> PathBuf {
    let prefix = format!("{slot:06} - ");
    std::fs::read_dir(saves)
        .expect("saves dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&prefix))
        })
        .unwrap_or_else(|| panic!("slot {slot} not found"))
}

fn load(saves: &PathBuf, slot: u32) -> SessionState {
    let mut session = SessionState::new();
    let path = slot_dir(saves, slot);
    let mut report = LoadReport::new(LoadInput {
        file_path: path.to_string_lossy().to_string(),
        player_index: None,
        file_size: None,
    });
    session
        .load_character(&path.to_string_lossy(), None, true, &mut report)
        .expect("load save");
    session
}

#[tokio::test]
#[ignore = "writes into the user's save folder"]
async fn prepare_spell_test_saves() {
    let ctx = create_test_context().await;
    let game_data = ctx.loader.game_data().expect("Game data not loaded");
    let saves = NWN2Paths::new().saves().expect("saves folder");
    let decoder = app_lib::services::item_property_decoder::ItemPropertyDecoder::load(
        &*ctx.resource_manager.read().await,
    );

    if std::env::var("SKIP_EDITS").is_err() {
        let mut session = load(&saves, 962);
        {
            let character = session.character_mut().expect("character");
            let added = character
                .add_item(BASE_ITEM_AMULET, 1, None, None, game_data)
                .expect("add amulet");
            let index = added.inventory_index.expect("inventory index");
            let update: HashMap<String, serde_json::Value> = HashMap::from([(
                "PropertiesList".to_string(),
                json!([{
                    "PropertyName": 0,
                    "Subtype": 3,
                    "CostTable": 1,
                    "CostValue": 2,
                    "Param1": 255,
                    "Param1Value": 0,
                    "ChanceAppear": 100,
                    "Useable": true,
                    "UsesPerDay": 255
                }]),
            )]);
            character
                .update_inventory_item(index, &update, game_data)
                .expect("set properties");
            let equipped = character
                .equip_item(index, EquipmentSlot::Neck, game_data)
                .expect("equip amulet");
            println!("962 amulet slot: {:?}", equipped.slot);
        }
        session
            .save_character(game_data, &app_lib::config::NWN2Paths::new(), 0)
            .expect("save 962");

        let mut session = load(&saves, 965);
        session
            .character_mut()
            .expect("character")
            .add_feat(FEAT_DARK_BLESSING)
            .expect("add Dark Blessing");
        session
            .save_character(game_data, &app_lib::config::NWN2Paths::new(), 0)
            .expect("save 965");
    }

    for slot in 962..=971 {
        let session = load(&saves, slot);
        let character = session.character().expect("character");
        let decoder = &decoder;
        let spells = character.get_spells_state(game_data, decoder);
        let saves_now = character.get_saving_throws(game_data, decoder);
        let slots: Vec<String> = spells
            .spellcasting_classes
            .iter()
            .map(|c| {
                let summary = spells
                    .spell_summary
                    .caster_classes
                    .iter()
                    .find(|s| s.id == c.class_id);
                let per_level: Vec<i32> = (0..10)
                    .map(|l| {
                        summary
                            .and_then(|s| s.slots_by_level.get(&l).copied())
                            .unwrap_or(0)
                    })
                    .collect();
                format!(
                    "class {} CL {} {:?}",
                    c.class_id.0, c.caster_level, per_level
                )
            })
            .collect();
        println!(
            "{slot}: saves F{} R{} W{} (class bonus {}) | {}",
            saves_now.fortitude.total,
            saves_now.reflex.total,
            saves_now.will.total,
            saves_now.fortitude.class_bonus,
            slots.join(" ; ")
        );
        println!(
            "      INT total {} equipped {}",
            character.get_total_abilities(game_data, decoder).int,
            character.equipped_count()
        );
        if slot == 964 || slot == 971 || slot == 962 {
            println!(
                "      feats {:?}",
                character.feat_ids().iter().map(|f| f.0).collect::<Vec<_>>()
            );
            for (_, item) in character.equipped_items() {
                println!("      equipped {} base {}", item.tag, item.base_item);
            }
        }
        println!(
            "      feat save bonus F{} R{} W{}",
            saves_now.fortitude.feat, saves_now.reflex.feat, saves_now.will.feat
        );
    }
}
