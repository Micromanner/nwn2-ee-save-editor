use serde::Serialize;

use crate::parsers::xml::XmlData;
use crate::services::toolset_bridge::{ConvoFunctor, JournalCategory, ResolutionKind};

/// One-shot aggregated view of everything the quest-state editor UI needs:
/// campaign identity, every module the save touches, per-quest transition graph,
/// and live overlay of the save's current state.
#[derive(Debug, Clone, Serialize)]
pub struct SaveGraph {
    pub campaign: CampaignSummary,
    pub modules: Vec<AggregatedModule>,
    pub quests: Vec<QuestAggregate>,
    pub globals: XmlData,
    pub current_module_variables: Vec<LiveModuleVar>,
    pub orphans: Vec<OrphanNote>,
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
