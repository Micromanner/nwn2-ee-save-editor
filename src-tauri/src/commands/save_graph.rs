use std::sync::Arc;

use parking_lot::RwLock;
use tauri::{AppHandle, State};

use crate::commands::campaign::cached_module_info;
use crate::commands::{CommandError, CommandResult};
use crate::services::save_graph::editing::{PendingQuestEdit, QuestPreview};
use crate::services::save_graph::{
    self, BuildContext, OrphanKind, QuestGraphProgress, SaveGraph, SaveGraphSummary, TransitionNode,
};
use crate::services::savegame_handler::SaveGameHandler;
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
    let graph = ensure_quest_graph(&app, &state).await?;
    let mut summary = SaveGraphSummary::from(graph.as_ref());
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let journal = save_graph::read_live_journal(
        handler,
        session
            .primary_player_index
            .unwrap_or(session.selected_player_index),
    )
    .map_err(CommandError::from)?;
    for quest in &mut summary.quests {
        quest.live_state = journal.iter().find(|j| j.tag == quest.tag).map(|j| j.state);
    }
    Ok(summary)
}

#[tauri::command]
pub async fn save_preview_quest_edit(
    app: AppHandle,
    state: State<'_, AppState>,
    tag: String,
    target: u32,
    route: Option<usize>,
) -> CommandResult<QuestPreview> {
    let graph = ensure_quest_graph(&app, &state).await?;
    let resources = state.resource_manager.read().await;
    let tables = save_graph::editing::RewardTables::from_graph(&graph, Some(&resources));
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    session
        .quest_edits
        .preview(
            &graph,
            &tables,
            handler,
            &state.paths.read(),
            session
                .primary_player_index
                .unwrap_or(session.selected_player_index),
            &tag,
            target,
            route,
        )
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn save_summarize_quest_routes(
    app: AppHandle,
    state: State<'_, AppState>,
    tag: String,
    target: u32,
) -> CommandResult<Vec<crate::services::save_graph::editing::RouteSummary>> {
    let graph = ensure_quest_graph(&app, &state).await?;
    let resources = state.resource_manager.read().await;
    let tables = save_graph::editing::RewardTables::from_graph(&graph, Some(&resources));
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    session
        .quest_edits
        .summarize_routes(
            &graph,
            &tables,
            handler,
            &state.paths.read(),
            session
                .primary_player_index
                .unwrap_or(session.selected_player_index),
            &tag,
            target,
        )
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn save_stage_quest_edit(
    app: AppHandle,
    state: State<'_, AppState>,
    tag: String,
    target: u32,
    route: Option<usize>,
    revision: String,
    selected: Vec<usize>,
    replace: bool,
) -> CommandResult<Vec<PendingQuestEdit>> {
    let graph = ensure_quest_graph(&app, &state).await?;
    let resources = state.resource_manager.read().await;
    let tables = save_graph::editing::RewardTables::from_graph(&graph, Some(&resources));
    let stale = || {
        CommandError::from(
            "The save or pending edits changed. Refresh the preview before applying.".to_string(),
        )
    };
    // Previewed under the read lock so edits and saves are not held up meanwhile.
    let (preview, generations) = {
        let session = state.session.read();
        let handler = session
            .savegame_handler
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        let preview = session
            .quest_edits
            .preview(
                &graph,
                &tables,
                handler,
                &state.paths.read(),
                session
                    .primary_player_index
                    .unwrap_or(session.selected_player_index),
                &tag,
                target,
                route,
            )
            .map_err(CommandError::from)?;
        let generations = (session.load_generation, session.quest_edits.generation());
        (preview, generations)
    };
    if preview.revision != revision {
        return Err(stale());
    }
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    if (session.load_generation, session.quest_edits.generation()) != generations {
        return Err(stale());
    }
    session
        .stage_quest_edit(preview, &graph, &selected, replace, &game_data)
        .map_err(CommandError::from)?;
    Ok(session.quest_edits.pending.clone())
}

#[tauri::command]
pub async fn save_list_quest_edits(
    state: State<'_, AppState>,
) -> CommandResult<Vec<PendingQuestEdit>> {
    Ok(state.session.read().quest_edits.pending.clone())
}

#[tauri::command]
pub async fn save_discard_quest_edits(
    state: State<'_, AppState>,
    tag: Option<String>,
) -> CommandResult<Vec<PendingQuestEdit>> {
    let mut session = state.session.write();
    session
        .discard_quest_edits(tag.as_deref())
        .map_err(CommandError::from)?;
    Ok(session.quest_edits.pending.clone())
}

#[derive(serde::Serialize)]
pub struct QuestVariableValue {
    key: save_graph::editing::VariableKey,
    saved: Option<serde_json::Value>,
    effective: Option<serde_json::Value>,
    available: bool,
}

#[tauri::command]
pub async fn save_get_quest_variables(
    app: AppHandle,
    state: State<'_, AppState>,
    tag: String,
) -> CommandResult<Vec<QuestVariableValue>> {
    let graph = ensure_quest_graph(&app, &state).await?;
    let quest = graph
        .quests
        .iter()
        .find(|q| q.tag == tag)
        .ok_or_else(|| CommandError::from("Quest unavailable".to_string()))?;
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let mut keys = std::collections::BTreeSet::new();
    let tables = save_graph::editing::RewardTables::from_graph(&graph, None);
    for tr in &quest.transitions {
        for action in tr.actions.iter().chain(&tr.gating_conditions) {
            use crate::services::toolset_bridge::FunctorKind;
            let kind = match action.kind {
                FunctorKind::GlobalInt => "int",
                FunctorKind::GlobalFloat => "float",
                FunctorKind::GlobalString => "string",
                FunctorKind::GlobalBool => "bool",
                _ => {
                    // Gold and XP are shown with the rewards in the edit dialog, not as story variables.
                    if let Ok((key, _)) =
                        save_graph::editing::decode_action(action, &tr.module, &tables)
                        && key.value_type != save_graph::editing::CHARACTER_TYPE
                        && key.value_type != save_graph::editing::ITEM_TYPE
                        && key.value_type != save_graph::editing::PC_LOCAL_TYPE
                        && key.value_type != save_graph::editing::DOOR_LOCK_TYPE
                        && key.value_type != save_graph::editing::MAP_NOTE_TYPE
                    {
                        keys.insert(key);
                    }
                    continue;
                }
            };
            if let Some(name) = action.params.first().and_then(serde_json::Value::as_str) {
                keys.insert(save_graph::editing::VariableKey {
                    module: None,
                    name: name.into(),
                    value_type: kind.into(),
                });
            }
        }
    }
    let paths = state.paths.read();
    let mut reader = save_graph::editing::VariableReader::new(
        handler,
        &paths,
        session
            .primary_player_index
            .unwrap_or(session.selected_player_index),
    );
    Ok(keys
        .into_iter()
        .map(|key| {
            let value = reader.read(&key);
            let available = value.is_ok();
            let saved = value.ok().flatten();
            let effective = session.quest_edits.pending_value(&key).or(saved.clone());
            QuestVariableValue {
                key,
                saved,
                effective,
                available,
            }
        })
        .collect())
}

/// Returns the full transition list for a single quest tag, served out of the
/// session-cached `SaveGraph`. Builds the graph if it hasn't been cached yet.
#[tauri::command]
pub async fn save_get_quest_transitions(
    app: AppHandle,
    state: State<'_, AppState>,
    tag: String,
) -> CommandResult<Vec<TransitionNode>> {
    let graph = ensure_quest_graph(&app, &state).await?;
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
pub async fn save_get_quest_graph_progress(
    state: State<'_, AppState>,
) -> CommandResult<QuestGraphProgress> {
    Ok(state.quest_graph_progress.read().clone())
}

async fn ensure_quest_graph(
    app: &AppHandle,
    state: &State<'_, AppState>,
) -> CommandResult<Arc<SaveGraph>> {
    let generation = {
        let session = state.session.read();
        if let Some(cached) = session.quest_graph_cache.clone() {
            return Ok(cached);
        }
        session.load_generation
    };

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
        let graph = degraded_graph(
            state,
            &module_info,
            &module_vars,
            OrphanKind::EnhancedEditionRequired,
            explanation,
        )?;
        return store_quest_graph(state, generation, graph);
    }

    let client = match toolset_bridge::build_client(app, state) {
        Ok(c) => c,
        Err(BridgeError::PlatformUnsupported(msg)) => {
            let graph = degraded_graph(
                state,
                &module_info,
                &module_vars,
                OrphanKind::PlatformUnsupported,
                format!("Quest graph is Windows-only. {msg}"),
            )?;
            return store_quest_graph(state, generation, graph);
        }
        Err(e) => return Err(CommandError::from(e)),
    };

    let progress_handle: Arc<RwLock<QuestGraphProgress>> = state.quest_graph_progress.clone();
    *progress_handle.write() = QuestGraphProgress {
        step: "starting".to_string(),
        progress: 0.0,
        message: "Resolving campaign…".to_string(),
    };
    // Build from copies outside the session lock: the build takes seconds and a
    // held read lock would stall every edit and save queued behind it.
    let paths = state.paths.read().clone();
    let (save_dir, player_index) = {
        let session = state.session.read();
        let handler = session
            .savegame_handler
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        let player_index = session
            .primary_player_index
            .unwrap_or(session.selected_player_index);
        (handler.save_dir().to_path_buf(), player_index)
    };
    let graph_result = tokio::task::spawn_blocking(move || {
        let handler = SaveGameHandler::new(&save_dir).map_err(|e| e.to_string())?;
        let progress_sink = |snapshot: QuestGraphProgress| {
            *progress_handle.write() = snapshot;
        };
        save_graph::build(BuildContext {
            handler: &handler,
            paths: &paths,
            client: &client,
            player_index,
            current_module: &module_info,
            current_module_vars: &module_vars,
            progress: Some(&progress_sink),
        })
    })
    .await
    .map_err(|e| CommandError::from(format!("Quest graph build failed: {e}")))?;

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

    store_quest_graph(state, generation, graph)
}

/// Caches a built graph unless the save changed during the build; if another
/// build stored first, that one wins.
fn store_quest_graph(
    state: &State<'_, AppState>,
    generation: u64,
    graph: SaveGraph,
) -> CommandResult<Arc<SaveGraph>> {
    let mut session = state.session.write();
    if session.load_generation != generation {
        return Err(CommandError::from(
            "The save changed while the quest graph was being built".to_string(),
        ));
    }
    if let Some(existing) = &session.quest_graph_cache {
        return Ok(existing.clone());
    }
    let shared = Arc::new(graph);
    session.quest_graph_cache = Some(shared.clone());
    Ok(shared)
}

fn degraded_graph(
    state: &State<'_, AppState>,
    module_info: &crate::services::campaign::content::ModuleInfo,
    module_vars: &crate::services::campaign::content::ModuleVariables,
    kind: OrphanKind,
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
        kind,
        explanation,
    ))
}
