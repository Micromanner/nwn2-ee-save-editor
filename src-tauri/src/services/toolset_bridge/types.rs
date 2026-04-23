use serde::{Deserialize, Serialize};

/// Shape of `list-modules` output from the bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignModules {
    pub campaign_path: String,
    pub campaign_file: String,
    pub display_name: String,
    pub start_module: String,
    pub journal_synch: bool,
    pub modules: Vec<CampaignModuleEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignModuleEntry {
    pub name: String,
    pub resolved_path: String,
    pub resolution_kind: ResolutionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResolutionKind {
    /// File found in the campaign folder (overrides the install copy).
    Campaign,
    /// Canonical `<install>/modules/<name>.mod`.
    Install,
    /// File not found — bridge could not map the name to disk.
    Unresolved,
}
