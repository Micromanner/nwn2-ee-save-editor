export type ResolutionKind = 'campaign' | 'install' | 'unresolved';

export type OrphanKind =
  | 'unresolved_campaign'
  | 'unresolved_module'
  | 'graph_failed'
  | 'journal_read_failed';

export type FunctorKind =
  | 'journal'
  | 'module_local'
  | 'global_int'
  | 'global_string'
  | 'global_float'
  | 'global_bool'
  | 'custom';

export interface JournalEntry {
  id: number;
  text: string;
  final: boolean;
}

export interface JournalCategory {
  tag: string;
  name: string;
  priority: string;
  xp: number;
  source: string;
  entries: JournalEntry[];
}

export interface ConvoFunctor {
  kind: FunctorKind;
  script: string;
  params: unknown[];
}

export interface TransitionNode {
  module: string;
  dlg: string;
  node: number;
  new_state: number;
  text_strref: number | null;
  co_authored_globals: ConvoFunctor[];
  co_authored_locals: ConvoFunctor[];
  gating_conditions: ConvoFunctor[];
}

/// Lightweight per-quest row returned by `save_get_quest_graph`. Transitions
/// are omitted to keep the initial payload small; fetch them on demand via
/// `save_get_quest_transitions`.
export interface QuestSummary {
  tag: string;
  category: JournalCategory;
  defined_in: string[];
  live_state: number | null;
  transition_count: number;
}

export interface AggregatedModule {
  name: string;
  display_name: string;
  resolved_path: string;
  resolution_kind: ResolutionKind;
  is_current: boolean;
  journal_category_tags: string[];
}

export interface CampaignSummary {
  campaign_id: string;
  campaign_path: string | null;
  display_name: string;
  start_module: string;
  journal_synch: boolean;
  current_module_id: string;
}

export type ModuleVarValue =
  | { kind: 'int'; value: number }
  | { kind: 'float'; value: number }
  | { kind: 'string'; value: string };

export interface LiveModuleVar {
  module_id: string;
  name: string;
  value: ModuleVarValue;
}

export interface OrphanNote {
  kind: OrphanKind;
  message: string;
}

export interface XmlData {
  integers: Record<string, number>;
  booleans: Record<string, number>;
  floats: Record<string, number>;
  strings: Record<string, string>;
}

export interface SaveGraph {
  campaign: CampaignSummary;
  modules: AggregatedModule[];
  quests: QuestSummary[];
  globals: XmlData;
  current_module_variables: LiveModuleVar[];
  orphans: OrphanNote[];
}
