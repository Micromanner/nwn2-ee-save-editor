import React from 'react';
import { Card } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { viewModeConfig } from './SpellSections';

interface SpellHeaderProps {
  filteredCount: number;
  totalCount: number;
  learnedCount: number;
  viewMode: 'grid' | 'list';
  onViewModeChange: (mode: 'grid' | 'list') => void;
}

export default function SpellHeader({
  filteredCount,
  totalCount,
  learnedCount,
  viewMode,
  onViewModeChange
}: SpellHeaderProps) {
  return (
    <Card padding="p-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <div className="text-sm text-secondary">
            {filteredCount} of {totalCount} spells
          </div>
          <Badge variant="secondary">
            {learnedCount} learned
          </Badge>
        </div>
        <div className="flex items-center gap-2">
          <div className="flex rounded-md border border-primary overflow-hidden">
            {viewModeConfig.modes.map((mode, index) => (
              <Button
                key={mode.id}
                variant={viewMode === mode.id ? 'spell-learned' : 'spell-ghost'}
                size="sm"
                className={`
                  ${index === 0 ? 'rounded-r-none' : ''}
                  ${index === viewModeConfig.modes.length - 1 ? 'rounded-l-none' : ''}
                  ${index > 0 && index < viewModeConfig.modes.length - 1 ? 'rounded-none' : ''}
                  border-0
                `}
                onClick={() => onViewModeChange(mode.id as 'grid' | 'list')}
                title={mode.label}
              >
                {mode.icon}
              </Button>
            ))}
          </div>
        </div>
      </div>
    </Card>
  );
}