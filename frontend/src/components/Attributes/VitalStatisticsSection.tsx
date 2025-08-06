'use client';

import { useState } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Label } from '@/components/ui/Label';
import { display, formatModifier } from '@/utils/dataHelpers';

interface CharacterStats {
  hitPoints: number;
  maxHitPoints: number;
  experience: number;
  level: number;
  armorClass: {
    base: number; // Natural Armor bonus
    total: number;
  };
  initiative: {
    base: number; // Miscellaneous Initiative bonus
    total: number;
  };
  fortitude: {
    base: number;
    total: number;
  };
  reflex: {
    base: number;
    total: number;
  };
  will: {
    base: number;
    total: number;
  };
}

interface VitalStatisticsSectionProps {
  stats?: CharacterStats;
  onStatsChange?: (stats: CharacterStats) => void;
}

export default function VitalStatisticsSection({
  stats: externalStats,
  onStatsChange
}: VitalStatisticsSectionProps) {
  const t = useTranslations();
  
  // Default stats if none provided
  const [internalStats, setInternalStats] = useState<CharacterStats>({
    hitPoints: 100,
    maxHitPoints: 100,
    experience: 0,
    level: 1,
    armorClass: { base: 0, total: 10 }, // base = natural armor (default 0)
    initiative: { base: 0, total: 0 },
    fortitude: { base: 0, total: 0 },
    reflex: { base: 0, total: 0 },
    will: { base: 0, total: 0 },
  });

  // Use external stats if provided, otherwise use internal state
  const stats = externalStats || internalStats;

  const updateStats = (updates: Partial<CharacterStats>) => {
    const newStats = { ...stats, ...updates };
    
    if (onStatsChange) {
      onStatsChange(newStats);
    } else {
      setInternalStats(newStats);
    }
  };

  const healthPercentage = Math.min(100, (stats.hitPoints / stats.maxHitPoints) * 100);
  
  // Determine health bar color based on percentage
  const getHealthBarClass = (percentage: number) => {
    if (percentage >= 70) return 'high';
    if (percentage >= 30) return 'medium';
    return 'low';
  };

  return (
    <Card variant="container">
      <CardContent className="p-6">
        <h3 className="section-title">{t('attributes.vitalStatistics')}</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          
          {/* Hit Points Card */}
          <Card variant="container" className="md:col-span-1">
            <CardHeader padding="p-4" noBorder>
              <CardTitle textSize="text-base">
                {t('attributes.health')}
              </CardTitle>
            </CardHeader>
            <CardContent padding="p-4 pt-0">
              <div className="mx-auto max-w-[80%] mb-4">
                <div 
                  className="relative bg-[rgb(var(--color-surface-3))] rounded-full h-6 overflow-hidden border border-[rgb(var(--color-surface-border)/0.5)]"
                >
                  <div 
                    className={`h-full rounded-full transition-all duration-300 ${getHealthBarClass(healthPercentage) === 'high' ? 'bg-green-500' : getHealthBarClass(healthPercentage) === 'medium' ? 'bg-yellow-500' : 'bg-red-500'}`}
                    style={{ width: `${healthPercentage}%` }}
                  />
                  <div className="absolute inset-0 flex items-center justify-center text-xs font-semibold text-[rgb(var(--color-text-primary))] drop-shadow-sm">
                    {Math.round(healthPercentage)}%
                  </div>
                </div>
              </div>
              <div>
                <Label className="mb-2">{t('character.hitPoints')}</Label>
                <div className="flex items-center justify-center gap-2">
                  <Input
                    type="number"
                    value={stats.hitPoints}
                    onChange={(e) => updateStats({ hitPoints: parseInt(e.target.value) || 0 })}
                    className="w-16 text-center"
                    min="0"
                  />
                  <span className="text-[rgb(var(--color-text-muted))]">/</span>
                  <Input
                    type="number"
                    value={stats.maxHitPoints}
                    onChange={(e) => updateStats({ maxHitPoints: parseInt(e.target.value) || 0 })}
                    className="w-16 text-center"
                    min="1"
                  />
                </div>
              </div>
            </CardContent>
          </Card>


          {/* Defense Card */}
          <Card variant="container" className="md:col-span-2">
            <CardHeader padding="p-4" noBorder>
              <CardTitle textSize="text-base">
                {t('attributes.defense')}
              </CardTitle>
            </CardHeader>
            <CardContent padding="p-4 pt-0">
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                
                {/* Armor Class Card - Base + Calculated */}
                <Card variant="container">
                  <CardContent padding="p-3">
                    <Label className="mb-3">
                      {t('character.armorClass')}
                    </Label>
                    <div className="space-y-2">
                      {/* Natural Armor - Editable */}
                      <div className="mb-4">
                        <Label className="text-xs text-muted mb-1 block">Natural Armor</Label>
                        <div className="attribute-controls-mobile">
                          <Button
                            variant="outline"
                            size="md"
                            onClick={() => updateStats({ 
                              ...stats, 
                              armorClass: { ...stats.armorClass, base: stats.armorClass.base - 1 }
                            })}
                            aria-label="Decrease Natural Armor"
                          >
                            −
                          </Button>
                          <input
                            type="number"
                            value={stats.armorClass.base}
                            onChange={(e) => updateStats({ 
                              ...stats, 
                              armorClass: { ...stats.armorClass, base: parseInt(e.target.value) || 0 }
                            })}
                            className="attribute-input-responsive"
                            min="0"
                            aria-label="Natural Armor value"
                          />
                          <Button
                            variant="outline"
                            size="md"
                            onClick={() => updateStats({ 
                              ...stats, 
                              armorClass: { ...stats.armorClass, base: stats.armorClass.base + 1 }
                            })}
                            aria-label="Increase Natural Armor"
                          >
                            +
                          </Button>
                        </div>
                      </div>
                      
                      {/* Total AC - Calculated */}
                      <div className="text-center pt-1">
                        <div className="text-xs text-muted">Total AC</div>
                        <span className="text-xl font-bold text-[rgb(var(--color-text-primary))]">
                          {display(stats.armorClass.total)}
                        </span>
                        <div className="text-xs text-muted">10 + Natural + DEX + Equipment</div>
                      </div>
                    </div>
                  </CardContent>
                </Card>

                {/* Initiative Card - Base + Calculated */}
                <Card variant="container">
                  <CardContent padding="p-3">
                    <Label className="mb-3">
                      {t('character.initiative')}
                    </Label>
                    <div className="space-y-2">
                      {/* Misc Initiative Bonus - Editable */}
                      <div className="mb-4">
                        <Label className="text-xs text-muted mb-1 block">Misc Initiative Bonus</Label>
                        <div className="attribute-controls-mobile">
                          <Button
                            variant="outline"
                            size="md"
                            onClick={() => updateStats({ 
                              ...stats, 
                              initiative: { ...stats.initiative, base: stats.initiative.base - 1 }
                            })}
                            aria-label="Decrease Misc Initiative Bonus"
                          >
                            −
                          </Button>
                          <input
                            type="number"
                            value={stats.initiative.base}
                            onChange={(e) => updateStats({ 
                              ...stats, 
                              initiative: { ...stats.initiative, base: parseInt(e.target.value) || 0 }
                            })}
                            className="attribute-input-responsive"
                            aria-label="Misc Initiative Bonus value"
                          />
                          <Button
                            variant="outline"
                            size="md"
                            onClick={() => updateStats({ 
                              ...stats, 
                              initiative: { ...stats.initiative, base: stats.initiative.base + 1 }
                            })}
                            aria-label="Increase Misc Initiative Bonus"
                          >
                            +
                          </Button>
                        </div>
                      </div>
                      
                      {/* Total Initiative - Calculated */}
                      <div className="text-center pt-1">
                        <div className="text-xs text-muted">Total Initiative</div>
                        <span className="text-xl font-bold text-[rgb(var(--color-text-primary))]">
                          {formatModifier(stats.initiative.total)}
                        </span>
                        <div className="text-xs text-muted">Misc + DEX + Feats</div>
                      </div>
                    </div>
                  </CardContent>
                </Card>

                {/* Saving Throws Card - Base + Calculated */}
                <Card variant="container" className="sm:col-span-2 lg:col-span-1">
                  <CardContent padding="p-3">
                    <Label className="mb-4 block">
                      {t('attributes.savingThrows')}
                    </Label>
                    <div className="space-y-4 mt-2">
                      
                      {/* Fortitude */}
                      <div className="saving-throw-item">
                        <Label className="mb-3 block">
                          {t('attributes.fortitude')}
                        </Label>
                        <div className="grid grid-cols-2 gap-2">
                          {/* Misc Fort - Editable */}
                          <div>
                            <Label className="text-xs text-muted mb-1 block">Misc Bonus</Label>
                            <div className="attribute-controls-mobile">
                              <Button
                                variant="outline"
                                size="md"
                                onClick={() => updateStats({ 
                                  ...stats, 
                                  fortitude: { ...stats.fortitude, base: stats.fortitude.base - 1 }
                                })}
                                aria-label="Decrease Misc Fortitude Bonus"
                              >
                                −
                              </Button>
                              <input
                                type="number"
                                value={stats.fortitude.base}
                                onChange={(e) => updateStats({ 
                                  ...stats, 
                                  fortitude: { ...stats.fortitude, base: parseInt(e.target.value) || 0 }
                                })}
                                className="attribute-input-responsive"
                                min="0"
                                aria-label="Misc Fortitude Bonus value"
                              />
                              <Button
                                variant="outline"
                                size="md"
                                onClick={() => updateStats({ 
                                  ...stats, 
                                  fortitude: { ...stats.fortitude, base: stats.fortitude.base + 1 }
                                })}
                                aria-label="Increase Misc Fortitude Bonus"
                              >
                                +
                              </Button>
                            </div>
                          </div>
                          
                          {/* Total Fort - Calculated */}
                          <div className="text-center">
                            <div className="text-xs text-muted mb-1">Total</div>
                            <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">
                              {formatModifier(stats.fortitude.total)}
                            </div>
                            <div className="text-xs text-muted">Class + CON + Misc</div>
                          </div>
                        </div>
                      </div>
                      
                      {/* Separator */}
                      <div className="border-t border-[rgb(var(--color-surface-border)/0.3)] my-3"></div>
                      
                      {/* Reflex */}
                      <div className="saving-throw-item">
                        <Label className="mb-3 block">
                          {t('attributes.reflex')}
                        </Label>
                        <div className="grid grid-cols-2 gap-2">
                          {/* Misc Reflex - Editable */}
                          <div>
                            <Label className="text-xs text-muted mb-1 block">Misc Bonus</Label>
                            <div className="attribute-controls-mobile">
                              <Button
                                variant="outline"
                                size="md"
                                onClick={() => updateStats({ 
                                  ...stats, 
                                  reflex: { ...stats.reflex, base: stats.reflex.base - 1 }
                                })}
                                aria-label="Decrease Misc Reflex Bonus"
                              >
                                −
                              </Button>
                              <input
                                type="number"
                                value={stats.reflex.base}
                                onChange={(e) => updateStats({ 
                                  ...stats, 
                                  reflex: { ...stats.reflex, base: parseInt(e.target.value) || 0 }
                                })}
                                className="attribute-input-responsive"
                                min="0"
                                aria-label="Misc Reflex Bonus value"
                              />
                              <Button
                                variant="outline"
                                size="md"
                                onClick={() => updateStats({ 
                                  ...stats, 
                                  reflex: { ...stats.reflex, base: stats.reflex.base + 1 }
                                })}
                                aria-label="Increase Misc Reflex Bonus"
                              >
                                +
                              </Button>
                            </div>
                          </div>
                          
                          {/* Total Reflex - Calculated */}
                          <div className="text-center">
                            <div className="text-xs text-muted mb-1">Total</div>
                            <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">
                              {formatModifier(stats.reflex.total)}
                            </div>
                            <div className="text-xs text-muted">Class + DEX + Misc</div>
                          </div>
                        </div>
                      </div>
                      
                      {/* Separator */}
                      <div className="border-t border-[rgb(var(--color-surface-border)/0.3)] my-3"></div>
                      
                      {/* Will */}
                      <div className="saving-throw-item">
                        <Label className="mb-3 block">
                          {t('attributes.will')}
                        </Label>
                        <div className="grid grid-cols-2 gap-2">
                          {/* Misc Will - Editable */}
                          <div>
                            <Label className="text-xs text-muted mb-1 block">Misc Bonus</Label>
                            <div className="attribute-controls-mobile">
                              <Button
                                variant="outline"
                                size="md"
                                onClick={() => updateStats({ 
                                  ...stats, 
                                  will: { ...stats.will, base: stats.will.base - 1 }
                                })}
                                aria-label="Decrease Misc Will Bonus"
                              >
                                −
                              </Button>
                              <input
                                type="number"
                                value={stats.will.base}
                                onChange={(e) => updateStats({ 
                                  ...stats, 
                                  will: { ...stats.will, base: parseInt(e.target.value) || 0 }
                                })}
                                className="attribute-input-responsive"
                                min="0"
                                aria-label="Misc Will Bonus value"
                              />
                              <Button
                                variant="outline"
                                size="md"
                                onClick={() => updateStats({ 
                                  ...stats, 
                                  will: { ...stats.will, base: stats.will.base + 1 }
                                })}
                                aria-label="Increase Misc Will Bonus"
                              >
                                +
                              </Button>
                            </div>
                          </div>
                          
                          {/* Total Will - Calculated */}
                          <div className="text-center">
                            <div className="text-xs text-muted mb-1">Total</div>
                            <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">
                              {formatModifier(stats.will.total)}
                            </div>
                            <div className="text-xs text-muted">Class + WIS + Misc</div>
                          </div>
                        </div>
                      </div>
                      
                    </div>
                  </CardContent>
                </Card>
              </div>
            </CardContent>
          </Card>
        </div>
      </CardContent>
    </Card>
  );
}