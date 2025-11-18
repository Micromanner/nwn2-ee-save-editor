'use client';

import React from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { formatNumber } from '@/utils/dataHelpers';

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
  weight?: number;
  value?: number;
  rawData?: Record<string, unknown>;
  onEquip?: () => void;
  onUnequip?: () => void;
  onDestroy?: () => void;
  isEquipping?: boolean;
  canEquip?: boolean;
  canUnequip?: boolean;
}

export default function ItemDetailsPanel({
  item,
  decodedProperties,
  description,
  weight = 0,
  value = 0,
  rawData,
  onEquip,
  onUnequip,
  onDestroy,
  isEquipping = false,
  canEquip = false,
  canUnequip = false,
}: ItemDetailsPanelProps) {
  const t = useTranslations();
  const [showDebug, setShowDebug] = React.useState(false);

  if (!item) {
    return null;
  }

  const getRarityColor = (rarity?: string) => {
    switch (rarity) {
      case 'uncommon': return 'text-[rgb(var(--color-success))]';
      case 'rare': return 'text-[rgb(var(--color-primary))]';
      case 'epic': return 'text-[rgb(var(--color-secondary))]';
      case 'legendary': return 'text-[rgb(var(--color-warning))]';
      default: return 'text-[rgb(var(--color-text-primary))]';
    }
  };

  return (
    <Card>
      <CardContent className="p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">
            {t('inventory.itemDetails')}
          </h3>
          <button
            onClick={() => setShowDebug(!showDebug)}
            className="text-xs px-2 py-1 rounded bg-[rgb(var(--color-surface-2))] hover:bg-[rgb(var(--color-surface-3))] text-[rgb(var(--color-text-secondary))] transition-colors"
            title="Toggle debug info"
          >
            Debug
          </button>
        </div>

        <div className="space-y-4">
          <div className="text-center">
            <div className="w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto mb-2 flex items-center justify-center">
              <div className="w-8 h-8 bg-[rgb(var(--color-surface-3))] rounded flex items-center justify-center text-xs font-bold">
                {item.name.charAt(0)}
              </div>
            </div>
            <h4 className={`font-medium ${getRarityColor(item.rarity)}`}>
              {item.name}
            </h4>
            <p className="text-xs text-[rgb(var(--color-text-muted))] mt-1">
              {item.rarity || 'common'}
            </p>
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
                  <span className="text-[rgb(var(--color-text-muted))]">{t('inventory.value')}:</span>
                  <span className="text-[rgb(var(--color-warning))]">{formatNumber(value)} gp</span>
                </div>
              )}

              {weight > 0 && (
                <div className="flex justify-between">
                  <span className="text-[rgb(var(--color-text-muted))]">{t('inventory.weight')}:</span>
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
              <p className="text-sm text-[rgb(var(--color-text-muted))] max-h-32 overflow-y-auto">
                {description}
              </p>
            </div>
          )}

          {decodedProperties && decodedProperties.length > 0 && (
            <div className="border-t border-[rgb(var(--color-surface-border)/0.4)] pt-4">
              <h5 className="text-sm font-semibold text-[rgb(var(--color-text-primary))] mb-2">
                {t('inventory.properties')}
              </h5>
              <div className="space-y-1 max-h-48 overflow-y-auto">
                {decodedProperties.map((prop, idx) => {
                  const showDescription = prop.description &&
                    typeof prop.description === 'string' &&
                    prop.description.toLowerCase() !== prop.label.toLowerCase() &&
                    !prop.description.toLowerCase().startsWith(prop.label.toLowerCase());

                  const usesPerDay = (prop as Record<string, unknown>).uses_per_day;
                  const showUsesPerDay = usesPerDay !== undefined && usesPerDay !== null;

                  return (
                    <div key={idx} className="flex flex-col text-sm bg-[rgb(var(--color-surface-1)/0.5)] rounded px-2 py-1.5">
                      <div className="flex justify-between items-center">
                        <span className="text-[rgb(var(--color-text-primary))] font-medium">
                          {prop.label}
                        </span>
                        <div className="flex items-center gap-2">
                          {showUsesPerDay && (
                            <span className="text-xs text-[rgb(var(--color-text-muted))]">
                              ({typeof usesPerDay === 'string' ? usesPerDay : `${usesPerDay} ${t('inventory.perDay')}`})
                            </span>
                          )}
                          {prop.bonus_value !== undefined && (
                            <span className="text-[rgb(var(--color-success))] font-medium">
                              {prop.bonus_value > 0 ? `+${prop.bonus_value}` : prop.bonus_value}
                            </span>
                          )}
                        </div>
                      </div>
                      {showDescription && (
                        <span className="text-xs text-[rgb(var(--color-text-muted))] mt-0.5">
                          {prop.description}
                        </span>
                      )}
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

          <div className="border-t border-[rgb(var(--color-surface-border)/0.4)] pt-4 space-y-2">
            {canUnequip && onUnequip && (
              <Button
                className="w-full"
                size="sm"
                onClick={onUnequip}
                disabled={isEquipping}
              >
                {isEquipping ? t('actions.unequipping') : t('actions.unequip')}
              </Button>
            )}

            {canEquip && onEquip && (
              <Button
                className="w-full"
                size="sm"
                onClick={onEquip}
                disabled={isEquipping}
              >
                {isEquipping ? t('actions.equipping') : t('actions.equip')}
              </Button>
            )}

            <Button
              variant="danger"
              size="sm"
              className="w-full"
              disabled={item.is_plot}
              onClick={onDestroy}
            >
              {item.is_plot ? 'Cannot Destroy' : t('actions.destroy')}
            </Button>
          </div>

          {/* Debug Panel */}
          {showDebug && (
            <div className="border-t border-[rgb(var(--color-surface-border)/0.4)] pt-4 mt-4">
              <h5 className="text-sm font-semibold text-[rgb(var(--color-text-primary))] mb-2">
                Debug Data
              </h5>
              <div className="space-y-3">
                {/* Computed Values */}
                <div>
                  <div className="text-xs font-semibold text-[rgb(var(--color-text-secondary))] mb-1">Computed Values:</div>
                  <div className="text-xs bg-[rgb(var(--color-surface-1))] p-2 rounded font-mono">
                    <div>weight: {weight} lbs</div>
                    <div>value: {value} gp</div>
                    <div>description: {description ? `"${description.substring(0, 50)}..."` : 'null'}</div>
                  </div>
                </div>

                {/* Decoded Properties */}
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

                {/* Raw Backend Data */}
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
      </CardContent>
    </Card>
  );
}
