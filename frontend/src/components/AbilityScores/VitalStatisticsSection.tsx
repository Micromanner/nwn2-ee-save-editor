'use client';

import { useState } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { formatModifier } from '@/utils/dataHelpers';

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
    armorClass: { base: 0, total: 10 },
    initiative: { base: 0, total: 0 },
    fortitude: { base: 0, total: 0 },
    reflex: { base: 0, total: 0 },
    will: { base: 0, total: 0 },
  });

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

  // Helper functions for updating stats via parent callback
  const updateArmorClass = (base: number) => {
    updateStats({ armorClass: { ...stats.armorClass, base } });
  };

  const updateInitiative = (base: number) => {
    updateStats({ initiative: { ...stats.initiative, base } });
  };

  const updateFortitude = (base: number) => {
    updateStats({ fortitude: { ...stats.fortitude, base } });
  };

  const updateReflex = (base: number) => {
    updateStats({ reflex: { ...stats.reflex, base } });
  };

  const updateWill = (base: number) => {
    updateStats({ will: { ...stats.will, base } });
  };

  return (
    <Card variant="container">
      <CardContent className="attribute-section-responsive">
        <h3 className="section-title">{t('abilityScores.vitalStatistics')}</h3>
        <div className="vital-stats-grid">

          {/* Health */}
          <Card variant="interactive" className="flex flex-col h-full">
            <div className="attribute-header-responsive">
              <span className="attribute-name-responsive">Health</span>
            </div>
            
            <div className="attribute-breakdown">
              {/* Health Bar */}
              <div className="breakdown-row">
                <div className="health-bar-container">
                  <div className="health-bar-track">
                    <div 
                      className={`health-bar-fill ${healthPercentage >= 70 ? 'health-high' : healthPercentage >= 30 ? 'health-medium' : 'health-low'}`}
                      style={{ width: `${healthPercentage}%` }}
                    />
                    <div className="health-bar-text">
                      {Math.round(healthPercentage)}%
                    </div>
                  </div>
                </div>
              </div>
              
              {/* HP Input */}
              <div className="breakdown-row breakdown-base">
                <span className="breakdown-label">Hit Points:</span>
                <div className="hp-controls">
                  <input
                    type="number"
                    value={stats.hitPoints}
                    onChange={(e) => updateStats({ hitPoints: parseInt(e.target.value) || 0 })}
                    className="breakdown-input"
                    min="0"
                    max={stats.maxHitPoints}
                  />
                  <span className="hp-separator">/</span>
                  <input
                    type="number"
                    value={stats.maxHitPoints}
                    onChange={(e) => updateStats({ maxHitPoints: parseInt(e.target.value) || 1 })}
                    className="breakdown-input"
                    min="1"
                  />
                </div>
              </div>
            </div>
          </Card>

          {/* Armor Class */}
          <Card variant="interactive" className="flex flex-col h-full">
            <div className="attribute-header-responsive">
              <span className="attribute-name-responsive">Armor Class</span>
            </div>
            
            <div className="attribute-breakdown">
              <div className="breakdown-row breakdown-base">
                <span className="breakdown-label">Natural:</span>
                <div className="breakdown-controls">
                  <Button
                    onClick={() => updateArmorClass(Math.max(0, stats.armorClass.base - 1))}
                    variant="outline"
                    size="xs"
                    className="breakdown-button"
                    disabled={stats.armorClass.base <= 0}
                  >
                    −
                  </Button>
                  <input
                    type="number"
                    value={stats.armorClass.base}
                    onChange={(e) => updateArmorClass(Math.max(0, Math.min(255, parseInt(e.target.value) || 0)))}
                    className="breakdown-input"
                    min="0"
                    max="255"
                  />
                  <Button
                    onClick={() => updateArmorClass(Math.min(255, stats.armorClass.base + 1))}
                    variant="outline"
                    size="xs"
                    className="breakdown-button"
                    disabled={stats.armorClass.base >= 255}
                  >
                    +
                  </Button>
                </div>
              </div>
              
              <div className="breakdown-row">
                <span className="breakdown-label">DEX Mod:</span>
                <div className="breakdown-value-container">
                  <span className="breakdown-value breakdown-calculated">TODO</span>
                </div>
              </div>
              
              <div className="breakdown-row">
                <span className="breakdown-label">Equipment:</span>
                <div className="breakdown-value-container">
                  <span className="breakdown-value breakdown-calculated">TODO</span>
                </div>
              </div>
              
              <hr className="breakdown-divider" />
              <div className="breakdown-row breakdown-effective-row">
                <span className="breakdown-label">Total AC:</span>
                <div className="breakdown-value-container">
                  <span className="breakdown-value breakdown-effective">{stats.armorClass.total}</span>
                </div>
              </div>
              
              <div className="calculation-hint">10 + Natural + DEX + Equipment</div>
            </div>
          </Card>

          {/* Initiative */}
          <Card variant="interactive" className="flex flex-col h-full">
            <div className="attribute-header-responsive">
              <span className="attribute-name-responsive">Initiative</span>
            </div>
            
            <div className="attribute-breakdown">
              <div className="breakdown-row breakdown-base">
                <span className="breakdown-label">Misc:</span>
                <div className="breakdown-controls">
                  <Button
                    onClick={() => updateInitiative(Math.max(-128, stats.initiative.base - 1))}
                    variant="outline"
                    size="xs"
                    className="breakdown-button"
                    disabled={stats.initiative.base <= -128}
                  >
                    −
                  </Button>
                  <input
                    type="number"
                    value={stats.initiative.base}
                    onChange={(e) => updateInitiative(Math.max(-128, Math.min(127, parseInt(e.target.value) || 0)))}
                    className="breakdown-input"
                    min="-128"
                    max="127"
                  />
                  <Button
                    onClick={() => updateInitiative(Math.min(127, stats.initiative.base + 1))}
                    variant="outline"
                    size="xs"
                    className="breakdown-button"
                    disabled={stats.initiative.base >= 127}
                  >
                    +
                  </Button>
                </div>
              </div>
              
              <div className="breakdown-row">
                <span className="breakdown-label">Feats:</span>
                <div className="breakdown-value-container">
                  <span className="breakdown-value breakdown-calculated">TODO</span>
                </div>
              </div>
              
              <hr className="breakdown-divider" />
              <div className="breakdown-row breakdown-effective-row">
                <span className="breakdown-label">Total:</span>
                <div className="breakdown-value-container">
                  <span className="breakdown-value breakdown-effective">{formatModifier(stats.initiative.total)}</span>
                </div>
              </div>
              
              <div className="calculation-hint">Misc + Feats</div>
            </div>
          </Card>

          {/* Fortitude */}
          <Card variant="interactive" className="flex flex-col h-full">
            <div className="attribute-header-responsive">
              <span className="attribute-name-responsive">Fortitude</span>
            </div>
            
            <div className="attribute-breakdown">
              <div className="breakdown-row breakdown-base">
                <span className="breakdown-label">Misc:</span>
                <div className="breakdown-controls">
                  <Button
                    onClick={() => updateFortitude(Math.max(-35, stats.fortitude.base - 1))}
                    variant="outline"
                    size="xs"
                    className="breakdown-button"
                    disabled={stats.fortitude.base <= -35}
                  >
                    −
                  </Button>
                  <input
                    type="number"
                    value={stats.fortitude.base}
                    onChange={(e) => updateFortitude(Math.max(-35, Math.min(255, parseInt(e.target.value) || 0)))}
                    className="breakdown-input"
                    min="-35"
                    max="255"
                  />
                  <Button
                    onClick={() => updateFortitude(Math.min(255, stats.fortitude.base + 1))}
                    variant="outline"
                    size="xs"
                    className="breakdown-button"
                    disabled={stats.fortitude.base >= 255}
                  >
                    +
                  </Button>
                </div>
              </div>
              
              <div className="breakdown-row">
                <span className="breakdown-label">Class:</span>
                <div className="breakdown-value-container">
                  <span className="breakdown-value breakdown-calculated">TODO</span>
                </div>
              </div>
              
              <div className="breakdown-row">
                <span className="breakdown-label">CON Mod:</span>
                <div className="breakdown-value-container">
                  <span className="breakdown-value breakdown-calculated">TODO</span>
                </div>
              </div>
              
              <hr className="breakdown-divider" />
              <div className="breakdown-row breakdown-effective-row">
                <span className="breakdown-label">Total:</span>
                <div className="breakdown-value-container">
                  <span className="breakdown-value breakdown-effective">{formatModifier(stats.fortitude.total)}</span>
                </div>
              </div>
              
              <div className="calculation-hint">Class + CON + Misc</div>
            </div>
          </Card>

          {/* Reflex */}
          <Card variant="interactive" className="flex flex-col h-full">
            <div className="attribute-header-responsive">
              <span className="attribute-name-responsive">Reflex</span>
            </div>
            
            <div className="attribute-breakdown">
              <div className="breakdown-row breakdown-base">
                <span className="breakdown-label">Misc:</span>
                <div className="breakdown-controls">
                  <Button
                    onClick={() => updateReflex(Math.max(-35, stats.reflex.base - 1))}
                    variant="outline"
                    size="xs"
                    className="breakdown-button"
                    disabled={stats.reflex.base <= -35}
                  >
                    −
                  </Button>
                  <input
                    type="number"
                    value={stats.reflex.base}
                    onChange={(e) => updateReflex(Math.max(-35, Math.min(255, parseInt(e.target.value) || 0)))}
                    className="breakdown-input"
                    min="-35"
                    max="255"
                  />
                  <Button
                    onClick={() => updateReflex(Math.min(255, stats.reflex.base + 1))}
                    variant="outline"
                    size="xs"
                    className="breakdown-button"
                    disabled={stats.reflex.base >= 255}
                  >
                    +
                  </Button>
                </div>
              </div>
              
              <div className="breakdown-row">
                <span className="breakdown-label">Class:</span>
                <div className="breakdown-value-container">
                  <span className="breakdown-value breakdown-calculated">TODO</span>
                </div>
              </div>
              
              <div className="breakdown-row">
                <span className="breakdown-label">DEX Mod:</span>
                <div className="breakdown-value-container">
                  <span className="breakdown-value breakdown-calculated">TODO</span>
                </div>
              </div>
              
              <hr className="breakdown-divider" />
              <div className="breakdown-row breakdown-effective-row">
                <span className="breakdown-label">Total:</span>
                <div className="breakdown-value-container">
                  <span className="breakdown-value breakdown-effective">{formatModifier(stats.reflex.total)}</span>
                </div>
              </div>
              
              <div className="calculation-hint">Class + DEX + Misc</div>
            </div>
          </Card>

          {/* Will */}
          <Card variant="interactive" className="flex flex-col h-full">
            <div className="attribute-header-responsive">
              <span className="attribute-name-responsive">Will</span>
            </div>
            
            <div className="attribute-breakdown">
              <div className="breakdown-row breakdown-base">
                <span className="breakdown-label">Misc:</span>
                <div className="breakdown-controls">
                  <Button
                    onClick={() => updateWill(Math.max(-35, stats.will.base - 1))}
                    variant="outline"
                    size="xs"
                    className="breakdown-button"
                    disabled={stats.will.base <= -35}
                  >
                    −
                  </Button>
                  <input
                    type="number"
                    value={stats.will.base}
                    onChange={(e) => updateWill(Math.max(-35, Math.min(255, parseInt(e.target.value) || 0)))}
                    className="breakdown-input"
                    min="-35"
                    max="255"
                  />
                  <Button
                    onClick={() => updateWill(Math.min(255, stats.will.base + 1))}
                    variant="outline"
                    size="xs"
                    className="breakdown-button"
                    disabled={stats.will.base >= 255}
                  >
                    +
                  </Button>
                </div>
              </div>
              
              <div className="breakdown-row">
                <span className="breakdown-label">Class:</span>
                <div className="breakdown-value-container">
                  <span className="breakdown-value breakdown-calculated">TODO</span>
                </div>
              </div>
              
              <div className="breakdown-row">
                <span className="breakdown-label">WIS Mod:</span>
                <div className="breakdown-value-container">
                  <span className="breakdown-value breakdown-calculated">TODO</span>
                </div>
              </div>
              
              <hr className="breakdown-divider" />
              <div className="breakdown-row breakdown-effective-row">
                <span className="breakdown-label">Total:</span>
                <div className="breakdown-value-container">
                  <span className="breakdown-value breakdown-effective">{formatModifier(stats.will.total)}</span>
                </div>
              </div>
              
              <div className="calculation-hint">Class + WIS + Misc</div>
            </div>
          </Card>

        </div>
      </CardContent>
    </Card>
  );
}