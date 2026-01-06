export type ItemRarity = 'common' | 'uncommon' | 'rare' | 'epic' | 'legendary';

export function getRarityBorderColor(rarity?: string): string {
  switch (rarity) {
    case 'uncommon': return 'border-[rgb(var(--color-success))]';
    case 'rare': return 'border-[rgb(var(--color-primary))]';
    case 'epic': return 'border-[rgb(var(--color-secondary))]';
    case 'legendary': return 'border-[rgb(var(--color-warning))]';
    default: return 'border-[rgb(var(--color-surface-border)/0.6)]';
  }
}

export function getRarityTextColor(rarity?: string): string {
  switch (rarity) {
    case 'uncommon': return 'text-[rgb(var(--color-success))]';
    case 'rare': return 'text-[rgb(var(--color-primary))]';
    case 'epic': return 'text-[rgb(var(--color-secondary))]';
    case 'legendary': return 'text-[rgb(var(--color-warning))]';
    default: return 'text-[rgb(var(--color-text-primary))]';
  }
}
