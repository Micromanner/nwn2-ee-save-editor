use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::commands::{CommandError, CommandResult};
use crate::services::toolset_bridge::{
    self, CampaignModules, Faction, ModuleGraph, ModuleVariable,
};
use crate::state::AppState;

#[tauri::command]
pub async fn toolset_list_modules(
    app: AppHandle,
    state: State<'_, AppState>,
    campaign_path: String,
) -> CommandResult<CampaignModules> {
    let client = toolset_bridge::build_client(&app, &state).map_err(CommandError::from)?;
    let path = PathBuf::from(campaign_path);
    client.list_modules(&path).map_err(CommandError::from)
}

#[tauri::command]
pub async fn toolset_get_quest_graph(
    app: AppHandle,
    state: State<'_, AppState>,
    module_path: String,
) -> CommandResult<ModuleGraph> {
    let client = toolset_bridge::build_client(&app, &state).map_err(CommandError::from)?;
    let path = PathBuf::from(module_path);
    client.graph(&path).map_err(CommandError::from)
}

#[tauri::command]
pub async fn toolset_get_faction_table(
    app: AppHandle,
    state: State<'_, AppState>,
    module_path: String,
) -> CommandResult<Vec<Faction>> {
    let client = toolset_bridge::build_client(&app, &state).map_err(CommandError::from)?;
    let path = PathBuf::from(module_path);
    Ok(client.graph(&path).map_err(CommandError::from)?.factions)
}

#[tauri::command]
pub async fn toolset_get_module_variables(
    app: AppHandle,
    state: State<'_, AppState>,
    module_path: String,
) -> CommandResult<Vec<ModuleVariable>> {
    let client = toolset_bridge::build_client(&app, &state).map_err(CommandError::from)?;
    let path = PathBuf::from(module_path);
    Ok(client
        .graph(&path)
        .map_err(CommandError::from)?
        .module_variables)
}
