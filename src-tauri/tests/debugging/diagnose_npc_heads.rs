use app_lib::character::Character;
use app_lib::parsers::gff::GffParser;
use app_lib::services::savegame_handler::SaveGameHandler;
use app_lib::services::tint_analysis::{TintCapabilityCache, color_capabilities};
use std::path::PathBuf;

#[path = "../common/mod.rs"]
mod common;

fn save_dir() -> PathBuf {
    let dir = std::env::var("NPC_HEADS_SAVE")
        .unwrap_or_else(|_| "tests/fixtures/saves/Classic_Campaign".to_string());
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(dir)
}

fn character_from_bytes(bytes: Vec<u8>) -> Character {
    let parser = GffParser::from_bytes(bytes).expect("Failed to parse GFF");
    let root = parser.read_struct_fields(0).expect("Failed to read root");
    Character::from_gff(root).expect("character from gff")
}

/// `NPC_HEADS_SAVE=<dir relative to src-tauri> cargo test --features integration-tests --test debugging diagnose_npc_heads -- --ignored --nocapture`
#[test]
#[ignore]
fn diagnose_npc_heads() {
    let handler = SaveGameHandler::new(save_dir(), false).expect("Failed to open save");
    let ctx = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(common::create_test_context());
    let game_data = ctx.loader.game_data().expect("Game data not loaded");
    let rm = ctx.resource_manager.blocking_read();
    let cache = TintCapabilityCache::default();

    let mut entries: Vec<(String, Character)> = Vec::new();
    if let Ok(Some(bic)) = handler.extract_player_bic() {
        entries.push(("player.bic".into(), character_from_bytes(bic)));
    }
    for name in handler.list_companions().expect("list_companions") {
        if let Ok(bytes) = handler.extract_companion(&name) {
            entries.push((name, character_from_bytes(bytes)));
        }
    }

    for (name, character) in &entries {
        let parts = character.resolve_model_parts(&game_data, &rm);
        let state = character.get_appearance_state(&game_data, &rm, parts.as_ref());
        let caps = parts.as_ref().map(|p| color_capabilities(&rm, &cache, p));
        println!(
            "\n== {name}: race={} subrace_idx={:?} appearance={} ({}) gender={}",
            state.race_id,
            character.subrace_index(),
            state.appearance_type,
            state.appearance_type_name,
            state.gender
        );
        println!(
            "   head={} hair={} fhair={} available_heads={:?} available_hairs={:?}",
            state.appearance_head,
            state.appearance_hair,
            state.appearance_fhair,
            state.available_heads,
            state.available_hairs
        );
        if let Some(p) = &parts {
            println!(
                "   head_resref={:?} hair_resref={:?}",
                p.head_resref, p.hair_resref
            );
        }
        println!("   caps={caps:?}");
    }
}
