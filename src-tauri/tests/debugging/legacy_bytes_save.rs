//! Renames the PC of slot 800 (Westgate, original NWN2) through the session save
//! pipeline, to check the original engine reads the editor's UTF-8 name.

use app_lib::config::NWN2Paths;
use app_lib::services::load_diagnostics::{LoadInput, LoadReport};
use app_lib::state::session_state::SessionState;

#[path = "../common/mod.rs"]
mod common;
use common::create_test_context;

#[tokio::test]
#[ignore = "writes into the user's save folder"]
async fn legacy_bytes_save() {
    let ctx = create_test_context().await;
    let game_data = &ctx.game_data;
    let path = NWN2Paths::new()
        .saves()
        .expect("saves")
        .join("000800 - Westgate Original");
    let path = path.to_string_lossy().to_string();
    let mut report = LoadReport::new(LoadInput {
        file_path: path.clone(),
        player_index: None,
        file_size: None,
    });
    let mut session = SessionState::new();
    session
        .load_character(&path, None, true, &mut report)
        .expect("load");
    let character = session.character_mut().expect("character");
    println!("old name: {}", character.first_name());
    character.set_first_name("Jörgд".to_string());
    session
        .save_character(game_data, &NWN2Paths::new(), 0)
        .expect("save");
}
