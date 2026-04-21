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
  AbilitiesUpdates,
  SessionInfo,
  SkillChangeResult,
  SpellChangeResult,
  EquipResult,
  UnequipResult,
  AddItemResult,
  RemoveItemResult,
  NaturalArmorChange,
  InitiativeChange,
  SaveChange,
  HitPoints,
  EquipmentSlot,
  FeatId,
  SpellId,
  SkillId,
} from '../bindings';

/**
 * Character State API - uses aggregated state commands
 */
export const CharacterStateAPI = {
  // ===========================================================================
  // Aggregated State Getters (1 call per screen)
  // ===========================================================================

  /**
   * Get complete overview state for the character overview screen.
   * Includes identity, classes, HP, AC, BAB, saves, gold, domains.
   */
  getOverview: () => invoke<OverviewState>('get_overview_state'),

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
   * Update multiple ability scores at once.
   * Returns the updated AbilitiesState.
   */
  updateAbilities: (updates: AbilitiesUpdates) =>
    invoke<AbilitiesState>('update_abilities', { updates }),

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

  getAvailableVoicesets: () => invoke<VoiceSetInfo[]>('get_available_voicesets'),

  previewVoiceset: (resref: string) => invoke<number[]>('preview_voiceset', { resref }),

  // ===========================================================================
  // Session Management
  // ===========================================================================

  /**
   * Load a character from a save file.
   */
  loadCharacter: (filePath: string, playerIndex?: number) =>
    invoke<boolean>('load_character', { filePath, playerIndex }),

  /**
   * Save the current character.
   */
  saveCharacter: (filePath?: string) =>
    invoke<boolean>('save_character', { filePath }),

  /**
   * Close the current character.
   */
  closeCharacter: () => invoke<boolean>('close_character'),

  /**
   * Get session information.
   */
  getSessionInfo: () => invoke<SessionInfo>('get_session_info'),

  /**
   * Check if there are unsaved changes.
   */
  hasUnsavedChanges: () => invoke<boolean>('has_unsaved_changes'),

  // ===========================================================================
  // Individual Field Updates (for backwards compatibility or specific needs)
  // ===========================================================================

  // Skills
  setSkillRank: (skillId: SkillId, ranks: number) =>
    invoke<SkillChangeResult>('set_skill_rank', { skillId, ranks }),

  resetAllSkills: () => invoke<number>('reset_all_skills'),

  // Feats
  addFeat: (featId: FeatId) => invoke<void>('add_feat', { featId }),
  removeFeat: (featId: FeatId) => invoke<void>('remove_feat', { featId }),
  swapFeat: (oldFeatId: FeatId, newFeatId: FeatId) =>
    invoke<void>('swap_feat', { oldFeatId, newFeatId }),

  // Spells
  addKnownSpell: (classIndex: number, spellLevel: number, spellId: SpellId) =>
    invoke<SpellChangeResult>('add_known_spell', { classIndex, spellLevel, spellId }),

  removeKnownSpell: (classIndex: number, spellLevel: number, spellId: SpellId) =>
    invoke<SpellChangeResult>('remove_known_spell', { classIndex, spellLevel, spellId }),

  // Combat
  updateNaturalArmor: (value: number) =>
    invoke<NaturalArmorChange>('update_natural_armor', { value }),

  updateInitiativeBonus: (value: number) =>
    invoke<InitiativeChange>('update_initiative_bonus', { value }),

  setMiscSaveBonus: (saveType: number, value: number) =>
    invoke<SaveChange>('set_misc_save_bonus', { saveType, value }),

  updateHitPoints: (current?: number, max?: number) =>
    invoke<HitPoints>('update_hit_points', { current, max }),

  // Inventory
  setGold: (amount: number) => invoke<number>('set_gold', { amount }),

  equipItem: (inventoryIndex: number, slot: EquipmentSlot) =>
    invoke<EquipResult>('equip_item', { inventoryIndex, slot }),

  unequipItem: (slot: EquipmentSlot) =>
    invoke<UnequipResult>('unequip_item', { slot }),

  addToInventory: (baseItemId: number, stackSize: number) =>
    invoke<AddItemResult>('add_to_inventory', { baseItemId, stackSize }),

  removeFromInventory: (index: number) =>
    invoke<RemoveItemResult>('remove_from_inventory', { index }),
};

export default CharacterStateAPI;
