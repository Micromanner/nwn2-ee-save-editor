//! Save-graph aggregation: fan out across every module in the save's owning
//! campaign, merge per-module reference graphs with live save state (player journal,
//! `globals.xml`, current module's `VarTable`), and emit one `SaveGraph` blob.

use std::collections::{HashMap, HashSet};

use tracing::{debug, info, warn};

use crate::config::NWN2Paths;
use crate::parsers::xml::XmlData;
use crate::services::campaign::CampaignManager;
use crate::services::campaign::content::{ModuleInfo, ModuleVariables, find_campaign_path};
use crate::services::savegame_handler::SaveGameHandler;
use crate::services::toolset_bridge::{
    BridgeClient, ConvoFunctor, ConvoNode, FunctorKind, JournalCategory, ResolutionKind,
};

use super::journal_reader::read_live_journal;
use super::module_name::read_module_display_name;
use super::types::{
    AggregatedModule, CampaignSummary, LiveModuleVar, ModuleVarValue, OrphanKind, OrphanNote,
    QuestAggregate, QuestGraphProgress, SaveGraph, TransitionNode,
};

/// Synthetic functor script name the bridge emits for `NWN2ConversationConnector.Quest`
/// tuples — these are first-class quest transitions declared on the node itself,
/// independent of any `ga_journal` action in the Actions list.
const CONNECTOR_QUEST_TUPLE: &str = "__connector_quest_tuple__";

/// `JournalCategory.source` values emitted when the aggregator has to fabricate a
/// category for a quest tag the bridge never declared. Distinct from the bridge's
/// own `"campaign"`/`"module"` sources so the UI can render these rows differently.
const SOURCE_LIVE_ONLY: &str = "live_only";
const SOURCE_TRANSITION_ONLY: &str = "transition_only";

pub struct BuildContext<'a> {
    pub handler: &'a SaveGameHandler,
    pub paths: &'a NWN2Paths,
    pub client: &'a BridgeClient,
    pub player_index: usize,
    pub current_module: &'a ModuleInfo,
    pub current_module_vars: &'a ModuleVariables,
    /// Optional sink invoked at each build milestone. The command layer wires
    /// this into a shared `Arc<RwLock<QuestGraphProgress>>` so the Quests tab
    /// can poll it. `None` for tests and headless callers.
    pub progress: Option<&'a dyn Fn(QuestGraphProgress)>,
}

pub fn build(ctx: BuildContext<'_>) -> Result<SaveGraph, String> {
    let current_module_id = ctx.current_module.current_module.clone();
    let campaign_id = ctx.current_module.campaign_id.clone();

    let report = |step: &str, progress: f32, message: String| {
        debug!("save_graph progress: step={step} progress={progress:.1} message={message}");
        if let Some(sink) = ctx.progress {
            sink(QuestGraphProgress {
                step: step.to_string(),
                progress,
                message,
            });
        }
    };

    report("starting", 0.0, "Resolving campaign…".to_string());

    let mut orphans = Vec::new();

    let campaign_cam_path = if campaign_id.is_empty() {
        None
    } else {
        find_campaign_path(&campaign_id, ctx.paths)
    };

    if !campaign_id.is_empty() && campaign_cam_path.is_none() {
        orphans.push(OrphanNote {
            kind: OrphanKind::UnresolvedCampaign,
            message: format!(
                "Campaign_ID {campaign_id} could not be mapped to a campaign.cam (install: {}, user: {})",
                ctx.paths
                    .campaigns()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "<unset>".into()),
                ctx.paths
                    .user_campaigns()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "<unset>".into())
            ),
        });
    }

    let (campaign_summary, module_entries) = match campaign_cam_path.as_deref() {
        Some(cam_path) => match ctx.client.list_modules(cam_path) {
            Ok(list) => {
                let summary = CampaignSummary {
                    campaign_id: campaign_id.clone(),
                    campaign_path: Some(list.campaign_path.clone()),
                    display_name: list.display_name.clone(),
                    start_module: list.start_module.clone(),
                    journal_synch: list.journal_synch,
                    current_module_id: current_module_id.clone(),
                };
                (summary, list.modules)
            }
            Err(e) => {
                warn!("list_modules failed for {}: {e}", cam_path.display());
                orphans.push(OrphanNote {
                    kind: OrphanKind::GraphFailed,
                    message: format!("list_modules failed: {e}"),
                });
                (
                    single_module_fallback(&campaign_id, &current_module_id),
                    Vec::new(),
                )
            }
        },
        None => (
            single_module_fallback(&campaign_id, &current_module_id),
            Vec::new(),
        ),
    };

    report("campaign", 5.0, "Listing modules…".to_string());

    let mut quests: HashMap<String, QuestAggregate> = HashMap::new();
    let mut modules_out: Vec<AggregatedModule> = Vec::with_capacity(module_entries.len());

    let module_total = module_entries.len();
    for (idx, entry) in module_entries.iter().enumerate() {
        let module_progress = if module_total == 0 {
            85.0
        } else {
            5.0 + ((idx + 1) as f32 / module_total as f32) * 80.0
        };

        let display_name = if entry.resolution_kind == ResolutionKind::Unresolved {
            String::new()
        } else {
            read_module_display_name(std::path::Path::new(&entry.resolved_path)).unwrap_or_default()
        };
        let label = if display_name.is_empty() {
            entry.name.as_str()
        } else {
            display_name.as_str()
        };
        report(
            "modules",
            module_progress,
            format!("Loading {} ({} of {})", label, idx + 1, module_total),
        );
        if entry.resolution_kind == ResolutionKind::Unresolved {
            orphans.push(OrphanNote {
                kind: OrphanKind::UnresolvedModule,
                message: format!(
                    "Module '{}' referenced by campaign.cam but not found on disk",
                    entry.name
                ),
            });
            modules_out.push(AggregatedModule {
                name: entry.name.clone(),
                display_name,
                resolved_path: entry.resolved_path.clone(),
                resolution_kind: entry.resolution_kind,
                is_current: names_match(&entry.name, &current_module_id),
                journal_category_tags: Vec::new(),
            });
            continue;
        }

        let graph = match ctx.client.graph(std::path::Path::new(&entry.resolved_path)) {
            Ok(g) => g,
            Err(e) => {
                warn!(
                    "bridge graph failed for module {} ({}): {e}",
                    entry.name, entry.resolved_path
                );
                orphans.push(OrphanNote {
                    kind: OrphanKind::GraphFailed,
                    message: format!("graph({}) failed: {e}", entry.name),
                });
                modules_out.push(AggregatedModule {
                    name: entry.name.clone(),
                    display_name,
                    resolved_path: entry.resolved_path.clone(),
                    resolution_kind: entry.resolution_kind,
                    is_current: names_match(&entry.name, &current_module_id),
                    journal_category_tags: Vec::new(),
                });
                continue;
            }
        };

        let mut seen_tags: HashSet<&str> = HashSet::with_capacity(graph.journal.categories.len());
        let mut tags_in_module: Vec<String> = Vec::with_capacity(graph.journal.categories.len());
        for category in &graph.journal.categories {
            if seen_tags.insert(category.tag.as_str()) {
                tags_in_module.push(category.tag.clone());
            }
            merge_category(&mut quests, category, &entry.name);
        }
        for node in &graph.convo.nodes {
            collect_transitions(node, &entry.name, &mut quests);
        }

        modules_out.push(AggregatedModule {
            name: entry.name.clone(),
            display_name,
            resolved_path: entry.resolved_path.clone(),
            resolution_kind: entry.resolution_kind,
            is_current: names_match(&entry.name, &current_module_id),
            journal_category_tags: tags_in_module,
        });
    }

    report("journal", 90.0, "Reading live journal…".to_string());

    let live = match read_live_journal(ctx.handler, ctx.player_index) {
        Ok(entries) => entries,
        Err(e) => {
            warn!("live journal read failed: {e}");
            orphans.push(OrphanNote {
                kind: OrphanKind::JournalReadFailed,
                message: e,
            });
            Vec::new()
        }
    };
    for entry in &live {
        quests
            .entry(entry.tag.clone())
            .and_modify(|q| q.live_state = Some(entry.state))
            .or_insert_with(|| QuestAggregate {
                tag: entry.tag.clone(),
                category: fabricated_category(&entry.tag, SOURCE_LIVE_ONLY),
                defined_in: Vec::new(),
                live_state: Some(entry.state),
                transitions: Vec::new(),
            });
    }

    report("globals", 95.0, "Reading campaign globals…".to_string());

    let globals = CampaignManager::get_campaign_variables(ctx.handler).unwrap_or_else(|e| {
        warn!("globals.xml overlay unavailable: {e}");
        orphans.push(OrphanNote {
            kind: OrphanKind::GraphFailed,
            message: format!("globals.xml read failed: {e}"),
        });
        XmlData::default()
    });

    let current_module_variables = flatten_module_vars(&current_module_id, ctx.current_module_vars);

    let mut quests_out: Vec<QuestAggregate> = quests.into_values().collect();
    quests_out.sort_by(|a, b| a.tag.cmp(&b.tag));

    info!(
        "save_graph built: {} modules, {} quests, {} live journal entries",
        modules_out.len(),
        quests_out.len(),
        live.len()
    );

    report("ready", 100.0, "Done".to_string());

    Ok(SaveGraph {
        campaign: campaign_summary,
        modules: modules_out,
        quests: quests_out,
        globals,
        current_module_variables,
        orphans,
    })
}

fn single_module_fallback(campaign_id: &str, current_module_id: &str) -> CampaignSummary {
    CampaignSummary {
        campaign_id: campaign_id.to_string(),
        campaign_path: None,
        display_name: String::new(),
        start_module: current_module_id.to_string(),
        journal_synch: false,
        current_module_id: current_module_id.to_string(),
    }
}

/// NWN2 modules live under `<install>/modules/<name>.mod`. `campaign.cam` records
/// module names without extension, while `currentmodule.txt` is a lowercase file
/// stem. Compare case-insensitively.
fn names_match(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn fabricated_category(tag: &str, source: &str) -> JournalCategory {
    JournalCategory {
        tag: tag.to_string(),
        name: tag.to_string(),
        priority: String::new(),
        xp: 0,
        source: source.to_string(),
        entries: Vec::new(),
    }
}

fn merge_category(
    quests: &mut HashMap<String, QuestAggregate>,
    category: &JournalCategory,
    module_name: &str,
) {
    quests
        .entry(category.tag.clone())
        .and_modify(|existing| {
            if !existing.defined_in.iter().any(|m| m == module_name) {
                existing.defined_in.push(module_name.to_string());
            }
        })
        .or_insert_with(|| QuestAggregate {
            tag: category.tag.clone(),
            category: category.clone(),
            defined_in: vec![module_name.to_string()],
            live_state: None,
            transitions: Vec::new(),
        });
}

fn collect_transitions(
    node: &ConvoNode,
    module_name: &str,
    quests: &mut HashMap<String, QuestAggregate>,
) {
    // Partition once per node — every transition on this node reuses the same
    // co-authored snapshot, so avoid N×2 walks of node.actions inside the loop.
    let mut co_globals: Vec<ConvoFunctor> = Vec::new();
    let mut co_locals: Vec<ConvoFunctor> = Vec::new();
    for action in &node.actions {
        if action.kind.is_global() {
            co_globals.push(action.clone());
        } else if action.kind == FunctorKind::ModuleLocal {
            co_locals.push(action.clone());
        }
    }

    // Bridge emits -1 as "no valid strref"; normalize to None for the UI.
    let text_strref = if node.text_strref < 0 {
        None
    } else {
        Some(node.text_strref)
    };

    for action in &node.actions {
        let Some((tag, new_state)) = extract_journal_transition(action) else {
            continue;
        };
        let entry = quests.entry(tag.clone()).or_insert_with(|| QuestAggregate {
            tag: tag.clone(),
            category: fabricated_category(&tag, SOURCE_TRANSITION_ONLY),
            defined_in: Vec::new(),
            live_state: None,
            transitions: Vec::new(),
        });
        entry.transitions.push(TransitionNode {
            module: module_name.to_string(),
            dlg: node.dlg.clone(),
            node: node.node,
            new_state,
            text_strref,
            co_authored_globals: co_globals.clone(),
            co_authored_locals: co_locals.clone(),
            gating_conditions: node.conditions.clone(),
        });
    }
}

/// `ga_journal(tag, state)` or a synthetic `__connector_quest_tuple__` entry.
/// Both carry the quest tag as `params[0]` (string) and state int as `params[1]`.
fn extract_journal_transition(action: &ConvoFunctor) -> Option<(String, u32)> {
    let is_journal_kind = action.kind == FunctorKind::Journal;
    let is_connector_tuple = action.script == CONNECTOR_QUEST_TUPLE;
    if !is_journal_kind && !is_connector_tuple {
        return None;
    }
    let tag = action.params.first()?.as_str()?.to_string();
    if tag.is_empty() {
        return None;
    }
    let state = action.params.get(1).and_then(param_as_u32)?;
    Some((tag, state))
}

fn param_as_u32(v: &serde_json::Value) -> Option<u32> {
    if let Some(n) = v.as_i64() {
        if n < 0 { None } else { Some(n as u32) }
    } else {
        v.as_u64().map(|n| n as u32)
    }
}

fn flatten_module_vars(module_id: &str, vars: &ModuleVariables) -> Vec<LiveModuleVar> {
    let int_iter = vars
        .integers
        .iter()
        .map(|(n, v)| (n.clone(), ModuleVarValue::Int(*v)));
    let float_iter = vars
        .floats
        .iter()
        .map(|(n, v)| (n.clone(), ModuleVarValue::Float(*v)));
    let string_iter = vars
        .strings
        .iter()
        .map(|(n, v)| (n.clone(), ModuleVarValue::String(v.clone())));

    int_iter
        .chain(float_iter)
        .chain(string_iter)
        .map(|(name, value)| LiveModuleVar {
            module_id: module_id.to_string(),
            name,
            value,
        })
        .collect()
}

/// Build a degraded `SaveGraph` for environments where the toolset bridge can't
/// run — original NWN2 installs (32-bit toolset DLLs that our 64-bit
/// bridge can't load) and non-Windows platforms. The graph keeps the live
/// state we can read without the bridge (globals.xml, current module variables)
/// and emits a single `BridgeUnavailable` orphan note explaining the gap.
pub fn build_without_bridge(
    handler: &SaveGameHandler,
    current_module: &ModuleInfo,
    current_module_vars: &ModuleVariables,
    explanation: String,
) -> SaveGraph {
    let current_module_id = current_module.current_module.clone();
    let campaign_id = current_module.campaign_id.clone();

    let globals = CampaignManager::get_campaign_variables(handler).unwrap_or_else(|e| {
        warn!("globals.xml overlay unavailable: {e}");
        XmlData::default()
    });

    SaveGraph {
        campaign: single_module_fallback(&campaign_id, &current_module_id),
        modules: Vec::new(),
        quests: Vec::new(),
        globals,
        current_module_variables: flatten_module_vars(&current_module_id, current_module_vars),
        orphans: vec![OrphanNote {
            kind: OrphanKind::BridgeUnavailable,
            message: explanation,
        }],
    }
}
