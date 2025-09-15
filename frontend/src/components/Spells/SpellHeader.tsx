import React from 'react';
import { Card } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';

interface SpellHeaderProps {
  filteredCount: number;
  totalCount: number;
  learnedCount: number;
}

export default function SpellHeader({
  filteredCount,
  totalCount,
  learnedCount
}: SpellHeaderProps) {
  return (
    <Card padding="p-4">
      <div className="flex items-center gap-4">
        <div className="text-sm text-secondary">
          {filteredCount} of {totalCount} spells
        </div>
        <Badge variant="secondary">
          {learnedCount} learned
        </Badge>
      </div>
    </Card>
  );
}