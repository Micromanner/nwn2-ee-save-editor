use crate::commands::{CommandError, CommandResult};
use crate::parsers::xml::{CompanionStatus, FullSummary, XmlData};
use crate::services::campaign::CampaignManager;
use crate::services::campaign::content::{ModuleInfo, ModuleSummary, ModuleVariables};
use crate::services::campaign::settings::{CampaignBackupInfo, CampaignSettings};
use crate::state::AppState;
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub async fn get_campaign_summary(state: State<'_, AppState>) -> CommandResult<FullSummary> {
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::get_summary(handler).map_err(CommandError::from)
}

#[tauri::command]
pub async fn get_campaign_variables(state: State<'_, AppState>) -> CommandResult<XmlData> {
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::get_campaign_variables(handler).map_err(CommandError::from)
}

pub fn cached_module_info(
    state: &State<'_, AppState>,
) -> CommandResult<(ModuleInfo, ModuleVariables)> {
    if let Some(cached) = state.session.read().module_info_cache.clone() {
        return Ok(cached);
    }

    let mut session = state.session.write();
    if let Some(cached) = session.module_info_cache.clone() {
        return Ok(cached);
    }
    let paths = state.paths.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let result = CampaignManager::get_module_info(handler, &paths).map_err(CommandError::from)?;
    session.module_info_cache = Some(result.clone());
    Ok(result)
}

#[tauri::command]
pub async fn get_module_info(
    state: State<'_, AppState>,
) -> CommandResult<(ModuleInfo, ModuleVariables)> {
    cached_module_info(&state)
}

#[tauri::command]
pub async fn list_modules(state: State<'_, AppState>) -> CommandResult<Vec<ModuleSummary>> {
    let paths = state.paths.read();
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::list_modules(handler, &paths).map_err(CommandError::from)
}

#[tauri::command]
pub async fn get_module_info_by_id(
    state: State<'_, AppState>,
    module_id: String,
) -> CommandResult<(ModuleInfo, ModuleVariables)> {
    let paths = state.paths.read();
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::get_module_info_by_id(handler, &paths, &module_id).map_err(CommandError::from)
}

#[tauri::command]
pub async fn update_global_int(
    state: State<'_, AppState>,
    name: String,
    value: i32,
) -> CommandResult<()> {
    let mut session = state.session.write();
    let handler = session
        .savegame_handler
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::update_global_int(handler, &name, value).map_err(CommandError::from)
}

#[tauri::command]
pub async fn update_global_float(
    state: State<'_, AppState>,
    name: String,
    value: f32,
) -> CommandResult<()> {
    let mut session = state.session.write();
    let handler = session
        .savegame_handler
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::update_global_float(handler, &name, value).map_err(CommandError::from)
}

#[tauri::command]
pub async fn update_global_string(
    state: State<'_, AppState>,
    name: String,
    value: String,
) -> CommandResult<()> {
    let mut session = state.session.write();
    let handler = session
        .savegame_handler
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::update_global_string(handler, &name, &value).map_err(CommandError::from)
}

#[tauri::command]
pub async fn get_campaign_settings(
    state: State<'_, AppState>,
    campaign_id: String,
) -> CommandResult<CampaignSettings> {
    let paths = state.paths.read();
    CampaignManager::get_campaign_settings(&campaign_id, &paths).map_err(CommandError::from)
}

#[tauri::command]
pub async fn update_campaign_settings(
    state: State<'_, AppState>,
    settings: CampaignSettings,
) -> CommandResult<()> {
    let paths = state.paths.read();
    CampaignManager::update_campaign_settings(&settings, &paths).map_err(CommandError::from)
}

#[tauri::command]
pub async fn get_companion_influence(
    state: State<'_, AppState>,
) -> CommandResult<HashMap<String, CompanionStatus>> {
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::get_companion_influence(handler).map_err(CommandError::from)
}

#[tauri::command]
pub async fn update_companion_influence(
    state: State<'_, AppState>,
    companion_id: String,
    new_influence: i32,
) -> CommandResult<()> {
    let mut session = state.session.write();
    let handler = session
        .savegame_handler
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::update_companion_influence(handler, &companion_id, new_influence)
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn update_module_variable(
    state: State<'_, AppState>,
    variable_name: String,
    value: String,
    variable_type: String,
    module_id: Option<String>,
) -> CommandResult<()> {
    let mut session = state.session.write();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::update_module_variable(
        handler,
        &variable_name,
        &value,
        &variable_type,
        module_id.as_deref(),
    )
    .map_err(CommandError::from)?;
    session.invalidate_module_info_cache();
    Ok(())
}

#[tauri::command]
pub async fn batch_update_module_variables(
    state: State<'_, AppState>,
    updates: Vec<(String, String, String)>,
    module_id: Option<String>,
) -> CommandResult<()> {
    let mut session = state.session.write();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::batch_update_module_variables(handler, &updates, module_id.as_deref())
        .map_err(CommandError::from)?;
    session.invalidate_module_info_cache();
    Ok(())
}

#[tauri::command]
pub async fn list_campaign_backups(
    state: State<'_, AppState>,
    campaign_id: String,
) -> CommandResult<Vec<CampaignBackupInfo>> {
    let paths = state.paths.read();
    CampaignManager::list_campaign_backups(&campaign_id, &paths).map_err(CommandError::from)
}

#[tauri::command]
pub async fn batch_update_campaign_variables(
    state: State<'_, AppState>,
    updates: Vec<(String, String, String)>,
) -> CommandResult<()> {
    let mut session = state.session.write();
    let handler = session
        .savegame_handler
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::batch_update_campaign_variables(handler, &updates).map_err(CommandError::from)
}

#[tauri::command]
pub async fn restore_campaign_backup(
    state: State<'_, AppState>,
    backup_path: String,
    campaign_id: String,
) -> CommandResult<()> {
    let paths = state.paths.read();
    CampaignManager::restore_campaign_backup(&backup_path, &campaign_id, &paths)
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn update_campaign_variable(
    state: State<'_, AppState>,
    variable_name: String,
    value: String,
    variable_type: String,
) -> CommandResult<()> {
    let mut session = state.session.write();
    let handler = session
        .savegame_handler
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::update_campaign_variable(handler, &variable_name, &value, &variable_type)
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn list_campaign_variable_backups(
    state: State<'_, AppState>,
) -> CommandResult<Vec<CampaignBackupInfo>> {
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::list_campaign_variable_backups(handler).map_err(CommandError::from)
}

#[tauri::command]
pub async fn restore_campaign_variable_backup(
    state: State<'_, AppState>,
    backup_path: String,
) -> CommandResult<()> {
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::restore_campaign_variable_backup(handler, &backup_path)
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn list_module_backups(
    state: State<'_, AppState>,
) -> CommandResult<Vec<CampaignBackupInfo>> {
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::list_module_backups(handler).map_err(CommandError::from)
}

#[tauri::command]
pub async fn restore_module_backup(
    state: State<'_, AppState>,
    backup_path: String,
) -> CommandResult<()> {
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::restore_module_backup(handler, &backup_path).map_err(CommandError::from)
}
