import React, { useState } from 'react';
import { Card } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { ScrollArea } from '@/components/ui/ScrollArea';
import { Skeleton } from '@/components/ui/skeleton';
import { ChevronRight, ChevronDown } from 'lucide-react';
import NWN2Icon from '@/components/ui/NWN2Icon';
import { Button } from '@/components/ui/Button';
import { getSchoolIcon } from './SpellSections';
import { display, joinArray } from '@/utils/dataHelpers';

interface Spell {
  id: number;
  name: string;
  icon: string;
  icon_url?: string;
  school_id: number;
  school_name: string | null;
  level: number;
  available_classes: string[];
  
  // Spell details from backend
  description: string;
  range: string;
  cast_time: string;
  conjuration_time: string;
  components: string;
  metamagic: string;
  target_type: string;
  
  // Client-side properties for UI state
  isLearned?: boolean;
  
  // Computed properties for compatibility
  school: string | null;
  innate_level: number;
}

interface SpellListProps {
  spells: Spell[];
  viewMode: 'grid' | 'list';
  isLoading: boolean;
  onToggleLearned: (spellId: number) => void;
  defaultExpandedLevels?: number[];
}

export default function SpellList({
  spells,
  viewMode,
  isLoading,
  onToggleLearned,
  defaultExpandedLevels = [0, 1]
}: SpellListProps) {
  const [expandedLevels, setExpandedLevels] = useState<Set<number>>(
    new Set(defaultExpandedLevels)
  );

  const toggleLevel = (level: number) => {
    const newExpanded = new Set(expandedLevels);
    if (newExpanded.has(level)) {
      newExpanded.delete(level);
    } else {
      newExpanded.add(level);
    }
    setExpandedLevels(newExpanded);
  };

  // Group spells by level
  const groupedSpells = spells.reduce((acc, spell) => {
    // Use the corrected innate_level from backend processing
    // No longer treat -1 as cantrip - backend now provides proper level
    let level = spell.innate_level;
    
    // If innate_level is null/undefined, skip this spell or put in special category
    if (level === null || level === undefined) {
      level = -1; // Special category for unavailable spells
    }
    
    if (!acc[level]) {
      acc[level] = [];
    }
    acc[level].push(spell);
    return acc;
  }, {} as Record<number, Spell[]>);

  const SpellCard = ({ spell }: { spell: Spell }) => {
    if (viewMode === 'list') {
      return (
        <div className={`spell-list-item ${spell.isLearned ? 'learned' : ''}`}>
          <div className="spell-list-grid">
            {/* Icon and Name Column */}
            <div className="spell-col-icon-name">
              <div className="flex items-center gap-2">
                <NWN2Icon icon={spell.icon} iconUrl={spell.icon_url} size="sm" />
                <div className="min-w-0 flex-1">
                  <h4 className="font-medium text-sm text-primary truncate">
                    {spell.name}
                  </h4>
                  <div className="flex items-center gap-1 mt-0.5">
                    {spell.school_name && (
                      <div className="spell-school-icon shrink-0" title={spell.school_name}>
                        {getSchoolIcon(spell.school_name, 'sm')}
                      </div>
                    )}
                    <span className="text-xs text-muted">
                      Level {spell.level === 0 ? 'Cantrip' : spell.level}
                    </span>
                  </div>
                </div>
              </div>
            </div>

            {/* Spell Details Column */}
            <div className="spell-col-details">
              <p className="text-xs text-secondary line-clamp-2 mb-1">
                {display(spell.description)}
              </p>
              <div className="flex items-center gap-2 text-xs text-muted">
                <span>{display(spell.range)}</span>
                <span>•</span>
                <span>{display(spell.cast_time)}</span>
                {spell.components && (
                  <>
                    <span>•</span>
                    <span>{display(spell.components)}</span>
                  </>
                )}
              </div>
            </div>

            {/* Classes Column */}
            <div className="spell-col-classes">
              {spell.available_classes.length > 0 && (
                <div className="flex flex-wrap gap-1">
                  {spell.available_classes.slice(0, 2).map((cls, idx) => (
                    <Badge key={idx} variant="outline" className="text-xs">
                      {cls}
                    </Badge>
                  ))}
                  {spell.available_classes.length > 2 && (
                    <span className="text-xs text-muted">+{spell.available_classes.length - 2}</span>
                  )}
                </div>
              )}
            </div>

            {/* Action Column */}
            <div className="spell-col-action">
              <Button
                variant={spell.isLearned ? 'spell-learned' : 'spell-ghost'}
                size="sm"
                onClick={() => onToggleLearned(spell.id)}
                hoverText={spell.isLearned ? 'Remove' : undefined}
              >
                {spell.isLearned ? 'Learned' : 'Learn'}
              </Button>
            </div>
          </div>
        </div>
      );
    }
    
    return (
      <Card 
        variant="interactive"
        learned={spell.isLearned}
        className="hover:shadow-elevation-3"
      >
        <div className="p-3">
          <div className="flex items-start justify-between mb-2">
            <div className="flex-1 mr-2">
              <div className="flex items-center space-x-2 mb-2">
                <NWN2Icon icon={spell.icon} iconUrl={spell.icon_url} size="md" />
                <div className="flex-1 min-w-0">
                  <h4 className="font-medium text-primary text-sm truncate">
                    {spell.name}
                  </h4>
                  <div className="flex items-center gap-1 mt-1">
                    {spell.school_name && (
                      <div className="spell-school-icon shrink-0" title={spell.school_name}>
                        {getSchoolIcon(spell.school_name, 'sm')}
                      </div>
                    )}
                    <span className="text-xs text-muted">
                      Level {spell.level === 0 ? 'Cantrip' : spell.level}
                    </span>
                  </div>
                </div>
              </div>
              
              {/* Spell details */}
              <div className="space-y-1 mb-2">
                <p className="text-xs text-secondary line-clamp-1">
                  {display(spell.description)}
                </p>
                <div className="flex flex-wrap gap-1 text-xs">
                  <span className="spell-detail-tag">
                    {display(spell.range)}
                  </span>
                  <span className="spell-detail-tag">
                    {display(spell.cast_time)}
                  </span>
                  {spell.components && (
                    <span className="spell-detail-tag">
                      {display(spell.components)}
                    </span>
                  )}
                </div>
                {spell.available_classes.length > 0 && (
                  <div className="flex flex-wrap gap-1">
                    {spell.available_classes.slice(0, 3).map((cls, idx) => (
                      <Badge key={idx} variant="outline" className="text-xs">
                        {cls}
                      </Badge>
                    ))}
                    {spell.available_classes.length > 3 && (
                      <span className="text-xs text-muted">+{spell.available_classes.length - 3}</span>
                    )}
                  </div>
                )}
              </div>
            </div>
            <div className="flex items-end">
              <Button
                variant={spell.isLearned ? 'spell-learned' : 'spell-ghost'}
                size="sm"
                onClick={() => onToggleLearned(spell.id)}
                hoverText={spell.isLearned ? 'Remove' : undefined}
              >
                {spell.isLearned ? 'Learned' : 'Learn'}
              </Button>
            </div>
          </div>
        </div>
      </Card>
    );
  };

  return (
    <Card className="flex-1" padding="p-0">
      <ScrollArea className="h-full p-4">
        {isLoading ? (
          <div className="space-y-4">
            {[...Array(5)].map((_, i) => (
              <Skeleton key={i} className="h-20 w-full" />
            ))}
          </div>
        ) : (
          <div className="space-y-6">
            {Object.entries(groupedSpells)
              .sort(([a], [b]) => {
                const numA = Number(a);
                const numB = Number(b);
                // Put -1 (unavailable) at the end
                if (numA === -1 && numB !== -1) return 1;
                if (numB === -1 && numA !== -1) return -1;
                return numA - numB;
              })
              .map(([level, levelSpells]) => (
                <div key={level} className="space-y-3">
                  <div 
                    className="flex items-center justify-between cursor-pointer"
                    onClick={() => toggleLevel(Number(level))}
                  >
                    <h3 className="text-lg font-semibold text-primary flex items-center gap-2">
                      {expandedLevels.has(Number(level)) ? 
                        <ChevronDown className="w-4 h-4" /> : 
                        <ChevronRight className="w-4 h-4" />
                      }
                      {Number(level) === 0 ? 'Cantrips' : Number(level) === -1 ? 'Unavailable' : `Level ${level}`}
                      <Badge variant="secondary">
                        {levelSpells.filter(s => s.isLearned).length}/{levelSpells.length}
                      </Badge>
                    </h3>
                  </div>
                  
                  {expandedLevels.has(Number(level)) && (
                    <div className={
                      viewMode === 'grid' 
                        ? 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3' 
                        : 'space-y-0'
                    }>
                      {levelSpells.map((spell) => (
                        <SpellCard key={spell.id} spell={spell} />
                      ))}
                    </div>
                  )}
                </div>
              ))}
          </div>
        )}
      </ScrollArea>
    </Card>
  );
}