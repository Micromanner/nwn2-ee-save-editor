export * from './gamedata';
export * from './hooks';
export * from './errors';
export * from './character-state';

// Re-export commonly used items
export { gameData } from './gamedata';
export { CharacterStateAPI } from './character-state';

// Re-export bindings types for convenience (except those already in errors.ts)
export type {
  // Core types
  RaceId,
  ClassId,
  FeatId,
  SpellId,
  SkillId,
  DomainId,
  ItemId,
  BaseItemId,
  AbilityIndex,
  AbilityScores,
  AbilityModifiers,
  SaveBonuses,
  HitPoints,
  ValidationResult,
  Alignment,

  // State types
  OverviewState,
  AbilitiesState,
  ClassesState,
  FeatsState,
  SpellsState,
  SkillsState,

  // Update types
  CharacterUpdates,
  AbilitiesUpdates,
  CombatUpdates,
  FeatAction,
  SpellAction,
  ItemAction,

  // Session
  SessionInfo,

  // Combat
  CombatSummary,
  SaveSummary,
  ArmorClass,
  AttackBonuses,
  Initiative,

  // Inventory
  FullInventorySummary,
  FullInventoryItem,
  EquipmentSlot,
  EquipResult,
  UnequipResult,
  AddItemResult,
  RemoveItemResult,

  // Class types
  ClassEntry,
  ClassSummaryEntry,
  XpProgress,
  LevelHistoryEntry,

  // Feat types
  FeatInfo,
  FeatEntry,
  FeatSummary,
  FeatSlots,
  DomainInfo,

  // Skill types
  SkillSummaryEntry,
  SkillChangeResult,

  // Spell types
  SpellSummary,
  SpellDetails,
  SpellcastingClass,
  CasterClassSummary,
  MetamagicFeat,
  KnownSpellEntry,
  MemorizedSpellEntry,
  SpellChangeResult,

  // GameData types
  AvailableRace,
  AvailableClass,
  AvailableFeat,
  AvailableSkill,
  AvailableSpell,
  AvailableDeity,
  AvailableDomain,
  AvailableGender,
  AvailableAlignment,
  AvailableBackground,
  AvailableAbility,
  AvailableBaseItem,
  AvailableSpellSchool,
  AvailableItemProperty,

  // Path types
  PathConfig,
  PathInfo,
  PathUpdateResponse,

  // File types
  SaveFile,
  FileInfo,
  InitStatus,
} from '../bindings';