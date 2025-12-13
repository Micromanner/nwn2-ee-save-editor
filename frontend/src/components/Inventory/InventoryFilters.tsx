'use client';

import React, { memo } from 'react';
import { Search, X, Filter } from 'lucide-react';
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

        <div className="relative">
             <Button
                variant={hasActiveFilters ? "primary" : "outline"}
                size="sm"
                className="gap-2"
                onClick={() => {
                   const el = document.getElementById('filter-dropdown');
                   if (el) el.classList.toggle('hidden');
                }}
              >
                <Filter className="w-4 h-4" />
                {hasActiveFilters && (
                    <span className="flex h-5 w-5 items-center justify-center rounded-full bg-white/20 text-xs">
                        {statusFilters.size + (typeFilter !== 'all' ? 1 : 0)}
                    </span>
                )}
              </Button>
              
              <div id="filter-dropdown" className="hidden absolute right-0 top-full mt-2 w-64 p-4 rounded-md border border-[rgb(var(--color-surface-border))] bg-[rgb(var(--color-surface-1))] shadow-lg z-50 space-y-4">
                  
                  {/* Type Filters */}
                  <div>
                      <h4 className="text-xs font-semibold mb-2 text-[rgb(var(--color-text-muted))] uppercase tracking-wider">{t('inventory.filters.type')}</h4>
                      <div className="space-y-1">
                        {ITEM_TYPE_FILTERS.map(({ value, labelKey, color }) => (
                          <div 
                            key={value}
                            className={cn(
                                "flex items-center gap-2 p-2 rounded cursor-pointer transition-colors text-sm",
                                typeFilter === value ? "bg-[rgb(var(--color-surface-3))]" : "hover:bg-[rgb(var(--color-surface-2))]"
                            )}
                            onClick={() => handleTypeToggle(value)}
                          >
                             <div className={cn("w-3 h-3 rounded-full", color)}></div>
                             <span className={cn("flex-1", typeFilter === value ? "text-[rgb(var(--color-text-primary))]" : "text-[rgb(var(--color-text-secondary))]" )}>
                                {t(labelKey)}
                             </span>
                             {typeFilter === value && <span className="text-[rgb(var(--color-primary))]">✓</span>}
                          </div>
                        ))}
                      </div>
                  </div>

                  <div className="border-t border-[rgb(var(--color-surface-border)/0.5)]"></div>

                  {/* Status Filters */}
                  <div>
                      <h4 className="text-xs font-semibold mb-2 text-[rgb(var(--color-text-muted))] uppercase tracking-wider">{t('inventory.filters.status')}</h4>
                      <div className="space-y-1">
                        {STATUS_FILTERS.map(({ key, labelKey, color }) => (
                             <div 
                                key={key}
                                className={cn(
                                    "flex items-center gap-2 p-2 rounded cursor-pointer transition-colors text-sm",
                                    statusFilters.has(key) ? "bg-[rgb(var(--color-surface-3))]" : "hover:bg-[rgb(var(--color-surface-2))]"
                                )}
                                onClick={() => handleStatusToggle(key)}
                              >
                                 <div className={cn("w-3 h-3 rounded-full", color)}></div>
                                 <span className={cn("flex-1", statusFilters.has(key) ? "text-[rgb(var(--color-text-primary))]" : "text-[rgb(var(--color-text-secondary))]" )}>
                                    {t(labelKey)}
                                 </span>
                                 {statusFilters.has(key) && <span className="text-[rgb(var(--color-primary))]">✓</span>}
                              </div>
                        ))}
                      </div>
                  </div>

                  {hasActiveFilters && (
                    <div className="pt-2 border-t border-[rgb(var(--color-surface-border)/0.5)]">
                        <Button variant="ghost" size="sm" className="w-full text-xs" onClick={clearFilters}>
                            {t('inventory.filters.clearFilters')}
                        </Button>
                    </div>
                  )}

                  {/* Close on click outside is not implemented natively here for simplicity, 
                      but in a real app ideally use a Popover component or click-outside hook. 
                      For now, the user can toggle nicely. */}
              </div>
        </div>
      </div>
    </div>
  );
}

export const InventoryFilters = memo(InventoryFiltersComponent);
