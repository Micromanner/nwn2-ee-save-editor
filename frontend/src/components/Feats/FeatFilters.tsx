'use client';

import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { Checkbox } from '@/components/ui/Checkbox';
import { Label } from '@/components/ui/Label';
import { Search, Shield, Swords, Sparkles, Sun, Zap } from 'lucide-react';

interface FilterState {
  search: string;
  types: number[];
  protected: boolean;
  custom: boolean;
  activeOnly: boolean;
}

interface FeatFiltersProps {
  filters: FilterState;
  onFiltersChange: (filters: FilterState) => void;
  featTypes: number[];
  featTypeCounts: Record<number, number>;
  onSearch?: (search: string) => void;
}

export default function FeatFilters({ filters, onFiltersChange, featTypes, featTypeCounts, onSearch }: FeatFiltersProps) {
  // Map feat types - based on NWN2 feat types
  const getFeatTypeName = (type: number): string => {
    switch (type) {
      case 1: return 'General';
      case 2: return 'Combat';
      case 8: return 'Metamagic';
      case 16: return 'Divine';
      case 32: return 'Epic';
      case 64: return 'Class';
      default: return 'General';
    }
  };

  const getTypeIcon = (type: number) => {
    switch (type) {
      case 2: return <Swords className="w-4 h-4" />; // Combat
      case 8: return <Sparkles className="w-4 h-4" />; // Metamagic
      case 16: return <Sun className="w-4 h-4" />; // Divine
      case 32: return <Zap className="w-4 h-4" />; // Epic
      case 64: return <Shield className="w-4 h-4" />; // Class
      default: return null;
    }
  };

  const updateFilters = (updates: Partial<FilterState>) => {
    onFiltersChange({ ...filters, ...updates });
  };

  const resetFilters = () => {
    onFiltersChange({
      search: '',
      types: [],
      protected: false,
      custom: false,
      activeOnly: false,
    });
  };

  return (
    <Card className="w-80" padding="p-0">
      <div className="p-4 overflow-y-auto">
        <h3 className="font-semibold mb-4">Advanced Filters</h3>
        
        {/* Search */}
        <div className="mb-4">
          <Label className="text-sm mb-2">Search</Label>
          <div className="relative">
            <Search className="absolute left-2 top-2.5 h-4 w-4 text-[rgb(var(--color-text-muted))]" />
            <Input
              placeholder="Search feats..."
              value={filters.search}
              onChange={(e) => {
                const newSearch = e.target.value;
                updateFilters({ search: newSearch });
                onSearch?.(newSearch);
              }}
              className="pl-8"
            />
          </div>
        </div>

        {/* Quick Filters */}
        <div className="mb-4 space-y-2">
          <Label className="text-sm">Quick Filters</Label>
          <div className="space-y-2">
            <label className="flex items-center space-x-2">
              <Checkbox 
                checked={filters.activeOnly}
                onCheckedChange={(checked: boolean | 'indeterminate') => 
                  updateFilters({ activeOnly: checked === true })
                }
              />
              <span className="text-sm">Current Feats Only</span>
            </label>
            <label className="flex items-center space-x-2">
              <Checkbox 
                checked={filters.protected}
                onCheckedChange={(checked: boolean | 'indeterminate') => 
                  updateFilters({ protected: checked === true })
                }
              />
              <span className="text-sm">Protected Only</span>
            </label>
            <label className="flex items-center space-x-2">
              <Checkbox 
                checked={filters.custom}
                onCheckedChange={(checked: boolean | 'indeterminate') => 
                  updateFilters({ custom: checked === true })
                }
              />
              <span className="text-sm">Custom Content Only</span>
            </label>
          </div>
        </div>

        {/* Feat Types */}
        <div className="mb-4">
          <Label className="text-sm mb-2">Feat Types</Label>
          <div className="space-y-1">
            {featTypes.map(type => (
              <label key={type} className="flex items-center space-x-2">
                <Checkbox 
                  checked={filters.types.includes(type)}
                  onCheckedChange={(checked: boolean | 'indeterminate') => {
                    if (checked === true) {
                      updateFilters({ types: [...filters.types, type] });
                    } else {
                      updateFilters({ types: filters.types.filter(t => t !== type) });
                    }
                  }}
                />
                <span className="text-sm flex items-center gap-1">
                  {getTypeIcon(type)}
                  {getFeatTypeName(type)}
                </span>
                <Badge variant="secondary" className="text-xs ml-auto">
                  {featTypeCounts[type] || 0}
                </Badge>
              </label>
            ))}
          </div>
        </div>

        {/* Reset Filters */}
        <Button 
          variant="outline" 
          className="w-full"
          onClick={resetFilters}
        >
          Reset Filters
        </Button>
      </div>
    </Card>
  );
}