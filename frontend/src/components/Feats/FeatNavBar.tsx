'use client';

import React, { memo } from 'react';
import { Search, X } from 'lucide-react';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/Select';
import { cn } from '@/lib/utils';

export type FeatTab = 'my-feats' | 'all-feats';

export interface FeatNavBarProps {
  activeTab: FeatTab;
  onTabChange: (tab: FeatTab) => void;

  searchTerm: string;
  onSearchChange: (value: string) => void;

  sortBy: string;
  onSortChange: (value: string) => void;

  selectedTypes: Set<number>;
  onTypesChange: (types: Set<number>) => void;

  myFeatsCount: number;
  availableFeatsCount: number;
  filteredCount: number;

  currentPage: number;
  totalPages: number;
  hasNext: boolean;
  hasPrevious: boolean;
  onPageChange: (page: number) => void;
}

const SORT_OPTIONS = [
  { value: 'name', label: 'Name' },
  { value: 'type', label: 'Type' },
  { value: 'level', label: 'Level Required' },
];

const FEAT_TYPE_FILTERS = [
  { value: 1, label: 'General', color: 'bg-blue-500' },
  { value: 2, label: 'Proficiency', color: 'bg-gray-500' },
  { value: 4, label: 'Skill/Save', color: 'bg-cyan-500' },
  { value: 8, label: 'Metamagic', color: 'bg-purple-500' },
  { value: 16, label: 'Divine', color: 'bg-yellow-500' },
  { value: 32, label: 'Epic', color: 'bg-orange-500' },
  { value: 64, label: 'Class', color: 'bg-green-500' },
  { value: 128, label: 'Background', color: 'bg-teal-500' },
  { value: 256, label: 'Spellcasting', color: 'bg-indigo-500' },
  { value: 512, label: 'History', color: 'bg-amber-500' },
  { value: 1024, label: 'Heritage', color: 'bg-rose-500' },
  { value: 2048, label: 'Item Creation', color: 'bg-lime-500' },
  { value: 4096, label: 'Racial', color: 'bg-pink-500' },
  { value: 8192, label: 'Domain', color: 'bg-indigo-500' },
];

function FeatNavBarComponent({
  activeTab,
  onTabChange,
  searchTerm,
  onSearchChange,
  sortBy,
  onSortChange,
  selectedTypes,
  onTypesChange,
  myFeatsCount,
  availableFeatsCount,
  filteredCount,
  currentPage,
  totalPages,
  hasNext,
  hasPrevious,
  onPageChange,
}: FeatNavBarProps) {

  const handleTypeToggle = (type: number) => {
    const newTypes = new Set(selectedTypes);
    if (newTypes.has(type)) {
      newTypes.delete(type);
    } else {
      newTypes.add(type);
    }
    onTypesChange(newTypes);
  };

  const clearFilters = () => {
    onSearchChange('');
    onTypesChange(new Set());
  };

  const hasActiveFilters = searchTerm.length > 0 || selectedTypes.size > 0;

  return (
    <div className="flex flex-col gap-4 bg-[rgb(var(--color-surface-1))] border border-[rgb(var(--color-surface-border))] rounded-lg p-4">
      <div className="flex items-center justify-between gap-4">
        <div className="flex gap-2">
          <Button
            variant={activeTab === 'my-feats' ? 'primary' : 'outline'}
            onClick={() => onTabChange('my-feats')}
            size="sm"
          >
            My Feats
            <Badge variant="secondary" className="ml-2">
              {myFeatsCount}
            </Badge>
          </Button>

          <Button
            variant={activeTab === 'all-feats' ? 'primary' : 'outline'}
            onClick={() => onTabChange('all-feats')}
            size="sm"
          >
            All Feats
            <Badge variant="secondary" className="ml-2">
              {availableFeatsCount}
            </Badge>
          </Button>
        </div>

        <div className="flex items-center gap-4">
          <span className="text-sm text-[rgb(var(--color-text-muted))]">
            Showing {filteredCount} {activeTab === 'my-feats' ? 'feats' : 'available'}
          </span>

          {activeTab === 'all-feats' && totalPages > 1 && (
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
            placeholder="Search feats..."
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
            <SelectTrigger className="hover:bg-[rgb(var(--color-surface-2))] hover:border-[rgb(var(--color-primary)/0.5)] focus:ring-[rgb(var(--color-primary)/0.2)]">
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

      <div className="flex flex-wrap gap-2">
        {FEAT_TYPE_FILTERS.map(({ value, label, color }) => (
          <button
            key={value}
            onClick={() => handleTypeToggle(value)}
            className={cn(
              'px-3 py-1 rounded-full text-xs font-medium transition-all',
              selectedTypes.has(value)
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
  );
}

export const FeatNavBar = memo(FeatNavBarComponent);
