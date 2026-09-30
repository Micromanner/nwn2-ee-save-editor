// Shared type definitions for the Feats system

export interface PrereqFeat {
  id: number;
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

export interface BackendFeatPrerequisites {
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
  id: number;
  feat_id?: number;
  label: string;
  name: string;
  type: number;
  category?: string;
  protected: boolean;
  custom: boolean;
  description?: string;
  icon?: string;
  prerequisites: BackendFeatPrerequisites;
  has_feat?: boolean;
}

export interface FeatsState {
  summary: {
    total: number;
    class_feats: FeatInfo[];
    general_feats: FeatInfo[];
    custom_feats: FeatInfo[];
    background_feats: FeatInfo[];
    domain_feats: FeatInfo[];
  };
  all_feats: FeatInfo[];
  available_feats: FeatInfo[];
  legitimate_feats: FeatInfo[];
  recommended_feats: FeatInfo[];
  point_summary?: {
    total_general_slots: number;
    total_bonus_slots: number;
    total_slots: number;
    total_feats: number;
    open_slots: number;
    filled_slots: number;
    available: number;
  };
}

export interface FilterState {
  activeOnly: boolean;
}

export interface CategoryInfo {
  id: string;
  name: string;
  count: number;
  subcategories?: SubcategoryInfo[];
}

export interface SubcategoryInfo {
  id: string;
  name: string;
  count: number;
}

// Feat type constants based on NWN2
export const FEAT_TYPES = {
  GENERAL: 1,
  PROFICIENCY: 2,
  SKILL_SAVE: 4,
  METAMAGIC: 8,
  DIVINE: 16,
  EPIC: 32,
  CLASS: 64,
  BACKGROUND: 128,
  SPELLCASTING: 256,
  HISTORY: 512,
  HERITAGE: 1024,
  ITEM_CREATION: 2048,
  RACIAL: 4096,
  DOMAIN: 8192,
} as const;

export type FeatType = typeof FEAT_TYPES[keyof typeof FEAT_TYPES];

// View modes for feat display
export type ViewMode = 'grid' | 'list';

// Mock categories based on NWN2 official structure
export const FEAT_CATEGORIES: CategoryInfo[] = [
  { id: 'general', name: 'General', count: 0 },
  { id: 'combat', name: 'Combat', count: 0 },
  {
    id: 'class',
    name: 'Class',
    count: 0,
    subcategories: [
      { id: 'barbarian', name: 'Barbarian', count: 0 },
      { id: 'bard', name: 'Bard', count: 0 },
      { id: 'cleric', name: 'Cleric', count: 0 },
      { id: 'druid', name: 'Druid', count: 0 },
      { id: 'fighter', name: 'Fighter', count: 0 },
      { id: 'monk', name: 'Monk', count: 0 },
      { id: 'paladin', name: 'Paladin', count: 0 },
      { id: 'ranger', name: 'Ranger', count: 0 },
      { id: 'rogue', name: 'Rogue', count: 0 },
      { id: 'sorcerer', name: 'Sorcerer', count: 0 },
      { id: 'wizard', name: 'Wizard', count: 0 },
      { id: 'warlock', name: 'Warlock', count: 0 },
    ],
  },
  {
    id: 'race',
    name: 'Race',
    count: 0,
    subcategories: [
      { id: 'human', name: 'Human', count: 0 },
      { id: 'elf', name: 'Elf [All]', count: 0 },
      { id: 'dwarf', name: 'Dwarf [All]', count: 0 },
      { id: 'halfling', name: 'Halfling [All]', count: 0 },
      { id: 'gnome', name: 'Gnome [All]', count: 0 },
      { id: 'half-elf', name: 'Half-Elf', count: 0 },
      { id: 'half-orc', name: 'Half-Orc', count: 0 },
      { id: 'planetouched', name: 'Planetouched', count: 0 },
    ],
  },
  { id: 'epic', name: 'Epic', count: 0 },
  { id: 'divine', name: 'Divine', count: 0 },
  { id: 'metamagic', name: 'Metamagic', count: 0 },
  { id: 'item_creation', name: 'Item Creation', count: 0 },
  { id: 'skills_saves', name: 'Skills & Saves', count: 0 },
  { id: 'spellcasting', name: 'Spellcasting', count: 0 },
  { id: 'teamwork', name: 'Teamwork', count: 0 },
];