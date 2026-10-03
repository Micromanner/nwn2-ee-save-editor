use crate::commands::paths::sync_resource_manager_paths;
use crate::commands::{CommandError, CommandResult};
use crate::config::{AppConfig, system_locale};
use crate::state::AppState;
use crate::utils::text_encoding::{GameLanguage, set_game_language};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tracing::{debug, instrument};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfigResponse {
    pub theme: String,
    pub language: String,
    pub font_size: String,
    pub auto_backup: bool,
    pub backup_count: u32,
    pub auto_close_on_launch: bool,
    pub show_launch_dialog: bool,
    /// BCP 47 tag of the OS regional format (Windows user locale, `LANG` on
    /// Linux), for date and number formatting independent of the UI language.
    pub system_locale: String,
}

#[tauri::command]
#[instrument(name = "get_app_config", skip(state))]
pub async fn get_app_config(state: State<'_, AppState>) -> CommandResult<AppConfigResponse> {
    debug!("Getting app configuration");
    let config = state.config.read();
    Ok(build_response(&config))
}

fn build_response(config: &AppConfig) -> AppConfigResponse {
    AppConfigResponse {
        theme: config.theme.clone(),
        language: config.language.clone(),
        font_size: config.font_size.clone(),
        auto_backup: config.auto_backup,
        backup_count: config.backup_count,
        auto_close_on_launch: config.auto_close_on_launch,
        show_launch_dialog: config.show_launch_dialog,
        system_locale: system_locale(),
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfigUpdate {
    pub theme: Option<String>,
    pub language: Option<String>,
    pub font_size: Option<String>,
    pub auto_backup: Option<bool>,
    pub backup_count: Option<u32>,
    pub auto_close_on_launch: Option<bool>,
    pub show_launch_dialog: Option<bool>,
}

impl AppConfigUpdate {
    fn apply_to(self, config: &mut AppConfig) {
        if let Some(theme) = self.theme {
            config.theme = theme;
        }
        if let Some(language) = self.language {
            config.language = language;
        }
        if let Some(font_size) = self.font_size {
            config.font_size = font_size;
        }
        if let Some(auto_backup) = self.auto_backup {
            config.auto_backup = auto_backup;
        }
        if let Some(backup_count) = self.backup_count {
            config.backup_count = backup_count;
        }
        if let Some(auto_close) = self.auto_close_on_launch {
            config.auto_close_on_launch = auto_close;
        }
        if let Some(show_dialog) = self.show_launch_dialog {
            config.show_launch_dialog = show_dialog;
        }
    }
}

#[tauri::command]
#[instrument(name = "update_app_config", skip(app, state))]
pub async fn update_app_config(
    app: AppHandle,
    state: State<'_, AppState>,
    updates: AppConfigUpdate,
) -> CommandResult<AppConfigResponse> {
    debug!("Updating app configuration: {:?}", updates);
    let (updated, previous) = {
        let config = state.config.read();
        let mut updated = config.clone();
        updates.apply_to(&mut updated);
        (updated, GameLanguage::from_locale(&config.language))
    };

    // The game language selects the TLK and VO archives, so it reloads
    // game data exactly like a game-folder change.
    let language = GameLanguage::from_locale(&updated.language);
    if language != previous {
        set_game_language(language);
        let snapshot = state.paths.read().clone();
        if let Err(err) = sync_resource_manager_paths(&app, &state, snapshot).await {
            set_game_language(previous);
            return Err(err);
        }
    }

    updated.save().map_err(|e| CommandError::OperationFailed {
        operation: "save_app_config".to_string(),
        reason: e.to_string(),
    })?;
    let response = build_response(&updated);
    *state.config.write() = updated;
    Ok(response)
}
