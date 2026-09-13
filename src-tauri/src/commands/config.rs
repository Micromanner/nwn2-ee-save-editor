use crate::commands::{CommandError, CommandResult};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;
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

fn build_response(config: &crate::config::AppConfig) -> AppConfigResponse {
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

/// `NWN2EE_LOCALE` overrides the OS regional format for testing other
/// regions' date formats without changing system settings.
fn system_locale() -> String {
    std::env::var("NWN2EE_LOCALE")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(sys_locale::get_locale)
        .unwrap_or_else(|| "en-US".to_string())
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

#[tauri::command]
#[instrument(name = "update_app_config", skip(state))]
pub async fn update_app_config(
    state: State<'_, AppState>,
    updates: AppConfigUpdate,
) -> CommandResult<AppConfigResponse> {
    debug!("Updating app configuration: {:?}", updates);
    let mut config = state.config.write();

    if let Some(theme) = updates.theme {
        config.theme = theme;
    }
    if let Some(language) = updates.language {
        config.language = language;
    }
    if let Some(font_size) = updates.font_size {
        config.font_size = font_size;
    }
    if let Some(auto_backup) = updates.auto_backup {
        config.auto_backup = auto_backup;
    }
    if let Some(backup_count) = updates.backup_count {
        config.backup_count = backup_count;
    }
    if let Some(auto_close) = updates.auto_close_on_launch {
        config.auto_close_on_launch = auto_close;
    }
    if let Some(show_dialog) = updates.show_launch_dialog {
        config.show_launch_dialog = show_dialog;
    }

    config.save().map_err(|e| CommandError::OperationFailed {
        operation: "save_app_config".to_string(),
        reason: e.to_string(),
    })?;

    Ok(build_response(&config))
}
