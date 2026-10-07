pub mod backup;
pub mod content;
pub mod globals;
pub mod settings;

use self::backup::{
    backup_campaign_variables, list_campaign_variable_backups as list_cv_bk,
    list_module_backups as list_module_bk, restore_campaign_variable_backup as restore_cv_bk,
    restore_module_backup as restore_module_bk,
};
use self::content::{
    ModuleInfo, ModuleSummary, ModuleVariables,
    batch_update_module_variables as batch_update_mod_vars, extract_module_info,
    extract_module_info_by_id, list_modules as list_mods,
};
use self::globals::GlobalsParser;
use self::settings::{
    CampaignBackupInfo, CampaignSettings, list_campaign_backups as list_cam_backups,
    read_campaign_settings, restore_campaign_from_backup as restore_cam_backup,
    update_campaign_settings as update_settings,
};
use crate::config::NWN2Paths;
use crate::parsers::xml::{CompanionStatus, FullSummary, QuestOverview, RosterCompanion, XmlData};
use crate::services::roster::companion_members;
use crate::services::save_graph::editing::QuestEdits;
use crate::services::savegame_handler::SaveGameHandler;
use indexmap::IndexMap;
use std::collections::HashMap;

pub struct CampaignManager;

/// The save's companions with their `roster.rst` `RosCharName`.
fn roster_companions(handler: &SaveGameHandler) -> Result<Vec<RosterCompanion>, String> {
    Ok(companion_members(handler)?
        .into_iter()
        .map(|m| RosterCompanion {
            ros_name: m.ros_name,
            name: m.char_name,
        })
        .collect())
}

impl CampaignManager {
    pub fn get_summary(handler: &SaveGameHandler) -> Result<FullSummary, String> {
        let xml_content = handler.extract_globals_xml().map_err(|e| e.to_string())?;
        let parser = GlobalsParser::from_string(&xml_content)?;
        Ok(parser.get_full_summary_struct(&roster_companions(handler)?))
    }

    pub fn get_general_info(
        handler: &SaveGameHandler,
    ) -> Result<HashMap<String, Option<String>>, String> {
        let xml_content = handler.extract_globals_xml().map_err(|e| e.to_string())?;
        let parser = GlobalsParser::from_string(&xml_content)?;
        Ok(parser.get_general_info())
    }

    pub fn get_campaign_variables(handler: &SaveGameHandler) -> Result<XmlData, String> {
        let xml_content = handler.extract_globals_xml().map_err(|e| e.to_string())?;
        let parser = GlobalsParser::from_string(&xml_content)?;
        Ok(parser.data)
    }

    pub fn get_module_info(
        handler: &SaveGameHandler,
        paths: &NWN2Paths,
    ) -> Result<(ModuleInfo, ModuleVariables), String> {
        extract_module_info(handler, paths)
    }

    pub fn list_modules(
        handler: &SaveGameHandler,
        paths: &NWN2Paths,
    ) -> Result<Vec<ModuleSummary>, String> {
        list_mods(handler, paths)
    }

    pub fn get_module_info_by_id(
        handler: &SaveGameHandler,
        paths: &NWN2Paths,
        module_id: &str,
    ) -> Result<(ModuleInfo, ModuleVariables), String> {
        extract_module_info_by_id(handler, paths, module_id)
    }

    pub fn analyze_quest_progress(handler: &SaveGameHandler) -> Result<QuestOverview, String> {
        let xml_content = handler.extract_globals_xml().map_err(|e| e.to_string())?;
        let parser = GlobalsParser::from_string(&xml_content)?;
        Ok(parser.get_quest_overview_struct())
    }

    pub fn get_campaign_settings(
        campaign_id: &str,
        paths: &NWN2Paths,
    ) -> Result<CampaignSettings, String> {
        read_campaign_settings(campaign_id, paths)
    }

    pub fn update_campaign_settings(
        settings: &CampaignSettings,
        paths: &NWN2Paths,
    ) -> Result<(), String> {
        update_settings(settings, paths)
    }

    pub fn update_global_int(
        handler: &mut SaveGameHandler,
        name: &str,
        value: i32,
    ) -> Result<(), String> {
        Self::update_campaign_variable(handler, name, &value.to_string(), "int")
    }

    pub fn update_global_string(
        handler: &mut SaveGameHandler,
        name: &str,
        value: &str,
    ) -> Result<(), String> {
        Self::update_campaign_variable(handler, name, value, "string")
    }

    /// Influence as it will be once the pending quest edits are saved.
    pub fn get_companion_influence(
        handler: &SaveGameHandler,
        quest_edits: &QuestEdits,
    ) -> Result<IndexMap<String, CompanionStatus>, String> {
        let xml_content = handler.extract_globals_xml().map_err(|e| e.to_string())?;
        let mut parser = GlobalsParser::from_string(&xml_content)?;
        quest_edits.overlay_globals(&mut parser.data);
        Ok(parser.get_companion_status(&roster_companions(handler)?))
    }

    pub fn batch_update_module_variables(
        handler: &SaveGameHandler,
        updates: &[(String, String, String)],
        module_id: Option<&str>,
    ) -> Result<(), String> {
        batch_update_mod_vars(handler, updates, module_id)
    }

    pub fn list_campaign_backups(
        campaign_id: &str,
        paths: &NWN2Paths,
    ) -> Result<Vec<CampaignBackupInfo>, String> {
        list_cam_backups(campaign_id, paths)
    }

    pub fn restore_campaign_backup(
        backup_path: &str,
        campaign_id: &str,
        paths: &NWN2Paths,
    ) -> Result<(), String> {
        restore_cam_backup(backup_path, campaign_id, paths)
    }

    pub fn update_campaign_variable(
        handler: &mut SaveGameHandler,
        var_name: &str,
        value: &str,
        var_type: &str,
    ) -> Result<(), String> {
        Self::batch_update_campaign_variables(
            handler,
            &[(
                var_name.to_string(),
                value.to_string(),
                var_type.to_string(),
            )],
        )
    }

    pub fn batch_update_campaign_variables(
        handler: &mut SaveGameHandler,
        updates: &[(String, String, String)],
    ) -> Result<(), String> {
        if updates.is_empty() {
            return Ok(());
        }

        backup_campaign_variables(handler)?;

        let xml_content = handler.extract_globals_xml().map_err(|e| e.to_string())?;
        let mut parser = GlobalsParser::from_string(&xml_content)?;

        for (name, value, var_type) in updates {
            match var_type.as_str() {
                "int" | "bool" => {
                    let v: i32 = value
                        .parse()
                        .map_err(|e| format!("Invalid {var_type} value for '{name}': {e}"))?;
                    let map = if var_type == "int" {
                        &mut parser.data.integers
                    } else {
                        &mut parser.data.booleans
                    };
                    map.insert(name.clone(), v);
                }
                "float" => {
                    let v: f32 = value
                        .parse()
                        .map_err(|e| format!("Invalid float value for '{name}': {e}"))?;
                    parser.data.floats.insert(name.clone(), v);
                }
                "string" => {
                    parser.data.strings.insert(name.clone(), value.clone());
                }
                _ => return Err(format!("Unknown variable type: {var_type}")),
            }
        }

        let new_xml = parser.to_xml_string()?;
        handler
            .update_file("globals.xml", new_xml.as_bytes())
            .map_err(|e| e.to_string())?;

        tracing::info!("Batch updated {} campaign variables", updates.len());
        Ok(())
    }

    pub fn list_campaign_variable_backups(
        handler: &SaveGameHandler,
    ) -> Result<Vec<CampaignBackupInfo>, String> {
        list_cv_bk(handler)
    }

    pub fn restore_campaign_variable_backup(
        handler: &SaveGameHandler,
        backup_path: &str,
    ) -> Result<(), String> {
        restore_cv_bk(handler, backup_path)
    }

    pub fn list_module_backups(
        handler: &SaveGameHandler,
        module_id: &str,
    ) -> Result<Vec<CampaignBackupInfo>, String> {
        list_module_bk(handler, module_id)
    }

    pub fn restore_module_backup(
        handler: &SaveGameHandler,
        backup_path: &str,
    ) -> Result<(), String> {
        restore_module_bk(handler, backup_path)
    }
}
