use std::sync::Arc;

use parking_lot::RwLock;
use tauri::{AppHandle, State};

use crate::commands::campaign::cached_module_info;
use crate::commands::{CommandError, CommandResult};
use crate::services::save_graph::{
    self, BuildContext, QuestGraphProgress, SaveGraph, SaveGraphSummary, TransitionNode,
};
use crate::services::toolset_bridge::{self, BridgeError};
use crate::state::AppState;

/// Lightweight Quests-tab payload: campaign + module roster + per-quest summaries
/// without transition arrays. Full `SaveGraph` is cached in `SessionState` so the
/// companion `save_get_quest_transitions` command is an in-memory lookup.
#[tauri::command]
pub async fn save_get_quest_graph(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<SaveGraphSummary> {
    let graph = ensure_quest_graph(&app, &state)?;
    Ok(SaveGraphSummary::from(graph.as_ref()))
}

/// Returns the full transition list for a single quest tag, served out of the
/// session-cached `SaveGraph`. Builds the graph if it hasn't been cached yet.
#[tauri::command]
pub async fn save_get_quest_transitions(
    app: AppHandle,
    state: State<'_, AppState>,
    tag: String,
) -> CommandResult<Vec<TransitionNode>> {
    let graph = ensure_quest_graph(&app, &state)?;
    let transitions = graph
        .quests
        .iter()
        .find(|q| q.tag == tag)
        .map(|q| q.transitions.clone())
        .unwrap_or_default();
    Ok(transitions)
}

/// Snapshot of the current (or most recent) `save_get_quest_graph` build progress.
/// Polled by the Quests-tab loading UI.
#[tauri::command]
pub fn save_get_quest_graph_progress(state: State<'_, AppState>) -> QuestGraphProgress {
    state.quest_graph_progress.read().clone()
}

fn ensure_quest_graph(
    app: &AppHandle,
    state: &State<'_, AppState>,
) -> CommandResult<Arc<SaveGraph>> {
    if let Some(cached) = state.session.read().quest_graph_cache.clone() {
        return Ok(cached);
    }

    // cached_module_info internally takes session read/write locks — resolve it
    // before we take our own session lock for the build.
    let (module_info, module_vars) = cached_module_info(state)?;

    // The toolset bridge is 64-bit only and Windows-only. Original NWN2
    // ships 32-bit toolset DLLs that the bridge can't load, and Linux is gated
    // off entirely. In either case, return a degraded graph with an explanation
    // orphan instead of trying (and crashing) the bridge subprocess.
    let is_ee_install = state.paths.read().is_enhanced_edition();
    if !is_ee_install {
        let explanation = "Quest graph requires NWN2 Enhanced Edition. The original NWN2 install ships 32-bit toolset DLLs that this editor's 64-bit toolset bridge cannot load. Live module variables and globals are still shown above.".to_string();
        let graph = degraded_graph(state, &module_info, &module_vars, explanation)?;
        let shared = Arc::new(graph);
        state.session.write().quest_graph_cache = Some(shared.clone());
        return Ok(shared);
    }

    let client = match toolset_bridge::build_client(app, state) {
        Ok(c) => c,
        Err(BridgeError::PlatformUnsupported(msg)) => {
            let graph = degraded_graph(
                state,
                &module_info,
                &module_vars,
                format!("Quest graph is Windows-only. {msg}"),
            )?;
            let shared = Arc::new(graph);
            state.session.write().quest_graph_cache = Some(shared.clone());
            return Ok(shared);
        }
        Err(e) => return Err(CommandError::from(e)),
    };

    let progress_handle: Arc<RwLock<QuestGraphProgress>> = state.quest_graph_progress.clone();
    *progress_handle.write() = QuestGraphProgress {
        step: "starting".to_string(),
        progress: 0.0,
        message: "Resolving campaign…".to_string(),
    };
    let progress_sink = |snapshot: QuestGraphProgress| {
        *progress_handle.write() = snapshot;
    };

    let graph_result = {
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
            progress: Some(&progress_sink),
        })
    };

    let graph = match graph_result {
        Ok(g) => g,
        Err(e) => {
            *state.quest_graph_progress.write() = QuestGraphProgress {
                step: "error".to_string(),
                progress: 0.0,
                message: e.clone(),
            };
            return Err(CommandError::from(e));
        }
    };

    let shared = Arc::new(graph);
    state.session.write().quest_graph_cache = Some(shared.clone());
    Ok(shared)
}

fn degraded_graph(
    state: &State<'_, AppState>,
    module_info: &crate::services::campaign::content::ModuleInfo,
    module_vars: &crate::services::campaign::content::ModuleVariables,
    explanation: String,
) -> CommandResult<SaveGraph> {
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(save_graph::build_without_bridge(
        handler,
        module_info,
        module_vars,
        explanation,
    ))
}
