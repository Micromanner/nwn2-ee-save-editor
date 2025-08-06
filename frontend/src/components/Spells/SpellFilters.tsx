import React from 'react';
import { Card } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Checkbox } from '@/components/ui/Checkbox';
import { Label } from '@/components/ui/Label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/Select';
import { Button } from '@/components/ui/Button';
import { spellFilterConfigs, spellSlotConfig, getSchoolIcon } from './SpellSections';

interface Filters {
  search: string;
  level: number | 'all';
  school: string;
  onlyLearned: boolean;
}

interface SpellFiltersProps {
  filters: Filters;
  onFilterChange: (filters: Filters) => void;
  schools: string[];
  spellSlots: Array<{level: number; total: number}>;
  showResetButton?: boolean;
}

export default function SpellFilters({ 
  filters, 
  onFilterChange, 
  schools, 
  spellSlots,
  showResetButton = true 
}: SpellFiltersProps) {
  const handleFilterChange = (key: string, value: string | number | boolean | 'all') => {
    onFilterChange({ ...filters, [key]: value });
  };

  const handleReset = () => {
    onFilterChange({
      search: '',
      level: 'all',
      school: 'all',
      onlyLearned: false,
    });
  };

  return (
    <Card className="w-80" padding="p-0">
      <div className="p-4 overflow-y-auto">
        <h3 className="font-semibold mb-4">Spell Filters</h3>
        
        {/* Search */}
        <div className="mb-4">
          <Label className="text-sm mb-2">Search</Label>
          <div className="relative">
            <span className="absolute left-2 top-2.5">
              {spellFilterConfigs.find(f => f.id === 'search')?.icon}
            </span>
            <Input
              placeholder="Search spells..."
              value={filters.search}
              onChange={(e) => handleFilterChange('search', e.target.value)}
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
                checked={filters.onlyLearned}
                onCheckedChange={(checked: boolean | 'indeterminate') => 
                  handleFilterChange('onlyLearned', checked === true)
                }
              />
              <span className="text-sm">Learned Only</span>
            </label>
          </div>
        </div>

        {/* Level Filter */}
        <div className="mb-4">
          <Label className="text-sm mb-2">Spell Level</Label>
          <Select
            value={filters.level.toString()}
            onValueChange={(value: string) => 
              handleFilterChange('level', value === 'all' ? 'all' : parseInt(value))
            }
          >
            <SelectTrigger>
              <SelectValue placeholder="All Levels" />
            </SelectTrigger>
            <SelectContent>
              {spellFilterConfigs.find(f => f.id === 'level')?.options?.map(option => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* School Filter */}
        <div className="mb-4">
          <Label className="text-sm mb-2">Schools</Label>
          <Select
            value={filters.school}
            onValueChange={(value: string) => handleFilterChange('school', value)}
          >
            <SelectTrigger>
              <SelectValue placeholder="All Schools" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Schools</SelectItem>
              {schools.map(school => (
                <SelectItem key={school} value={school}>
                  <span className="flex items-center gap-1">
                    {getSchoolIcon(school, 'sm')}
                    {school}
                  </span>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* Reset Filters */}
        {showResetButton && (
          <Button 
            variant="spell-ghost"
            className="w-full mb-6"
            onClick={handleReset}
          >
            Reset Filters
          </Button>
        )}

        {/* Spell Slots Summary */}
        <Card className="p-4" backgroundColor="rgb(var(--color-surface-1))" shadow="shadow-elevation-2">
          <h4 className="font-semibold text-sm mb-3 flex items-center gap-2">
            {spellSlotConfig.icon}
            {spellSlotConfig.title}
          </h4>
          <div className="space-y-2">
            {spellSlots.map((slot) => (
              <div key={slot.level} className="flex items-center justify-between">
                <span className="text-xs text-muted">
                  {slot.level === 0 ? 'Cantrips' : `Level ${slot.level}`}
                </span>
                <div className="flex items-center gap-2">
                  <div className="flex space-x-0.5">
                    {Array.from({ length: slot.total }).map((_, index) => (
                      <div
                        key={index}
                        className="spell-slot-dot"
                      />
                    ))}
                  </div>
                  <span className="text-xs font-medium">
                    {slot.total}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </Card>
      </div>
    </Card>
  );
}