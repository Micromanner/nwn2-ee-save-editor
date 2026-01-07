
import React from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { formatNumber } from '@/utils/dataHelpers';
import { getRarityTextColor } from '@/utils/itemHelpers';
import { Trash2, Edit } from 'lucide-react';

export interface DecodedProperty {
  property_id: number;
  label: string;
  description: string;
  bonus_type: string;
  bonus_value?: number;
  [key: string]: unknown;
}

interface Item {
  id: string;
  name: string;
  icon?: string;
  stackSize?: number;
  maxStack?: number;
  type: 'weapon' | 'armor' | 'accessory' | 'consumable' | 'misc';
  equipped?: boolean;
  slot?: string;
  rarity?: 'common' | 'uncommon' | 'rare' | 'epic' | 'legendary';
  enhancement_bonus?: number;
  charges?: number;
  is_custom?: boolean;
  is_identified?: boolean;
  is_plot?: boolean;
  is_cursed?: boolean;
  is_stolen?: boolean;
}

interface ItemDetailsPanelProps {
  item: Item | null;
  decodedProperties?: DecodedProperty[];
  description?: string;
  baseItemName?: string;
  weight?: number;
  value?: number;
  baseAc?: number | null;
  rawData?: Record<string, unknown>;
  onEquip?: () => void;
  onUnequip?: () => void;
  onEdit?: () => void;
  onDestroy?: () => void;
  isEquipping?: boolean;
  canEquip?: boolean;
  canUnequip?: boolean;
}

export default function ItemDetailsPanel({
  item,
  decodedProperties,
  description,
  baseItemName,
  weight = 0,
  value = 0,
  baseAc,
  rawData,
  onEquip,
  onUnequip,
  onEdit,
  onDestroy,
  isEquipping = false,
  canEquip = false,
  canUnequip = false,
}: ItemDetailsPanelProps) {
  const t = useTranslations();
  const [showDebug, setShowDebug] = React.useState(false);
  const isDev = process.env.NODE_ENV === 'development';

  if (!item) {
    return null;
  }

  return (
    <Card className="min-w-[320px] h-full flex flex-col">
      <CardContent className="p-6 flex flex-col h-full gap-4">
        <div className="flex-1 min-h-0 space-y-4 overflow-y-auto custom-scrollbar pr-2">
          <div className="text-center">
            <div className="w-[58px] h-[58px] bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto mb-4 overflow-hidden">
              <div className="w-full h-full bg-[rgb(var(--color-surface-3))] flex items-center justify-center text-2xl font-bold">
                {item.name.charAt(0)}
              </div>
            </div>
            <h4 className={`font-medium ${getRarityTextColor(item.rarity)}`}>
              {item.name}
            </h4>
            {item.rarity && item.rarity !== 'common' && (
              <p className="text-xs text-[rgb(var(--color-text-muted))] mt-1">
                {item.rarity}
              </p>
            )}
          </div>

          <div className="border-t border-[rgb(var(--color-surface-border)/0.4)] pt-4">
            <div className="space-y-2 text-sm">
              {item.slot && (
                <div className="flex justify-between">
                  <span className="text-[rgb(var(--color-text-muted))]">Equipped in:</span>
                  <span className="text-[rgb(var(--color-text-primary))]">{item.slot}</span>
                </div>
              )}

              {item.enhancement_bonus !== undefined && item.enhancement_bonus > 0 && (
                <div className="flex justify-between">
                  <span className="text-[rgb(var(--color-text-muted))]">{t('inventory.enhancement')}:</span>
                  <span className="text-[rgb(var(--color-success))]">+{item.enhancement_bonus}</span>
                </div>
              )}

              {baseAc !== undefined && baseAc !== null && baseAc > 0 && (
                <div className="flex justify-between">
                  <span className="text-[rgb(var(--color-text-muted))]">{t('inventory.baseArmorClass')}:</span>
                  <span className="text-[rgb(var(--color-primary))]">+{baseAc}</span>
                </div>
              )}

              {item.charges !== undefined && item.charges > 0 && (
                <div className="flex justify-between">
                  <span className="text-[rgb(var(--color-text-muted))]">{t('inventory.charges')}:</span>
                  <span className="text-[rgb(var(--color-text-primary))]">{item.charges}</span>
                </div>
              )}

              {item.stackSize !== undefined && item.stackSize > 1 && (
                <div className="flex justify-between">
                  <span className="text-[rgb(var(--color-text-muted))]">{t('inventory.stack')}:</span>
                  <span className="text-[rgb(var(--color-text-primary))]">
                    {item.stackSize} / {item.maxStack || item.stackSize}
                  </span>
                </div>
              )}

              {value > 0 && (
                <div className="flex justify-between">
                  <span className="text-white font-medium">{t('inventory.value')}:</span>
                  <span className="text-[rgb(var(--color-warning))]">{formatNumber(value)} gp</span>
                </div>
              )}

              {weight > 0 && (
                <div className="flex justify-between">
                  <span className="text-white font-medium">{t('inventory.weight')}:</span>
                  <span className="text-[rgb(var(--color-text-primary))]">{weight.toFixed(1)} lbs</span>
                </div>
              )}
            </div>
          </div>

          {description && (
            <div className="border-t border-[rgb(var(--color-surface-border)/0.4)] pt-4">
              <h5 className="text-sm font-semibold text-[rgb(var(--color-text-primary))] mb-2">
                {t('inventory.description')}
              </h5>
              <p className="text-sm text-[rgb(var(--color-text-muted))] leading-relaxed mt-2 max-h-[100px] overflow-y-auto custom-scrollbar pr-2 whitespace-pre-line">
                {description}
              </p>
            </div>
          )}

          {baseItemName && (
            <div className="border-t border-[rgb(var(--color-surface-border)/0.4)] pt-4">
              <div className="text-sm">
                <span className="text-[rgb(var(--color-text-primary))] font-medium">{t('inventory.baseItem')}: </span>
                <span className="text-[rgb(var(--color-text-muted))]">{baseItemName}</span>
              </div>
            </div>
          )}

          {decodedProperties && decodedProperties.length > 0 && (
            <div className="border-t border-[rgb(var(--color-surface-border)/0.4)] pt-4">
              <h5 className="text-sm font-semibold text-[rgb(var(--color-text-primary))] mb-2">
                {t('inventory.properties')}
              </h5>
              <div className="space-y-1 max-h-[135px] overflow-y-auto custom-scrollbar pr-2">
                {decodedProperties.map((prop, idx) => {
                  const displayText = prop.label || prop.description || 'Unknown Property';

                  return (
                    <div key={idx} className="text-sm text-[rgb(var(--color-text-primary))] bg-[rgb(var(--color-surface-1)/0.5)] rounded px-2 py-1.5">
                      {displayText}
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {(item.is_custom || item.is_plot || item.is_cursed || item.is_stolen) && (
            <div className="border-t border-[rgb(var(--color-surface-border)/0.4)] pt-4">
              <div className="space-y-1 text-sm">
                {item.is_custom && (
                  <div className="flex items-center text-[rgb(var(--color-warning))]">
                    <span className="mr-2">⚠️</span>
                    <span>{t('inventory.customItem')}</span>
                  </div>
                )}
                {item.is_plot && (
                  <div className="flex items-center text-[rgb(var(--color-primary))]">
                    <span className="mr-2">📜</span>
                    <span>{t('inventory.plotItem')}</span>
                  </div>
                )}
                {item.is_cursed && (
                  <div className="flex items-center text-[rgb(var(--color-danger))]">
                    <span className="mr-2">💀</span>
                    <span>{t('inventory.cursedItem')}</span>
                  </div>
                )}
                {item.is_stolen && (
                  <div className="flex items-center text-[rgb(var(--color-danger))]">
                    <span className="mr-2">🗡️</span>
                    <span>{t('inventory.stolenItem')}</span>
                  </div>
                )}
              </div>
            </div>
          )}

          {isDev && showDebug && (
            <div className="border-t border-[rgb(var(--color-surface-border)/0.4)] pt-4 mt-4">
              <h5 className="text-sm font-semibold text-[rgb(var(--color-text-primary))] mb-2">
                Debug Data
              </h5>
              <div className="space-y-3">
                <div>
                  <div className="text-xs font-semibold text-[rgb(var(--color-text-secondary))] mb-1">Computed Values:</div>
                  <div className="text-xs bg-[rgb(var(--color-surface-1))] p-2 rounded font-mono">
                    <div>weight: {weight} lbs</div>
                    <div>value: {value} gp</div>
                    <div>description: {description ? `"${description.substring(0, 50)}..."` : 'null'}</div>
                  </div>
                </div>

                {decodedProperties && decodedProperties.length > 0 && (
                  <div>
                    <div className="text-xs font-semibold text-[rgb(var(--color-text-secondary))] mb-1">
                      Decoded Properties ({decodedProperties.length}):
                    </div>
                    <div className="max-h-40 overflow-y-auto text-xs bg-[rgb(var(--color-surface-1))] p-2 rounded font-mono">
                      <pre className="whitespace-pre-wrap">{JSON.stringify(decodedProperties, null, 2)}</pre>
                    </div>
                  </div>
                )}

                {rawData && (
                  <div>
                    <div className="flex items-center justify-between mb-1">
                      <div className="text-xs font-semibold text-[rgb(var(--color-text-secondary))]">Raw Backend Data:</div>
                      <button
                        onClick={() => navigator.clipboard.writeText(JSON.stringify(rawData, null, 2))}
                        className="text-xs px-2 py-0.5 rounded bg-[rgb(var(--color-primary))] text-white hover:opacity-80"
                      >
                        Copy
                      </button>
                    </div>
                    <div className="max-h-60 overflow-y-auto text-xs bg-[rgb(var(--color-surface-1))] p-2 rounded font-mono">
                      <pre className="whitespace-pre-wrap">{JSON.stringify(rawData, null, 2)}</pre>
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}

        </div>

        <div className="flex-shrink-0 mt-auto">
          <div className="border-t border-[rgb(var(--color-surface-border)/0.4)] pt-4 flex gap-2">
            {canUnequip && onUnequip && (
              <Button
                className="flex-1"
                variant="primary"
                onClick={onUnequip}
                disabled={isEquipping}
              >
                {isEquipping ? t('actions.unequipping') : t('actions.unequip')}
              </Button>
            )}

            {canEquip && onEquip && (
              <Button
                className="flex-1"
                variant="primary"
                onClick={onEquip}
                disabled={isEquipping}
              >
                {isEquipping ? t('actions.equipping') : t('actions.equip')}
              </Button>
            )}

            <Button
              variant="outline"
              size="icon"
              className="flex-shrink-0 w-10 border-[rgb(var(--color-primary)/0.5)] text-[rgb(var(--color-primary))] hover:bg-[rgb(var(--color-primary)/0.1)] hover:border-[rgb(var(--color-primary))]"
              onClick={onEdit}
              title="Edit Item"
            >
              <Edit className="w-5 h-5" />
            </Button>

            <Button
              variant="outline"
              size="icon"
              className="flex-shrink-0 w-10 border-red-500/50 text-red-400 hover:bg-red-500/10 hover:border-red-500/30 hover:text-red-500"
              disabled={!onDestroy || item.is_plot}
              onClick={onDestroy}
              title={item.is_plot ? t('inventory.cannotDestroyPlot') : t('actions.destroy')}
            >
              <Trash2 className="w-5 h-5" />
            </Button>

            {isDev && (
              <button
                onClick={() => setShowDebug(!showDebug)}
                className="flex-shrink-0 w-10 h-10 flex items-center justify-center rounded border border-[rgb(var(--color-surface-border))] text-[rgb(var(--color-text-muted))] hover:bg-[rgb(var(--color-surface-2))] hover:text-[rgb(var(--color-text-secondary))] transition-colors ml-2"
                title="Toggle debug info"
              >
                <span className="text-xs font-mono">DBG</span>
              </button>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
