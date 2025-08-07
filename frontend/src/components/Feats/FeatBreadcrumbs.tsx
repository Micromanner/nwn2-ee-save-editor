'use client';

import { ChevronRight, Home } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import type { CategoryInfo } from './types';

interface FeatBreadcrumbsProps {
  category: string | null;
  subcategory: string | null;
  categories: CategoryInfo[];
  onNavigate: (category: string | null, subcategory: string | null) => void;
  totalFeats?: number;
}

export default function FeatBreadcrumbs({
  category,
  subcategory,
  categories,
  onNavigate,
  totalFeats = 0,
}: FeatBreadcrumbsProps) {
  // Find the current category object
  const currentCategory = categories.find(c => c.id === category);
  const currentSubcategory = currentCategory?.subcategories?.find(s => s.id === subcategory);

  return (
    <div className="flex items-center gap-2 text-sm">
      {/* Home/All Feats */}
      <Button
        variant="ghost"
        size="sm"
        className="px-2 py-1 h-auto"
        onClick={() => onNavigate(null, null)}
      >
        <Home className="w-3 h-3 mr-1" />
        All Feats
      </Button>

      {/* Category */}
      {category && currentCategory && (
        <>
          <ChevronRight className="w-4 h-4 text-[rgb(var(--color-text-muted))]" />
          <Button
            variant={subcategory ? 'ghost' : 'primary'}
            size="sm"
            className="px-2 py-1 h-auto"
            onClick={() => onNavigate(category, null)}
          >
            {currentCategory.name}
          </Button>
        </>
      )}

      {/* Subcategory */}
      {subcategory && currentSubcategory && (
        <>
          <ChevronRight className="w-4 h-4 text-[rgb(var(--color-text-muted))]" />
          <Button
            variant="primary"
            size="sm"
            className="px-2 py-1 h-auto"
            onClick={() => onNavigate(category, subcategory)}
          >
            {currentSubcategory.name}
          </Button>
        </>
      )}

      {/* Total count badge */}
      {totalFeats > 0 && (
        <Badge variant="secondary" className="ml-auto">
          {totalFeats} {totalFeats === 1 ? 'feat' : 'feats'}
        </Badge>
      )}
    </div>
  );
}