import { describe, it, expect } from 'vitest';
import {
  aggregateFeats,
  filterFeatsByType,
  sortFeats,
  filterOwnedFeats,
} from '../featUtils';
import type { FeatPrerequisites, FeatInfo } from '@/lib/bindings';

const noPrerequisites: FeatPrerequisites = {
  feats: [],
  any_feats: [],
  abilities: [],
  bab: null,
  level: null,
  class_level: null,
  max_level: null,
  epic: null,
  base_fortitude: null,
  caster_level: null,
  spell_level: null,
  skills: [],
};

const makeFeat = (id: number, name: string, type = 0): FeatInfo => ({
  id, name, label: name, type, description: '', icon: '', category: 'Unknown', has_feat: false, protected: false, custom: false,
  prerequisites: noPrerequisites,
});

describe('aggregateFeats', () => {
  it('flattens all feat categories and deduplicates by id', () => {
    const summary = {
      class_feats: [makeFeat(2, 'Cleave')],
      general_feats: [makeFeat(1, 'Power Attack')],
      custom_feats: [],
      background_feats: [makeFeat(3, 'Luck of Heroes')],
      domain_feats: [],
    };
    const result = aggregateFeats(summary);
    expect(result).toHaveLength(3);
    expect(result.map(f => f.id)).toEqual([2, 1, 3]);
  });

  it('returns empty array for null summary', () => {
    expect(aggregateFeats(null)).toEqual([]);
  });
});

describe('filterFeatsByType', () => {
  const feats = [
    makeFeat(1, 'A', 1),
    makeFeat(2, 'B', 2),
    makeFeat(3, 'C', 3),
  ];

  it('returns all feats when selectedTypes is empty', () => {
    expect(filterFeatsByType(feats, new Set())).toHaveLength(3);
  });

  it('filters by bitmask - single type', () => {
    const result = filterFeatsByType(feats, new Set([1]));
    expect(result.map(f => f.id)).toEqual([1, 3]);
  });

  it('filters by bitmask - multiple types', () => {
    const result = filterFeatsByType(feats, new Set([2]));
    expect(result.map(f => f.id)).toEqual([2, 3]);
  });
});

describe('sortFeats', () => {
  const feats = [
    makeFeat(1, 'Cleave', 2),
    makeFeat(2, 'Alertness', 1),
    makeFeat(3, 'Bash', 3),
  ];

  it('sorts by name ascending', () => {
    const result = sortFeats(feats, 'name');
    expect(result.map(f => f.name)).toEqual(['Alertness', 'Bash', 'Cleave']);
  });

  it('sorts by type ascending', () => {
    const result = sortFeats(feats, 'type');
    expect(result.map(f => f.type)).toEqual([1, 2, 3]);
  });

  it('does not mutate input array', () => {
    const original = [...feats];
    sortFeats(feats, 'name');
    expect(feats).toEqual(original);
  });
});

describe('filterOwnedFeats', () => {
  it('removes feats that exist in ownedIds set', () => {
    const feats = [makeFeat(1, 'A'), makeFeat(2, 'B'), makeFeat(3, 'C')];
    const owned = new Set([1, 3]);
    expect(filterOwnedFeats(feats, owned).map(f => f.id)).toEqual([2]);
  });
});
