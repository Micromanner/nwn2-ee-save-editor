//! Dump per-character known spells and the level helper's pending spell picks for a save.
//! Run: `PENDING_SAVE=<save dir> cargo test --features integration-tests --test debugging diagnose_pending_spells -- --ignored --nocapture`

#[path = "../common/mod.rs"]
mod common;
use app_lib::character::Character;
use app_lib::parsers::gff::GffParser;
use app_lib::services::savegame_handler::SaveGameHandler;
use common::create_test_context;

#[tokio::test]
#[ignore = "diagnostic; needs PENDING_SAVE"]
async fn diagnose_pending_spells() {
    let ctx = create_test_context().await;
    let gd = ctx.loader.game_data().expect("Game data not loaded");
    let save = std::env::var("PENDING_SAVE").expect("PENDING_SAVE not set");
    let handler = SaveGameHandler::new(&save, false).expect("handler");
    let mut files = vec![(
        "player".to_string(),
        handler
            .extract_player_bic()
            .expect("player")
            .unwrap_or_default(),
    )];
    for name in handler.list_companions().unwrap_or_default() {
        if let Ok(bytes) = handler.extract_companion(&name) {
            files.push((name, bytes));
        }
    }
    for (name, bytes) in files {
        let Ok(parser) = GffParser::from_bytes(bytes) else {
            continue;
        };
        let Ok(root) = parser.read_struct_fields(0) else {
            continue;
        };
        let c = Character::from_gff(root).expect("character from gff");
        let classes: Vec<_> = c
            .class_entries()
            .iter()
            .map(|ce| (ce.class_id.0, c.class_level(ce.class_id)))
            .collect();
        println!("{name} classes {classes:?}");
        for ce in c.class_entries() {
            for lvl in 0..=9 {
                let known = c.known_spells(ce.class_id, lvl);
                if !known.is_empty() {
                    println!("  class {} known L{lvl}: {known:?}", ce.class_id.0);
                }
            }
        }
        let st = c.get_spells_state(gd, &ctx.decoder);
        println!("  pending {:?}", st.pending_spell_learning);
    }
}
