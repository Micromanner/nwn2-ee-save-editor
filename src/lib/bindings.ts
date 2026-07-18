/**
 * TypeScript bindings for Rust Tauri backend types.
 *
 * These types match the Rust structs in src-tauri/src/
 * See RUST_API_ARCHITECTURE.md for the full API documentation.
 */

// =============================================================================
// ID Types (newtype wrappers)
// =============================================================================

export type RaceId = number;
export type ClassId = number;
export type FeatId = number;
export type SpellId = number;
export type SkillId = number;
export type DomainId = number;
export type ItemId = number;
export type BaseItemId = number;

// AbilityIndex: 0=STR, 1=DEX, 2=CON, 3=INT, 4=WIS, 5=CHA
export type AbilityIndex = number;

// =============================================================================
// Core Types
// =============================================================================

export interface AbilityScores {
  Str: number;
  Dex: number;
  Con: number;
  Int: number;
  Wis: number;
  Cha: number;
}

export interface AbilityModifiers {
  Str: number;
  Dex: number;
  Con: number;
  Int: number;
  Wis: number;
  Cha: number;
}

export interface SaveBonuses {
  fortitude: number;
  reflex: number;
  will: number;
}

export interface HitPoints {
  current: number;
  max: number;
  temp: number;
}

export interface ValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

// =============================================================================
// Alignment
// =============================================================================

export interface Alignment {
  law_chaos: number;
  good_evil: number;
}

// =============================================================================
// Appearance State
// =============================================================================

export interface TintChannel {
  r: number;
  g: number;
  b: number;
  a: number;
}

export interface TintChannels {
  channel1: TintChannel;
  channel2: TintChannel;
  channel3: TintChannel;
}

export interface PartTintCapability {
  has_tint_map: boolean;
  channels: [boolean, boolean, boolean];
}

export interface ColorCapabilities {
  head: PartTintCapability;
  hair: PartTintCapability;
  tail: PartTintCapability | null;
  wings: PartTintCapability | null;
}

export interface NestedArmorPart {
  armor_visual_type: number | null;
  variation: number;
}

export interface ItemAppearance {
  variation: number;
  model_parts: [number, number, number];
  tints: TintChannels;
  armor_visual_type: number | null;
  boots: NestedArmorPart | null;
  gloves: NestedArmorPart | null;
}

export interface ItemAppearanceOptions {
  available_variations: number[];
  available_part1: number[];
  available_part2: number[];
  available_part3: number[];
}

export interface AppearanceState {
  race_id: number;
  race_name: string;
  gender: number;
  gender_name: string;
  appearance_type: number;
  appearance_head: number;
  appearance_hair: number;
  appearance_fhair: number;
  tint_head: TintChannels;
  tint_hair: TintChannels;
  tint_body: TintChannels;
  color_tattoo1: number;
  color_tattoo2: number;
  height: number;
  girth: number;
  soundset: number;
  wings: number;
  wings_name: string;
  tail: number;
  tail_name: string;
  available_heads: number[];
  available_hairs: number[];
  is_parts_based: boolean;
  has_fhair_meshes: boolean;
  never_draw_helmet: boolean;
  cloak_tint: TintChannels | null;
  armor_tint: TintChannels | null;
  color_capabilities: ColorCapabilities;
}

export interface AppearanceOption {
  id: number;
  name: string;
}

export interface VoiceSetInfo {
  id: number;
  name: string;
  gender: number;
  resref: string;
  voice_type: number;
}

export interface AppearanceUpdates {
  appearance_type?: number;
  appearance_head?: number;
  appearance_hair?: number;
  appearance_fhair?: number;
  tint_head?: TintChannels;
  tint_hair?: TintChannels;
  tint_body?: TintChannels;
  tint_body_match_skin?: boolean;
  color_tattoo1?: number;
  color_tattoo2?: number;
  height?: number;
  girth?: number;
  soundset?: number;
  wings?: number;
  tail?: number;
  never_draw_helmet?: boolean;
}

// =============================================================================
// Overview State (Aggregated)
// =============================================================================

export interface OverviewState {
  // Identity
  first_name: string;
  last_name: string;
  full_name: string;
  race_id: number;
  race_name: string;
  subrace: string | null;
  gender: string;
  gender_id: number;
  age: number;
  deity: string;
  alignment: Alignment;
  alignment_string: string;
  description: string;

  // Progression
  total_level: number;
  experience: number;
  xp_progress: XpProgress;
  classes: ClassSummaryEntry[];

  // Combat Stats
  hit_points: HitPoints;
  armor_class: number;
  base_attack_bonus: number;
  saving_throws: SaveBonuses;

  // Resources
  gold: number;
  skill_points_available: number;

  // Domains (for clerics)
  domains: DomainInfo[];
}

// =============================================================================
// Abilities State (Aggregated)
// =============================================================================

export interface AbilityIncrease {
  level: number;
  ability: AbilityIndex;
}

export interface AbilityPointsSummary {
  base_scores: AbilityScores;
  level_increases: AbilityIncrease[];
  expected_increases: number;
  actual_increases: number;
  available: number;
}

export interface EncumbranceInfo {
  light_limit: number;
  medium_limit: number;
  heavy_limit: number;
  max_limit: number;
}

export interface PointBuyState {
  starting_scores: AbilityScores;
  point_buy_cost: number;
  budget: number;
  remaining: number;
}

export interface AbilitiesState {
  base_scores: AbilityScores;
  effective_scores: AbilityScores;
  modifiers: AbilityModifiers;
  racial_modifiers: AbilityModifiers;
  equipment_modifiers: AbilityModifiers;
  hit_points: HitPoints;
  encumbrance: EncumbranceInfo;
  point_summary: AbilityPointsSummary;
  point_buy: PointBuyState;
}

// =============================================================================
// Classes State (Aggregated)
// =============================================================================

export interface ClassEntry {
  class_id: ClassId;
  level: number;
}

export interface ClassSummaryEntry {
  class_id: ClassId;
  name: string;
  level: number;
  hit_die: number;
  base_attack_bonus: number;
  fortitude_save: number;
  reflex_save: number;
  will_save: number;
  skill_points_per_level: number;
}

export interface XpProgress {
  current_xp: number;
  current_level: number;
  xp_for_current_level: number;
  xp_for_next_level: number;
  xp_remaining: number;
  progress_percent: number;
}

export interface SkillRankEntry {
  skill_id: SkillId;
  ranks: number;
}

export interface ResolvedFeatEntry {
  feat_id: number;
  name: string;
}

export interface ResolvedSkillEntry {
  skill_id: number;
  name: string;
  ranks: number;
}

export interface LevelHistoryEntry {
  character_level: number;
  class_id: number;
  class_name: string;
  class_level: number;
  hp_gained: number;
  skill_points_remaining: number;
  ability_increase: string | null;
  feats_gained: ResolvedFeatEntry[];
  skills_gained: ResolvedSkillEntry[];
}

export interface SkillPointsSummary {
  available_points: number;
  total_points: number;
}

export interface ClassesState {
  total_level: number;
  entries: ClassSummaryEntry[];
  xp_progress: XpProgress;
  level_history: LevelHistoryEntry[];
  skill_points_summary: SkillPointsSummary;
}

// =============================================================================
// Feats State (Aggregated)
// =============================================================================

export type FeatType = 'general' | 'bonus' | 'class' | 'racial' | 'epic' | 'domain' | 'background';
export type FeatCategory = 'combat' | 'metamagic' | 'itemcreation' | 'divine' | 'spellcasting' | 'skill' | 'save' | 'other';

export interface FeatInfo {
  id: FeatId;
  name: string;
  description: string;
  icon: string | null;
  feat_type: FeatType;
  category: FeatCategory;
  prerequisites: string[];
  benefits: string | null;
}

export interface FeatEntry {
  feat_id: FeatId;
  source: string;
}

export interface FeatSummary {
  total: number;
  class_feats: FeatInfo[];
  general_feats: FeatInfo[];
  custom_feats: FeatInfo[];
  background_feats: FeatInfo[];
  domain_feats: FeatInfo[];
}

export interface FeatSlots {
  total_general_slots: number;
  total_bonus_slots: number;
  total_slots: number;
  filled_slots: number;
  open_slots: number;
  open_general_slots: number;
  open_bonus_slots: number;
}

export interface DomainInfo {
  id: DomainId;
  name: string;
  description: string | null;
  icon: string | null;
  granted_feats: FeatId[];
  has_domain: boolean;
}

export interface FeatsState {
  summary: FeatSummary;
  feat_slots: FeatSlots;
  domains: DomainInfo[];
}

// =============================================================================
// Spells State (Aggregated)
// =============================================================================

export interface SpellcastingClass {
  index: number;
  class_id: ClassId;
  class_name: string;
  class_level: number;
  caster_level: number;
  spell_type: string;
  can_edit_spells: boolean;
}

export interface CasterClassSummary {
  id: ClassId;
  name: string;
  total_slots: number;
  max_spell_level: number;
  slots_by_level: Record<number, number>;
}

export interface MetamagicFeat {
  id: number;
  name: string;
  level_cost: number;
}

export interface SpellSummary {
  caster_classes: CasterClassSummary[];
  total_spell_levels: number;
  metamagic_feats: MetamagicFeat[];
  spell_resistance: number;
}

export interface KnownSpellEntry {
  level: number;
  spell_id: SpellId;
  name: string;
  icon: string;
  school_name: string | null;
  description: string | null;
  class_id: ClassId;
  is_domain_spell: boolean;
}

export interface MemorizedSpellEntry {
  level: number;
  spell_id: SpellId;
  name: string;
  icon: string;
  school_name: string | null;
  description: string | null;
  class_id: ClassId;
  metamagic: number;
  ready: boolean;
}

export interface SpellDetails {
  id: SpellId;
  name: string;
  icon: string;
  school_id: number | null;
  school_name: string | null;
  description: string | null;
  spell_range: string | null;
  cast_time: string | null;
  conjuration_time: string | null;
  components: string | null;
  target_type: string | null;
}

export interface PendingSpellLearning {
  class_id: number;
  class_name: string;
  caster_type: string;
  by_level: Record<number, number>;
  total: number;
}

export interface SpellsState {
  spellcasting_classes: SpellcastingClass[];
  spell_summary: SpellSummary;
  memorized_spells: MemorizedSpellEntry[];
  known_spells: KnownSpellEntry[];
  pending_spell_learning: PendingSpellLearning[];
}

// =============================================================================
// Skills
// =============================================================================

export interface SkillSummaryEntry {
  skill_id: SkillId;
  name: string;
  ranks: number;
  modifier: number;
  total: number;
  is_class_skill: boolean;
  ability: string;
  untrained: boolean;
  armor_check_penalty: boolean;
  feat_bonus: number;
  item_bonus: number;
}

export interface SkillsState {
  class_skills: SkillSummaryEntry[];
  cross_class_skills: SkillSummaryEntry[];
  total_available: number;
  spent_points: number;
}

export interface SkillChangeResult {
  skill_id: SkillId;
  old_ranks: number;
  new_ranks: number;
  points_spent: number;
  points_remaining: number;
}

// =============================================================================
// Combat & Saves
// =============================================================================

export interface ACBreakdown {
  base: number;
  armor: number;
  shield: number;
  dex: number;
  size: number;
  natural: number;
  deflection: number;
  misc: number;
}

export interface ArmorClass {
  total: number;
  touch: number;
  flat_footed: number;
  breakdown: ACBreakdown;
}

export interface AttackBreakdown {
  base: number;
  ability: number;
  size: number;
  misc: number;
}

export interface AttackBonuses {
  melee_attack_bonus: number;
  ranged_attack_bonus: number;
  base_attack_bonus: number;
  melee_breakdown: AttackBreakdown;
  ranged_breakdown: AttackBreakdown;
}

export interface Initiative {
  total: number;
  dex: number;
  feat: number;
}

export interface DamageReduction {
  amount: number;
  bypass: string;
}

export interface MovementSpeed {
  base: number;
  current: number;
  armor_penalty: boolean;
}

export interface CombatManeuverBonus {
  total: number;
  bab: number;
  str_mod: number;
  size_mod: number;
}

export interface CombatSummary {
  armor_class: ArmorClass;
  attack_bonuses: AttackBonuses;
  base_attack_bonus: number;
  attack_sequence: number[];
  initiative: Initiative;
  movement: MovementSpeed;
  cmb: CombatManeuverBonus;
  damage_reductions: DamageReduction[];
}

export type SaveType = 'Fortitude' | 'Reflex' | 'Will';

export interface SaveBreakdown {
  total: number;
  base: number;
  ability: number;
  equipment: number;
  feat: number;
  racial: number;
  class_bonus: number;
  misc: number;
}

export interface SavingThrows {
  fortitude: SaveBreakdown;
  reflex: SaveBreakdown;
  will: SaveBreakdown;
}

export interface SaveSummary {
  saves: SavingThrows;
  hit_points: HitPoints;
}

export interface NaturalArmorChange {
  old_value: number;
  new_value: number;
}

export interface SaveChange {
  save_type: SaveType;
  old_misc: number;
  new_misc: number;
}

export interface SaveCheck {
  roll: number;
  modifier: number;
  total: number;
  dc: number;
  success: boolean;
}

// =============================================================================
// Inventory
// =============================================================================

export type EquipmentSlot =
  | 'Head'
  | 'Chest'
  | 'Boots'
  | 'Arms'
  | 'RightHand'
  | 'LeftHand'
  | 'Cloak'
  | 'RightRing'
  | 'LeftRing'
  | 'Neck'
  | 'Belt'
  | 'Arrows'
  | 'Bullets'
  | 'Bolts'
  | 'CreatureWeaponLeft'
  | 'CreatureWeaponRight'
  | 'CreatureWeaponBite'
  | 'CreatureArmor';

export interface DecodedPropertyInfo {
  property_name: string;
  subtype_name: string | null;
  cost_value: number;
  param1_value: number | null;
  display_string: string;
}

export interface FullInventoryItem {
  index: number;
  item: Record<string, unknown>;
  base_item: number;
  base_item_name: string;
  name: string;
  icon: string | null;
  description: string;
  weight: number;
  value: number;
  is_custom: boolean;
  stack_size: number;
  enhancement: number;
  charges: number | null;
  identified: boolean;
  plot: boolean;
  cursed: boolean;
  stolen: boolean;
  base_ac: number | null;
  category: string;
  equippable_slots: string[];
  default_slot: string | null;
  decoded_properties: DecodedPropertyInfo[];
  appearance: ItemAppearance;
}

export interface FullEquippedItem {
  slot: string;
  base_item: number;
  base_item_name: string;
  custom: boolean;
  name: string;
  icon: string | null;
  description: string;
  weight: number;
  value: number;
  item_data: Record<string, unknown>;
  base_ac: number | null;
  decoded_properties: DecodedPropertyInfo[];
  appearance: ItemAppearance;
}

export interface FullEncumbrance {
  total_weight: number;
  light_load: number;
  medium_load: number;
  heavy_load: number;
  encumbrance_level: string;
}

export type EncumbranceStatus = 'Light' | 'Medium' | 'Heavy' | 'Overloaded';

export interface FullInventorySummary {
  inventory: FullInventoryItem[];
  equipped: FullEquippedItem[];
  gold: number;
  encumbrance: FullEncumbrance;
}

export interface BasicItemInfo {
  index: number;
  base_item_id: BaseItemId;
  base_item_name: string;
  name: string;
  stack_size: number;
  weight: number;
}

export interface EquipResult {
  success: boolean;
  message: string;
  unequipped_item: FullInventoryItem | null;
}

export interface UnequipResult {
  success: boolean;
  message: string;
  inventory_index: number;
}

export interface AddItemResult {
  success: boolean;
  message: string;
  inventory_index: number;
}

export interface RemoveItemResult {
  success: boolean;
  message: string;
}

export interface ItemBonuses {
  str_bonus: number;
  dex_bonus: number;
  con_bonus: number;
  int_bonus: number;
  wis_bonus: number;
  cha_bonus: number;
  ac_bonus: number;
  attack_bonus: number;
  damage_bonus: number;
  save_bonus: number;
}

export interface ProficiencyRequirement {
  feat_id: FeatId;
  feat_name: string;
  has_proficiency: boolean;
}

export interface ItemProficiencyInfo {
  base_item_id: BaseItemId;
  base_item_name: string;
  requirements: ProficiencyRequirement[];
  is_proficient: boolean;
}

// =============================================================================
// Update Types (for batch operations)
// =============================================================================

export interface CharacterUpdates {
  first_name?: string;
  last_name?: string;
  age?: number;
  gender?: number;
  deity?: string;
  description?: string;
  alignment?: [number, number]; // [law_chaos, good_evil]
  experience?: number;
}

export interface AbilitiesUpdates {
  Str?: number;
  Dex?: number;
  Con?: number;
  Int?: number;
  Wis?: number;
  Cha?: number;
}

export interface CombatUpdates {
  natural_armor?: number;
  fortitude_misc?: number;
  reflex_misc?: number;
  will_misc?: number;
}

export type FeatAction =
  | { action: 'add'; feat_id: FeatId }
  | { action: 'remove'; feat_id: FeatId }
  | { action: 'swap'; old_feat_id: FeatId; new_feat_id: FeatId };

export type SpellAction =
  | { action: 'learn'; class_id: ClassId; spell_id: SpellId }
  | { action: 'forget'; class_id: ClassId; spell_id: SpellId }
  | { action: 'prepare'; class_id: ClassId; spell_id: SpellId; level: number }
  | { action: 'unprepare'; class_id: ClassId; spell_id: SpellId; level: number };

export type ItemAction =
  | { action: 'add'; template_resref: string }
  | { action: 'remove'; index: number }
  | { action: 'unequip'; slot: string };

// =============================================================================
// Class Categorization
// =============================================================================

export interface CategorizedClassInfo {
  id: number;
  name: string;
  label: string;
  type: 'base' | 'prestige' | 'npc';
  focus: string;
  max_level: number;
  hit_die: number;
  skill_points: number;
  is_spellcaster: boolean;
  has_arcane: boolean;
  has_divine: boolean;
  primary_ability: string;
  bab_progression: string;
  alignment_restricted: boolean;
  description?: string;
}

export interface ClassFocusInfo {
  id: string;
  name: string;
  description: string;
  icon: string;
}

export interface ClassCategories {
  base: Record<string, CategorizedClassInfo[]>;
  prestige: Record<string, CategorizedClassInfo[]>;
  npc: Record<string, CategorizedClassInfo[]>;
}

export interface CategorizedClasses {
  categories: ClassCategories;
  focus_info: Record<string, ClassFocusInfo>;
  total_classes: number;
}

// =============================================================================
// Session
// =============================================================================

export interface SessionInfo {
  character_loaded: boolean;
  file_path: string | null;
  dirty: boolean;
}

export interface UndoResult {
  applied: boolean;
  label: string | null;
  can_undo: boolean;
  can_redo: boolean;
}

export interface HistoryState {
  can_undo: boolean;
  can_redo: boolean;
  undo_label: string | null;
  redo_label: string | null;
}

// =============================================================================
// Error Handling
// =============================================================================

export type CommandErrorCode =
  | 'NoCharacterLoaded'
  | 'NoGameDataLoaded'
  | 'CharacterNotFound'
  | 'ValidationError'
  | 'InvalidValue'
  | 'FileError'
  | 'ParseError'
  | 'InsufficientResources'
  | 'PrerequisitesNotMet'
  | 'NotFound'
  | 'AlreadyExists'
  | 'OperationFailed'
  | 'Internal';

export interface CommandError {
  code: CommandErrorCode;
  details?: {
    field?: string;
    reason?: string;
    path?: string;
    message?: string;
    item?: string;
    resource?: string;
    required?: number;
    available?: number;
    missing?: string[];
    expected?: string;
    actual?: string;
    operation?: string;
  };
}

// =============================================================================
// GameData Types
// =============================================================================

export interface AvailableRace {
  id: RaceId;
  name: string;
  description: string | null;
  icon: string | null;
  ability_adjustments: AbilityModifiers;
  favored_class: ClassId | null;
  default_subrace: number | null;
  appearance: number;
}

export interface AvailableSubrace {
  id: number;
  name: string;
  label: string;
  icon: string | null;
  description: string | null;
  ability_adjustments: AbilityModifiers;
  favored_class: string | null;
  size: string | null;
  base_race: RaceId;
  base_race_name: string;
  appearance: number;
}

export interface AvailableClass {
  id: ClassId;
  name: string;
  description: string | null;
  icon: string | null;
  hit_die: number;
  is_prestige: boolean;
  is_spellcaster: boolean;
  primary_ability: AbilityIndex;
}

export interface AvailableFeat {
  id: FeatId;
  name: string;
  description: string | null;
  icon: string | null;
  feat_type: FeatType;
  category: FeatCategory;
}

export interface AvailableSkill {
  id: SkillId;
  name: string;
  description: string | null;
  icon: string | null;
  key_ability: AbilityIndex;
  untrained: boolean;
  armor_check_penalty: boolean;
}

export interface AvailableSpell {
  id: SpellId;
  name: string;
  description: string | null;
  icon: string | null;
  school: string;
  levels: Record<ClassId, number>;
}

export interface AvailableDeity {
  id: number;
  name: string;
  description: string | null;
  alignment: string | null;
  domains: DomainId[];
}

export interface AvailableDomain {
  id: DomainId;
  name: string;
  description: string | null;
  icon: string | null;
  granted_spells: SpellId[];
  granted_feats: FeatId[];
}

export interface AvailableGender {
  id: number;
  name: string;
}

export interface AvailableAlignment {
  id: string;
  name: string;
  law_chaos: number;
  good_evil: number;
}

export interface AvailableBackground {
  id: number;
  name: string;
  description: string | null;
}

export interface AvailableAbility {
  id: AbilityIndex;
  name: string;
  short_name: string;
  description: string;
}

export interface AvailableBaseItem {
  id: BaseItemId;
  name: string;
  description: string | null;
  weight: number;
  base_cost: number;
  weapon_type: string | null;
}

export interface AvailableSpellSchool {
  id: number;
  name: string;
  description: string | null;
}

export interface AvailableItemProperty {
  id: number;
  name: string;
  description: string | null;
  subtypes: { id: number; name: string }[];
}

// =============================================================================
// File Operations
// =============================================================================

export interface SaveFile {
  path: string;
  name: string;
  thumbnail: string | null;
  modified: number | null;
}

// =============================================================================
// Paths Configuration
// =============================================================================

export interface PathInfo {
  path: string | null;
  exists: boolean;
  source: string;
}

export interface CustomFolderInfo {
  path: string;
  exists: boolean;
  priority: number;
}

export interface PathConfig {
  game_folder: PathInfo;
  documents_folder: PathInfo;
  steam_workshop_folder: PathInfo;
  custom_override_folders: CustomFolderInfo[];
  custom_module_folders: CustomFolderInfo[];
  custom_hak_folders: CustomFolderInfo[];
}

export interface PathUpdateResponse {
  success: boolean;
  message: string;
  config: PathConfig;
}

export interface AutoDetectResponse {
  game_folder: string | null;
  documents_folder: string | null;
  steam_workshop_folder: string | null;
}

// =============================================================================
// Campaign
// =============================================================================

export interface ModuleInfo {
  name: string;
  description: string | null;
  hak_list: string[];
  custom_tlk: string | null;
  start_movie: string | null;
}

export interface ModuleVariables {
  global_ints: Record<string, number>;
  global_floats: Record<string, number>;
  global_strings: Record<string, string>;
}

export interface CampaignSettings {
  campaign_id: string;
  // Add other settings as needed
}

export interface QuestDefinition {
  tag: string;
  name: string;
  entries: QuestEntry[];
}

export interface QuestEntry {
  id: number;
  text: string;
}

// =============================================================================
// Backup
// =============================================================================

export interface BackupInfo {
  path: string;
  timestamp: number;
  size: number;
  name: string;
}

export interface RestoreResult {
  success: boolean;
  message: string;
  restored_files: string[];
}

export interface CleanupResult {
  success: boolean;
  deleted_count: number;
  remaining_count: number;
}

// =============================================================================
// Initialization
// =============================================================================

export interface InitStatus {
  step: string;
  progress: number;
  message: string;
}

// =============================================================================
// Class Progression (for level up helper)
// =============================================================================

export interface ClassBasicInfo {
  id: ClassId;
  name: string;
  hit_die: number;
  is_prestige: boolean;
  is_spellcaster: boolean;
}

export interface ClassFeature {
  level: number;
  name: string;
  description: string | null;
}

export interface SpellSlots {
  class_id: ClassId;
  slots_by_level: Record<number, number>;
}

export interface LevelProgressionEntry {
  level: number;
  bab: number;
  fort_save: number;
  ref_save: number;
  will_save: number;
  features: ClassFeature[];
  spell_slots: SpellSlots | null;
}

export interface ClassProgression {
  class_info: ClassBasicInfo;
  progression: LevelProgressionEntry[];
}

// =============================================================================
// Race Types
// =============================================================================

export type SizeCategory = 'Fine' | 'Diminutive' | 'Tiny' | 'Small' | 'Medium' | 'Large' | 'Huge' | 'Gargantuan' | 'Colossal';

export interface SubraceInfo {
  name: string;
  description: string | null;
  ability_adjustments: AbilityModifiers;
}

export interface RaceChangedEvent {
  old_race_id: RaceId | null;
  new_race_id: RaceId;
  old_subrace: string | null;
  new_subrace: string | null;
}

// =============================================================================
// Prerequisite Types
// =============================================================================

export interface PrerequisiteResult {
  met: boolean;
  missing: string[];
  details: string | null;
}

export interface LevelUpResult {
  class_id: ClassId;
  new_level: number;
  hp_gained: number;
  skill_points_gained: number;
  general_feat_slots_gained: number;
  bonus_feat_slots_gained: number;
  ability_increase_gained: boolean;
  new_spells_gained: boolean;
  granted_feats: FeatId[];
}

// =============================================================================
// Biography
// =============================================================================

export interface Biography {
  first_name: string;
  last_name: string;
  full_name: string;
  age: number;
  description: string;
  background: string | null;
  experience: number;
}

// =============================================================================
// Spell Change Result
// =============================================================================

export interface SpellChangeResult {
  success: boolean;
  message: string;
  spell_id: SpellId;
  class_index: number;
  spell_level: number;
}

// =============================================================================
// Character Summary (for save info)
// =============================================================================

export interface CharacterSummary {
  name: string;
  race: string;
  classes: string;
  level: number;
}

// =============================================================================
// File Info (for save listing)
// =============================================================================

export interface FileInfo {
  path: string;
  name: string;
  size: number;
  modified: number;
}
