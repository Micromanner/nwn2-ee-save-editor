'use client';

import React, { memo } from 'react';
import { Search, X } from 'lucide-react';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/Select';
import { cn } from '@/lib/utils';

export type SpellTab = 'my-spells' | 'all-spells';

export interface SpellNavBarProps {
  activeTab: SpellTab;
  onTabChange: (tab: SpellTab) => void;

  searchTerm: string;
  onSearchChange: (value: string) => void;

  sortBy: string;
  onSortChange: (value: string) => void;

  selectedClasses: Set<string>;
  onClassesChange: (classes: Set<string>) => void;

  selectedSchools: Set<string>;
  onSchoolsChange: (schools: Set<string>) => void;

  selectedLevels: Set<number>;
  onLevelsChange: (levels: Set<number>) => void;

  mySpellsCount: number;
  availableSpellsCount: number;
  filteredCount: number;

  currentPage: number;
  totalPages: number;
  hasNext: boolean;
  hasPrevious: boolean;
  onPageChange: (page: number) => void;

  availableClasses?: Array<{name: string; value: string}>;
}

const SORT_OPTIONS = [
  { value: 'name', label: 'Name' },
  { value: 'level', label: 'Spell Level' },
  { value: 'school', label: 'School' },
];

const SPELL_SCHOOL_FILTERS = [
  { value: 'Abjuration', label: 'Abjuration', color: 'bg-blue-500' },
  { value: 'Conjuration', label: 'Conjuration', color: 'bg-purple-500' },
  { value: 'Divination', label: 'Divination', color: 'bg-cyan-500' },
  { value: 'Enchantment', label: 'Enchantment', color: 'bg-pink-500' },
  { value: 'Evocation', label: 'Evocation', color: 'bg-red-500' },
  { value: 'Illusion', label: 'Illusion', color: 'bg-indigo-500' },
  { value: 'Necromancy', label: 'Necromancy', color: 'bg-gray-600' },
  { value: 'Transmutation', label: 'Transmutation', color: 'bg-green-500' },
  { value: 'Universal', label: 'Universal', color: 'bg-yellow-500' },
];

const SPELL_LEVEL_FILTERS = [
  { value: 0, label: 'Cantrips (0)' },
  { value: 1, label: 'Level 1' },
  { value: 2, label: 'Level 2' },
  { value: 3, label: 'Level 3' },
  { value: 4, label: 'Level 4' },
  { value: 5, label: 'Level 5' },
  { value: 6, label: 'Level 6' },
  { value: 7, label: 'Level 7' },
  { value: 8, label: 'Level 8' },
  { value: 9, label: 'Level 9' },
];

function SpellNavBarComponent({
  activeTab,
  onTabChange,
  searchTerm,
  onSearchChange,
  sortBy,
  onSortChange,
  selectedClasses,
  onClassesChange,
  selectedSchools,
  onSchoolsChange,
  selectedLevels,
  onLevelsChange,
  mySpellsCount,
  availableSpellsCount,
  filteredCount,
  currentPage,
  totalPages,
  hasNext,
  hasPrevious,
  onPageChange,
  availableClasses = [],
}: SpellNavBarProps) {

  const handleClassToggle = (className: string) => {
    const newClasses = new Set(selectedClasses);
    if (newClasses.has(className)) {
      newClasses.delete(className);
    } else {
      newClasses.add(className);
    }
    onClassesChange(newClasses);
  };

  const handleSchoolToggle = (school: string) => {
    const newSchools = new Set(selectedSchools);
    if (newSchools.has(school)) {
      newSchools.delete(school);
    } else {
      newSchools.add(school);
    }
    onSchoolsChange(newSchools);
  };

  const handleLevelToggle = (level: number) => {
    const newLevels = new Set(selectedLevels);
    if (newLevels.has(level)) {
      newLevels.delete(level);
    } else {
      newLevels.add(level);
    }
    onLevelsChange(newLevels);
  };

  const clearFilters = () => {
    onSearchChange('');
    onClassesChange(new Set());
    onSchoolsChange(new Set());
    onLevelsChange(new Set());
  };

  const hasActiveFilters = searchTerm.length > 0 || selectedClasses.size > 0 || selectedSchools.size > 0 || selectedLevels.size > 0;

  return (
    <div className="flex flex-col gap-4 bg-[rgb(var(--color-surface-1))] border border-[rgb(var(--color-surface-border))] rounded-lg p-4">
      <div className="flex items-center justify-between gap-4">
        <div className="flex gap-2">
          <Button
            variant={activeTab === 'my-spells' ? 'primary' : 'outline'}
            onClick={() => onTabChange('my-spells')}
            size="sm"
          >
            My Spells
            <Badge variant="secondary" className="ml-2">
              {mySpellsCount}
            </Badge>
          </Button>

          <Button
            variant={activeTab === 'all-spells' ? 'primary' : 'outline'}
            onClick={() => onTabChange('all-spells')}
            size="sm"
          >
            All Spells
            <Badge variant="secondary" className="ml-2">
              {availableSpellsCount}
            </Badge>
          </Button>
        </div>

        <div className="flex items-center gap-4">
          <span className="text-sm text-[rgb(var(--color-text-muted))]">
            Showing {filteredCount} {activeTab === 'my-spells' ? 'spells' : 'available'}
          </span>

          {activeTab === 'all-spells' && totalPages > 1 && (
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => onPageChange(currentPage - 1)}
                disabled={!hasPrevious}
              >
                Previous
              </Button>
              <span className="text-sm text-[rgb(var(--color-text-primary))] px-2">
                Page {currentPage} of {totalPages}
              </span>
              <Button
                variant="outline"
                size="sm"
                onClick={() => onPageChange(currentPage + 1)}
                disabled={!hasNext}
              >
                Next
              </Button>
            </div>
          )}
        </div>
      </div>

      <div className="flex items-center gap-3">
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[rgb(var(--color-text-muted))]" />
          <Input
            type="text"
            placeholder="Search spells..."
            value={searchTerm}
            onChange={(e) => onSearchChange(e.target.value)}
            className="pl-9 pr-9"
          />
          {searchTerm && (
            <button
              onClick={() => onSearchChange('')}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-[rgb(var(--color-text-muted))] hover:text-[rgb(var(--color-text-primary))]"
            >
              <X className="w-4 h-4" />
            </button>
          )}
        </div>

        <div className="w-[180px]">
          <Select value={sortBy} onValueChange={onSortChange}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {SORT_OPTIONS.map(option => (
                <SelectItem key={option.value} value={option.value}>
                  Sort: {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {hasActiveFilters && (
          <Button
            variant="outline"
            size="sm"
            onClick={clearFilters}
          >
            Clear Filters
          </Button>
        )}
      </div>

      <div className="flex flex-col gap-3">
        {availableClasses.length > 0 && (
          <div className="flex items-center gap-2">
            <span className="text-xs font-medium text-[rgb(var(--color-text-secondary))] min-w-[60px]">Classes:</span>
            <div className="flex flex-wrap gap-2">
              {availableClasses.map(({ name, value }) => (
                <button
                  key={value}
                  onClick={() => handleClassToggle(value)}
                  className={cn(
                    'px-3 py-1 rounded-full text-xs font-medium transition-all',
                    selectedClasses.has(value)
                      ? 'bg-[rgb(var(--color-primary))] text-white'
                      : 'bg-[rgb(var(--color-surface-2))] text-[rgb(var(--color-text-secondary))] hover:bg-[rgb(var(--color-surface-3))]'
                  )}
                >
                  {name}
                </button>
              ))}
            </div>
          </div>
        )}

        <div className="flex items-center gap-2">
          <span className="text-xs font-medium text-[rgb(var(--color-text-secondary))] min-w-[60px]">Schools:</span>
          <div className="flex flex-wrap gap-2">
            {SPELL_SCHOOL_FILTERS.map(({ value, label, color }) => (
              <button
                key={value}
                onClick={() => handleSchoolToggle(value)}
                className={cn(
                  'px-3 py-1 rounded-full text-xs font-medium transition-all',
                  selectedSchools.has(value)
                    ? 'bg-[rgb(var(--color-primary))] text-white'
                    : 'bg-[rgb(var(--color-surface-2))] text-[rgb(var(--color-text-secondary))] hover:bg-[rgb(var(--color-surface-3))]'
                )}
              >
                <span className={cn('inline-block w-2 h-2 rounded-full mr-1.5', color)} />
                {label}
              </button>
            ))}
          </div>
        </div>

        <div className="flex items-center gap-2">
          <span className="text-xs font-medium text-[rgb(var(--color-text-secondary))] min-w-[60px]">Levels:</span>
          <div className="flex flex-wrap gap-2">
            {SPELL_LEVEL_FILTERS.map(({ value, label }) => (
              <button
                key={value}
                onClick={() => handleLevelToggle(value)}
                className={cn(
                  'px-3 py-1 rounded-full text-xs font-medium transition-all',
                  selectedLevels.has(value)
                    ? 'bg-[rgb(var(--color-primary))] text-white'
                    : 'bg-[rgb(var(--color-surface-2))] text-[rgb(var(--color-text-secondary))] hover:bg-[rgb(var(--color-surface-3))]'
                )}
              >
                {label}
              </button>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

export const SpellNavBar = memo(SpellNavBarComponent);
