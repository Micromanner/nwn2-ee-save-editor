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

/// Typed mirror of the C# `BridgeOutput` (see `toolset-bridge/Schema/BridgeOutput.cs`).
/// Produced by `graph <module>`; combines journal definitions, faction matrix, module
/// variable declarations, and conversation-node functor map into one payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleGraph {
    pub module: GraphModuleInfo,
    pub journal: JournalData,
    pub factions: Vec<Faction>,
    pub module_variables: Vec<ModuleVariable>,
    pub convo: ConvoGraph,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphModuleInfo {
    pub path: String,
    pub name: String,
    pub haks: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JournalData {
    #[serde(default)]
    pub categories: Vec<JournalCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalCategory {
    pub tag: String,
    pub name: String,
    pub priority: String,
    pub xp: i32,
    pub source: String,
    #[serde(default)]
    pub entries: Vec<JournalEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: i32,
    pub text: String,
    #[serde(rename = "final")]
    pub is_final: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Faction {
    pub id: i32,
    pub name: String,
    pub parent_id: i32,
    #[serde(default)]
    pub reputations: Vec<FactionRep>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionRep {
    pub faction_id: i32,
    pub rep: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleVariable {
    pub name: String,
    #[serde(rename = "type")]
    pub var_type: String,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    pub storage: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConvoGraph {
    #[serde(default)]
    pub nodes: Vec<ConvoNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvoNode {
    pub dlg: String,
    pub node: i32,
    pub speaker: String,
    pub text_strref: i32,
    #[serde(default)]
    pub actions: Vec<ConvoFunctor>,
    #[serde(default)]
    pub conditions: Vec<ConvoFunctor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvoFunctor {
    pub kind: FunctorKind,
    pub script: String,
    #[serde(default)]
    pub params: Vec<serde_json::Value>,
}

/// Classification of an Action or Condition functor on a conversation node.
/// The bridge emits the lowercase snake_case form; `Custom` catches any unrecognized
/// wrapper (including module-specific wrappers that the classifier doesn't know).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctorKind {
    Journal,
    ModuleLocal,
    GlobalInt,
    GlobalString,
    GlobalFloat,
    GlobalBool,
    #[serde(other)]
    Custom,
}

impl FunctorKind {
    /// Whether this kind sets/reads a value on `globals.xml`.
    pub fn is_global(self) -> bool {
        matches!(
            self,
            Self::GlobalInt | Self::GlobalString | Self::GlobalFloat | Self::GlobalBool
        )
    }
}
