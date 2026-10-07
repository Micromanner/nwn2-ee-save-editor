import type { SpellInfo } from '@/components/Spells/types';
import type { SpellcastingClass, KnownSpellEntry, MemorizedSpellEntry } from '@/lib/bindings';

export interface CasterClassInfo {
  index: number;
  name: string;
  class_id: number;
  can_edit_spells: boolean;
}

export function mapKnownSpellsToSpellInfo(knownSpells: KnownSpellEntry[] | null | undefined): SpellInfo[] {
  if (!knownSpells) return [];
  return knownSpells.map(ks => ({
    id: ks.spell_id,
    name: ks.name,
    level: ks.level,
    icon: ks.icon,
    school_name: ks.school_name,
    description: ks.description,
    class_id: ks.class_id,
    available_classes: [],
    is_domain_spell: ks.is_domain_spell,
  }));
}

export function groupMemorizedSpells(memorizedSpells: MemorizedSpellEntry[] | null | undefined): SpellInfo[] {
  if (!memorizedSpells) return [];

  const spellMap = new Map<string, { spell: SpellInfo; count: number }>();

  for (const ms of memorizedSpells) {
    const key = `${ms.spell_id}-${ms.level}-${ms.class_id}`;
    const existing = spellMap.get(key);

    if (existing) {
      existing.count++;
    } else {
      spellMap.set(key, {
        spell: {
          id: ms.spell_id,
          name: ms.name,
          level: ms.level,
          icon: ms.icon,
          school_name: ms.school_name,
          description: ms.description,
          class_id: ms.class_id,
          available_classes: [],
        },
        count: 1,
      });
    }
  }

  return Array.from(spellMap.values()).map(({ spell, count }) => ({
    ...spell,
    memorized_count: count,
  }));
}

export function mapCasterClasses(spellcastingClasses: SpellcastingClass[] | null | undefined): CasterClassInfo[] {
  if (!spellcastingClasses) return [];
  return spellcastingClasses.map(cls => ({
    index: cls.index,
    name: cls.class_name,
    class_id: cls.class_id,
    can_edit_spells: cls.can_edit_spells,
  }));
}

export interface SpellFilterOptions {
  classes?: Set<string>;
  schools?: Set<string>;
  levels?: Set<number>;
}

export function filterSpells(spells: SpellInfo[], options: SpellFilterOptions): SpellInfo[] {
  let filtered = spells;

  if (options.classes && options.classes.size > 0) {
    const classIds = new Set(Array.from(options.classes).map(Number));
    filtered = filtered.filter(s => s.class_id !== undefined && classIds.has(s.class_id));
  }

  if (options.schools && options.schools.size > 0) {
    filtered = filtered.filter(s => s.school_name && options.schools!.has(s.school_name));
  }

  if (options.levels && options.levels.size > 0) {
    filtered = filtered.filter(s => options.levels!.has(s.level));
  }

  return filtered;
}

export function sortSpells(spells: SpellInfo[], sortBy: string): SpellInfo[] {
  return [...spells].sort((a, b) => {
    switch (sortBy) {
      case 'name': return a.name.localeCompare(b.name);
      case 'level': return a.level - b.level;
      case 'school': return (a.school_name || '').localeCompare(b.school_name || '');
      default: return 0;
    }
  });
}
