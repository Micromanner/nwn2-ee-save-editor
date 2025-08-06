use tauri::{AppHandle, Manager, WebviewWindowBuilder, WebviewUrl};

#[tauri::command]
pub async fn open_settings_window(app: AppHandle) -> Result<(), String> {
    // Check if settings window already exists
    if let Some(_) = app.get_webview_window("settings") {
        // If it exists, just focus it
        if let Some(window) = app.get_webview_window("settings") {
            window.set_focus().map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    // Create new settings window
    let _settings_window = WebviewWindowBuilder::new(
        &app,
        "settings",
        WebviewUrl::App("settings".into())
    )
    .title("Settings - NWN2:EE Save Editor")
    .inner_size(600.0, 500.0)
    .min_inner_size(500.0, 400.0)
    .resizable(true)
    .decorations(true)
    .center()
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn close_settings_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}