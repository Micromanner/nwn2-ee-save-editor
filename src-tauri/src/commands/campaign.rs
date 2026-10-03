use crate::commands::{CommandError, CommandResult};
use crate::loaders::GameData;
use crate::parsers::xml::{CompanionStatus, XmlData};
use crate::services::campaign::CampaignManager;
use crate::services::campaign::content::{
    ModuleInfo, ModuleSummary, ModuleVariables, area_name_from_resref, strip_area_code,
};
use crate::services::campaign::settings::{CampaignBackupInfo, CampaignSettings};
use crate::services::savegame_handler::backup_manifest::VariableChange;
use crate::state::AppState;
use indexmap::IndexMap;
use tauri::State;

#[tauri::command]
pub async fn get_campaign_variables(state: State<'_, AppState>) -> CommandResult<XmlData> {
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let mut data = CampaignManager::get_campaign_variables(handler).map_err(CommandError::from)?;
    session.quest_edits.overlay_globals(&mut data);
    Ok(data)
}

pub fn cached_module_info(
    state: &State<'_, AppState>,
) -> CommandResult<(ModuleInfo, ModuleVariables)> {
    if let Some(cached) = state.session.read().module_info_cache.clone() {
        return Ok(cached);
    }

    let game_data = state.game_data.read();
    let paths = state.paths.read();
    let mut session = state.session.write();
    if let Some(cached) = session.module_info_cache.clone() {
        return Ok(cached);
    }
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let mut result =
        CampaignManager::get_module_info(handler, &paths).map_err(CommandError::from)?;
    resolve_area_names(&mut result.0, &game_data);
    session.module_info_cache = Some(result.clone());
    Ok(result)
}

/// Replace the raw area resrefs with their TLK names (area-code prefix
/// stripped), falling back to a title-cased resref.
fn resolve_area_names(info: &mut ModuleInfo, game_data: &GameData) {
    let resolve = |strref: i32, resref: &str| {
        game_data
            .get_string(strref)
            .map(|s| strip_area_code(&s).to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| area_name_from_resref(resref))
    };
    info.area_name = resolve(info.area_name_strref, &info.area_name);
    info.entry_area = resolve(info.entry_area_strref, &info.entry_area);
}

#[tauri::command]
pub async fn get_module_info(
    state: State<'_, AppState>,
) -> CommandResult<(ModuleInfo, ModuleVariables)> {
    let mut result = cached_module_info(&state)?;
    let session = state.session.read();
    if let Some(module) = session
        .savegame_handler
        .as_ref()
        .and_then(|h| h.extract_current_module().ok())
    {
        session.quest_edits.overlay_module(&module, &mut result.1);
    }
    Ok(result)
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
    let game_data = state.game_data.read();
    let paths = state.paths.read();
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let mut result = CampaignManager::get_module_info_by_id(handler, &paths, &module_id)
        .map_err(CommandError::from)?;
    resolve_area_names(&mut result.0, &game_data);
    session
        .quest_edits
        .overlay_module(&module_id, &mut result.1);
    Ok(result)
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
) -> CommandResult<IndexMap<String, CompanionStatus>> {
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::get_companion_influence(handler, &session.quest_edits)
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn update_companion_influence(
    state: State<'_, AppState>,
    companion_id: String,
    new_influence: i32,
) -> CommandResult<()> {
    let variable =
        CampaignManager::companion_influence_var(&companion_id).map_err(CommandError::from)?;
    let mut session = state.session.write();
    check_pending_quest_variable(&session, None, variable, "int")?;
    let handler = session
        .savegame_handler
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::update_companion_influence(handler, &companion_id, new_influence)
        .map_err(CommandError::from)?;
    session.record_variable_changes([VariableChange::new(None, variable, new_influence)]);
    Ok(())
}

#[tauri::command]
pub async fn batch_update_module_variables(
    state: State<'_, AppState>,
    updates: Vec<(String, String, String)>,
    module_id: Option<String>,
) -> CommandResult<()> {
    let mut session = state.session.write();
    let target_module = Some(module_id.clone().unwrap_or_else(|| {
        session
            .savegame_handler
            .as_ref()
            .and_then(|h| h.extract_current_module().ok())
            .unwrap_or_default()
    }));
    for (name, _, kind) in &updates {
        check_pending_quest_variable(&session, target_module.as_deref(), name, kind)?;
    }

    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::batch_update_module_variables(handler, &updates, module_id.as_deref())
        .map_err(CommandError::from)?;
    session.invalidate_module_info_cache();
    session.record_variable_changes(
        updates
            .iter()
            .map(|(name, value, _)| VariableChange::new(target_module.clone(), name, value)),
    );
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
    let target_module = None::<String>;
    for (name, _, kind) in &updates {
        check_pending_quest_variable(&session, target_module.as_deref(), name, kind)?;
    }

    let handler = session
        .savegame_handler
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::batch_update_campaign_variables(handler, &updates)
        .map_err(CommandError::from)?;
    session.record_variable_changes(
        updates
            .iter()
            .map(|(name, value, _)| VariableChange::new(None, name, value)),
    );
    Ok(())
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
    let target_module = None::<String>;
    check_pending_quest_variable(
        &session,
        target_module.as_deref(),
        &variable_name,
        &variable_type,
    )?;

    let handler = session
        .savegame_handler
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::update_campaign_variable(handler, &variable_name, &value, &variable_type)
        .map_err(CommandError::from)?;
    session.record_variable_changes([VariableChange::new(None, &variable_name, &value)]);
    Ok(())
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

/// Backups of `module_id`, or of the current module when `None`.
#[tauri::command]
pub async fn list_module_backups(
    state: State<'_, AppState>,
    module_id: Option<String>,
) -> CommandResult<Vec<CampaignBackupInfo>> {
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let module_id = match module_id {
        Some(id) => id,
        None => handler
            .extract_current_module()
            .map_err(CommandError::from)?,
    };
    CampaignManager::list_module_backups(handler, &module_id).map_err(CommandError::from)
}

#[tauri::command]
pub async fn restore_module_backup(
    state: State<'_, AppState>,
    backup_path: String,
) -> CommandResult<()> {
    let mut session = state.session.write();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    CampaignManager::restore_module_backup(handler, &backup_path).map_err(CommandError::from)?;
    session.invalidate_module_info_cache();
    Ok(())
}

fn check_pending_quest_variable(
    session: &crate::state::SessionState,
    module: Option<&str>,
    name: &str,
    kind: &str,
) -> CommandResult<()> {
    if let Some(edit) = session.quest_edits.pending.iter().find(|p| {
        p.writes.iter().any(|w| {
            w.key.module.as_deref() == module && w.key.name == name && w.key.value_type == kind
        })
    }) {
        return Err(CommandError::from(format!(
            "Variable {name} has a pending quest edit ({}) currently owning it. Discard that quest edit before changing this variable here.",
            edit.tag
        )));
    }
    Ok(())
}
