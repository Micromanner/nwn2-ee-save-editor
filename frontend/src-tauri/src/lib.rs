mod sidecar_manager;
mod file_operations;
mod window_manager;
mod config;

use tauri::Manager;
use sidecar_manager::{FastAPISidecar, start_fastapi_sidecar, stop_fastapi_sidecar, check_fastapi_health, check_background_loading_status, graceful_shutdown_on_exit};
use file_operations::{
    select_save_file, 
    select_nwn2_directory, 
    find_nwn2_saves, 
    get_steam_workshop_path,
    validate_nwn2_installation,
    get_save_thumbnail,
    detect_nwn2_installation,
    launch_nwn2_game
};
use window_manager::{open_settings_window, close_settings_window};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let fastapi_sidecar = FastAPISidecar::new();

  tauri::Builder::default()
    .plugin(tauri_plugin_process::init())
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_fs::init())
    .plugin(tauri_plugin_http::init())
    .setup(move |app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      // Auto-start FastAPI sidecar in development
      if cfg!(debug_assertions) {
        let app_handle = app.handle().clone();
        tauri::async_runtime::spawn(async move {
          if let Err(e) = start_fastapi_sidecar(app_handle).await {
            log::error!("Failed to start FastAPI sidecar: {}", e);
          }
        });
      }

      Ok(())
    })
    .manage(fastapi_sidecar)
    .invoke_handler(tauri::generate_handler![
      start_fastapi_sidecar,
      stop_fastapi_sidecar,
      check_fastapi_health,
      check_background_loading_status,
      graceful_shutdown_on_exit,
      select_save_file,
      select_nwn2_directory,
      find_nwn2_saves,
      get_steam_workshop_path,
      validate_nwn2_installation,
      get_save_thumbnail,
      detect_nwn2_installation,
      launch_nwn2_game,
      open_settings_window,
      close_settings_window
    ])
    .on_window_event(|window, event| match event {
      tauri::WindowEvent::CloseRequested { .. } => {
        // Gracefully shutdown FastAPI when window is closing
        let window_clone = window.clone();
        tauri::async_runtime::spawn(async move {
          log::info!("Window close requested - initiating graceful FastAPI shutdown");
          if let Err(e) = graceful_shutdown_on_exit(window_clone.app_handle().clone()).await {
            log::error!("Failed to gracefully shutdown FastAPI on exit: {}", e);
            // Fallback to force stop
            let _ = stop_fastapi_sidecar(window_clone.app_handle().clone()).await;
          }
        });
      }
      _ => {}
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
