//! Prepares in-game test slots for the playerinfo.bin load-menu fixes.

use app_lib::config::NWN2Paths;
use app_lib::services::playerinfo::PlayerInfo;
use app_lib::state::session_state::SessionState;

#[path = "../common/mod.rs"]
mod common;

fn prepare(
    src: &std::path::Path,
    slot: &str,
    game_data: &app_lib::loaders::GameData,
    edit: impl FnOnce(&mut app_lib::character::Character),
) {
    let dst = NWN2Paths::new().saves().expect("saves").join(slot);
    assert!(!dst.exists(), "{slot} already exists");
    common::copy_dir_recursive(src, &dst).expect("copy");
    std::fs::write(dst.join("savename.txt"), slot.split(" - ").nth(1).unwrap()).unwrap();

    let before = PlayerInfo::load(dst.join("playerinfo.bin")).expect("game playerinfo");
    let mut session = SessionState::new();
    common::load_save(&mut session, &dst);
    edit(session.character_mut().expect("player"));
    session
        .save_character(game_data, &NWN2Paths::new(), 0)
        .expect("save");
    let after = PlayerInfo::load(dst.join("playerinfo.bin")).expect("editor playerinfo");
    println!(
        "{slot}\n  game:   {:?}\n  editor: {:?}",
        before.data, after.data
    );
}

#[tokio::test]
#[ignore = "writes into the user's save folder"]
async fn prepare_playerinfo_saves() {
    let ctx = common::create_test_context().await;
    let game_data = &ctx.game_data;
    let saves = NWN2Paths::new().saves().expect("saves");

    prepare(
        &saves.join("000020 - 21-07-2025-17-21"),
        "000910 - CLAUDE PlayerInfo Aasimar TN",
        game_data,
        |c| c.set_alignment(Some(50), Some(50)).expect("alignment"),
    );
    prepare(
        &common::fixtures_path().join("saves/000055 - SAVE GAME COMPLETE"),
        "000911 - CLAUDE PlayerInfo a a",
        game_data,
        |c| {
            let gold = c.gold();
            c.set_gold(gold + 1);
        },
    );
}
