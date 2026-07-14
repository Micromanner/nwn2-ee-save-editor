const MAX_XP = 1_770_000;

// Game-verified limit: the NWN2 character-selection screen hides local-vault
// characters whose experience exceeds this value.
export const VAULT_MAX_VISIBLE_XP = 561_000;

export function capXP(xp: number): number {
  if (xp < 0) return 0;
  return Math.min(xp, MAX_XP);
}

interface ClassWithStats {
  baseAttackBonus: number;
  fortitudeSave: number;
  reflexSave: number;
  willSave: number;
}

export interface AggregatedClassStats {
  totalBAB: number;
  totalFort: number;
  totalRef: number;
  totalWill: number;
}

export function aggregateClassStats(classes: ClassWithStats[]): AggregatedClassStats {
  return classes.reduce(
    (acc, c) => ({
      totalBAB: acc.totalBAB + c.baseAttackBonus,
      totalFort: acc.totalFort + c.fortitudeSave,
      totalRef: acc.totalRef + c.reflexSave,
      totalWill: acc.totalWill + c.willSave,
    }),
    { totalBAB: 0, totalFort: 0, totalRef: 0, totalWill: 0 }
  );
}

interface XPProgress {
  current_level: number;
}

export function hasLevelMismatch(xpProgress: XPProgress | null | undefined, totalLevel: number): boolean {
  if (!xpProgress) return false;
  return xpProgress.current_level !== totalLevel;
}
