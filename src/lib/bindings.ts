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

export interface HitPoints {
  current: number;
  max: number;
  temp: number;
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
  tints: TintChannels | null;
}

/// The 22 accessory slots of a chest armor UTI (serde variant names).
export type AccessorySlot =
  | 'LtShoulder' | 'RtShoulder' | 'LtArm' | 'RtArm' | 'LtElbow' | 'RtElbow'
  | 'LtBracer' | 'RtBracer' | 'LtLeg' | 'RtLeg' | 'LtKnee' | 'RtKnee'
  | 'LtShin' | 'RtShin' | 'LtHip' | 'RtHip' | 'LtAnkle' | 'RtAnkle'
  | 'LtFoot' | 'RtFoot' | 'BkHip' | 'FtHip';

/// Mesh id (0 = none) plus the slot's own tints; null = AC* field absent.
export interface AccessorySlotData {
  id: number;
  tints: TintChannels;
}

export interface ArmorAccessories {
  lt_shoulder: AccessorySlotData | null;
  rt_shoulder: AccessorySlotData | null;
  lt_arm: AccessorySlotData | null;
  rt_arm: AccessorySlotData | null;
  lt_elbow: AccessorySlotData | null;
  rt_elbow: AccessorySlotData | null;
  lt_bracer: AccessorySlotData | null;
  rt_bracer: AccessorySlotData | null;
  lt_leg: AccessorySlotData | null;
  rt_leg: AccessorySlotData | null;
  lt_knee: AccessorySlotData | null;
  rt_knee: AccessorySlotData | null;
  lt_shin: AccessorySlotData | null;
  rt_shin: AccessorySlotData | null;
  lt_hip: AccessorySlotData | null;
  rt_hip: AccessorySlotData | null;
  lt_ankle: AccessorySlotData | null;
  rt_ankle: AccessorySlotData | null;
  lt_foot: AccessorySlotData | null;
  rt_foot: AccessorySlotData | null;
  bk_hip: AccessorySlotData | null;
  ft_hip: AccessorySlotData | null;
}

export interface ItemAppearance {
  variation: number;
  model_parts: [number, number, number];
  tints: TintChannels;
  armor_visual_type: number | null;
  boots: NestedArmorPart | null;
  gloves: NestedArmorPart | null;
  accessories: ArmorAccessories;
}

/// Accessory ids with a mesh on disk for one slot (chest armor only).
export interface AccessorySlotOptions {
  slot: AccessorySlot;
  ids: number[];
}

/// Which GFF field the variation gallery drives. Modeltype-0 items (shields,
/// bows, crossbows) resolve their mesh from ModelPart1.
export type VariationField = 'variation' | 'model_part1';

/// How the preview poses the model; shield MDBs are authored lying flat.
export type PreviewPose = 'weapon' | 'shield' | 'armor';

/// One armor.2da row (id) the material select writes into ArmorVisualType.
export interface MaterialOption {
  id: number;
  name: string;
}

export interface ItemAppearanceOptions {
  available_variations: number[];
  available_materials: MaterialOption[];
  available_part1: number[];
  available_part2: number[];
  available_part3: number[];
  has_model: boolean;
  variation_field: VariationField;
  preview_pose: PreviewPose;
  available_accessories: AccessorySlotOptions[];
}

export interface AppearanceState {
  race_id: number;
  race_name: string;
  subrace_id: number;
  gender: number;
  gender_name: string;
  appearance_type: number;
  appearance_type_name: string;
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
  fits_humanoid: boolean;
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
// Animal Companion / Familiar
// =============================================================================

export interface PetTypeOption {
  id: number;
  name: string;
}

export interface PetSlot {
  name: string;
  type_id: number;
  type_name: string | null;
  has_feat: boolean;
  options: PetTypeOption[];
}

export interface PetsState {
  animal_companion: PetSlot;
  familiar: PetSlot;
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
  point_summary: AbilityPointsSummary;
  point_buy: PointBuyState;
}

// =============================================================================
// Classes State (Aggregated)
// =============================================================================

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

export interface ClassesState {
  total_level: number;
  entries: ClassSummaryEntry[];
  xp_progress: XpProgress;
  level_history: LevelHistoryEntry[];
}

// =============================================================================
// Feats State (Aggregated)
// =============================================================================

/** Bitmask of `feat.2da` `FeatCategory` bits. */
export type FeatType = number;
export type FeatCategory =
  | 'Unknown'
  | 'General'
  | 'Proficiency'
  | 'SkillSave'
  | 'Metamagic'
  | 'Divine'
  | 'Epic'
  | 'Class'
  | 'Background'
  | 'Spellcasting'
  | 'History'
  | 'Heritage'
  | 'ItemCreation'
  | 'Racial'
  | 'Domain';

export interface PrereqFeat {
  id: FeatId;
  name: string;
  met: boolean;
}

export interface PrereqAbility {
  ability: string;
  required: number;
  current: number;
  met: boolean;
}

export interface PrereqValue {
  required: number;
  current: number;
  met: boolean;
}

export interface PrereqClassLevel extends PrereqValue {
  class_name: string;
}

export interface PrereqSkill {
  skill: string;
  required: number;
  current: number;
  met: boolean;
}

export interface FeatPrerequisites {
  feats: PrereqFeat[];
  any_feats: PrereqFeat[];
  abilities: PrereqAbility[];
  bab: PrereqValue | null;
  level: PrereqValue | null;
  class_level: PrereqClassLevel | null;
  max_level: PrereqValue | null;
  epic: PrereqValue | null;
  base_fortitude: PrereqValue | null;
  caster_level: PrereqValue | null;
  spell_level: PrereqValue | null;
  skills: PrereqSkill[];
}

export interface FeatInfo {
  id: FeatId;
  label: string;
  name: string;
  description: string;
  icon: string;
  type: FeatType;
  category: FeatCategory;
  protected: boolean;
  custom: boolean;
  has_feat: boolean;
  prerequisites: FeatPrerequisites;
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
  description: string;
  granted_feat: FeatId | null;
  is_castable: boolean;
  epithet_feat: FeatId | null;
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
  expected_spells_known: Record<number, number>;
}

export interface CasterClassSummary {
  id: ClassId;
  name: string;
  total_slots: number;
  max_spell_level: number;
  slots_by_level: Record<number, number>;
}

export interface SpellSummary {
  caster_classes: CasterClassSummary[];
  total_spell_levels: number;
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

export interface PendingSpellLearning {
  class_id: number;
  class_name: string;
  caster_type: string;
  by_level: Record<number, number>;
  total: number;
}

export interface AbilitySpellEntry {
  spell_id: number;
  name: string;
  icon: string;
  description: string | null;
  school_name: string | null;
  innate_level: number;
}

export interface SpellsState {
  spellcasting_classes: SpellcastingClass[];
  spell_summary: SpellSummary;
  memorized_spells: MemorizedSpellEntry[];
  known_spells: KnownSpellEntry[];
  pending_spell_learning: PendingSpellLearning[];
  ability_spells: AbilitySpellEntry[];
}

// =============================================================================
// Skills
// =============================================================================

export interface SkillSummaryEntry {
  skill_id: SkillId;
  name: string;
  ranks: number;
  max_ranks: number;
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
  /** Class new ranks are bought as (the class taken at the last level). */
  levelling_class_id: number | null;
  levelling_class_name: string | null;
}

// =============================================================================
// Combat & Saves
// =============================================================================

export interface ACBreakdown {
  base: number;
  armor: number;
  shield: number;
  dex: number;
  natural: number;
  natural_item: number;
  dodge: number;
  deflection: number;
  size: number;
  misc: number;
}

export interface ArmorClass {
  total: number;
  touch: number;
  flat_footed: number;
  breakdown: ACBreakdown;
}

export interface Initiative {
  total: number;
  dex: number;
  feat: number;
}

export interface CombatSummary {
  armor_class: ArmorClass;
  base_attack_bonus: number;
  initiative: Initiative;
}

export interface SaveBreakdown {
  total: number;
  base: number;
  ability: number;
  equipment: number;
  feat: number;
  class_bonus: number;
  misc: number;
}

export interface SavingThrows {
  fortitude: SaveBreakdown;
  reflex: SaveBreakdown;
  will: SaveBreakdown;
}

export interface SaveSummary {
  fortitude: number;
  reflex: number;
  will: number;
  saves: SavingThrows;
}

export interface NaturalArmorChange {
  old_value: number;
  new_value: number;
}

// =============================================================================
// Inventory
// =============================================================================

export interface DecodedPropertyInfo {
  display_string: string;
}

export interface FullInventoryItem {
  index: number;
  container_index: number | null;
  is_container: boolean;
  contents: FullInventoryItem[];
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
  max_stack: number;
  max_props: number;
  prop_column: number;
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
  max_stack: number;
  max_props: number;
  prop_column: number;
  decoded_properties: DecodedPropertyInfo[];
  appearance: ItemAppearance;
}

export interface FullEncumbrance {
  total_weight: number;
  heavy_load: number;
  status: EncumbranceStatus;
}

export type EncumbranceStatus = 'normal' | 'encumbered' | 'overloaded';

export interface FullInventorySummary {
  inventory: FullInventoryItem[];
  equipped: FullEquippedItem[];
  gold: number;
  encumbrance: FullEncumbrance;
}

export interface BasicItemInfo {
  tag: string;
  base_item: number;
  stack_size: number;
  identified: boolean;
}

export interface AddItemResult {
  inventory_index: number | null;
  stacked: boolean;
  item: BasicItemInfo | null;
}

export interface TransformItemRequest {
  item_index: number | null;
  container_index: number | null;
  slot: string | null;
  target_resref: string;
}

// =============================================================================
// Session
// =============================================================================

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

export interface CharacterHistory {
  /** null for the player / standalone character. */
  ros_name: string | null;
  name: string;
  active: boolean;
  /** Newest first. */
  undo_labels: string[];
  /** Nearest first. */
  redo_labels: string[];
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
  appearance: number;
}

export interface AvailableSubrace {
  id: number;
  name: string;
  label: string;
  icon: string | null;
  description: string | null;
  ability_adjustments: AbilityModifiers;
  base_race: RaceId;
  base_race_name: string;
  appearance: number;
}

export interface AvailableBaseItem {
  id: number;
  name: string;
  item_class: string | null;
  store_panel: number;
  sub_category: string;
  weight: number | null;
  base_cost: number | null;
}
