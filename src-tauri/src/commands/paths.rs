use crate::commands::{CommandError, CommandResult};
use crate::config::PathSource;
use crate::events::PATHS_CHANGED_EVENT;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tracing::{debug, error, instrument};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    pub path: Option<String>,
    pub exists: bool,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomFolderInfo {
    pub path: String,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    pub game_folder: PathInfo,
    pub documents_folder: PathInfo,
    pub steam_workshop_folder: PathInfo,
    pub localvault_folder: PathInfo,
    pub custom_override_folders: Vec<CustomFolderInfo>,
    pub custom_module_folders: Vec<CustomFolderInfo>,
    pub custom_hak_folders: Vec<CustomFolderInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathUpdateResponse {
    pub success: bool,
    pub message: String,
    pub paths: PathConfig,
}

fn source_to_string(source: PathSource) -> String {
    match source {
        PathSource::Discovery | PathSource::Environment => "auto".to_string(),
        PathSource::Config => "manual".to_string(),
    }
}

fn build_path_config(paths: &crate::config::NWN2Paths) -> PathConfig {
    PathConfig {
        game_folder: PathInfo {
            path: paths.game_folder().map(|p| p.to_string_lossy().to_string()),
            exists: paths.game_folder().is_some_and(|p| p.exists()),
            source: source_to_string(paths.game_folder_source()),
        },
        documents_folder: PathInfo {
            path: paths
                .documents_folder()
                .map(|p| p.to_string_lossy().to_string()),
            exists: paths.documents_folder().is_some_and(|p| p.exists()),
            source: source_to_string(paths.documents_folder_source()),
        },
        steam_workshop_folder: PathInfo {
            path: paths
                .steam_workshop_folder()
                .map(|p| p.to_string_lossy().to_string()),
            exists: paths.steam_workshop_folder().is_some_and(|p| p.exists()),
            source: source_to_string(paths.steam_workshop_folder_source()),
        },
        localvault_folder: PathInfo {
            path: paths.localvault().map(|p| p.to_string_lossy().to_string()),
            exists: paths.localvault().is_some_and(|p| p.exists()),
            source: "derived".to_string(),
        },
        custom_override_folders: paths
            .custom_override_folders()
            .iter()
            .map(|p| CustomFolderInfo {
                path: p.to_string_lossy().to_string(),
                exists: p.exists(),
            })
            .collect(),
        custom_module_folders: paths
            .custom_module_folders()
            .iter()
            .map(|p| CustomFolderInfo {
                path: p.to_string_lossy().to_string(),
                exists: p.exists(),
            })
            .collect(),
        custom_hak_folders: paths
            .custom_hak_folders()
            .iter()
            .map(|p| CustomFolderInfo {
                path: p.to_string_lossy().to_string(),
                exists: p.exists(),
            })
            .collect(),
    }
}

async fn sync_resource_manager_paths(
    app: &AppHandle,
    state: &AppState,
    snapshot: crate::config::NWN2Paths,
) -> PathConfig {
    let config = build_path_config(&snapshot);

    {
        let mut resource_manager = state.resource_manager.write().await;
        resource_manager.update_paths(snapshot).await;
    }
    {
        let mut game_data = state.game_data.write();
        game_data.clear();
    }
    {
        let mut session = state.session.write();
        session.close_character();
    }
    *state.model_list_cache.lock() = None;

    if let Err(err) = crate::commands::gamedata::load_game_data_impl(state).await {
        error!(
            error = %err,
            "Failed to reload game data after path change; clearing stale state"
        );
        let mut game_data = state.game_data.write();
        game_data.clear();
    }

    if let Err(err) = app.emit(PATHS_CHANGED_EVENT, config.clone()) {
        error!(error = %err, "Failed to emit paths-changed event");
    }
    config
}

#[tauri::command]
#[instrument(name = "get_paths_config", skip(state))]
pub async fn get_paths_config(state: State<'_, AppState>) -> CommandResult<PathConfig> {
    debug!("Getting paths configuration");
    let paths = state.paths.read();
    Ok(build_path_config(&paths))
}

#[tauri::command]
#[instrument(name = "set_game_folder", skip(app, state))]
pub async fn set_game_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> CommandResult<PathUpdateResponse> {
    debug!("Setting game folder to: {}", path);
    let snapshot = {
        let mut paths = state.paths.write();

        paths.set_game_folder(&path).map_err(|e| {
            if e.contains("does not exist") {
                CommandError::NotFound {
                    item: format!("Path: {path}"),
                }
            } else {
                CommandError::OperationFailed {
                    operation: "set_game_folder".to_string(),
                    reason: e,
                }
            }
        })?;
        paths.clone()
    };
    let paths = sync_resource_manager_paths(&app, &state, snapshot).await;

    Ok(PathUpdateResponse {
        success: true,
        message: format!("Game folder set to: {path}"),
        paths,
    })
}

#[tauri::command]
#[instrument(name = "set_documents_folder", skip(app, state))]
pub async fn set_documents_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> CommandResult<PathUpdateResponse> {
    debug!("Setting documents folder to: {}", path);
    let snapshot = {
        let mut paths = state.paths.write();

        paths.set_documents_folder(&path).map_err(|e| {
            if e.contains("does not exist") {
                CommandError::NotFound {
                    item: format!("Path: {path}"),
                }
            } else {
                CommandError::OperationFailed {
                    operation: "set_documents_folder".to_string(),
                    reason: e,
                }
            }
        })?;
        paths.clone()
    };
    let paths = sync_resource_manager_paths(&app, &state, snapshot).await;

    Ok(PathUpdateResponse {
        success: true,
        message: format!("Documents folder set to: {path}"),
        paths,
    })
}

#[tauri::command]
#[instrument(name = "set_steam_workshop_folder", skip(app, state))]
pub async fn set_steam_workshop_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> CommandResult<PathUpdateResponse> {
    debug!("Setting Steam workshop folder to: {}", path);
    let snapshot = {
        let mut paths = state.paths.write();

        paths.set_steam_workshop_folder(&path).map_err(|e| {
            if e.contains("does not exist") {
                CommandError::NotFound {
                    item: format!("Path: {path}"),
                }
            } else {
                CommandError::OperationFailed {
                    operation: "set_steam_workshop_folder".to_string(),
                    reason: e,
                }
            }
        })?;
        paths.clone()
    };
    let paths = sync_resource_manager_paths(&app, &state, snapshot).await;

    Ok(PathUpdateResponse {
        success: true,
        message: format!("Steam workshop folder set to: {path}"),
        paths,
    })
}

#[tauri::command]
#[instrument(name = "add_override_folder", skip(app, state))]
pub async fn add_override_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> CommandResult<PathUpdateResponse> {
    debug!("Adding override folder: {}", path);
    let snapshot = {
        let mut paths = state.paths.write();

        paths.add_custom_override_folder(&path).map_err(|e| {
            if e.contains("does not exist") {
                CommandError::NotFound {
                    item: format!("Path: {path}"),
                }
            } else if e.contains("already scanned") {
                CommandError::ValidationError {
                    field: "override_folder".to_string(),
                    reason: "This folder is already scanned automatically as the built-in Override directory. Custom folders are only needed for additional override directories outside the standard location.".to_string(),
                }
            } else {
                CommandError::OperationFailed {
                    operation: "add_override_folder".to_string(),
                    reason: e,
                }
            }
        })?;
        paths.clone()
    };
    let paths = sync_resource_manager_paths(&app, &state, snapshot).await;

    Ok(PathUpdateResponse {
        success: true,
        message: format!("Override folder added: {path}"),
        paths,
    })
}

#[tauri::command]
#[instrument(name = "remove_override_folder", skip(app, state))]
pub async fn remove_override_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> CommandResult<PathUpdateResponse> {
    debug!("Removing override folder: {}", path);
    let snapshot = {
        let mut paths = state.paths.write();

        paths
            .remove_custom_override_folder(&path)
            .map_err(|e| CommandError::OperationFailed {
                operation: "remove_override_folder".to_string(),
                reason: e,
            })?;
        paths.clone()
    };
    let paths = sync_resource_manager_paths(&app, &state, snapshot).await;

    Ok(PathUpdateResponse {
        success: true,
        message: format!("Override folder removed: {path}"),
        paths,
    })
}

#[tauri::command]
#[instrument(name = "add_hak_folder", skip(app, state))]
pub async fn add_hak_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> CommandResult<PathUpdateResponse> {
    debug!("Adding HAK folder: {}", path);
    let snapshot = {
        let mut paths = state.paths.write();

        paths.add_custom_hak_folder(&path).map_err(|e| {
            if e.contains("does not exist") {
                CommandError::NotFound {
                    item: format!("Path: {path}"),
                }
            } else if e.contains("already scanned") {
                CommandError::ValidationError {
                    field: "hak_folder".to_string(),
                    reason: "This folder is already scanned automatically as the built-in HAK directory. Custom folders are only needed for additional HAK directories outside the standard location.".to_string(),
                }
            } else {
                CommandError::OperationFailed {
                    operation: "add_hak_folder".to_string(),
                    reason: e,
                }
            }
        })?;
        paths.clone()
    };
    let paths = sync_resource_manager_paths(&app, &state, snapshot).await;

    Ok(PathUpdateResponse {
        success: true,
        message: format!("HAK folder added: {path}"),
        paths,
    })
}

#[tauri::command]
#[instrument(name = "remove_hak_folder", skip(app, state))]
pub async fn remove_hak_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> CommandResult<PathUpdateResponse> {
    debug!("Removing HAK folder: {}", path);
    let snapshot = {
        let mut paths = state.paths.write();

        paths
            .remove_custom_hak_folder(&path)
            .map_err(|e| CommandError::OperationFailed {
                operation: "remove_hak_folder".to_string(),
                reason: e,
            })?;
        paths.clone()
    };
    let paths = sync_resource_manager_paths(&app, &state, snapshot).await;

    Ok(PathUpdateResponse {
        success: true,
        message: format!("HAK folder removed: {path}"),
        paths,
    })
}

#[tauri::command]
#[instrument(name = "reset_game_folder", skip(app, state))]
pub async fn reset_game_folder(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<PathUpdateResponse> {
    debug!("Resetting game folder to auto-detected");
    let snapshot = {
        let mut paths = state.paths.write();

        paths
            .reset_game_folder()
            .map_err(|e| CommandError::OperationFailed {
                operation: "reset_game_folder".to_string(),
                reason: e,
            })?;
        paths.clone()
    };
    let paths = sync_resource_manager_paths(&app, &state, snapshot).await;

    Ok(PathUpdateResponse {
        success: true,
        message: "Game folder reset to auto-detected".to_string(),
        paths,
    })
}

#[tauri::command]
#[instrument(name = "reset_documents_folder", skip(app, state))]
pub async fn reset_documents_folder(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<PathUpdateResponse> {
    debug!("Resetting documents folder to auto-detected");
    let snapshot = {
        let mut paths = state.paths.write();

        paths
            .reset_documents_folder()
            .map_err(|e| CommandError::OperationFailed {
                operation: "reset_documents_folder".to_string(),
                reason: e,
            })?;
        paths.clone()
    };
    let paths = sync_resource_manager_paths(&app, &state, snapshot).await;

    Ok(PathUpdateResponse {
        success: true,
        message: "Documents folder reset to auto-detected".to_string(),
        paths,
    })
}

#[tauri::command]
#[instrument(name = "reset_steam_workshop_folder", skip(app, state))]
pub async fn reset_steam_workshop_folder(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<PathUpdateResponse> {
    debug!("Resetting Steam workshop folder to auto-detected");
    let snapshot = {
        let mut paths = state.paths.write();

        paths
            .reset_steam_workshop_folder()
            .map_err(|e| CommandError::OperationFailed {
                operation: "reset_steam_workshop_folder".to_string(),
                reason: e,
            })?;
        paths.clone()
    };
    let paths = sync_resource_manager_paths(&app, &state, snapshot).await;

    Ok(PathUpdateResponse {
        success: true,
        message: "Steam workshop folder reset to auto-detected".to_string(),
        paths,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoDetectResponse {
    pub game_installations: Vec<String>,
    pub documents_folder: Option<String>,
    pub steam_workshop: Option<String>,
    pub current_paths: PathConfig,
}

#[tauri::command]
#[instrument(name = "auto_detect_paths", skip(app, state))]
pub async fn auto_detect_paths(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<AutoDetectResponse> {
    debug!("Auto-detecting all paths");
    let snapshot = {
        let mut paths = state.paths.write();

        let _ = paths.reset_game_folder();
        let _ = paths.reset_documents_folder();
        let _ = paths.reset_steam_workshop_folder();
        paths.clone()
    };
    let game_installations = snapshot
        .game_folder()
        .map(|p| vec![p.to_string_lossy().to_string()])
        .unwrap_or_default();
    let documents_folder = snapshot
        .documents_folder()
        .map(|p| p.to_string_lossy().to_string());
    let steam_workshop = snapshot
        .steam_workshop_folder()
        .map(|p| p.to_string_lossy().to_string());
    let current_paths = sync_resource_manager_paths(&app, &state, snapshot).await;

    Ok(AutoDetectResponse {
        game_installations,
        documents_folder,
        steam_workshop,
        current_paths,
    })
}
