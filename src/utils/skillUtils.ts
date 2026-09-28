import type { SkillSummaryEntry } from '@/lib/bindings';

export function isValidSkillName(name: string): boolean {
  return !name.startsWith('DEL_') && !name.startsWith('***');
}

export function applySkillOverrides(
  skills: SkillSummaryEntry[],
  overrides: Record<number, number>
): SkillSummaryEntry[] {
  return skills.map(skill => {
    const overrideRanks = overrides[skill.skill_id];
    if (overrideRanks !== undefined) {
      const rankDiff = overrideRanks - skill.ranks;
      return { ...skill, ranks: overrideRanks, total: skill.total + rankDiff };
    }
    return skill;
  });
}

export function categorizeSkills(
  classSkills: SkillSummaryEntry[] | undefined,
  crossClassSkills: SkillSummaryEntry[] | undefined
): SkillSummaryEntry[] {
  const validClass = (classSkills || []).filter(s => isValidSkillName(s.name));
  const validCross = (crossClassSkills || []).filter(s => isValidSkillName(s.name));
  return [...validClass, ...validCross];
}

export interface SkillBudget {
  available: number;
  displayedSpent: number;
  overdrawn: number;
}

export function calculateSkillBudget(totalAvailable: number, spentPoints: number): SkillBudget {
  const balance = totalAvailable - spentPoints;
  return {
    available: Math.max(0, balance),
    displayedSpent: Math.min(spentPoints, totalAvailable),
    overdrawn: balance < 0 ? Math.abs(balance) : 0,
  };
}

export function skillMisc(skill: SkillSummaryEntry): number {
  return (skill.feat_bonus || 0) + (skill.item_bonus || 0);
}

export type SkillSortColumn = 'name' | 'ranks' | 'modifier' | 'misc' | 'total' | 'class';

export function filterAndSortSkills(
  skills: SkillSummaryEntry[],
  searchTerm: string,
  sortColumn: SkillSortColumn | null,
  sortDirection: 'asc' | 'desc'
): SkillSummaryEntry[] {
  let result = skills;

  if (searchTerm) {
    const lower = searchTerm.toLowerCase();
    result = result.filter(s => s.name.toLowerCase().includes(lower));
  }

  if (!sortColumn) return [...result];

  return [...result].sort((a, b) => {
    let cmp = 0;
    switch (sortColumn) {
      case 'name': cmp = a.name.localeCompare(b.name); break;
      case 'total': cmp = a.total - b.total; break;
      case 'ranks': cmp = a.ranks - b.ranks; break;
      case 'modifier': cmp = a.modifier - b.modifier; break;
      case 'misc': cmp = skillMisc(a) - skillMisc(b); break;
      case 'class': cmp = Number(b.is_class_skill) - Number(a.is_class_skill); break;
    }
    if (sortDirection === 'desc') cmp = -cmp;
    return cmp || a.name.localeCompare(b.name);
  });
}
