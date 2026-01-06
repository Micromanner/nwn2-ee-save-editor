'use client';

import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { display, formatModifier } from '@/utils/dataHelpers';

interface InventoryCharacterSummaryProps {
  combatStats?: {
    ac: number;
    bab: number;
  };
}

export default function InventoryCharacterSummary({ combatStats }: InventoryCharacterSummaryProps) {
  const t = useTranslations();
  const { character } = useCharacterContext();

  if (!character) return null;

  return (
    <Card className="min-w-[320px] h-full">
      <CardContent className="p-6 space-y-8">
        <div className="text-center space-y-2">
           <h3 className="text-xl font-bold text-[rgb(var(--color-text-primary))]">
            {display(character.name)}
          </h3>
          <div className="flex flex-col items-center gap-1 text-sm text-[rgb(var(--color-text-secondary))]">
            {character.classes?.map((cls, idx) => (
                <span key={idx} className="px-2 py-0.5 bg-[rgb(var(--color-surface-2))] rounded">
                  {cls.name} {cls.level}
                </span>
            ))}
             <span className="text-xs text-[rgb(var(--color-text-muted))] mt-1">
                {character.subrace ? `${character.subrace} ${character.race}` : display(character.race)}
            </span>
          </div>
        </div>

        <div className="border-t border-[rgb(var(--color-surface-border)/0.4)] my-4" />

        <div className="space-y-4">
             <h4 className="text-xs font-semibold uppercase text-[rgb(var(--color-text-muted))] tracking-wider border-b border-[rgb(var(--color-surface-border)/0.4)] pb-2">
                {t('inventory.quickStats')}
            </h4>
            <div className="grid grid-cols-2 gap-4">
                 <div className="bg-[rgb(var(--color-surface-2))] p-3 rounded-lg flex flex-col items-center justify-center">
                    <span className="text-2xl font-bold text-[rgb(var(--color-text-primary))]">
                        {combatStats?.ac || '-'}
                    </span>
                    <span className="text-xs text-[rgb(var(--color-text-muted))] uppercase">AC</span>
                 </div>
                 <div className="bg-[rgb(var(--color-surface-2))] p-3 rounded-lg flex flex-col items-center justify-center">
                    <span className="text-2xl font-bold text-[rgb(var(--color-text-primary))]">
                        {combatStats?.bab !== undefined ? formatModifier(combatStats.bab) : '-'}
                    </span>
                    <span className="text-xs text-[rgb(var(--color-text-muted))] uppercase">BAB</span>
                 </div>
            </div>
        </div>

      </CardContent>
    </Card>
  );
}
