pub mod backup;
pub mod content;
pub mod globals;
pub mod settings;

// use tracing::error;

use self::backup::{
    backup_campaign_variables, list_campaign_variable_backups as list_cv_bk,
    list_module_backups as list_module_bk, restore_campaign_variable_backup as restore_cv_bk,
    restore_module_backup as restore_module_bk,
};
use self::content::{
    ModuleInfo, ModuleSummary, ModuleVariables,
    batch_update_module_variables as batch_update_mod_vars, extract_module_info,
    extract_module_info_by_id, list_modules as list_mods, update_module_variable as update_mod_var,
};
use self::globals::GlobalsParser;
use self::settings::{
    CampaignBackupInfo, CampaignSettings, list_campaign_backups as list_cam_backups,
    read_campaign_settings, restore_campaign_from_backup as restore_cam_backup,
    update_campaign_settings as update_settings,
};
use crate::config::NWN2Paths;
use crate::parsers::xml::{
    CompanionStatus, FullSummary, QuestOverview, XmlData, get_companion_definitions,
};
use crate::services::savegame_handler::SaveGameHandler;
use std::collections::HashMap;

pub struct CampaignManager;

impl CampaignManager {
    pub fn get_summary(handler: &SaveGameHandler) -> Result<FullSummary, String> {
        let xml_content = handler.extract_globals_xml().map_err(|e| e.to_string())?;
        let parser = GlobalsParser::from_string(&xml_content)?;
        Ok(parser.get_full_summary_struct())
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
        if let Err(e) = backup_campaign_variables(handler) {
            tracing::warn!("Failed to backup globals.xml: {}", e);
        }

        let xml_content = handler.extract_globals_xml().map_err(|e| e.to_string())?;
        let mut parser = GlobalsParser::from_string(&xml_content)?;

        parser.data.integers.insert(name.to_string(), value);

        let new_xml = parser.to_xml_string()?;
        handler
            .update_file("globals.xml", new_xml.as_bytes())
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn update_global_float(
        handler: &mut SaveGameHandler,
        name: &str,
        value: f32,
    ) -> Result<(), String> {
        if let Err(e) = backup_campaign_variables(handler) {
            tracing::warn!("Failed to backup globals.xml: {}", e);
        }

        let xml_content = handler.extract_globals_xml().map_err(|e| e.to_string())?;
        let mut parser = GlobalsParser::from_string(&xml_content)?;

        parser.data.floats.insert(name.to_string(), value);

        let new_xml = parser.to_xml_string()?;
        handler
            .update_file("globals.xml", new_xml.as_bytes())
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn update_global_string(
        handler: &mut SaveGameHandler,
        name: &str,
        value: &str,
    ) -> Result<(), String> {
        if let Err(e) = backup_campaign_variables(handler) {
            tracing::warn!("Failed to backup globals.xml: {}", e);
        }

        let xml_content = handler.extract_globals_xml().map_err(|e| e.to_string())?;
        let mut parser = GlobalsParser::from_string(&xml_content)?;

        parser
            .data
            .strings
            .insert(name.to_string(), value.to_string());

        let new_xml = parser.to_xml_string()?;
        handler
            .update_file("globals.xml", new_xml.as_bytes())
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_companion_influence(
        handler: &SaveGameHandler,
    ) -> Result<HashMap<String, CompanionStatus>, String> {
        let xml_content = handler.extract_globals_xml().map_err(|e| e.to_string())?;
        let parser = GlobalsParser::from_string(&xml_content)?;
        Ok(parser.get_companion_status())
    }

    pub fn update_companion_influence(
        handler: &mut SaveGameHandler,
        companion_id: &str,
        new_influence: i32,
    ) -> Result<(), String> {
        let defs = get_companion_definitions();
        let def = defs
            .get(companion_id)
            .ok_or_else(|| format!("Unknown companion: {companion_id}"))?;
        Self::update_global_int(handler, def.influence_var, new_influence)
    }

    pub fn update_module_variable(
        handler: &SaveGameHandler,
        var_name: &str,
        value: &str,
        var_type: &str,
        module_id: Option<&str>,
    ) -> Result<(), String> {
        update_mod_var(handler, var_name, value, var_type, module_id)
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
        match var_type {
            "int" => {
                let v: i32 = value
                    .parse()
                    .map_err(|e| format!("Invalid int value: {e}"))?;
                Self::update_global_int(handler, var_name, v)
            }
            "float" => {
                let v: f32 = value
                    .parse()
                    .map_err(|e| format!("Invalid float value: {e}"))?;
                Self::update_global_float(handler, var_name, v)
            }
            "string" => Self::update_global_string(handler, var_name, value),
            _ => Err(format!("Unknown variable type: {var_type}")),
        }
    }

    pub fn batch_update_campaign_variables(
        handler: &mut SaveGameHandler,
        updates: &[(String, String, String)],
    ) -> Result<(), String> {
        if updates.is_empty() {
            return Ok(());
        }

        if let Err(e) = backup_campaign_variables(handler) {
            tracing::warn!("Failed to backup globals.xml: {}", e);
        }

        let xml_content = handler.extract_globals_xml().map_err(|e| e.to_string())?;
        let mut parser = GlobalsParser::from_string(&xml_content)?;

        for (name, value, var_type) in updates {
            match var_type.as_str() {
                "int" => {
                    let v: i32 = value
                        .parse()
                        .map_err(|e| format!("Invalid int value for '{name}': {e}"))?;
                    parser.data.integers.insert(name.clone(), v);
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
    ) -> Result<Vec<CampaignBackupInfo>, String> {
        list_module_bk(handler)
    }

    pub fn restore_module_backup(
        handler: &SaveGameHandler,
        backup_path: &str,
    ) -> Result<(), String> {
        restore_module_bk(handler, backup_path)
    }
}
