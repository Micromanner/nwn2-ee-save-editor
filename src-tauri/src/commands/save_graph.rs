use tauri::{AppHandle, State};

use crate::commands::campaign::cached_module_info;
use crate::commands::{CommandError, CommandResult};
use crate::services::save_graph::{self, BuildContext, SaveGraph};
use crate::services::toolset_bridge;
use crate::state::AppState;

#[tauri::command]
pub async fn save_get_quest_graph(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<SaveGraph> {
    let (module_info, module_vars) = cached_module_info(&state)?;

    let client = toolset_bridge::build_client(&app, &state).map_err(CommandError::from)?;

    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let player_index = session
        .primary_player_index
        .unwrap_or(session.selected_player_index);
    let paths = state.paths.read();

    save_graph::build(BuildContext {
        handler,
        paths: &paths,
        client: &client,
        player_index,
        current_module: &module_info,
        current_module_vars: &module_vars,
    })
    .map_err(CommandError::from)
}
