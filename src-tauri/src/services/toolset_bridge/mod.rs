//! Thin client for the `toolset-bridge.exe` .NET helper.
//!
//! Spawns the bridge to extract read-only reference data (journal categories, faction
//! matrix, conversation nodes with classified functors, module variable declarations)
//! from an authored `.mod` file or from a `campaign.cam` manifest. Output is cached
//! per-module on disk to avoid re-running the bridge when neither the module file nor
//! the bridge binary has changed.
//!
//! Scope contract (matches the bridge itself):
//! - Never writes.
//! - Never opens a save. Save-side state (live journal, globals) is handled elsewhere.
//! - Read-only; output is reference data about the module author's design-time content.

mod cache;
mod client;
mod types;

pub use client::{BridgeClient, BridgeError, BridgeResult};
pub use types::{CampaignModuleEntry, CampaignModules, ResolutionKind};
