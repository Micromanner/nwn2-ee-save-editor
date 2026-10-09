// Shared type definitions for the Feats system

import type { FeatInfo } from '@/lib/bindings';

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