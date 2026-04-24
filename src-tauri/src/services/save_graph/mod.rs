//! Save-level quest-graph aggregation.
//!
//! Orchestrates the toolset bridge across every module in a save's owning campaign,
//! merges per-module journal/convo reference data, and overlays live save state
//! (player's VarTable-backed journal, `globals.xml`, current module `VarTable`).
//! The output (`SaveGraph`) is the single payload the quest-state editor UI renders.

mod aggregator;
mod journal_reader;
mod types;

pub use aggregator::{BuildContext, build};
pub use types::{
    AggregatedModule, CampaignSummary, LiveModuleVar, ModuleVarValue, OrphanKind, OrphanNote,
    QuestAggregate, QuestSummary, SaveGraph, SaveGraphSummary, TransitionNode,
};
