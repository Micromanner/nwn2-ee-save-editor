import { describe, it, expect } from 'vitest';
import {
  mapKnownSpellsToSpellInfo,
  groupMemorizedSpells,
  mapCasterClasses,
  filterSpells,
  sortSpells,
} from '../spellUtils';
import type { SpellInfo } from '@/components/Spells/types';

const makeKnownSpell = (id: number, name: string, level: number, classId: number) => ({
  spell_id: id, name, level, icon: 'icon', school_name: 'Evocation',
  description: '', class_id: classId, is_domain_spell: false,
});

const makeMemorizedSpell = (id: number, name: string, level: number, classId: number) => ({
  spell_id: id, name, level, icon: 'icon', school_name: 'Evocation',
  description: '', class_id: classId, metamagic: 0, ready: true,
});

describe('mapKnownSpellsToSpellInfo', () => {
  it('transforms KnownSpell array into SpellInfo array', () => {
    const known = [makeKnownSpell(1, 'Fireball', 3, 10)];
    const result = mapKnownSpellsToSpellInfo(known);
    expect(result).toHaveLength(1);
    expect(result[0]).toMatchObject({ id: 1, name: 'Fireball', level: 3, class_id: 10 });
  });

  it('returns empty array for null input', () => {
    expect(mapKnownSpellsToSpellInfo(null)).toEqual([]);
  });
});

describe('groupMemorizedSpells', () => {
  it('groups duplicate memorized spells by spell_id-level-class_id key', () => {
    const memorized = [
      makeMemorizedSpell(1, 'Fireball', 3, 10),
      makeMemorizedSpell(1, 'Fireball', 3, 10),
      makeMemorizedSpell(2, 'Magic Missile', 1, 10),
    ];
    const result = groupMemorizedSpells(memorized);
    expect(result).toHaveLength(2);
    const fireball = result.find(s => s.id === 1);
    expect(fireball?.memorized_count).toBe(2);
    const mm = result.find(s => s.id === 2);
    expect(mm?.memorized_count).toBe(1);
  });

  it('returns empty array for null input', () => {
    expect(groupMemorizedSpells(null)).toEqual([]);
  });
});

describe('mapCasterClasses', () => {
  it('maps spellcasting_classes to simplified format', () => {
    const classes = [{
      index: 0, class_name: 'Wizard', class_id: 10, can_edit_spells: true,
      class_level: 5, caster_level: 5, spell_type: 'prepared' as const,
      expected_spells_known: {},
    }];
    const result = mapCasterClasses(classes);
    expect(result).toEqual([{ index: 0, name: 'Wizard', class_id: 10, can_edit_spells: true }]);
  });
});

describe('filterSpells', () => {
  const spells: SpellInfo[] = [
    { id: 1, name: 'Fireball', level: 3, school_name: 'Evocation', class_id: 10, icon: '', available_classes: [] },
    { id: 2, name: 'Shield', level: 1, school_name: 'Abjuration', class_id: 10, icon: '', available_classes: [] },
    { id: 3, name: 'Heal', level: 6, school_name: 'Conjuration', class_id: 11, icon: '', available_classes: [] },
  ];

  it('filters by class', () => {
    const result = filterSpells(spells, { classes: new Set(['11']) });
    expect(result.map(s => s.id)).toEqual([3]);
  });

  it('filters by school', () => {
    const result = filterSpells(spells, { schools: new Set(['Evocation']) });
    expect(result.map(s => s.id)).toEqual([1]);
  });

  it('filters by level', () => {
    const result = filterSpells(spells, { levels: new Set([1, 3]) });
    expect(result.map(s => s.id)).toEqual([1, 2]);
  });

  it('applies all filters together', () => {
    const result = filterSpells(spells, { classes: new Set(['10']), levels: new Set([3]) });
    expect(result.map(s => s.id)).toEqual([1]);
  });
});

describe('sortSpells', () => {
  const spells: SpellInfo[] = [
    { id: 1, name: 'Fireball', level: 3, school_name: 'Evocation', icon: '', available_classes: [] },
    { id: 2, name: 'Aid', level: 2, school_name: 'Abjuration', icon: '', available_classes: [] },
  ];

  it('sorts by name', () => {
    expect(sortSpells(spells, 'name').map(s => s.name)).toEqual(['Aid', 'Fireball']);
  });

  it('sorts by level', () => {
    expect(sortSpells(spells, 'level').map(s => s.level)).toEqual([2, 3]);
  });

  it('sorts by school', () => {
    expect(sortSpells(spells, 'school').map(s => s.school_name)).toEqual(['Abjuration', 'Evocation']);
  });
});
