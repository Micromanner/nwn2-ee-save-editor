
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


  useEffect(() => {
    const performSearch = async () => {
      if (!searchQuery.trim() || !characterId) {
        setSearchResults([]);
        return;
      }

      try {
        const data = await apiClient.get<SearchResult>(
          `/characters/${characterId}/classes/categorized?search=${encodeURIComponent(searchQuery)}`
        );
        setSearchResults(data.search_results);
      } catch {
        setSearchResults([]);
      }
    };

    const timeoutId = setTimeout(performSearch, 300); // Debounce
    return () => clearTimeout(timeoutId);
  }, [searchQuery, characterId]);

  useEffect(() => {
    if (isOpen) {
      setSearchQuery('');
      setSearchResults([]);
    }
  }, [isOpen]);

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

  const checkClassPrerequisites = (classInfo: ClassInfo): MulticlassValidation => {
    const hasClass = currentClasses.some(c => c.name === classInfo.name);
    const atMaxClasses = currentClasses.length >= maxClasses;
    const atMaxLevel = totalLevel >= maxLevel;
    const isPrestige = classInfo.type === 'prestige';
    
    let reason = '';
    let canAdd = true;
    

    if (hasClass) {
      reason = 'Already have this class';
      canAdd = false;
    } else if (atMaxClasses && !isChangingClass) {
      reason = `Maximum ${maxClasses} classes allowed`;
      canAdd = false;
    } else if (atMaxLevel && !isChangingClass) {
      reason = `Character at maximum level (${maxLevel})`;
      canAdd = false;
    } else if (isPrestige) {
      if (totalLevel < 6) {
        reason = 'Prestige classes require character level 6+';
        canAdd = false;
      }
      }
    
    return {
      can_add: canAdd,
      reason: canAdd ? undefined : reason,
      requirements_met: {
        alignment: true,
        prerequisites: !isPrestige || totalLevel >= 6,
        level_limit: !atMaxLevel
      }
    };
  };


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
          
          {!validation.can_add && validation.reason && (
            <div className="text-xs text-red-400 mt-2 p-2 bg-red-500/10 rounded border border-red-500/20">
              <div className="flex items-center gap-1">
                <span className="w-1 h-1 bg-red-500 rounded-full"></span>
                <span className="font-medium">Cannot Add:</span>
              </div>
              <div className="mt-1">{validation.reason}</div>
            </div>
          )}
          

        </CardContent>
      </Card>
    );
  };

  if (!isOpen || !categorizedClasses) return null;

  return (
    <div className="class-modal-overlay">
      <Card className="class-modal-container">
        <CardContent padding="p-0" className="flex flex-col h-full">
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

          <div className="class-modal-content">
            {searchQuery.trim() ? (

              <div className="p-4">
                <h4 className="text-sm font-medium text-[rgb(var(--color-text-muted))] mb-3">
                  {searchResults.length} results for &quot;{searchQuery}&quot;
                </h4>
                <div className="space-y-2">
                  {searchResults.map(renderClassCard)}
                </div>
              </div>
            ) : (

              <Tabs value={selectedClassType} onValueChange={(value) => setSelectedClassType(value as 'base' | 'prestige' | 'npc')}>
                <TabsList className="w-full flex bg-transparent p-0 gap-2 px-4 mt-4 mb-0">
                  <TabsTrigger 
                    value="base" 
                    className="flex-1 h-10 px-4 rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10] truncate whitespace-nowrap"
                  >
                    Base Classes
                  </TabsTrigger>
                  <TabsTrigger 
                    value="prestige" 
                    className="flex-1 h-10 px-4 rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10] truncate whitespace-nowrap"
                  >
                    Prestige Classes
                  </TabsTrigger>
                  <TabsTrigger 
                    value="npc" 
                    className="flex-1 h-10 rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10] truncate whitespace-nowrap"
                  >
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