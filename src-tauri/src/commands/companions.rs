use crate::commands::{CommandError, CommandResult};
use crate::loaders::GameData;
use crate::state::AppState;
use tauri::State;
use tracing::{error, info, instrument};

#[derive(Clone, serde::Serialize, specta::Type)]
pub struct RosterClassInfo {
    pub class_id: i32,
    pub level: i32,
}

#[derive(Clone, serde::Serialize, specta::Type)]
pub struct RosterEntryInfo {
    pub ros_name: String,
    pub char_name: String,
    pub classes: Vec<RosterClassInfo>,
    pub in_party: bool,
    pub staged_in_party: Option<bool>,
    /// `RosGender` from the roster (0 = male, 1 = female); None when absent.
    pub gender: Option<i32>,
    /// Display name resolved from the roster's `RosRace` (`racialsubtypes.2da` row).
    pub subrace_name: Option<String>,
}

#[derive(Clone, serde::Deserialize, specta::Type)]
pub struct PartyMembershipChange {
    pub ros_name: String,
    pub in_party: bool,
}

fn subrace_display_name(game_data: &GameData, subrace: i32) -> Option<String> {
    let table = game_data.get_table("racialsubtypes")?;
    let row = table.get_by_id(subrace)?;
    if let Some(Some(strref_str)) = row.get("name")
        && let Ok(strref) = strref_str.parse::<i32>()
        && let Some(name) = game_data.get_string(strref)
    {
        return Some(name);
    }
    row.get("label").and_then(|v| v.clone())
}

#[tauri::command]
pub async fn list_roster(state: State<'_, AppState>) -> CommandResult<Vec<RosterEntryInfo>> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let listings = session
        .list_roster()
        .map_err(|message| CommandError::FileError {
            message,
            path: None,
            diagnostics_path: None,
        })?;
    Ok(listings
        .into_iter()
        .map(|l| RosterEntryInfo {
            ros_name: l.ros_name,
            char_name: l.char_name,
            classes: l
                .classes
                .into_iter()
                .map(|(class_id, level)| RosterClassInfo { class_id, level })
                .collect(),
            in_party: l.in_party,
            staged_in_party: l.staged_in_party,
            gender: l.gender,
            subrace_name: l
                .subrace
                .and_then(|subrace| subrace_display_name(&game_data, subrace)),
        })
        .collect())
}

#[tauri::command]
pub async fn get_party_limit(state: State<'_, AppState>) -> CommandResult<Option<i32>> {
    let session = state.session.read();
    session
        .party_limit()
        .map_err(|message| CommandError::FileError {
            message,
            path: None,
            diagnostics_path: None,
        })
}

/// Stage the desired party composition in the session; it is written to the
/// save when the character is saved.
#[tauri::command]
#[instrument(name = "stage_party_membership_command", skip(state, changes))]
pub async fn stage_party_membership(
    state: State<'_, AppState>,
    changes: Vec<PartyMembershipChange>,
) -> CommandResult<bool> {
    info!(
        "Stage party membership command invoked ({} entries)",
        changes.len()
    );
    let desired: Vec<(String, bool)> = changes
        .into_iter()
        .map(|c| (c.ros_name, c.in_party))
        .collect();
    let mut session = state.session.write();
    session
        .stage_party_membership(&desired)
        .map_err(|message| {
            error!("Failed to stage party membership: {message}");
            CommandError::FileError {
                message,
                path: None,
                diagnostics_path: None,
            }
        })?;
    Ok(true)
}

#[tauri::command]
#[instrument(name = "load_companion_command", skip(state), fields(ros_name = %ros_name))]
pub async fn load_companion(state: State<'_, AppState>, ros_name: String) -> CommandResult<bool> {
    info!("Load companion command invoked");
    let mut session = state.session.write();
    session.load_companion(&ros_name).map_err(|message| {
        error!("Failed to load companion: {message}");
        CommandError::FileError {
            message,
            path: None,
            diagnostics_path: None,
        }
    })?;
    Ok(true)
}

#[tauri::command]
#[instrument(name = "load_player_command", skip(state))]
pub async fn load_player(state: State<'_, AppState>) -> CommandResult<bool> {
    info!("Load player command invoked");
    let mut session = state.session.write();
    session.load_player().map_err(|message| {
        error!("Failed to load player: {message}");
        CommandError::FileError {
            message,
            path: None,
            diagnostics_path: None,
        }
    })?;
    Ok(true)
}
