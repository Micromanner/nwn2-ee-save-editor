export interface SpellClassLevel {
  class_id: number;
  class_name: string;
  level: number;
}

export interface SpellInfo {
  id: number;
  name: string;
  description?: string | null;
  icon?: string;

  school_id?: number;
  school_name?: string | null;
  level: number;

  cast_time?: string;
  range?: string;
  conjuration_time?: string;
  components?: string;

  target_type?: string;
  available_metamagic?: string;

  available_classes: string[];
  class_levels?: SpellClassLevel[];
  class_id?: number;

  isLearned?: boolean;
  icon_url?: string;

  school?: string;
  innate_level?: number;

  memorized_count?: number;

  is_domain_spell?: boolean;
}

export interface ParsedSpellDescription {
  range?: string;
  duration?: string;
  save?: string;
  sr?: string;
  [key: string]: string | undefined;
}
