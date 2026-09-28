import type { FeatInfo } from '@/components/Feats/types';
import { FEAT_TYPES } from '@/components/Feats/types';

export const FEAT_TYPE_LABELS: Record<number, string> = {
  [FEAT_TYPES.GENERAL]: 'feats.categories.general',
  [FEAT_TYPES.PROFICIENCY]: 'feats.categories.proficiency',
  [FEAT_TYPES.SKILL_SAVE]: 'feats.categories.skillSave',
  [FEAT_TYPES.METAMAGIC]: 'feats.categories.metamagic',
  [FEAT_TYPES.DIVINE]: 'feats.categories.divine',
  [FEAT_TYPES.EPIC]: 'feats.categories.epic',
  [FEAT_TYPES.CLASS]: 'feats.categories.class',
  [FEAT_TYPES.BACKGROUND]: 'feats.categories.background',
  [FEAT_TYPES.SPELLCASTING]: 'feats.categories.spellcasting',
  [FEAT_TYPES.HISTORY]: 'feats.categories.history',
  [FEAT_TYPES.HERITAGE]: 'feats.categories.heritage',
  [FEAT_TYPES.ITEM_CREATION]: 'feats.categories.itemCreation',
  [FEAT_TYPES.RACIAL]: 'feats.categories.racial',
  [FEAT_TYPES.DOMAIN]: 'feats.categories.domain',
};

const FEAT_TYPE_PRIORITY: number[] = [
  FEAT_TYPES.DOMAIN,
  FEAT_TYPES.EPIC,
  FEAT_TYPES.RACIAL,
  FEAT_TYPES.CLASS,
  FEAT_TYPES.HERITAGE,
  FEAT_TYPES.HISTORY,
  FEAT_TYPES.BACKGROUND,
  FEAT_TYPES.ITEM_CREATION,
  FEAT_TYPES.SPELLCASTING,
  FEAT_TYPES.METAMAGIC,
  FEAT_TYPES.DIVINE,
  FEAT_TYPES.SKILL_SAVE,
  FEAT_TYPES.PROFICIENCY,
  FEAT_TYPES.GENERAL,
];

export function getFeatTypeLabel(type: number): string {
  const bit = FEAT_TYPE_PRIORITY.find(b => (type & b) !== 0);
  return bit ? FEAT_TYPE_LABELS[bit] : FEAT_TYPE_LABELS[FEAT_TYPES.GENERAL];
}

interface FeatsSummary {
  class_feats?: FeatInfo[];
  general_feats?: FeatInfo[];
  custom_feats?: FeatInfo[];
  background_feats?: FeatInfo[];
  domain_feats?: FeatInfo[];
}

export function aggregateFeats(summary: FeatsSummary | null | undefined): FeatInfo[] {
  if (!summary) return [];

  const allFeats = [
    ...(summary.class_feats || []),
    ...(summary.general_feats || []),
    ...(summary.custom_feats || []),
    ...(summary.background_feats || []),
    ...(summary.domain_feats || []),
  ];

  const unique = new Map<number, FeatInfo>();
  for (const feat of allFeats) {
    unique.set(feat.id, feat);
  }
  return Array.from(unique.values());
}

export function filterFeatsByType(feats: FeatInfo[], selectedTypes: Set<number>): FeatInfo[] {
  if (selectedTypes.size === 0) return feats;
  return feats.filter(feat =>
    Array.from(selectedTypes).some(type => (feat.type & type) !== 0)
  );
}

export function sortFeats(feats: FeatInfo[], sortBy: string): FeatInfo[] {
  return [...feats].sort((a, b) => {
    switch (sortBy) {
      case 'name': return a.name.localeCompare(b.name);
      case 'type': return a.type - b.type;
      default: return 0;
    }
  });
}

export function filterOwnedFeats(feats: FeatInfo[], ownedIds: Set<number>): FeatInfo[] {
  return feats.filter(feat => !ownedIds.has(feat.id));
}
