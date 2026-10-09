//! Prepares the in-game slots for the quest editor review test list
//! (`docs/code-reviews/save_graph.md`, "TODO: tests").

use app_lib::character::Character;
use app_lib::config::NWN2Paths;
use app_lib::parsers::gff::{GffValue, gff_text};
use app_lib::state::session_state::SessionState;

#[path = "../common/mod.rs"]
mod common;

fn print_character(label: &str, c: &Character) {
    let a = c.alignment();
    println!(
        "  {label}: law_chaos {} good_evil {}",
        a.law_chaos, a.good_evil
    );
    for item in c.inventory_items() {
        println!("    inv  {} x{}", item.tag, item.stack_size);
    }
    for (slot, item) in c.equipped_items() {
        println!("    eq{slot:<2} {} x{}", item.tag, item.stack_size);
    }
}

fn print_save(dir: &std::path::Path) {
    let mut session = SessionState::new();
    common::load_save(&mut session, dir);
    println!("{}", dir.display());
    print_character("PC", session.character().expect("player"));
    for member in session.list_roster().expect("roster") {
        let c = session
            .companion_snapshot(&member.ros_name)
            .expect("companion");
        print_character(
            &format!("{} (party {})", member.ros_name, member.in_party),
            &c,
        );
    }
}

const OC_BASE: &str = "000087 - 08-10-2026-12-44";
const OC_SLOTS: &[&str] = &[
    "000920 - QT2 Cormick",
    "000921 - QT2 Align Preview",
    "000922 - QT2 Restage",
    "000923 - QT2 Destroy Item",
    "000924 - QT2 App Checks",
];
const FIXTURE_SLOTS: &[(&str, &str)] = &[
    ("STORM_Campaign", "000925 - QT2 SoZ Gift"),
    ("mid-playthrough-saves/MotB", "000926 - QT2 MotB Sleeper"),
];
const DESTROY_ITEM_PARTY: &[&str] = &["elanee", "neeshka"];

fn copy_slot(src: &std::path::Path, saves: &std::path::Path, slot: &str) -> std::path::PathBuf {
    let dst = saves.join(slot);
    assert!(!dst.exists(), "{slot} already exists");
    common::copy_dir_recursive(src, &dst).expect("copy");
    if dst.join("savename.txt").exists() {
        std::fs::write(dst.join("savename.txt"), slot.split(" - ").nth(1).unwrap()).unwrap();
    }
    dst
}

#[tokio::test]
#[ignore = "writes into the user's save folder"]
async fn prepare_quest_review_saves() {
    let ctx = common::create_test_context().await;
    let saves = NWN2Paths::new().saves().expect("saves");
    for slot in OC_SLOTS {
        copy_slot(&saves.join(OC_BASE), &saves, slot);
    }
    let destroy = saves.join(OC_SLOTS[3]);
    let mut session = SessionState::new();
    common::load_save(&mut session, &destroy);
    println!("party limit {:?}", session.party_limit().expect("limit"));
    let party: Vec<(String, bool)> = DESTROY_ITEM_PARTY
        .iter()
        .map(|name| (name.to_string(), true))
        .collect();
    session.stage_party_membership(&party).expect("party");
    session
        .save_character(&ctx.game_data, &NWN2Paths::new(), 0)
        .expect("save");
    print_save(&destroy);
    for (fixture, slot) in FIXTURE_SLOTS {
        let dst = copy_slot(
            &common::fixtures_path().join("saves").join(fixture),
            &saves,
            slot,
        );
        println!("{}", dst.display());
    }
}

const ORDER_SLOT: &str = "000927 - QT2 Destroy Order";
const ORDER_TAG: &str = "NW_IT_MPOTION001";
const ORDER_PARTY: &[&str] = &["elanee", "khelgar", "neeshka"];

fn is_order_item(item: &indexmap::IndexMap<String, GffValue<'static>>) -> bool {
    gff_text(item, "Tag").as_deref() == Some(ORDER_TAG)
}

/// `ga_destroy_item` faction order: the PC holds none of `ORDER_TAG`, each of
/// `ORDER_PARTY` exactly one, so each `rs ga_destroy_item(tag,1,1)` takes from the
/// first faction member still holding one.
#[tokio::test]
#[ignore = "writes into the user's save folder"]
async fn prepare_destroy_order_save() {
    let ctx = common::create_test_context().await;
    let saves = NWN2Paths::new().saves().expect("saves");
    let dst = copy_slot(&saves.join(OC_BASE), &saves, ORDER_SLOT);
    let mut session = SessionState::new();
    common::load_save(&mut session, &dst);

    let pc = session.character_mut().expect("player");
    let mut items = pc.get_list_owned("ItemList").expect("items");
    items.retain(|item| !is_order_item(item));
    pc.set_list("ItemList", items);

    let mut template = None;
    for name in ORDER_PARTY {
        session.load_companion(name).expect("companion");
        let c = session.character_mut().expect("companion");
        let mut items = c.get_list_owned("ItemList").expect("items");
        let had = items.iter().any(is_order_item);
        let mut kept = false;
        items.retain_mut(|item| {
            if !is_order_item(item) || kept {
                return !is_order_item(item);
            }
            kept = true;
            item.insert("StackSize".to_string(), GffValue::Word(1));
            template.get_or_insert_with(|| item.clone());
            true
        });
        c.set_list("ItemList", items);
        if !had {
            let item = template
                .clone()
                .expect("template from an earlier companion");
            c.add_item_from_struct(item, &ctx.game_data).expect("add");
        }
    }
    session.load_player().expect("player");
    let party: Vec<(String, bool)> = ORDER_PARTY
        .iter()
        .map(|name| (name.to_string(), true))
        .collect();
    session.stage_party_membership(&party).expect("party");
    session
        .save_character(&ctx.game_data, &NWN2Paths::new(), 0)
        .expect("save");
    print_save(&dst);
}

/// `QT_SAVE=<save dir>`: alignment and inventories of the PC and every roster member.
#[test]
#[ignore = "reads a save named by QT_SAVE"]
fn dump_quest_review_save() {
    let dir = std::env::var("QT_SAVE").expect("QT_SAVE");
    print_save(std::path::Path::new(&dir));
}

#[test]
#[ignore = "reads a save named by QT_SAVE"]
fn dump_order_items() {
    let dir = std::env::var("QT_SAVE").expect("QT_SAVE");
    let mut session = SessionState::new();
    common::load_save(&mut session, std::path::Path::new(&dir));
    for name in ORDER_PARTY {
        let c = session.companion_snapshot(name).expect("companion");
        for (i, item) in c
            .get_list_owned("ItemList")
            .expect("items")
            .iter()
            .enumerate()
        {
            if is_order_item(item) || i < 2 {
                let fields: Vec<String> = item
                    .iter()
                    .filter(|(_, v)| !matches!(v, GffValue::List(_) | GffValue::Struct(_)))
                    .map(|(k, v)| format!("{k}={v:?}"))
                    .collect();
                println!("{name}[{i}] {}", fields.join(" "));
            }
        }
    }
}

const LOCKED_SLOT: &str = "000929 - QT3 Neeshka Locked";

/// `RosSelectable` on the party screen: Neeshka in the party and not selectable, so
/// the Sunken Flagon party screen must not let her leave.
#[tokio::test]
#[ignore = "writes into the user's save folder"]
async fn prepare_locked_member_save() {
    let ctx = common::create_test_context().await;
    let saves = NWN2Paths::new().saves().expect("saves");
    let dst = copy_slot(&saves.join(OC_BASE), &saves, LOCKED_SLOT);
    let mut session = SessionState::new();
    common::load_save(&mut session, &dst);
    session
        .stage_party_membership(&[("neeshka".to_string(), true)])
        .expect("party");
    session
        .save_character(&ctx.game_data, &NWN2Paths::new(), 0)
        .expect("save");
    let mut handler =
        app_lib::services::savegame_handler::SaveGameHandler::new(&dst).expect("handler");
    handler.begin_batch();
    app_lib::services::roster::apply_selectable(&mut handler, &[("neeshka".to_string(), false)])
        .expect("selectable");
    handler.commit_batch().expect("commit");
    let members = app_lib::services::roster::companion_members(&handler).expect("roster");
    for m in members {
        println!(
            "{} in_party {} selectable {}",
            m.ros_name, !m.available, m.selectable
        );
    }
}

/// `QT_OUT=<dir>`: a copy of the SoZ fixture with the unrecruited cohort Umoja's STR
/// raised and saved through the editor, to check his hiring NPC copy in `g_x2.z`.
#[tokio::test]
#[ignore = "writes into QT_OUT"]
async fn edit_unrecruited_cohort() {
    use app_lib::character::AbilityIndex;
    let ctx = common::create_test_context().await;
    let dst = std::path::PathBuf::from(std::env::var("QT_OUT").expect("QT_OUT"));
    common::copy_dir_recursive(
        &common::fixtures_path().join("saves").join("STORM_Campaign"),
        &dst,
    )
    .expect("copy");
    let mut session = SessionState::new();
    common::load_save(&mut session, &dst);
    session.load_companion("co_umoja").expect("companion");
    let umoja = session.character_mut().expect("umoja");
    let strength = umoja.base_ability(AbilityIndex::STR);
    umoja
        .set_ability(AbilityIndex::STR, strength + 1)
        .expect("str");
    umoja.mark_modified();
    session.load_player().expect("player");
    session
        .save_character(&ctx.game_data, &NWN2Paths::new(), 0)
        .expect("save");
    let umoja = session.companion_snapshot("co_umoja").expect("umoja");
    println!(
        "umoja STR {strength} -> {}",
        umoja.base_ability(AbilityIndex::STR)
    );
}
