export type ResolutionKind = 'campaign' | 'install' | 'unresolved';

export type OrphanKind =
  | 'unresolved_campaign'
  | 'unresolved_module'
  | 'graph_failed'
  | 'journal_read_failed'
  | 'enhanced_edition_required'
  | 'platform_unsupported';

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

export interface DecodedWrite {
  module_local: boolean;
  value_type: string;
  name: string;
  value: string;
}

/** Static decode of a custom script's branch (`save_graph::script_decoder`). */
export interface ScriptDecode {
  writes: DecodedWrite[];
  conditional_writes: DecodedWrite[];
  calls: string[];
  unresolved: string[];
}

export interface ConvoFunctor {
  kind: FunctorKind;
  script: string;
  params: unknown[];
  /** Wrapper script call this functor was synthesized from, e.g. `10a_caleb_state(1)`. */
  derived_from?: string;
  decoded?: ScriptDecode;
  /** Conditions only: toolset "Not" checkbox. */
  negate?: boolean;
  /** Conditions only: "and" / "or" with the next condition. */
  join?: string;
}

/** A line reachable after a transition without another player decision. */
export interface FollowNode {
  node: number;
  is_player: boolean;
  has_text: boolean;
  speaker: string;
  text_strref: number | null;
  conditions: ConvoFunctor[];
  actions: ConvoFunctor[];
  children: number[];
}

export interface TransitionNode {
  actions: ConvoFunctor[];
  module: string;
  dlg: string;
  node: number;
  new_state: number;
  text_strref: number | null;
  co_authored_globals: ConvoFunctor[];
  co_authored_locals: ConvoFunctor[];
  gating_conditions: ConvoFunctor[];
  is_player: boolean;
  debug: boolean;
  speaker: string;
  children: number[];
  follow_up: FollowNode[];
}

/// Lightweight per-quest row returned by `save_get_quest_graph`. Transitions
/// are omitted to keep the initial payload small; fetch them on demand via
/// `save_get_quest_transitions`.
export interface QuestSummary {
  tag: string;
  category: JournalCategory;
  defined_in: string[];
  live_state: number | null;
  pending_state?: number;
  transition_count: number;
}

export interface VariableKey {
  module: string | null;
  name: string;
  value_type: string;
}

export interface VariableWrite {
  key: VariableKey;
  saved: string | number | null;
  before: string | number | null;
  after: string | number;
  action_index: number;
  derived_from?: string;
  /** Node id of the follow-up line this write comes from; absent for the chosen line. */
  line?: number;
}

/** `save_summarize_quest_routes`: one line reaching the target entry, with its outcome signature. */
export interface RouteSummary {
  route: number;
  module: string;
  speaker: string;
  is_player: boolean;
  text_strref: number | null;
  debug: boolean;
  signature: string;
  writes: { name: string; after: string | number }[];
  write_count: number;
  followed_count: number;
}

export interface FollowedLine {
  node: number;
  is_player: boolean;
  has_text: boolean;
  speaker: string;
  text_strref: number | null;
}

export interface FollowStop {
  reason: 'end' | 'player_choice' | 'unknown_condition' | 'depth';
  node?: number;
  conditions?: ConvoFunctor[];
  choices: number;
}

export interface QuestPreview {
  revision: string;
  tag: string;
  target: number;
  saved_state: number | null;
  before_state: number | null;
  route: number | null;
  writes: VariableWrite[];
  effects: { action: ConvoFunctor; reason: string; line?: number }[];
  followed?: FollowedLine[];
  follow_stop?: FollowStop;
}

export interface PendingQuestEdit {
  tag: string;
  target: number;
  saved_state: number | null;
  source: TransitionNode | null;
  writes: VariableWrite[];
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
