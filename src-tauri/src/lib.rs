pub mod character;
pub mod commands;
pub mod config;
pub mod error;
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
    browse_backups, browse_localvault, browse_saves, detect_nwn2_installation, find_nwn2_saves,
    get_default_backups_path, get_default_localvault_path, get_default_saves_path,
    get_save_thumbnail, get_steam_workshop_path, launch_nwn2_game, open_folder_in_explorer,
    select_nwn2_directory, select_save_file, validate_nwn2_installation,
};
use window_manager::{close_settings_window, open_settings_window, show_main_window};

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
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
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
            select_save_file,
            select_nwn2_directory,
            find_nwn2_saves,
            get_steam_workshop_path,
            validate_nwn2_installation,
            get_save_thumbnail,
            detect_nwn2_installation,
            launch_nwn2_game,
            open_folder_in_explorer,
            open_settings_window,
            close_settings_window,
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
            crate::commands::session::get_session_info,
            crate::commands::session::has_unsaved_changes,
            crate::commands::session::export_to_localvault,
            // Character - Identity
            crate::commands::character::get_character_name,
            crate::commands::character::get_first_name,
            crate::commands::character::set_first_name,
            crate::commands::character::get_last_name,
            crate::commands::character::set_last_name,
            crate::commands::character::get_full_name,
            crate::commands::character::set_character_age,
            crate::commands::character::get_character_age,
            crate::commands::character::get_experience_points,
            crate::commands::character::set_experience_points,
            crate::commands::character::get_alignment,
            crate::commands::character::set_alignment,
            crate::commands::character::set_deity,
            crate::commands::character::get_deity,
            crate::commands::character::set_biography,
            crate::commands::character::get_biography,
            crate::commands::character::get_background,
            crate::commands::character::get_domains,
            // Character - Abilities
            crate::commands::character::set_attribute,
            crate::commands::character::get_ability_scores,
            crate::commands::character::get_base_ability_scores,
            crate::commands::character::set_all_ability_scores,
            crate::commands::character::get_hit_points,
            crate::commands::character::update_hit_points,
            crate::commands::character::get_encumbrance_limits,
            crate::commands::character::get_ability_points_summary,
            // Character - Race
            crate::commands::character::get_race_id,
            crate::commands::character::get_race_name,
            crate::commands::character::get_subrace,
            crate::commands::character::get_available_subraces,
            crate::commands::character::get_ability_modifiers,
            crate::commands::character::get_racial_modifiers,
            crate::commands::character::change_race,
            crate::commands::character::validate_character,
            // Classes
            crate::commands::classes::get_total_level,
            crate::commands::classes::get_class_entries,
            crate::commands::classes::get_class_level,
            crate::commands::classes::get_class_summary,
            crate::commands::classes::get_class_name,
            crate::commands::classes::get_xp_progress,
            crate::commands::classes::get_level_history,
            crate::commands::classes::set_experience,
            crate::commands::classes::add_class_entry,
            crate::commands::classes::set_class_level,
            crate::commands::classes::remove_class_entry,
            crate::commands::classes::is_prestige_class,
            crate::commands::classes::add_class_level,
            crate::commands::classes::change_class,
            crate::commands::classes::remove_class_levels,
            crate::commands::classes::check_prestige_class_requirements,
            crate::commands::classes::get_available_prestige_classes,
            crate::commands::classes::decode_alignment_restriction,
            crate::commands::classes::get_class_progression_details,
            crate::commands::classes::get_all_categorized_classes,
            crate::commands::classes::get_class_detail,
            // Feats
            crate::commands::feats::get_feat_list,
            crate::commands::feats::has_feat,
            crate::commands::feats::get_feat_info,
            crate::commands::feats::get_feat_summary,
            crate::commands::feats::get_feat_slots,
            crate::commands::feats::validate_feat_prerequisites,
            crate::commands::feats::add_feat,
            crate::commands::feats::remove_feat,
            crate::commands::feats::swap_feat,
            crate::commands::feats::check_feat_progression,
            crate::commands::feats::get_character_domains,
            crate::commands::feats::get_available_domains,
            crate::commands::feats::add_domain,
            crate::commands::feats::remove_domain,
            crate::commands::feats::get_all_feats,
            crate::commands::feats::get_filtered_feats,
            crate::commands::feats::check_feat_availability,
            // Skills
            crate::commands::skills::get_all_skills,
            crate::commands::skills::get_skill_ranks,
            crate::commands::skills::is_class_skill,
            crate::commands::skills::calculate_skill_cost,
            crate::commands::skills::get_skill_summary,
            crate::commands::skills::get_skills_state,
            crate::commands::skills::set_skill_rank,
            crate::commands::skills::reset_all_skills,
            crate::commands::skills::get_skill_points_remaining,
            // Spells
            crate::commands::spells::get_spell_summary,
            crate::commands::spells::get_known_spells,
            crate::commands::spells::get_memorized_spells,
            crate::commands::spells::get_domain_spells,
            crate::commands::spells::get_spell_details,
            crate::commands::spells::get_max_castable_spell_level,
            crate::commands::spells::calculate_spell_slots,
            crate::commands::spells::add_known_spell,
            crate::commands::spells::remove_known_spell,
            crate::commands::spells::prepare_spell,
            crate::commands::spells::clear_memorized_spells,
            crate::commands::spells::get_character_available_spells,
            crate::commands::spells::is_spellcaster,
            crate::commands::spells::get_character_ability_spells,
            crate::commands::spells::get_max_castable_spell_level,
            // Inventory
            crate::commands::inventory::get_inventory_items,
            crate::commands::inventory::get_equipped_items,
            crate::commands::inventory::get_inventory_summary,
            crate::commands::inventory::get_gold,
            crate::commands::inventory::set_gold,
            crate::commands::inventory::add_gold,
            crate::commands::inventory::get_equipment_bonuses,
            crate::commands::inventory::equip_item,
            crate::commands::inventory::unequip_item,
            crate::commands::inventory::add_to_inventory,
            crate::commands::inventory::remove_from_inventory,
            crate::commands::inventory::calculate_encumbrance,
            crate::commands::inventory::calculate_total_weight,
            crate::commands::inventory::get_equipped_item,
            crate::commands::inventory::get_item_proficiency_info,
            crate::commands::inventory::get_available_templates,
            crate::commands::inventory::add_item_from_template,
            crate::commands::inventory::get_editor_metadata,
            crate::commands::inventory::update_item,
            // Item Appearance
            crate::commands::item_appearance::get_item_appearance_options,
            crate::commands::item_appearance::list_armor_mesh_candidates,
            crate::commands::item_appearance::load_item_model,
            crate::commands::item_appearance::load_item_part,
            // Combat & Saves
            crate::commands::combat::get_combat_summary,
            crate::commands::combat::calculate_base_attack_bonus,
            crate::commands::combat::get_attack_sequence,
            crate::commands::combat::get_damage_reduction,
            crate::commands::combat::update_natural_armor,
            crate::commands::combat::update_initiative_bonus,
            crate::commands::combat::get_save_summary,
            crate::commands::combat::set_misc_save_bonus,
            crate::commands::combat::check_save,
            crate::commands::combat::get_armor_class,
            crate::commands::combat::get_attack_bonuses,
            crate::commands::combat::get_initiative,
            crate::commands::combat::get_attacks_per_round,
            crate::commands::combat::get_saving_throws,
            crate::commands::combat::get_save_breakdown,
            // SaveGame
            crate::commands::savegame::list_backups,
            crate::commands::savegame::create_backup,
            crate::commands::savegame::restore_backup,
            crate::commands::savegame::restore_modules_from_backup,
            crate::commands::savegame::cleanup_backups,
            crate::commands::savegame::list_save_files,
            crate::commands::savegame::get_save_info,
            crate::commands::savegame::delete_backup,
            // GameData
            crate::commands::gamedata::get_tlk_string,
            crate::commands::gamedata::get_2da_row,
            crate::commands::gamedata::list_2da_tables,
            crate::commands::gamedata::initialize_game_data,
            crate::commands::gamedata::get_initialization_status,
            crate::commands::gamedata::get_2da_table,
            crate::commands::gamedata::get_available_classes,
            crate::commands::gamedata::get_available_feats,
            crate::commands::gamedata::get_available_skills,
            crate::commands::gamedata::get_available_spells,
            crate::commands::gamedata::get_available_races,
            crate::commands::gamedata::get_subraces_for_race,
            crate::commands::gamedata::get_all_playable_subraces,
            crate::commands::gamedata::get_available_genders,
            crate::commands::gamedata::get_available_alignments,
            crate::commands::gamedata::get_available_deities,
            crate::commands::gamedata::get_all_domains,
            crate::commands::gamedata::get_available_backgrounds,
            crate::commands::gamedata::get_available_abilities,
            crate::commands::gamedata::get_available_base_items,
            crate::commands::gamedata::get_available_spell_schools,
            crate::commands::gamedata::get_available_item_properties,
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
            crate::commands::paths::auto_detect_paths,
            // Config
            crate::commands::config::get_app_config,
            crate::commands::config::update_app_config,
            // Campaign
            crate::commands::campaign::get_campaign_summary,
            crate::commands::campaign::get_campaign_variables,
            crate::commands::campaign::get_module_info,
            crate::commands::campaign::list_modules,
            crate::commands::campaign::get_module_info_by_id,
            crate::commands::campaign::get_journal,
            crate::commands::campaign::update_global_int,
            crate::commands::campaign::update_global_float,
            crate::commands::campaign::update_global_string,
            crate::commands::campaign::get_campaign_settings,
            crate::commands::campaign::update_campaign_settings,
            crate::commands::campaign::get_companion_influence,
            crate::commands::campaign::update_companion_influence,
            crate::commands::campaign::update_module_variable,
            crate::commands::campaign::batch_update_module_variables,
            crate::commands::campaign::batch_update_campaign_variables,
            crate::commands::campaign::list_campaign_backups,
            crate::commands::campaign::restore_campaign_backup,
            crate::commands::campaign::update_campaign_variable,
            crate::commands::campaign::list_campaign_variable_backups,
            crate::commands::campaign::restore_campaign_variable_backup,
            crate::commands::campaign::list_module_backups,
            crate::commands::campaign::restore_module_backup,
            // Appearance
            crate::commands::appearance::get_appearance_state,
            crate::commands::appearance::update_appearance,
            crate::commands::appearance::get_available_wings,
            crate::commands::appearance::get_available_tails,
            crate::commands::appearance::load_character_model,
            crate::commands::appearance::load_character_part,
            crate::commands::appearance::get_available_voicesets,
            crate::commands::appearance::preview_voiceset,
            // Overview (Aggregated State Commands)
            crate::commands::overview::get_overview_state,
            crate::commands::overview::update_character,
            crate::commands::overview::get_abilities_state,
            crate::commands::overview::update_abilities,
            crate::commands::overview::apply_point_buy,
            crate::commands::overview::get_classes_state,
            crate::commands::overview::get_feats_state,
            crate::commands::overview::get_spells_state,
            // Debug
            crate::commands::test_deities::debug_deities,
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
