pub mod character;
pub mod commands;
pub mod config;
#[cfg(debug_assertions)]
pub mod dev_oracle;
pub mod error;
pub mod events;
pub mod file_operations;
pub mod loaders;
#[cfg(debug_assertions)]
pub mod mcp_server;
pub mod parsers;
pub mod services;
pub mod state;
pub mod utils;
mod window_manager;

#[cfg(debug_assertions)]
use std::sync::Arc;
use tauri::Manager;
use tauri::image::Image;
use tracing::{debug, info};

use file_operations::{
    browse_backups, browse_localvault, browse_saves, find_nwn2_saves, get_default_backups_path,
    get_default_localvault_path, get_default_saves_path, get_save_thumbnail, launch_nwn2_game,
    open_folder_in_explorer,
};
use window_manager::show_main_window;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting NWN2 Save Editor (Rust Tauri)");

    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            debug!("Tauri setup started");

            // Initialize AppState
            info!("Initializing AppState");
            let app_state = crate::state::AppState::new();

            // Start MCP SSE Bridge (debug builds only)
            #[cfg(debug_assertions)]
            {
                let mcp_state = crate::mcp_server::McpState::new(
                    Arc::clone(&app_state.game_data),
                    Arc::clone(&app_state.session),
                    app.handle().clone(),
                );

                tauri::async_runtime::spawn(async move {
                    crate::mcp_server::start(mcp_state, 14207).await;
                });
            }

            app.manage(app_state);
            info!("AppState initialized successfully");

            if let Some(window) = app.get_webview_window("main") {
                let icon_bytes = include_bytes!("../icons/icon.png");
                let img = image::load_from_memory(icon_bytes)
                    .expect("Failed to decode icon")
                    .into_rgba8();
                let (w, h) = img.dimensions();
                let icon = Image::new_owned(img.into_raw(), w, h);
                window.set_icon(icon)?;
            }

            debug!("Tauri setup completed");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            find_nwn2_saves,
            get_save_thumbnail,
            launch_nwn2_game,
            open_folder_in_explorer,
            show_main_window,
            browse_saves,
            get_default_saves_path,
            get_default_backups_path,
            browse_backups,
            browse_localvault,
            get_default_localvault_path,
            // Session
            crate::commands::session::load_character,
            crate::commands::session::list_save_characters,
            crate::commands::session::save_character,
            crate::commands::session::close_character,
            crate::commands::session::has_unsaved_changes,
            crate::commands::session::get_save_name,
            crate::commands::session::set_save_name,
            crate::commands::session::export_to_localvault,
            crate::commands::session::undo,
            crate::commands::session::redo,
            crate::commands::session::get_history_state,
            crate::commands::session::get_session_history,
            // Companions
            crate::commands::companions::list_roster,
            crate::commands::companions::load_companion,
            crate::commands::companions::load_player,
            crate::commands::companions::get_party_limit,
            crate::commands::companions::stage_party_membership,
            // Character - Identity
            crate::commands::character::get_character_name,
            crate::commands::character::get_alignment,
            crate::commands::character::set_alignment,
            crate::commands::character::set_deity,
            crate::commands::character::set_biography,
            // Character - Animal companion / familiar
            crate::commands::pets::get_pets,
            crate::commands::pets::set_animal_companion,
            crate::commands::pets::set_familiar,
            // Character - Abilities
            crate::commands::character::get_base_ability_scores,
            crate::commands::character::set_all_ability_scores,
            crate::commands::character::update_hit_points,
            // Character - Race
            crate::commands::character::change_race,
            // Classes
            crate::commands::classes::get_class_name,
            crate::commands::classes::get_class_icon,
            crate::commands::classes::set_experience,
            crate::commands::classes::remove_class_entry,
            crate::commands::classes::add_class_level,
            crate::commands::classes::change_class,
            crate::commands::classes::remove_class_levels,
            crate::commands::classes::get_available_prestige_classes,
            crate::commands::classes::get_all_categorized_classes,
            crate::commands::classes::get_class_detail,
            // Feats
            crate::commands::feats::get_feat_info,
            crate::commands::feats::add_feat,
            crate::commands::feats::remove_feat,
            crate::commands::feats::get_filtered_feats,
            // Skills
            crate::commands::skills::get_skills_state,
            crate::commands::skills::set_skill_rank,
            crate::commands::skills::reset_all_skills,
            // Spells
            crate::commands::spells::add_known_spell,
            crate::commands::spells::remove_known_spell,
            crate::commands::spells::get_character_available_spells,
            // Inventory
            crate::commands::inventory::get_inventory_summary,
            crate::commands::inventory::set_gold,
            crate::commands::inventory::equip_item,
            crate::commands::inventory::unequip_item,
            crate::commands::inventory::add_to_inventory,
            crate::commands::inventory::remove_from_inventory,
            crate::commands::inventory::get_available_templates,
            crate::commands::inventory::add_item_from_template,
            crate::commands::inventory::get_editor_metadata,
            crate::commands::inventory::update_item,
            crate::commands::inventory::transform_item,
            // Item Appearance
            crate::commands::item_appearance::get_item_appearance_options,
            crate::commands::item_appearance::load_item_model,
            crate::commands::item_appearance::load_item_part,
            crate::commands::item_appearance::load_accessory_model,
            // Combat & Saves
            crate::commands::combat::get_combat_summary,
            crate::commands::combat::update_natural_armor,
            crate::commands::combat::get_save_summary,
            crate::commands::combat::set_misc_save_bonus,
            // SaveGame
            crate::commands::savegame::restore_backup,
            crate::commands::savegame::delete_backup,
            crate::commands::savegame::delete_backups,
            // GameData
            crate::commands::gamedata::tlk_get_strings,
            crate::commands::gamedata::initialize_game_data,
            crate::commands::gamedata::get_initialization_status,
            crate::commands::gamedata::get_available_races,
            crate::commands::gamedata::get_subraces_for_race,
            crate::commands::gamedata::get_all_playable_subraces,
            crate::commands::gamedata::get_available_deities,
            crate::commands::gamedata::get_available_base_items,
            crate::commands::gamedata::get_available_icons,
            // Paths
            crate::commands::paths::get_paths_config,
            crate::commands::paths::set_game_folder,
            crate::commands::paths::set_documents_folder,
            crate::commands::paths::set_steam_workshop_folder,
            crate::commands::paths::add_override_folder,
            crate::commands::paths::remove_override_folder,
            crate::commands::paths::add_hak_folder,
            crate::commands::paths::remove_hak_folder,
            crate::commands::paths::reset_game_folder,
            crate::commands::paths::reset_documents_folder,
            crate::commands::paths::reset_steam_workshop_folder,
            // Config
            crate::commands::config::get_app_config,
            crate::commands::config::update_app_config,
            // Campaign
            crate::commands::campaign::get_campaign_variables,
            crate::commands::campaign::get_module_info,
            crate::commands::campaign::list_modules,
            crate::commands::campaign::get_module_info_by_id,
            crate::commands::campaign::get_campaign_settings,
            crate::commands::campaign::update_campaign_settings,
            crate::commands::campaign::get_companion_influence,
            crate::commands::campaign::update_companion_influence,
            crate::commands::campaign::batch_update_module_variables,
            crate::commands::campaign::batch_update_campaign_variables,
            crate::commands::campaign::list_campaign_backups,
            crate::commands::campaign::restore_campaign_backup,
            crate::commands::campaign::list_campaign_variable_backups,
            crate::commands::campaign::restore_campaign_variable_backup,
            crate::commands::campaign::list_module_backups,
            crate::commands::campaign::restore_module_backup,
            // Save Graph (aggregated quest view across save + campaign modules)
            crate::commands::save_graph::save_get_quest_graph,
            crate::commands::save_graph::save_preview_quest_edit,
            crate::commands::save_graph::save_summarize_quest_routes,
            crate::commands::save_graph::save_stage_quest_edit,
            crate::commands::save_graph::save_list_quest_edits,
            crate::commands::save_graph::save_get_quest_variables,
            crate::commands::save_graph::save_discard_quest_edits,
            crate::commands::save_graph::save_get_quest_transitions,
            crate::commands::save_graph::save_get_quest_graph_progress,
            // Appearance
            crate::commands::appearance::get_appearance_state,
            crate::commands::appearance::update_appearance,
            crate::commands::appearance::get_available_wings,
            crate::commands::appearance::get_available_tails,
            crate::commands::appearance::get_parts_based_appearances,
            crate::commands::appearance::load_character_model,
            crate::commands::appearance::load_character_part,
            crate::commands::appearance::load_companion_portrait_model,
            crate::commands::appearance::get_available_voicesets,
            crate::commands::appearance::preview_voiceset,
            // Overview (Aggregated State Commands)
            crate::commands::overview::get_overview_state,
            crate::commands::overview::update_character,
            crate::commands::overview::get_abilities_state,
            crate::commands::overview::apply_point_buy,
            crate::commands::overview::get_classes_state,
            crate::commands::overview::get_feats_state,
            crate::commands::overview::get_spells_state,
            // Debug
            crate::commands::debug::export_debug_log,
            // Models
            crate::commands::models::load_model,
            crate::commands::models::get_texture_bytes,
            crate::commands::models::get_icon_png,
            crate::commands::models::list_available_models,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
