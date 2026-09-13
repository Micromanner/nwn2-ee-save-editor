//! Save-level quest-graph aggregation.
//!
//! Orchestrates the toolset bridge across every module in a save's owning campaign,
//! merges per-module journal/convo reference data, and overlays live save state
//! (player's VarTable-backed journal, `globals.xml`, current module `VarTable`).
//! The output (`SaveGraph`) is the single payload the quest-state editor UI renders.

mod aggregator;
pub mod editing;
mod journal_reader;
pub use journal_reader::read_live_journal;
mod module_name;
pub mod script_decoder;
mod script_source;
mod types;

pub use aggregator::{BuildContext, build, build_without_bridge};
pub use types::{
    AggregatedModule, CampaignSummary, FollowNode, LiveModuleVar, ModuleVarValue, OrphanKind,
    OrphanNote, QuestAggregate, QuestGraphProgress, QuestSummary, SaveGraph, SaveGraphSummary,
    TransitionNode,
};
