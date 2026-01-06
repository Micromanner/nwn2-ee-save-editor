'use client';

import { useState, useEffect } from 'react';
import { Pencil, Swords, X, History, AlertTriangle } from 'lucide-react';
import { useTranslations } from '@/hooks/useTranslations';

import { Card, CardContent } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { formatModifier, formatNumber } from '@/utils/dataHelpers';
import { useClassesLevel, type ClassesData } from '@/hooks/useClassesLevel';
import ClassSelectorModal from './ClassSelectorModal';
import LevelHistoryModal from './LevelHistoryModal';

interface ClassInfo {
  id: number;
  name: string;
  label: string;
  type: 'base' | 'prestige';
  focus: string;
  max_level: number;
  hit_die: number;
  skill_points: number;
  is_spellcaster: boolean;
  has_arcane: boolean;
  has_divine: boolean;
  primary_ability: string;
  bab_progression: string;
  alignment_restricted: boolean;
  description?: string;
  prerequisites?: Record<string, unknown>;
}

interface ClassAndLevelsEditorProps {
  onNavigate?: (path: string) => void;
  onLevelGains?: () => void;
}

export default function ClassAndLevelsEditor({ onNavigate: _onNavigate, onLevelGains }: ClassAndLevelsEditorProps) {
  const t = useTranslations();
  const { character, isLoading, error } = useCharacterContext();
  
  const classesSubsystem = useSubsystem('classes');
  const {
    classes,
    totalLevel,
    categorizedClasses,

    adjustClassLevel,
    changeClass,
    addClass,
    removeClass,
    canLevelUp,
    getRemainingLevels,
    isAtMaxLevel,
    isMetadataLoading,
    xpProgress,
    fetchXPProgress,
    setExperience,
  } = useClassesLevel(classesSubsystem.data as ClassesData | null);

  const [expandedClassDropdown, setExpandedClassDropdown] = useState<number | null>(null);
  const [showClassSelector, setShowClassSelector] = useState(false);
  const [showHistoryModal, setShowHistoryModal] = useState(false);
  const [xpInput, setXpInput] = useState<string>('');
  const [processingActions, setProcessingActions] = useState<Set<string>>(new Set());

  const maxLevel = 60;
  const maxClasses = 4;

  // Load subsystem data when character changes
  useEffect(() => {
    const loadCharacterClasses = async () => {
      if (!character?.id) return;

      // Only load if data is missing and not already loading
      if (!classesSubsystem.data && !classesSubsystem.isLoading) {
        try {
          await classesSubsystem.load();
        } catch {
          // Error handled by subsystem
        }
      }
    };

    loadCharacterClasses();
  }, [character?.id, classesSubsystem.data, classesSubsystem.isLoading, classesSubsystem]);

  // Fetch XP when character loads
  useEffect(() => {
    if (character?.id && !xpProgress) {
      fetchXPProgress();
    }
  }, [character?.id, xpProgress, fetchXPProgress]);


  useEffect(() => {
    if (xpProgress) {
      setXpInput(xpProgress.current_xp.toString());
    }
  }, [xpProgress]);


  const handleAdjustClassLevel = async (index: number, delta: number) => {
    if (!classes[index]) return;
    const actionId = `adjust-${classes[index].id}-${delta}`;
    
    setProcessingActions(prev => {
      const next = new Set(prev);
      next.add(actionId);
      return next;
    });

    try {
      const response = await adjustClassLevel(classes[index].id, delta) as { level_changes?: { gains?: unknown } };
      const gains = response?.level_changes?.gains;
      
      if (delta > 0 && gains && onLevelGains) {
        onLevelGains();
      }
    } catch {
      // Error handled by hook
    } finally {
      setProcessingActions(prev => {
        const next = new Set(prev);
        next.delete(actionId);
        return next;
      });
    }
  };

  const handleChangeClass = async (index: number, newClassInfo: ClassInfo) => {
    if (!classes[index]) return;
    const actionId = `change-${classes[index].id}`;
    
    setProcessingActions(prev => {
      const next = new Set(prev);
      next.add(actionId);
      return next;
    });

    try {
      await changeClass(classes[index].id, newClassInfo);
      setExpandedClassDropdown(null);
    } catch {
      // Error handled by hook
    } finally {
      setProcessingActions(prev => {
        const next = new Set(prev);
        next.delete(actionId);
        return next;
      });
    }
  };

  const handleClassSelection = async (classInfo: ClassInfo) => {
    const actionId = expandedClassDropdown !== null ? `change-${classes[expandedClassDropdown].id}` : 'add';
    
    setProcessingActions(prev => {
      const next = new Set(prev);
      next.add(actionId);
      return next;
    });

    try {
      if (expandedClassDropdown !== null) {
        await handleChangeClass(expandedClassDropdown, classInfo);
        setShowClassSelector(false);
        return;
      }

      const response = await addClass(classInfo) as { changes?: { gains?: unknown } };
      const gains = response?.changes?.gains;
      
      if (gains && onLevelGains) {
        onLevelGains();
      }
    } catch {
      // Error handled by hook
    } finally {
      setProcessingActions(prev => {
        const next = new Set(prev);
        next.delete(actionId);
        return next;
      });
    }
    
    setShowClassSelector(false);
  };

  const handleRemoveClass = async (index: number) => {
    if (!classes[index]) return;
    const actionId = `remove-${classes[index].id}`;
    
    setProcessingActions(prev => {
      const next = new Set(prev);
      next.add(actionId);
      return next;
    });

    try {
      await removeClass(classes[index].id);
    } catch {
      // Error handled by hook
    } finally {
      setProcessingActions(prev => {
        const next = new Set(prev);
        next.delete(actionId);
        return next;
      });
    }
  };

  const handleXPSubmit = async () => {
    let newXP = parseInt(xpInput, 10);
    if (isNaN(newXP) || newXP < 0) return;
    
    // Cap at Level 60 XP (1,770,000)
    if (newXP > 1770000) {
      newXP = 1770000;
      setXpInput("1770000");
    }
    const actionId = 'xp';
    
    setProcessingActions(prev => {
      const next = new Set(prev);
      next.add(actionId);
      return next;
    });

    try {
      if (newXP !== xpProgress?.current_xp) {
        await setExperience(newXP);
        // fetchXPProgress called by hook automatically
      }
    } catch {
      // Error setting XP
      // Revert on error
      if (xpProgress) {
        setXpInput(xpProgress.current_xp.toString());
      }
    } finally {
      setProcessingActions(prev => {
        const next = new Set(prev);
        next.delete(actionId);
        return next;
      });
    }
  };

  // Check if XP level differs from class level
  const hasLevelMismatch = xpProgress && xpProgress.current_xp_level !== totalLevel;
  const totalBAB = classes.reduce((sum, c) => sum + c.baseAttackBonus, 0);
  const totalFort = classes.reduce((sum, c) => sum + c.fortitudeSave, 0);
  const totalRef = classes.reduce((sum, c) => sum + c.reflexSave, 0);
  const totalWill = classes.reduce((sum, c) => sum + c.willSave, 0);

  if (isLoading || classesSubsystem.isLoading || isMetadataLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-center">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-[rgb(var(--color-primary))] mx-auto mb-3"></div>
          <p className="text-sm text-[rgb(var(--color-text-muted))]">
            {isLoading ? 'Loading character...' : (classesSubsystem.isLoading || isMetadataLoading) ? 'Loading classes...' : 'Updating classes...'}
          </p>
        </div>
      </div>
    );
  }

  if (error || classesSubsystem.error) {
    return (
      <Card variant="error" className="border border-red-500/20">
        <CardContent padding="p-4">
          <div className="flex items-center gap-2 mb-2">
            <span className="w-2 h-2 bg-red-500 rounded-full"></span>
            <h3 className="font-medium text-red-400">Error Loading Character</h3>
          </div>
          <p className="text-red-300 text-sm">{error || classesSubsystem.error}</p>
          <Button 
            onClick={() => window.location.reload()} 
            variant="outline" 
            size="sm" 
            className="mt-3"
          >
            Retry
          </Button>
        </CardContent>
      </Card>
    );
  }

  if (!character) {
    return (
      <Card variant="warning" className="border border-yellow-500/20">
        <CardContent padding="p-4">
          <div className="flex items-center gap-2 mb-2">
            <span className="w-2 h-2 bg-yellow-500 rounded-full"></span>
            <h3 className="font-medium text-yellow-400">No Character Loaded</h3>
          </div>
          <p className="text-yellow-300 text-sm">
            Please import a save file or create a character to begin editing classes.
          </p>
        </CardContent>
      </Card>
    );
  }

  // Handle empty classes state
  if (classes.length === 0 && !classesSubsystem.isLoading) {
    return (
      <div className="space-y-6">
        {/* Summary still shows totals even when empty */}
        <div className="grid grid-cols-5 gap-3">
          <Card>
            <CardContent padding="p-3" className="text-center">
              <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('classes.totalLevel')}</div>
              <div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">0/40</div>
            </CardContent>
          </Card>
          <Card>
            <CardContent padding="p-3" className="text-center">
              <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('classes.bab')}</div>
              <div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">+0</div>
            </CardContent>
          </Card>
          {/* ... rest of summary cards ... */}
          <Card><CardContent padding="p-3" className="text-center"><div className="text-xs text-[rgb(var(--color-text-muted))]">{t('classes.fortitude')}</div><div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">+0</div></CardContent></Card>
          <Card><CardContent padding="p-3" className="text-center"><div className="text-xs text-[rgb(var(--color-text-muted))]">{t('classes.reflex')}</div><div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">+0</div></CardContent></Card>
          <Card><CardContent padding="p-3" className="text-center"><div className="text-xs text-[rgb(var(--color-text-muted))]">{t('classes.will')}</div><div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">+0</div></CardContent></Card>
        </div>

        <Card>
          <CardContent padding="p-4">
            <div className="text-center py-8">
              <div className="w-16 h-16 bg-[rgb(var(--color-surface-2))] rounded-full flex items-center justify-center mx-auto mb-4">
                <Swords className="w-8 h-8 text-[rgb(var(--color-text-muted))]" />
              </div>
              <h3 className="text-lg font-semibold text-[rgb(var(--color-text-primary))] mb-2">
                No Classes Assigned
              </h3>
              <p className="text-sm text-[rgb(var(--color-text-muted))] mb-4">
                This character doesn&apos;t have any classes yet. Add a class to get started.
              </p>
              <Button
                onClick={() => setShowClassSelector(true)}
                className="bg-blue-600 hover:bg-blue-700"
              >
                Choose First Class
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* XP Level Mismatch Warning */}


      {/* XP and Level Summary */}
      {/* XP and Level Summary */}
      {/* XP and Level Summary */}
      <div className="grid grid-cols-[220px_1fr] gap-4">
        {/* XP Card */}
        <Card className="overflow-hidden border-[rgb(var(--color-surface-border))] bg-[rgb(var(--color-surface-1))] max-w-[220px]">
          <CardContent padding="p-4" className="flex flex-col gap-3">
             <h4 className="text-xs font-semibold uppercase text-[rgb(var(--color-text-muted))] tracking-wider border-b border-[rgb(var(--color-surface-border)/0.4)] pb-2 text-center">
                {t('classes.experience')}
            </h4>

            <div className="space-y-3 flex flex-col items-center">
              <div className="flex items-center justify-center gap-2 w-full">
                <Input
                  type="text"
                  value={xpInput}
                  onChange={(e) => {
                    const value = e.target.value;
                    if (value === '' || /^\d+$/.test(value)) {
                      setXpInput(value);
                    }
                  }}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleXPSubmit();
                    if (e.key === 'Escape') {
                      setXpInput(String(xpProgress?.current_xp || 0));
                    }
                  }}
                  className="w-32 text-lg font-bold bg-[rgb(var(--color-surface-2))] border-[rgb(var(--color-surface-border))] h-9 px-2 text-center"
                />
                <div className="flex items-center gap-1">
                   <Button
                      size="sm"
                      variant="ghost"
                      onClick={handleXPSubmit}
                      disabled={xpInput === String(xpProgress?.current_xp || 0)}
                      className={`h-9 w-9 p-0 ${xpInput === String(xpProgress?.current_xp || 0) ? 'opacity-40 grayscale pointer-events-none' : 'opacity-100 hover:bg-[rgb(var(--color-surface-3))]'}`}
                      title={t('actions.save')}
                    >
                      <span className={xpInput !== String(xpProgress?.current_xp || 0) ? "text-[rgb(var(--color-success))]" : "text-[rgb(var(--color-text-muted))]"}>✓</span>
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => {
                        setXpInput(String(xpProgress?.current_xp || 0));
                      }}
                      disabled={xpInput === String(xpProgress?.current_xp || 0)}
                      className={`h-9 w-9 p-0 ${xpInput === String(xpProgress?.current_xp || 0) ? 'opacity-30 grayscale pointer-events-none' : 'opacity-100 hover:bg-[rgb(var(--color-surface-3))]'}`}
                       title={t('actions.cancel')}
                    >
                      <span className="text-[rgb(var(--color-text-muted))]">✕</span>
                    </Button>
                </div>
              </div>

              {/* Progress Bar */}
              {xpProgress && (
                <div className="space-y-2 flex flex-col items-center w-full">
                  <div className="h-1.5 w-[80%] bg-[rgb(var(--color-surface-2))] rounded-full overflow-hidden border border-[rgb(var(--color-surface-border)/0.3)] mx-auto">
                    <div
                      className="h-full bg-[rgb(var(--color-primary))] transition-all duration-500"
                      style={{ 
                        width: `${Math.max(0, Math.min(100, ( (xpProgress.current_xp - xpProgress.current_level_min_xp) / (xpProgress.next_level_min_xp - xpProgress.current_level_min_xp) ) * 100))}%` 
                      }}
                    />
                  </div>
                  <div className="flex flex-col items-center gap-0.5 text-[10px] uppercase font-medium tracking-tight text-[rgb(var(--color-text-muted))] w-full">
                     <div className="flex items-center justify-center gap-1.5 w-full">
                        <span>{t('classes.xpLevel')} {xpProgress.current_xp_level}</span>
                        {hasLevelMismatch && (
                          <div className="relative group cursor-help">
                            <AlertTriangle className="w-3.5 h-3.5 text-yellow-500" />
                            <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 px-3 py-2 bg-gray-900 text-yellow-200 text-xs rounded shadow-lg opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none w-64 z-[100] border border-yellow-500/30 text-center">
                              {t('classes.xpLevelMismatchWarning')
                                .replace('{xpLevel}', String(xpProgress?.current_xp_level || 0))
                                .replace('{classLevel}', String(totalLevel))}
                              <div className="absolute left-1/2 -translate-x-1/2 top-full w-2 h-2 bg-gray-900 border-r border-b border-yellow-500/30 rotate-45 -mt-1"></div>
                            </div>
                          </div>
                        )}
                     </div>
                     {xpProgress.xp_to_next !== null && xpProgress.xp_to_next > 0 && (
                        <span className="opacity-70">{formatNumber(xpProgress.xp_to_next)} to next</span>
                     )}
                  </div>
                </div>
              )}
            </div>
          </CardContent>
        </Card>

        {/* Stats Summary Card */}
        <Card>
          <CardContent padding="p-3">
            <div className="grid grid-cols-5 gap-2 text-center">
              <div>
                <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('classes.totalLevel')}</div>
                <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">{totalLevel}/60</div>
              </div>
              <div>
                <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('classes.bab')}</div>
                <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">+{totalBAB}</div>
              </div>
              <div>
                <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('classes.fortitude')}</div>
                <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">+{totalFort}</div>
              </div>
              <div>
                <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('classes.reflex')}</div>
                <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">+{totalRef}</div>
              </div>
              <div>
                <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('classes.will')}</div>
                <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">+{totalWill}</div>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Classes List */}
      <Card>
        <CardContent padding="p-4">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">
              {t('classes.currentClasses')}
            </h3>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowHistoryModal(true)}
              className="flex items-center gap-2 h-8"
            >
              <History className="w-4 h-4" />
              <span>History</span>
            </Button>
          </div>
          
          <div className="space-y-2">

            {classes.map((cls, index) => (
              <Card 
                key={`${cls.id}-${index}`} 
                className="bg-[rgb(var(--color-surface-1))]"
              >
                <CardContent padding="p-3">
                  {/* Main class row - using grid for consistent alignment */}
                  <div className="grid grid-cols-10 gap-3 items-center">
                    <div className="col-span-3">
                        <Button
                          onClick={() => {
                            setExpandedClassDropdown(index);
                            setShowClassSelector(true);
                          }}
                          variant="outline"
                          size="sm"
                          disabled={processingActions.has(`change-${cls.id}`)}
                          className="w-full justify-between h-9 px-3 text-sm font-medium group"
                          title={t('classes.changeClass')}
                        >
                        <span className="truncate">
                          {cls.name}
                        </span>
                        <Pencil className="w-4 h-4 opacity-70 group-hover:opacity-100 transition-opacity flex-shrink-0" />
                      </Button>
                    </div>

                    {/* Level Controls - Fixed width */}
                    <div className="col-span-2 flex flex-col items-center justify-center gap-1">
                      <div className="flex items-center gap-1">
                        <Button
                          onClick={() => handleAdjustClassLevel(index, -1)}
                          variant="outline"
                          size="sm"
                          disabled={processingActions.has(`adjust-${cls.id}--1`) || cls.level <= 1}
                          className="w-7 h-7 p-0"
                        >
                          <span className="text-sm">−</span>
                        </Button>
                        <div className="w-8 text-center">
                          <div className="text-lg font-semibold">{cls.level}</div>
                        </div>
                        <Button
                          onClick={() => handleAdjustClassLevel(index, 1)}
                          variant="outline"
                          size="sm"
                          disabled={processingActions.has(`adjust-${cls.id}-1`) || totalLevel >= maxLevel || !canLevelUp(cls.id)}
                          className="w-7 h-7 p-0"
                        >
                          <span className="text-sm">+</span>
                        </Button>
                      </div>
                      {/* Show remaining levels for prestige classes */}
                      {getRemainingLevels(cls.id) !== null && (
                        <div className="text-xs text-[rgb(var(--color-text-muted))]">
                          {isAtMaxLevel(cls.id) ? (
                            <span className="text-orange-400">Max Level</span>
                          ) : (
                            <span>{getRemainingLevels(cls.id)} levels left</span>
                          )}
                        </div>
                      )}
                    </div>

                    {/* Class Stats - Aligned columns */}
                    <div className="col-span-4 grid grid-cols-6 gap-2 text-sm text-[rgb(var(--color-text-muted))]">
                      <div className="text-center">
                        <div className="text-xs opacity-75">BAB</div>
                        <div className="font-medium">{formatModifier(cls.baseAttackBonus)}</div>
                      </div>
                      <div className="text-center">
                        <div className="text-xs opacity-75">Fort</div>
                        <div className="font-medium">{formatModifier(cls.fortitudeSave)}</div>
                      </div>
                      <div className="text-center">
                        <div className="text-xs opacity-75">Ref</div>
                        <div className="font-medium">{formatModifier(cls.reflexSave)}</div>
                      </div>
                      <div className="text-center">
                        <div className="text-xs opacity-75">Will</div>
                        <div className="font-medium">{formatModifier(cls.willSave)}</div>
                      </div>
                      <div className="text-center">
                        <div className="text-xs opacity-75">HD</div>
                        <div className="font-medium">d{cls.hitDie}</div>
                      </div>
                      <div className="text-center">
                        <div className="text-xs opacity-75">SP</div>
                        <div className="font-medium">{formatNumber(cls.skillPoints)}</div>
                      </div>
                    </div>

                    {/* Action Buttons - Fixed width */}
                    <div className="col-span-1 flex items-center justify-end gap-1">
                      {/* Level Up Button - TODO: Re-enable for post-0.1.0 release */}
                      {/* {canLevelUp(cls.id) && totalLevel < maxLevel && (
                        <Button
                          onClick={() => handleOpenLevelUp(cls.id, cls.name)}
                          variant="outline"
                          size="sm"
                          className="h-7 px-2 text-xs bg-[rgb(var(--color-primary))/10] border-[rgb(var(--color-primary))/30] hover:bg-[rgb(var(--color-primary))/20] text-[rgb(var(--color-primary))]"
                          title={t('levelup.title')}
                        >
                          <ArrowUp className="w-3 h-3 mr-1" />
                          {t('levelup.title')}
                        </Button>
                      )} */}
                      {classes.length > 1 && (
                        <Button
                          onClick={() => handleRemoveClass(index)}
                          variant="ghost"
                          size="sm"
                          disabled={processingActions.has(`remove-${cls.id}`)}
                          className="p-1 hover:text-[rgb(var(--color-danger))]"
                          title="Remove class"
                        >
                          <X className="w-4 h-4" />
                        </Button>
                      )}
                    </div>
                  </div>
                </CardContent>
              </Card>
            ))}

            {/* Add Class Button */}
            {classes.length < maxClasses && totalLevel < maxLevel && (
              <Button
                onClick={() => setShowClassSelector(true)}
                variant="outline"
                disabled={processingActions.has('add')}
                className="w-full p-3 border-2 border-dashed hover:bg-[rgb(var(--color-surface-2))]"
                style={{ 
                  borderColor: 'rgba(255, 255, 255, 0.06)',
                }}
              >
                + {t('classes.addClass')}
              </Button>
            )}
          </div>
        </CardContent>
      </Card>



      {/* Class Selector Modal */}
      <ClassSelectorModal
        isOpen={showClassSelector}
        onClose={() => {
          setShowClassSelector(false);
          setExpandedClassDropdown(null);
        }}
        onSelectClass={handleClassSelection}
        characterId={character?.id?.toString()}
        categorizedClasses={categorizedClasses}
        currentClasses={classes.map(c => ({ id: c.id, name: c.name, level: c.level }))}
        isChangingClass={expandedClassDropdown !== null}
        totalLevel={totalLevel}
        maxLevel={maxLevel}
        maxClasses={maxClasses}
      />
      
      <LevelHistoryModal
        isOpen={showHistoryModal}
        onClose={() => setShowHistoryModal(false)}
      />
    </div>
  );
}