use serde::Serialize;
use specta::Type;
use tauri::State;
use tracing::info;

use crate::config::nwn2_paths::PathSource;
use crate::state::AppState;

use super::{CommandError, CommandResult};

#[derive(Debug, Serialize, Type)]
pub struct DebugLog {
    pub app_version: String,
    pub os: String,
    pub arch: String,
    pub timestamp: String,
    pub paths: PathsDebug,
    pub mods: ModsDebug,
    pub config: ConfigDebug,
    pub resources: ResourcesDebug,
    pub game_data: GameDataDebug,
    pub session: SessionDebug,
    pub character_summary: Option<CharacterSummaryDebug>,
}

#[derive(Debug, Serialize, Type)]
pub struct PathsDebug {
    pub game_folder: Option<String>,
    pub game_folder_source: String,
    pub documents_folder: Option<String>,
    pub documents_folder_source: String,
    pub steam_workshop_folder: Option<String>,
    pub steam_workshop_folder_source: String,
    pub custom_override_folders: Vec<String>,
    pub custom_hak_folders: Vec<String>,
    pub game_version: Option<String>,
    pub is_enhanced_edition: bool,
    pub is_steam_installation: bool,
    pub is_gog_installation: bool,
}

#[derive(Debug, Serialize, Type)]
pub struct ModsDebug {
    pub override_files: Vec<String>,
    pub hak_files: Vec<String>,
    pub workshop_items: Vec<String>,
}

#[derive(Debug, Serialize, Type)]
pub struct ConfigDebug {
    pub theme: String,
    pub language: String,
    pub font_size: String,
    pub auto_backup: bool,
    pub backup_count: u32,
    pub auto_close_on_launch: bool,
    pub show_launch_dialog: bool,
    pub max_recent_saves: usize,
}

#[derive(Debug, Serialize, Type)]
pub struct CacheStatsDebug {
    pub size: usize,
    pub max_size: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_ratio: f64,
}

#[derive(Debug, Serialize, Type)]
pub struct ModuleInfoDebug {
    pub name: String,
    pub mod_id: String,
    pub custom_tlk: String,
    pub hak_list: Vec<String>,
    pub campaign_id: Option<String>,
    pub path: String,
}

#[derive(Debug, Serialize, Type)]
pub struct ResourcesDebug {
    pub initialized: bool,
    pub resource_count: usize,
    pub template_count: usize,
    pub data_zip_count: usize,
    pub resource_sources: std::collections::HashMap<String, usize>,
    pub cache_stats: CacheStatsDebug,
    pub module_cache_stats: CacheStatsDebug,
    pub module_info: Option<ModuleInfoDebug>,
}

#[derive(Debug, Serialize, Type)]
pub struct GameDataDebug {
    pub table_count: usize,
    pub priority_tables: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Serialize, Type)]
pub struct SessionDebug {
    pub character_loaded: bool,
    pub file_path: Option<String>,
    pub character_name: Option<String>,
    pub has_unsaved_changes: bool,
}

#[derive(Debug, Serialize, Type)]
pub struct CharacterClassDebug {
    pub class_id: i32,
    pub level: i32,
}

#[derive(Debug, Serialize, Type)]
pub struct CharacterSummaryDebug {
    pub name: String,
    pub race_id: i32,
    pub subrace: Option<String>,
    pub classes: Vec<CharacterClassDebug>,
    pub total_level: i32,
    pub alignment: String,
}

/// Replace the OS home directory and username with placeholders so exported
/// debug logs don't leak personal paths (e.g. `C:\Users\<name>\...`).
pub(crate) fn redact(input: &str) -> String {
    match dirs::home_dir() {
        Some(home) => redact_with_home(input, &home.to_string_lossy()),
        None => input.to_string(),
    }
}

/// Pure core of [`redact`], parameterized on the home directory for testing.
fn redact_with_home(input: &str, home: &str) -> String {
    if home.is_empty() {
        return input.to_string();
    }
    let mut out = input.replace(home, "~");
    out = out.replace(&home.replace('\\', "/"), "~");
    if let Some(user) = std::path::Path::new(home).file_name() {
        out = out.replace(&*user.to_string_lossy(), "<user>");
    }
    out
}

fn path_source_to_string(source: PathSource) -> String {
    match source {
        PathSource::Discovery => "auto-detected".to_string(),
        PathSource::Environment => "environment".to_string(),
        PathSource::Config => "manual".to_string(),
    }
}

/// Collect the current debug snapshot from shared state.
///
/// Pure data-gathering with no file I/O. Used by both the manual
/// `export_debug_log` command and the automatic load diagnostics report.
pub(crate) async fn gather_debug_data(state: &AppState) -> DebugLog {
    // Collect all sync-locked data before any .await (parking_lot guards are !Send)
    let (
        paths_debug,
        mods_debug,
        config_debug,
        session_debug,
        character_summary,
        table_count,
        priority_tables,
    ) = {
        let paths = state.paths.read();
        let config = state.config.read();
        let session = state.session.read();
        let game_data = state.game_data.read();

        let paths_debug = PathsDebug {
            game_folder: paths.game_folder().map(|p| redact(&p.to_string_lossy())),
            game_folder_source: path_source_to_string(paths.game_folder_source()),
            documents_folder: paths
                .documents_folder()
                .map(|p| redact(&p.to_string_lossy())),
            documents_folder_source: path_source_to_string(paths.documents_folder_source()),
            steam_workshop_folder: paths
                .steam_workshop_folder()
                .map(|p| redact(&p.to_string_lossy())),
            steam_workshop_folder_source: path_source_to_string(
                paths.steam_workshop_folder_source(),
            ),
            custom_override_folders: paths
                .custom_override_folders()
                .iter()
                .map(|p| redact(&p.to_string_lossy()))
                .collect(),
            custom_hak_folders: paths
                .custom_hak_folders()
                .iter()
                .map(|p| redact(&p.to_string_lossy()))
                .collect(),
            game_version: paths.get_game_version(),
            is_enhanced_edition: paths.is_enhanced_edition(),
            is_steam_installation: paths.is_steam_installation(),
            is_gog_installation: paths.is_gog_installation(),
        };

        let override_files = paths
            .override_dir()
            .filter(|d| d.exists())
            .and_then(|d| std::fs::read_dir(d).ok())
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let hak_files = paths
            .hak_dir()
            .filter(|d| d.exists())
            .and_then(|d| std::fs::read_dir(d).ok())
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("hak"))
                    })
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let workshop_items = paths
            .steam_workshop_folder()
            .filter(|d| d.exists())
            .and_then(|d| std::fs::read_dir(d).ok())
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .map(|e| {
                        let id = e.file_name().to_string_lossy().to_string();
                        let contents: Vec<String> = std::fs::read_dir(e.path())
                            .into_iter()
                            .flatten()
                            .filter_map(|c| c.ok())
                            .map(|c| c.file_name().to_string_lossy().to_string())
                            .collect();
                        format!("{id} ({})", contents.join(", "))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mods_debug = ModsDebug {
            override_files,
            hak_files,
            workshop_items,
        };

        let config_debug = ConfigDebug {
            theme: config.theme.clone(),
            language: config.language.clone(),
            font_size: config.font_size.clone(),
            auto_backup: config.auto_backup,
            backup_count: config.backup_count,
            auto_close_on_launch: config.auto_close_on_launch,
            show_launch_dialog: config.show_launch_dialog,
            max_recent_saves: config.max_recent_saves,
        };

        let session_debug = SessionDebug {
            character_loaded: session.character.is_some(),
            file_path: session
                .current_file_path
                .as_ref()
                .map(|p| redact(&p.to_string_lossy())),
            character_name: session.character.as_ref().map(|c| c.full_name()),
            has_unsaved_changes: session.has_unsaved_changes(),
        };

        let character_summary = session.character.as_ref().map(|c| CharacterSummaryDebug {
            name: c.full_name(),
            race_id: c.race_id().0,
            subrace: c.subrace(),
            classes: c
                .class_entries()
                .iter()
                .map(|e| CharacterClassDebug {
                    class_id: e.class_id.0,
                    level: e.level,
                })
                .collect(),
            total_level: c.total_level(),
            alignment: c.alignment().alignment_string(),
        });

        let table_count = game_data.table_count();
        let priority_tables = game_data.priority_tables.clone();

        (
            paths_debug,
            mods_debug,
            config_debug,
            session_debug,
            character_summary,
            table_count,
            priority_tables,
        )
    };
    // All sync locks dropped here

    // Now safe to .await the async resource manager lock
    let rm = state.resource_manager.read().await;

    let cache_stats = rm.get_cache_stats();
    let module_cache_stats = rm.get_module_cache_stats();

    let resources_debug = ResourcesDebug {
        initialized: rm.is_initialized(),
        resource_count: rm.resource_count(),
        template_count: rm.template_count(),
        data_zip_count: rm.data_zip_paths().len(),
        resource_sources: rm.resource_source_counts(),
        cache_stats: CacheStatsDebug {
            size: cache_stats.size,
            max_size: cache_stats.max_size,
            hits: cache_stats.hits,
            misses: cache_stats.misses,
            hit_ratio: cache_stats.hit_ratio,
        },
        module_cache_stats: CacheStatsDebug {
            size: module_cache_stats.size,
            max_size: module_cache_stats.max_size,
            hits: module_cache_stats.hits,
            misses: module_cache_stats.misses,
            hit_ratio: module_cache_stats.hit_ratio,
        },
        module_info: rm.get_module_info().map(|m| ModuleInfoDebug {
            name: m.name.clone(),
            mod_id: m.mod_id.clone(),
            custom_tlk: m.custom_tlk.clone(),
            hak_list: m.hak_list.clone(),
            campaign_id: m.campaign_id.clone(),
            path: redact(&m.path.to_string_lossy()),
        }),
    };

    // Build game data summary using module info from resource manager
    let hak_count = rm.get_module_info().map(|m| m.hak_list.len()).unwrap_or(0);
    let has_custom_tlk = rm
        .get_module_info()
        .is_some_and(|m| !m.custom_tlk.is_empty());
    let has_module = rm.get_module_info().is_some();

    let summary = if !has_module {
        format!("{table_count} tables loaded (vanilla)")
    } else {
        let mut modifiers = Vec::new();
        if hak_count > 0 {
            modifiers.push(format!(
                "{hak_count} HAK{}",
                if hak_count == 1 { "" } else { "s" }
            ));
        }
        if has_custom_tlk {
            modifiers.push("custom TLK".to_string());
        }
        if modifiers.is_empty() {
            format!("{table_count} tables loaded (module)")
        } else {
            format!(
                "{table_count} tables loaded (modded: {})",
                modifiers.join(", ")
            )
        }
    };

    let game_data_debug = GameDataDebug {
        table_count,
        priority_tables,
        summary,
    };

    DebugLog {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        timestamp: chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string(),
        paths: paths_debug,
        mods: mods_debug,
        config: config_debug,
        resources: resources_debug,
        game_data: game_data_debug,
        session: session_debug,
        character_summary,
    }
}

#[tauri::command]
#[specta::specta]
pub async fn export_debug_log(state: State<'_, AppState>) -> CommandResult<String> {
    info!("Exporting debug log");

    let debug_log = gather_debug_data(&state).await;

    let json = serde_json::to_string_pretty(&debug_log)
        .map_err(|e| CommandError::Internal(format!("Failed to serialize debug log: {e}")))?;

    let downloads_path = dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .ok_or_else(|| CommandError::Internal("Could not find Downloads folder".to_string()))?;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("nwn2ee-debug-{timestamp}.json");
    let file_path = downloads_path.join(&filename);

    std::fs::write(&file_path, &json).map_err(|e| CommandError::FileError {
        message: format!("Failed to write debug log: {e}"),
        path: Some(file_path.to_string_lossy().to_string()),
        diagnostics_path: None,
    })?;

    info!("Debug log exported to {}", file_path.display());

    Ok(file_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::redact_with_home;

    #[test]
    fn redacts_windows_home_prefix() {
        let home = r"C:\Users\Vino";
        let input = r"C:\Users\Vino\Documents\Neverwinter Nights 2\saves\000008 - 12-07-2026";
        assert_eq!(
            redact_with_home(input, home),
            r"~\Documents\Neverwinter Nights 2\saves\000008 - 12-07-2026"
        );
    }

    #[test]
    fn redacts_forward_slash_variant_of_home() {
        // Some paths in the report use forward slashes (e.g. game_folder).
        let home = r"C:\Users\Vino";
        let input = "C:/Users/Vino/Documents/save.zip";
        assert_eq!(redact_with_home(input, home), "~/Documents/save.zip");
    }

    #[test]
    fn redacts_bare_username_outside_home_path() {
        // Username can appear standalone; ensure it is scrubbed too.
        let home = r"C:\Users\Vino";
        let input = r"D:\Games\Vino\char.bic";
        assert_eq!(redact_with_home(input, home), r"D:\Games\<user>\char.bic");
    }

    #[test]
    fn leaves_unrelated_paths_untouched() {
        let home = r"C:\Users\Vino";
        let input = r"C:\Program Files (x86)\Steam\steamapps\common\NWN2 Enhanced Edition";
        assert_eq!(redact_with_home(input, home), input);
    }

    #[test]
    fn empty_home_does_not_blank_the_whole_string() {
        // file_name() is None for a bare drive/empty home; must not corrupt output.
        let input = "some text";
        assert_eq!(redact_with_home(input, ""), "some text");
    }

    /// End-to-end: prove no field in the exported snapshot leaks the username.
    ///
    /// `redact` keys off the real home dir, so we inject a path containing the
    /// actual username and assert it is gone from the serialized JSON. This
    /// covers wiring (every path field routed through `redact`), not just the
    /// helper in isolation.
    #[tokio::test]
    async fn debug_snapshot_scrubs_real_username_from_all_fields() {
        let Some(home) = dirs::home_dir() else {
            return; // no home dir in this environment; nothing to prove
        };
        let Some(user_os) = home.file_name() else {
            return;
        };
        let username = user_os.to_string_lossy().to_string();
        if username.is_empty() {
            return;
        }

        let state = crate::state::AppState::new();
        let fake_save = home
            .join("Documents")
            .join("Neverwinter Nights 2")
            .join("saves")
            .join("000123");
        {
            state.session.write().current_file_path = Some(fake_save);
        }

        let snapshot = super::gather_debug_data(&state).await;
        let json = serde_json::to_string(&snapshot).unwrap();

        assert!(
            !json.contains(&username),
            "username '{username}' leaked into debug snapshot JSON: {json}"
        );
        // Sanity: redaction stripped only the prefix, not the whole path.
        assert!(
            json.contains("000123"),
            "injected save path was lost entirely, not just redacted"
        );
    }
}
