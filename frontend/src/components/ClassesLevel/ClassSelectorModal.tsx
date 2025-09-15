'use client';

import { useState, useEffect } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/Tabs';
import { apiClient } from '@/lib/api/client';
import { display, formatNumber } from '@/utils/dataHelpers';

// SVG Icon Components
const X = ({ className }: { className?: string }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
  </svg>
);

const Search = ({ className }: { className?: string }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
  </svg>
);

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

interface FocusInfo {
  name: string;
  description: string;
  icon: string;
}

interface CategorizedClasses {
  categories: {
    base: Record<string, ClassInfo[]>;
    prestige: Record<string, ClassInfo[]>;
    npc: Record<string, ClassInfo[]>;
  };
  focus_info: Record<string, FocusInfo>;
  total_classes: number;
  character_context?: {
    current_classes: unknown;
    prestige_requirements?: unknown[];
    can_multiclass: boolean;
    multiclass_slots_used: number;
  };
}

interface SearchResult {
  search_results: ClassInfo[];
  query: string;
  total_results: number;
}

interface MulticlassValidation {
  can_add: boolean;
  reason?: string;
  requirements_met: {
    alignment: boolean;
    prerequisites: boolean;
    level_limit: boolean;
  };
}

interface ClassSelectorModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSelectClass: (classInfo: ClassInfo) => Promise<void>;
  characterId: string | undefined;
  categorizedClasses: CategorizedClasses | null;
  currentClasses: Array<{ id: number; name: string; level: number }>;
  isChangingClass: boolean; // true if changing existing class, false if adding new
  totalLevel: number;
  maxLevel: number;
  maxClasses: number;
}

export default function ClassSelectorModal({
  isOpen,
  onClose,
  onSelectClass,
  characterId,
  categorizedClasses,
  currentClasses,
  isChangingClass,
  totalLevel,
  maxLevel,
  maxClasses
}: ClassSelectorModalProps) {
  const t = useTranslations();
  
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<ClassInfo[]>([]);
  const [selectedClassType, setSelectedClassType] = useState<'base' | 'prestige' | 'npc'>('base');

  // Handle search
  useEffect(() => {
    const performSearch = async () => {
      if (!searchQuery.trim() || !characterId) {
        setSearchResults([]);
        return;
      }

      try {
        const data = await apiClient.get<SearchResult>(
          `/characters/${characterId}/classes/categorized/?search=${encodeURIComponent(searchQuery)}`
        );
        setSearchResults(data.search_results);
      } catch (err) {
        console.error('Search failed:', err);
        setSearchResults([]);
      }
    };

    const timeoutId = setTimeout(performSearch, 300); // Debounce
    return () => clearTimeout(timeoutId);
  }, [searchQuery, characterId]);

  // Reset search when modal opens/closes
  useEffect(() => {
    if (isOpen) {
      setSearchQuery('');
      setSearchResults([]);
    }
  }, [isOpen]);

  // TODO: Replace hardcoded focus labels with real data from backend
  // Backend should provide human-readable focus names and descriptions from 2DA files
  const getFocusLabel = (focus: string) => {
    switch (focus) {
      case 'combat': return '';
      case 'arcane_caster': return '';
      case 'divine_caster': return '';
      case 'skill_specialist': return '';  
      case 'stealth_infiltration': return 'Stealth';
      default: return '';
    }
  };

  // TODO: Replace frontend calculations with real backend prerequisite validation
  // Frontend should NOT calculate prerequisites - backend should provide validation results
  // Backend should check:
  // - Character alignment vs class alignment restrictions  
  // - Actual prerequisite requirements from PreReqTable (skills, feats, BAB, saves, etc.)
  // - Multi-class penalties and restrictions
  // - Level requirements for prestige classes
  // - Any custom mod requirements from HAK files
  const checkClassPrerequisites = (classInfo: ClassInfo): MulticlassValidation => {
    const hasClass = currentClasses.some(c => c.name === classInfo.name);
    const atMaxClasses = currentClasses.length >= maxClasses;
    const atMaxLevel = totalLevel >= maxLevel;
    const isPrestige = classInfo.type === 'prestige';
    
    let reason = '';
    let canAdd = true;
    
    // Check various conditions
    if (hasClass && !isChangingClass) {
      reason = 'Already have this class';
      canAdd = false;
    } else if (atMaxClasses && !isChangingClass) {
      reason = `Maximum ${maxClasses} classes allowed`;
      canAdd = false;
    } else if (atMaxLevel && !isChangingClass) {
      reason = `Character at maximum level (${maxLevel})`;
      canAdd = false;
    } else if (isPrestige) {
      // Additional prestige class checks
      if (totalLevel < 6) {
        reason = 'Prestige classes require character level 6+';
        canAdd = false;
      }
      // TODO: Replace with real prerequisite checking from backend API
      // Backend should validate: skills, feats, BAB, saves, alignment, etc.
    }
    
    return {
      can_add: canAdd,
      reason: canAdd ? undefined : reason,
      requirements_met: {
        alignment: true, // TODO: Get real alignment validation from backend
        prerequisites: !isPrestige || totalLevel >= 6, // TODO: Get real prerequisite validation from backend
        level_limit: !atMaxLevel
      }
    };
  };

  // Render class card with prerequisite validation
  const renderClassCard = (classInfo: ClassInfo) => {
    const validation = checkClassPrerequisites(classInfo);
    
    return (
      <Card 
        key={`${classInfo.label}-${classInfo.type}`}
        className={`class-modal-class-card ${
          validation.can_add ? 'available' : 'unavailable'
        }`}
        onClick={() => validation.can_add && onSelectClass(classInfo)}
      >
        <CardContent className="class-modal-class-card-content">
          <div className="flex items-center justify-between">
            <div className="flex-1">
              <div className="flex items-center gap-2">
                <span className="font-medium text-[rgb(var(--color-text-primary))]">
                  {display(classInfo.name)}
                </span>
                {!validation.can_add && (
                  <span className="text-xs px-2 py-1 bg-red-500/20 text-red-400 rounded">
                    Unavailable
                  </span>
                )}
              </div>
              {/* TODO: Replace with comprehensive class info from backend - no frontend calculations
                   Backend should provide:
                   - Real class descriptions from 2DA Description field (StrRef lookups)
                   - Actual weapon/armor proficiencies from class data
                   - Real spell progression tables for casters
                   - Detailed prerequisite information with human-readable descriptions
                   - Class-specific features and abilities per level
              */}
              <div className="text-xs text-[rgb(var(--color-text-muted))] mt-1">
                {classInfo.primary_ability} • d{classInfo.hit_die} • {formatNumber(classInfo.skill_points)} skills
                {classInfo.is_spellcaster && ` • ${classInfo.has_arcane ? 'Arcane' : 'Divine'} Caster`}
                {classInfo.alignment_restricted && ' • Alignment Restricted'}
              </div>
            </div>
            <div className="text-xs text-[rgb(var(--color-text-muted))]">
              {getFocusLabel(classInfo.focus)}
            </div>
          </div>
          
          {/* TODO: Replace with real prerequisite validation from backend
               Backend should provide detailed prerequisite information:
               - Specific skill requirements (e.g., "Stealth 8+ required")
               - Required feats (e.g., "Weapon Focus required")
               - BAB requirements (e.g., "Base Attack Bonus +5 required")
               - Save requirements (e.g., "Will Save +3 required")
               - Alignment restrictions with specific allowed alignments
               - Race restrictions if any
               - Class level requirements (e.g., "3 levels of Rogue required")
          */}
          {!validation.can_add && validation.reason && (
            <div className="text-xs text-red-400 mt-2 p-2 bg-red-500/10 rounded border border-red-500/20">
              <div className="flex items-center gap-1">
                <span className="w-1 h-1 bg-red-500 rounded-full"></span>
                <span className="font-medium">Cannot Add:</span>
              </div>
              <div className="mt-1">{validation.reason}</div>
            </div>
          )}
          
          {/* TODO: Replace with real prerequisite status from backend
               Show actual prerequisite checks with specific details:
               - ✓ Hide in Plain Sight: 15 (character has 20)
               - ✗ Uncanny Dodge: Required (character missing)
               - ✓ Sneak Attack: +3d6 (character has +4d6)
          */}
          
        </CardContent>
      </Card>
    );
  };

  if (!isOpen || !categorizedClasses) return null;

  return (
    <div className="class-modal-overlay">
      <Card className="class-modal-container">
        <CardContent padding="p-0" className="flex flex-col h-full">
          {/* Header */}
          <div className="class-modal-header">
            <div className="class-modal-header-row">
              <h3 className="class-modal-title">
                {isChangingClass ? 'Change Class' : t('classes.selectClass')}
              </h3>
              <Button
                onClick={onClose}
                variant="ghost"
                size="sm"
                className="class-modal-close-button"
              >
                <X className="w-4 h-4" />
              </Button>
            </div>
            
            {/* Search Bar */}
            <div className="class-modal-search-container">
              <Search className="class-modal-search-icon" />
              <Input
                placeholder={t('classes.searchClasses')}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="class-modal-search-input"
              />
            </div>
          </div>

          {/* Content */}
          <div className="class-modal-content">
            {searchQuery.trim() ? (
              /* Search Results */
              <div className="p-4">
                <h4 className="text-sm font-medium text-[rgb(var(--color-text-muted))] mb-3">
                  {searchResults.length} results for &quot;{searchQuery}&quot;
                </h4>
                <div className="space-y-2">
                  {searchResults.map(renderClassCard)}
                </div>
              </div>
            ) : (
              /* Categorized View */
              <Tabs value={selectedClassType} onValueChange={(value) => setSelectedClassType(value as 'base' | 'prestige' | 'npc')}>
                <TabsList className="grid w-full grid-cols-3 m-4 mb-0">
                  <TabsTrigger value="base" className="flex items-center gap-2">
                    Base Classes
                  </TabsTrigger>
                  <TabsTrigger value="prestige" className="flex items-center gap-2">
                    Prestige Classes
                  </TabsTrigger>
                  <TabsTrigger value="npc" className="flex items-center gap-2">
                    NPC Classes
                  </TabsTrigger>
                </TabsList>

                <TabsContent value="base" className="p-4 pt-3">
                  {Object.entries(categorizedClasses.categories.base).map(([focus, classList]) => {
                    if (!classList.length) return null;
                    
                    const focusInfo = categorizedClasses.focus_info[focus];
                    return (
                      <div key={focus} className="mb-6">
                        <h4 className="text-sm font-medium text-[rgb(var(--color-text-primary))] mb-2 flex items-center gap-2">
                          <span className="text-xs">{getFocusLabel(focus)}</span>
                          {focusInfo?.name || focus} ({classList.length})
                        </h4>
                        {focusInfo?.description && (
                          <p className="text-xs text-[rgb(var(--color-text-muted))] mb-3">
                            {focusInfo.description}
                          </p>
                        )}
                        <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-2">
                          {classList.map(renderClassCard)}
                        </div>
                      </div>
                    );
                  })}
                </TabsContent>

                <TabsContent value="prestige" className="p-4 pt-3">
                  {Object.entries(categorizedClasses.categories.prestige).map(([focus, classList]) => {
                    if (!classList.length) return null;
                    
                    const focusInfo = categorizedClasses.focus_info[focus];
                    return (
                      <div key={focus} className="mb-6">
                        <h4 className="text-sm font-medium text-[rgb(var(--color-text-primary))] mb-2 flex items-center gap-2">
                          <span className="text-xs">{getFocusLabel(focus)}</span>
                          {focusInfo?.name || focus} ({classList.length})
                        </h4>
                        {focusInfo?.description && (
                          <p className="text-xs text-[rgb(var(--color-text-muted))] mb-3">
                            {focusInfo.description}
                          </p>
                        )}
                        <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-2">
                          {classList.map(renderClassCard)}
                        </div>
                      </div>
                    );
                  })}
                </TabsContent>

                <TabsContent value="npc" className="p-4 pt-3">
                  {Object.entries(categorizedClasses.categories.npc).map(([focus, classList]) => {
                    if (!classList.length) return null;
                    
                    const focusInfo = categorizedClasses.focus_info[focus];
                    return (
                      <div key={focus} className="mb-6">
                        <h4 className="text-sm font-medium text-[rgb(var(--color-text-primary))] mb-2 flex items-center gap-2">
                          <span className="text-xs">{getFocusLabel(focus)}</span>
                          {focusInfo?.name || focus} ({classList.length})
                        </h4>
                        {focusInfo?.description && (
                          <p className="text-xs text-[rgb(var(--color-text-muted))] mb-3">
                            {focusInfo.description}
                          </p>
                        )}
                        <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-2">
                          {classList.map(renderClassCard)}
                        </div>
                      </div>
                    );
                  })}
                </TabsContent>
              </Tabs>
            )}
          </div>

          {/* Footer */}
          <div className="class-modal-footer">
            <div className="class-modal-footer-content">
              <span>
                {categorizedClasses.total_classes} total classes available
              </span>
              <span>
                {currentClasses.length}/{maxClasses} classes selected
              </span>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}