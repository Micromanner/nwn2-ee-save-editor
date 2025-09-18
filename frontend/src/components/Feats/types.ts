// Shared type definitions for the Feats system

export interface Prerequisite {
  type: 'ability' | 'feat' | 'class' | 'level' | 'bab' | 'spell_level';
  description: string;
  required_value?: number;
  current_value?: number;
  feat_id?: number;
  class_id?: number;
  met: boolean;
}

export interface DetailedPrerequisites {
  requirements: Prerequisite[];
  met: string[];
  unmet: string[];
}

export interface FeatInfo {
  id: number;
  feat_id?: number;
  label: string;
  name: string;
  type: number;
  protected: boolean;
  custom: boolean;
  description?: string;
  icon?: string;
  prerequisites?: Record<string, unknown>;
  can_take?: boolean;
  missing_requirements?: string[];
  has_feat?: boolean;
  detailed_prerequisites?: DetailedPrerequisites;
}

export interface FeatsState {
  summary: {
    total: number;
    protected: FeatInfo[];
    class_feats: FeatInfo[];
    general_feats: FeatInfo[];
    custom_feats: FeatInfo[];
  };
  all_feats: FeatInfo[];
  available_feats: FeatInfo[];
  legitimate_feats: FeatInfo[];
  feat_chains: Record<string, unknown>;
  recommended_feats: FeatInfo[];
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

export interface ValidationState {
  can_take: boolean;
  reason: string;
  has_feat: boolean;
  missing_requirements: string[];
}

export type ValidationCache = Record<number, ValidationState>;

export interface FeatManagementCallbacks {
  onDetails: (feat: FeatInfo) => void;
  onAdd: (featId: number) => void;
  onRemove: (featId: number) => void;
  onValidate?: (featId: number) => void;
}

// Feat type constants based on NWN2
export const FEAT_TYPES = {
  GENERAL: 1,
  COMBAT: 2,
  METAMAGIC: 8,
  DIVINE: 16,
  EPIC: 32,
  CLASS: 64,
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