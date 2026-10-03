/**
 * Character State API - Aggregated state commands
 *
 * This API uses the new aggregated state pattern where each screen/tab
 * gets all its data in a single call, and batch updates are supported.
 *
 * See RUST_API_ARCHITECTURE.md for details.
 */

import { invoke } from '@tauri-apps/api/core';
import type {
  OverviewState,
  AbilitiesState,
  AppearanceState,
  AppearanceOption,
  AppearanceUpdates,
  VoiceSetInfo,
  ClassesState,
  FeatsState,
  SpellsState,
  SkillsState,
  CombatSummary,
  SaveSummary,
  FullInventorySummary,
  CharacterUpdates,
  NaturalArmorChange,
  UndoResult,
  HistoryState,
  CharacterHistory,
} from '../bindings';

export interface RosterClassInfo {
  class_id: number;
  level: number;
}

export interface RosterEntryInfo {
  ros_name: string;
  char_name: string;
  classes: RosterClassInfo[];
  in_party: boolean;
  staged_in_party: boolean | null;
  gender: number | null;
  subrace_name: string | null;
}

export interface PartyMembershipChange {
  ros_name: string;
  in_party: boolean;
}

export interface SaveCharacterResult {
  saved: boolean;
  warning: string | null;
}

/**
 * Character State API - uses aggregated state commands
 */
export const CharacterStateAPI = {
  // ===========================================================================
  // Aggregated State Getters (1 call per screen)
  // ===========================================================================

  /**
   * Get abilities state including base/effective scores, modifiers, HP, encumbrance.
   */
  getAbilities: () => invoke<AbilitiesState>('get_abilities_state'),

  /**
   * Get appearance state (head, colors, phenotype, wings, tail, soundset).
   */
  getAppearance: () => invoke<AppearanceState>('get_appearance_state'),

  /**
   * Get classes state including level history (LevelStatList from GFF).
   */
  getClasses: () => invoke<ClassesState>('get_classes_state'),

  /**
   * Get feats state including feat slots and domains.
   */
  getFeats: () => invoke<FeatsState>('get_feats_state'),

  /**
   * Get spells state for spellcasting characters.
   */
  getSpells: () => invoke<SpellsState>('get_spells_state'),

  /**
   * Get skills state with class/cross-class separation.
   */
  getSkills: () => invoke<SkillsState>('get_skills_state'),

  /**
   * Get combat summary (AC, attack bonuses, saves, HP).
   */
  getCombat: () => invoke<CombatSummary>('get_combat_summary'),

  /**
   * Get save summary (saving throws + HP).
   */
  getSaves: () => invoke<SaveSummary>('get_save_summary'),

  /**
   * Get full inventory summary (items, equipped, gold, encumbrance).
   */
  getInventory: () => invoke<FullInventorySummary>('get_inventory_summary'),

  // ===========================================================================
  // Batch Update Commands
  // ===========================================================================

  /**
   * Update multiple character fields at once.
   * Returns the updated OverviewState.
   */
  updateCharacter: (updates: CharacterUpdates) =>
    invoke<OverviewState>('update_character', { updates }),

  /**
   * Apply point buy scores - resets level-up ability history and sets new base scores.
   * Returns the updated AbilitiesState.
   */
  applyPointBuy: (newScores: import('../bindings').AbilityScores) =>
    invoke<AbilitiesState>('apply_point_buy', { newScores }),

  /**
   * Update appearance fields. Returns the updated AppearanceState.
   */
  updateAppearance: (updates: AppearanceUpdates) =>
    invoke<AppearanceState>('update_appearance', { updates }),

  /**
   * Get available wing options from wingmodel.2da.
   */
  getAvailableWings: () => invoke<AppearanceOption[]>('get_available_wings'),

  /**
   * Get available tail options from tailmodel.2da.
   */
  getAvailableTails: () => invoke<AppearanceOption[]>('get_available_tails'),

  /**
   * Every parts-based appearance.2da row with a head template, including the
   * companion/NPC rows no playable race points at.
   */
  getPartsBasedAppearances: () => invoke<AppearanceOption[]>('get_parts_based_appearances'),

  getAvailableVoicesets: () => invoke<VoiceSetInfo[]>('get_available_voicesets'),

  previewVoiceset: (resref: string) => invoke<number[]>('preview_voiceset', { resref }),

  // ===========================================================================
  // Session Management
  // ===========================================================================

  /**
   * List all roster (companion) entries available for the loaded module.
   */
  listRoster: () => invoke<RosterEntryInfo[]>('list_roster'),

  /**
   * Companion cap of the party screen (`RosPartyLimit`), null when absent.
   */
  getPartyLimit: () => invoke<number | null>('get_party_limit'),

  /**
   * Stage the desired party composition in the session; written to the save
   * on the next Save. Staging a member back to their on-disk state un-stages.
   */
  stagePartyMembership: (changes: PartyMembershipChange[]) =>
    invoke<boolean>('stage_party_membership', { changes }),

  /**
   * Load a companion from the roster into the active editor session.
   */
  loadCompanion: (rosName: string) => invoke<boolean>('load_companion', { rosName }),

  /**
   * Load the player character back into the active editor session.
   */
  loadPlayer: () => invoke<boolean>('load_player'),

  /**
   * Close the current character.
   */
  closeCharacter: () => invoke<boolean>('close_character'),

  /**
   * Check if there are unsaved changes.
   */
  hasUnsavedChanges: () => invoke<boolean>('has_unsaved_changes'),

  // Combat
  updateNaturalArmor: (value: number) =>
    invoke<NaturalArmorChange>('update_natural_armor', { value }),

  // History
  getHistoryState: () => invoke<HistoryState>('get_history_state'),
  getSessionHistory: () => invoke<CharacterHistory[]>('get_session_history'),
  undo: () => invoke<UndoResult>('undo'),
  redo: () => invoke<UndoResult>('redo'),
};

export default CharacterStateAPI;
