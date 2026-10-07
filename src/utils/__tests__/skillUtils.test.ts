import { describe, it, expect } from 'vitest';
import {
  isValidSkillName,
  applySkillOverrides,
  categorizeSkills,
  calculateSkillBudget,
  filterAndSortSkills,
} from '../skillUtils';
import type { SkillSummaryEntry } from '@/lib/bindings';

const makeSkill = (id: number, name: string, ranks: number, total: number, isClass = true): SkillSummaryEntry => ({
  skill_id: id, name, ranks, max_ranks: 23, total, is_class_skill: isClass,
  modifier: 0, feat_bonus: 0, item_bonus: 0, ability: 'STR',
  armor_check_penalty: false, untrained: true,
});

describe('isValidSkillName', () => {
  it('rejects DEL_ prefixed names', () => {
    expect(isValidSkillName('DEL_OldSkill')).toBe(false);
  });
  it('rejects *** prefixed names', () => {
    expect(isValidSkillName('***Unused')).toBe(false);
  });
  it('accepts normal skill names', () => {
    expect(isValidSkillName('Tumble')).toBe(true);
  });
});

describe('applySkillOverrides', () => {
  it('applies rank override and adjusts total accordingly', () => {
    const skills = [makeSkill(1, 'Tumble', 5, 8)];
    const overrides = { 1: 7 };
    const result = applySkillOverrides(skills, overrides);
    expect(result[0].ranks).toBe(7);
    expect(result[0].total).toBe(10);
  });

  it('does not mutate original skills', () => {
    const skills = [makeSkill(1, 'Tumble', 5, 8)];
    applySkillOverrides(skills, { 1: 7 });
    expect(skills[0].ranks).toBe(5);
  });

  it('returns unchanged skills when no overrides match', () => {
    const skills = [makeSkill(1, 'Tumble', 5, 8)];
    const result = applySkillOverrides(skills, { 99: 10 });
    expect(result[0]).toBe(skills[0]);
  });
});

describe('categorizeSkills', () => {
  it('separates class and cross-class skills, filtering invalid names', () => {
    const classSkills = [makeSkill(1, 'Tumble', 5, 8, true), makeSkill(2, 'DEL_Old', 0, 0, true)];
    const crossClassSkills = [makeSkill(3, 'Spellcraft', 3, 5, false)];
    const result = categorizeSkills(classSkills, crossClassSkills);
    expect(result).toHaveLength(2);
    expect(result.map(s => s.name)).toEqual(['Tumble', 'Spellcraft']);
  });
});

describe('calculateSkillBudget', () => {
  it('calculates available, spent, and overdrawn points', () => {
    const result = calculateSkillBudget(100, 80);
    expect(result.available).toBe(20);
    expect(result.displayedSpent).toBe(80);
    expect(result.overdrawn).toBe(0);
  });

  it('detects overdrawn state', () => {
    const result = calculateSkillBudget(50, 70);
    expect(result.available).toBe(0);
    expect(result.overdrawn).toBe(20);
    expect(result.displayedSpent).toBe(50);
  });
});

describe('filterAndSortSkills', () => {
  const skills = [
    makeSkill(1, 'Tumble', 5, 8),
    makeSkill(2, 'Athletics', 3, 6),
    makeSkill(3, 'Spellcraft', 10, 15),
  ];

  it('filters by search term (case-insensitive)', () => {
    const result = filterAndSortSkills(skills, 'tumble', 'name', 'asc');
    expect(result).toHaveLength(1);
    expect(result[0].name).toBe('Tumble');
  });

  it('sorts by total descending', () => {
    const result = filterAndSortSkills(skills, '', 'total', 'desc');
    expect(result.map(s => s.total)).toEqual([15, 8, 6]);
  });

  it('sorts by name ascending', () => {
    const result = filterAndSortSkills(skills, '', 'name', 'asc');
    expect(result.map(s => s.name)).toEqual(['Athletics', 'Spellcraft', 'Tumble']);
  });
});
