'use client';

import React, { memo } from 'react';
import { Search, X } from 'lucide-react';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/Select';
import { cn } from '@/lib/utils';
import { useTranslations } from '@/hooks/useTranslations';

export type ItemTypeFilter = 'all' | 'weapon' | 'armor' | 'accessory' | 'consumable' | 'misc';
export type ItemSortOption = 'name' | 'value' | 'weight' | 'type';

export interface StatusFilter {
  custom: boolean;
  plot: boolean;
  identified: boolean;
  unidentified: boolean;
  enhanced: boolean;
}

export interface InventoryFiltersProps {
  searchTerm: string;
  onSearchChange: (value: string) => void;

  typeFilter: ItemTypeFilter;
  onTypeFilterChange: (value: ItemTypeFilter) => void;

  statusFilters: Set<keyof StatusFilter>;
  onStatusFiltersChange: (filters: Set<keyof StatusFilter>) => void;

  sortBy: ItemSortOption;
  onSortChange: (value: ItemSortOption) => void;

  filteredCount: number;
  totalCount: number;
}

const ITEM_TYPE_FILTERS: { value: ItemTypeFilter; labelKey: string; color: string }[] = [
  { value: 'weapon', labelKey: 'inventory.weapons', color: 'bg-red-500' },
  { value: 'armor', labelKey: 'inventory.armor', color: 'bg-blue-500' },
  { value: 'accessory', labelKey: 'inventory.accessories', color: 'bg-purple-500' },
  { value: 'consumable', labelKey: 'inventory.consumables', color: 'bg-green-500' },
  { value: 'misc', labelKey: 'inventory.miscellaneous', color: 'bg-gray-500' },
];

const STATUS_FILTERS: { key: keyof StatusFilter; labelKey: string; color: string }[] = [
  { key: 'custom', labelKey: 'inventory.filters.customItems', color: 'bg-orange-500' },
  { key: 'plot', labelKey: 'inventory.filters.plotItems', color: 'bg-yellow-500' },
  { key: 'identified', labelKey: 'inventory.filters.identified', color: 'bg-cyan-500' },
  { key: 'unidentified', labelKey: 'inventory.filters.unidentified', color: 'bg-slate-500' },
  { key: 'enhanced', labelKey: 'inventory.filters.enhanced', color: 'bg-indigo-500' },
];

const SORT_OPTIONS: { value: ItemSortOption; labelKey: string }[] = [
  { value: 'name', labelKey: 'inventory.filters.sortName' },
  { value: 'value', labelKey: 'inventory.filters.sortValue' },
  { value: 'weight', labelKey: 'inventory.filters.sortWeight' },
  { value: 'type', labelKey: 'inventory.filters.sortType' },
];

function InventoryFiltersComponent({
  searchTerm,
  onSearchChange,
  typeFilter,
  onTypeFilterChange,
  statusFilters,
  onStatusFiltersChange,
  sortBy,
  onSortChange,
  filteredCount,
  totalCount,
}: InventoryFiltersProps) {
  const t = useTranslations();

  const handleTypeToggle = (type: ItemTypeFilter) => {
    if (typeFilter === type) {
      onTypeFilterChange('all');
    } else {
      onTypeFilterChange(type);
    }
  };

  const handleStatusToggle = (status: keyof StatusFilter) => {
    const newFilters = new Set(statusFilters);
    if (newFilters.has(status)) {
      newFilters.delete(status);
    } else {
      newFilters.add(status);
    }
    onStatusFiltersChange(newFilters);
  };

  const clearFilters = () => {
    onSearchChange('');
    onTypeFilterChange('all');
    onStatusFiltersChange(new Set());
  };

  const hasActiveFilters = searchTerm.length > 0 || typeFilter !== 'all' || statusFilters.size > 0;

  return (
    <div className="flex flex-col gap-3 mb-4">
      <div className="flex items-center gap-3">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[rgb(var(--color-text-muted))]" />
          <Input
            type="text"
            placeholder={t('inventory.searchItems')}
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

        <div className="w-[160px]">
          <Select value={sortBy} onValueChange={(v) => onSortChange(v as ItemSortOption)}>
            <SelectTrigger className="hover:bg-[rgb(var(--color-surface-2))] hover:border-[rgb(var(--color-primary)/0.5)] focus:ring-[rgb(var(--color-primary)/0.2)]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {SORT_OPTIONS.map(option => (
                <SelectItem key={option.value} value={option.value}>
                  {t('inventory.filters.sortBy')}: {t(option.labelKey)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <span className="text-sm text-[rgb(var(--color-text-muted))] whitespace-nowrap">
          {t('inventory.filters.showing')} {filteredCount} / {totalCount} {t('inventory.filters.items')}
        </span>

        {hasActiveFilters && (
          <Button
            variant="outline"
            size="sm"
            onClick={clearFilters}
          >
            {t('inventory.filters.clearFilters')}
          </Button>
        )}
      </div>

      <div className="flex flex-wrap gap-2">
        {ITEM_TYPE_FILTERS.map(({ value, labelKey, color }) => (
          <button
            key={value}
            onClick={() => handleTypeToggle(value)}
            className={cn(
              'px-3 py-1 rounded-full text-xs font-medium transition-all',
              typeFilter === value
                ? 'bg-[rgb(var(--color-primary))] text-white'
                : 'bg-[rgb(var(--color-surface-2))] text-[rgb(var(--color-text-secondary))] hover:bg-[rgb(var(--color-surface-3))]'
            )}
          >
            <span className={cn('inline-block w-2 h-2 rounded-full mr-1.5', color)} />
            {t(labelKey)}
          </button>
        ))}

        <span className="w-px h-6 bg-[rgb(var(--color-surface-border))] mx-1 self-center" />

        {STATUS_FILTERS.map(({ key, labelKey, color }) => (
          <button
            key={key}
            onClick={() => handleStatusToggle(key)}
            className={cn(
              'px-3 py-1 rounded-full text-xs font-medium transition-all',
              statusFilters.has(key)
                ? 'bg-[rgb(var(--color-primary))] text-white'
                : 'bg-[rgb(var(--color-surface-2))] text-[rgb(var(--color-text-secondary))] hover:bg-[rgb(var(--color-surface-3))]'
            )}
          >
            <span className={cn('inline-block w-2 h-2 rounded-full mr-1.5', color)} />
            {t(labelKey)}
          </button>
        ))}
      </div>
    </div>
  );
}

export const InventoryFilters = memo(InventoryFiltersComponent);
