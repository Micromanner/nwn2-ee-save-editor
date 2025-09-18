'use client';

import { useState, useEffect } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { formatModifier, formatNumber } from '@/utils/dataHelpers';
import { useClassesLevel, type ClassesData } from '@/hooks/useClassesLevel';
import ClassSelectorModal from './ClassSelectorModal';

// SVG Icon Components
const ChevronDown = ({ className }: { className?: string }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
  </svg>
);

const X = ({ className }: { className?: string }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
  </svg>
);

const Sword = ({ className }: { className?: string }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" />
  </svg>
);


// Removed unused ClassLevel interface - using ClassLevel from useClassesLevel hook instead

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

// Removed unused FocusInfo interface

// Removed unused interfaces - using types from hooks instead


export default function ClassAndLevelsEditor() {
  const t = useTranslations();
  const { character, isLoading, error } = useCharacterContext();
  
  // Use the classes subsystem hook
  const classesSubsystem = useSubsystem('classes');
  
  // Use the classes level hook with subsystem data
  const {
    classes,
    totalLevel,
    categorizedClasses,
    isUpdating,
    adjustClassLevel,
    changeClass,
    addClass,
    removeClass,
  } = useClassesLevel(classesSubsystem.data as ClassesData | null);

  const [expandedClassDropdown, setExpandedClassDropdown] = useState<number | null>(null);
  const [showClassSelector, setShowClassSelector] = useState(false);

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
        } catch (err) {
          console.error('Failed to load character classes:', err);
        }
      }
    };

    loadCharacterClasses();
  }, [character?.id, classesSubsystem.data, classesSubsystem.isLoading, classesSubsystem]);


  const handleAdjustClassLevel = async (index: number, delta: number) => {
    if (!classes[index]) return;
    
    try {
      await adjustClassLevel(classes[index].id, delta);
      // Reload subsystem data to get updated values
      await classesSubsystem.load();
    } catch (err) {
      console.error('Failed to adjust class level:', err);
    }
  };

  const handleChangeClass = async (index: number, newClassInfo: ClassInfo) => {
    if (!classes[index]) return;
    
    try {
      await changeClass(classes[index].id, newClassInfo);
      setExpandedClassDropdown(null);
      // Reload subsystem data to get updated values
      await classesSubsystem.load();
    } catch (err) {
      console.error('Failed to change class:', err);
    }
  };

  const handleClassSelection = async (classInfo: ClassInfo) => {
    try {
      // Check if we're changing an existing class (expandedClassDropdown is set)
      if (expandedClassDropdown !== null) {
        await handleChangeClass(expandedClassDropdown, classInfo);
        setShowClassSelector(false);
        return;
      }
      
      // Adding a new class
      await addClass(classInfo);
      setShowClassSelector(false);
      // Reload subsystem data to get updated values
      await classesSubsystem.load();
    } catch (err) {
      console.error('Failed to handle class selection:', err);
    }
  };

  const handleRemoveClass = async (index: number) => {
    if (!classes[index]) return;
    
    try {
      await removeClass(classes[index].id);
      // Reload subsystem data to get updated values
      await classesSubsystem.load();
    } catch (err) {
      console.error('Failed to remove class:', err);
    }
  };





  const totalBAB = classes.reduce((sum, c) => sum + c.baseAttackBonus, 0);
  const totalFort = classes.reduce((sum, c) => sum + c.fortitudeSave, 0);
  const totalRef = classes.reduce((sum, c) => sum + c.reflexSave, 0);
  const totalWill = classes.reduce((sum, c) => sum + c.willSave, 0);

  if (isLoading || classesSubsystem.isLoading || isUpdating) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-center">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-[rgb(var(--color-primary))] mx-auto mb-3"></div>
          <p className="text-sm text-[rgb(var(--color-text-muted))]">
            {isLoading ? 'Loading character...' : classesSubsystem.isLoading ? 'Loading classes...' : 'Updating classes...'}
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
                <Sword className="w-8 h-8 text-[rgb(var(--color-text-muted))]" />
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
      {/* Compact Summary at Top */}
      <div className="grid grid-cols-5 gap-3">
        <Card>
          <CardContent padding="p-3" className="text-center">
            <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('classes.totalLevel')}</div>
            <div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">{totalLevel}/60</div>
            <p className="text-xs text-[rgb(var(--color-text-muted))]">Levels above 30 may only be supported in custom modules.</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent padding="p-3" className="text-center">
            <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('classes.bab')}</div>
            <div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">+{totalBAB}</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent padding="p-3" className="text-center">
            <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('classes.fortitude')}</div>
            <div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">+{totalFort}</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent padding="p-3" className="text-center">
            <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('classes.reflex')}</div>
            <div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">+{totalRef}</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent padding="p-3" className="text-center">
            <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('classes.will')}</div>
            <div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">+{totalWill}</div>
          </CardContent>
        </Card>
      </div>

      {/* Classes List */}
      <Card>
        <CardContent padding="p-4">
          <h3 className="text-lg font-semibold text-[rgb(var(--color-text-primary))] mb-3">
            {t('classes.currentClasses')}
          </h3>
          
          <div className="space-y-2">
            {classes.map((cls, index) => (
              <Card 
                key={`${cls.id}-${index}`} 
                className="bg-[rgb(var(--color-surface-1))]"
              >
                <CardContent padding="p-3">
                  {/* Main class row - using grid for consistent alignment */}
                  <div className="grid grid-cols-10 gap-3 items-center">
                    {/* Class Selector - Fixed width */}
                    <div className="col-span-3">
                      <Button
                        onClick={() => {
                          setExpandedClassDropdown(index);
                          setShowClassSelector(true);
                        }}
                        variant="ghost"
                        className="w-full justify-between px-3 py-1 h-auto text-left"
                      >
                        <span className="font-medium truncate">
                          {cls.name}
                        </span>
                        <ChevronDown className="w-4 h-4 flex-shrink-0" />
                      </Button>
                    </div>

                    {/* Level Controls - Fixed width */}
                    <div className="col-span-2 flex items-center justify-center gap-1">
                      <Button
                        onClick={() => handleAdjustClassLevel(index, -1)}
                        variant="outline"
                        size="sm"
                        disabled={cls.level <= 1}
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
                        disabled={totalLevel >= maxLevel}
                        className="w-7 h-7 p-0"
                      >
                        <span className="text-sm">+</span>
                      </Button>
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
                    <div className="col-span-1 flex items-center justify-end">
                      {classes.length > 1 && (
                        <Button
                          onClick={() => handleRemoveClass(index)}
                          variant="ghost"
                          size="sm"
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
    </div>
  );
}