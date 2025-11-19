// Shared type definitions for the Spells system

export interface SpellInfo {
  id: number;
  name: string;
  description?: string;
  icon?: string;

  // Spell properties
  school_id?: number;
  school_name?: string;
  level: number;

  // Casting properties
  cast_time?: string;
  range?: string;
  conjuration_time?: string;
  components?: string;

  // Target information
  target_type?: string;
  available_metamagic?: string;

  // Class availability
  available_classes: string[];

  // Client-side properties for UI state (matching existing SpellList)
  isLearned?: boolean;
  icon_url?: string;

  // Computed properties for compatibility
  school?: string;
  innate_level?: number;
}

export interface SpellcastingClass {
  index: number;
  class_id: number;
  class_name: string;
  class_level: number;
  caster_level: number;
  spell_type: 'prepared' | 'spontaneous';
}

export interface SpellSummary {
  caster_classes: Array<{
    id: number;
    name: string;
    total_slots: number;
    max_spell_level: number;
    slots_by_level: Record<number, number>;
  }>;
  total_spell_levels: number;
  metamagic_feats: Array<{
    id: number;
    name: string;
    level_cost: number;
  }>;
  spell_resistance: number;
}

export interface SpellsState {
  spellcasting_classes: SpellcastingClass[];
  spell_summary: SpellSummary;
  memorized_spells: Array<{
    level: number;
    spell_id: number;
    name: string;
    icon: string;
    class_id: number;
    metamagic: number;
    ready: boolean;
  }>;
  available_by_level?: Record<number, SpellInfo[]>;
}

export interface ParsedSpellDescription {
  range?: string;
  duration?: string;
  save?: string;
  sr?: string;
  [key: string]: string | undefined;
}
