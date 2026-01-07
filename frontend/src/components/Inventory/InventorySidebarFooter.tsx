
import { useTranslations } from '@/hooks/useTranslations';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { useGoldInput } from '@/hooks/useGoldInput';
import { useEncumbrance, EncumbranceData } from '@/hooks/useEncumbrance';

interface InventorySidebarFooterProps {
  encumbrance?: EncumbranceData;
  onAdd?: () => void;
}

export default function InventorySidebarFooter({ encumbrance, onAdd }: InventorySidebarFooterProps) {
  const t = useTranslations();
  const { character } = useCharacterContext();

  const {
    goldValue,
    isUpdatingGold,
    hasGoldChanged,
    handleUpdateGold,
    handleGoldInputChange,
    handleGoldKeyDown,
    resetGold,
  } = useGoldInput();

  const { currentWeight, maxWeight, weightPercentage, progressBarColor } = useEncumbrance(encumbrance);

  if (!character) return null;

  return (
    <div className="w-full pt-4 border-t border-[rgb(var(--color-surface-border)/0.4)] space-y-4">
        <div className="space-y-2">
             <div className="flex justify-between items-end">
                <h4 className="text-xs font-semibold uppercase text-[rgb(var(--color-text-muted))] tracking-wider">
                    {t('inventory.weight')}
                </h4>
                <span className="text-sm font-medium text-[rgb(var(--color-text-primary))]">
                    {currentWeight.toFixed(1)} <span className="text-[rgb(var(--color-text-muted))]">/ {maxWeight.toFixed(0)} lbs</span>
                </span>
            </div>

            <div className="h-2 w-full bg-[rgb(var(--color-surface-2))] rounded-full overflow-hidden border border-[rgb(var(--color-surface-border))]">
                <div
                    className={`h-full ${progressBarColor} transition-all duration-300`}
                    style={{ width: `${weightPercentage}%` }}
                />
            </div>
        </div>

        <div className="space-y-2">
            <h4 className="text-xs font-semibold uppercase text-[rgb(var(--color-text-muted))] tracking-wider">
                {t('inventory.gold')}
            </h4>
            <div className="flex items-center gap-2">
                <Input
                    type="text"
                    value={goldValue}
                    onChange={handleGoldInputChange}
                    onKeyDown={handleGoldKeyDown}
                    className="flex-1 text-lg font-bold text-[rgb(var(--color-text-primary))] bg-[rgb(var(--color-surface-2))] h-9"
                    disabled={isUpdatingGold}
                />
                <div className="flex gap-1">
                     <Button
                        size="sm"
                        onClick={handleUpdateGold}
                        disabled={isUpdatingGold || !hasGoldChanged}
                        className={`h-9 w-9 p-0 ${!hasGoldChanged ? 'opacity-50' : ''}`}
                        title={t('actions.save')}
                        variant="ghost"
                      >
                        <span className={hasGoldChanged ? "text-[rgb(var(--color-success))]" : "text-[rgb(var(--color-text-muted))]"} >✓</span>
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={resetGold}
                        disabled={isUpdatingGold || !hasGoldChanged}
                        className={`h-9 w-9 p-0 ${!hasGoldChanged ? 'opacity-30' : 'opacity-100'}`}
                         title={t('actions.cancel')}
                      >
                        <span className="text-[rgb(var(--color-text-muted))]">✕</span>
                      </Button>
                </div>
            </div>
        </div>

        {onAdd && (
            <Button
                onClick={onAdd}
                className="w-full bg-[rgb(var(--color-primary))] text-white hover:bg-[rgb(var(--color-primary)/0.9)]"
            >
                Add Item
            </Button>
        )}
    </div>
  );
}
