use serde::Serialize;

use crate::parsers::xml::XmlData;
use crate::services::toolset_bridge::{ConvoFunctor, JournalCategory, ResolutionKind};

/// Full aggregated graph kept server-side (cached in `SessionState`). Per-quest
/// `transitions` can balloon into multi-MB payloads across a long campaign, so the
/// IPC response uses the `SaveGraphSummary` projection and clients fetch
/// transitions on demand via `save_get_quest_transitions`.
#[derive(Debug, Clone, Serialize)]
pub struct SaveGraph {
    pub campaign: CampaignSummary,
    pub modules: Vec<AggregatedModule>,
    pub quests: Vec<QuestAggregate>,
    pub globals: XmlData,
    pub current_module_variables: Vec<LiveModuleVar>,
    pub orphans: Vec<OrphanNote>,
}

/// IPC-facing projection of `SaveGraph` that drops per-quest transition arrays in
/// favor of `transition_count`. Keeps the initial Quests-tab payload to a size the
/// webview can deserialize without stalling.
#[derive(Debug, Clone, Serialize)]
pub struct SaveGraphSummary {
    pub campaign: CampaignSummary,
    pub modules: Vec<AggregatedModule>,
    pub quests: Vec<QuestSummary>,
    pub globals: XmlData,
    pub current_module_variables: Vec<LiveModuleVar>,
    pub orphans: Vec<OrphanNote>,
}

impl From<&SaveGraph> for SaveGraphSummary {
    fn from(g: &SaveGraph) -> Self {
        let quests = g
            .quests
            .iter()
            .map(|q| QuestSummary {
                tag: q.tag.clone(),
                category: q.category.clone(),
                defined_in: q.defined_in.clone(),
                live_state: q.live_state,
                transition_count: q.transitions.len(),
            })
            .collect();
        Self {
            campaign: g.campaign.clone(),
            modules: g.modules.clone(),
            quests,
            globals: g.globals.clone(),
            current_module_variables: g.current_module_variables.clone(),
            orphans: g.orphans.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct QuestSummary {
    pub tag: String,
    pub category: JournalCategory,
    pub defined_in: Vec<String>,
    pub live_state: Option<u32>,
    pub transition_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CampaignSummary {
    pub campaign_id: String,
    pub campaign_path: Option<String>,
    pub display_name: String,
    pub start_module: String,
    pub journal_synch: bool,
    pub current_module_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AggregatedModule {
    pub name: String,
    pub resolved_path: String,
    pub resolution_kind: ResolutionKind,
    pub is_current: bool,
    /// Quest tags this module declares in its journal. Ordered by first-seen.
    pub journal_category_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuestAggregate {
    pub tag: String,
    /// Category metadata from the first module that declares this tag. Later
    /// re-declarations are ignored for display; the `defined_in` list records them.
    pub category: JournalCategory,
    pub defined_in: Vec<String>,
    /// Current state int from the player's VarTable (`NW_JOURNAL_ENTRY<tag>`).
    /// `None` means the quest hasn't been touched yet.
    pub live_state: Option<u32>,
    pub transitions: Vec<TransitionNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransitionNode {
    pub module: String,
    pub dlg: String,
    pub node: i32,
    pub new_state: u32,
    /// Conversation-node `text_strref` from the bridge's `ConvoNode` (stock
    /// `dialog.tlk` only; caller resolves via `tlk_get_strings`). `None` when the
    /// underlying node's strref is invalid (bridge emits `-1`).
    pub text_strref: Option<i32>,
    /// Same-node actions whose `kind` is one of the `global_*` variants.
    pub co_authored_globals: Vec<ConvoFunctor>,
    /// Same-node actions with `kind = module_local`.
    pub co_authored_locals: Vec<ConvoFunctor>,
    /// Full condition list on the node — whatever gates entry into this transition.
    pub gating_conditions: Vec<ConvoFunctor>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveModuleVar {
    pub module_id: String,
    pub name: String,
    pub value: ModuleVarValue,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
pub enum ModuleVarValue {
    Int(i32),
    Float(f32),
    String(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct OrphanNote {
    pub kind: OrphanKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OrphanKind {
    UnresolvedCampaign,
    UnresolvedModule,
    GraphFailed,
    JournalReadFailed,
}
