'use client';

import { useState } from 'react';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { ScrollArea } from '@/components/ui/ScrollArea';
import { 
  ChevronRight, 
  ChevronDown, 
  BookOpen, 
  Star,
  Search,
  Layers
} from 'lucide-react';
import FeatSummary from './FeatSummary';
import FeatSearchBar from './FeatSearchBar';
import type { CategoryInfo, FeatsState } from './types';

interface FeatCategorySidebarProps {
  categories: CategoryInfo[];
  selectedCategory: string | null;
  selectedSubcategory: string | null;
  onCategorySelect: (categoryId: string) => void;
  onSubcategorySelect: (subcategoryId: string) => void;
  featsData: FeatsState | null;
  availableFeatsCount: number;
  showMyFeats?: boolean;
  onMyFeatsClick?: () => void;
  globalSearch?: string;
  onGlobalSearchChange?: (search: string) => void;
}

export default function FeatCategorySidebar({
  categories,
  selectedCategory,
  selectedSubcategory,
  onCategorySelect,
  onSubcategorySelect,
  featsData,
  availableFeatsCount,
  showMyFeats = true,
  onMyFeatsClick,
  globalSearch = '',
  onGlobalSearchChange,
}: FeatCategorySidebarProps) {
  const [expandedCategories, setExpandedCategories] = useState<Set<string>>(
    new Set(selectedCategory ? [selectedCategory] : [])
  );

  const toggleCategory = (categoryId: string) => {
    const newExpanded = new Set(expandedCategories);
    if (newExpanded.has(categoryId)) {
      newExpanded.delete(categoryId);
    } else {
      newExpanded.add(categoryId);
    }
    setExpandedCategories(newExpanded);
  };

  const handleCategoryClick = (categoryId: string) => {
    onCategorySelect(categoryId);
    // Auto-expand if it has subcategories
    const category = categories.find(c => c.id === categoryId);
    if (category?.subcategories && category.subcategories.length > 0) {
      setExpandedCategories(prev => new Set([...prev, categoryId]));
    }
  };

  return (
    <div className="w-64 flex flex-col gap-4">
      {/* Quick Access */}
      <Card padding="p-3">
        {showMyFeats && onMyFeatsClick && (
          <Button
            variant="ghost"
            size="sm"
            className="w-full justify-start mb-2 text-left hover:bg-[rgb(var(--color-primary))]/10"
            onClick={onMyFeatsClick}
          >
            <span className="flex-1">My Feats</span>
            {featsData && (
              <Badge variant="default" className="ml-auto">
                {featsData.current_feats.total}
              </Badge>
            )}
          </Button>
        )}
      </Card>

      {/* Category List */}
      <Card padding="p-0" className="flex-1 overflow-hidden">
        <div className="p-3 border-b border-[rgb(var(--color-border))]">
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-semibold text-[rgb(var(--color-text-secondary))]">
              Browse Categories
            </h3>
          </div>
        </div>
        <ScrollArea className="h-full">
          <div className="p-3 space-y-1">
            {categories.map((category) => {
              const isSelected = selectedCategory === category.id && !selectedSubcategory;
              const hasSubcategories = category.subcategories && category.subcategories.length > 0;
              const isExpanded = expandedCategories.has(category.id);

              return (
                <div key={category.id}>
                  <Button
                    variant={isSelected ? 'primary' : 'ghost'}
                    size="sm"
                    className={`
                      w-full justify-start text-left
                      ${isSelected ? '' : 'hover:bg-[rgb(var(--color-surface-2))]'}
                    `}
                    onClick={() => {
                      handleCategoryClick(category.id);
                      if (hasSubcategories) {
                        toggleCategory(category.id);
                      }
                    }}
                  >
                    <div className="flex items-center w-full">
                      {hasSubcategories && (
                        <span className="mr-1">
                          {isExpanded ? (
                            <ChevronDown className="w-3 h-3" />
                          ) : (
                            <ChevronRight className="w-3 h-3" />
                          )}
                        </span>
                      )}
                      <span className="flex-1">{category.name}</span>
                      {category.count > 0 && (
                        <Badge 
                          variant={isSelected ? 'secondary' : 'outline'} 
                          className="ml-auto text-xs"
                        >
                          {category.count}
                        </Badge>
                      )}
                    </div>
                  </Button>

                  {/* Subcategories */}
                  {hasSubcategories && isExpanded && category.subcategories && (
                    <div className="ml-3 mt-1 space-y-0.5">
                      {category.subcategories.map((subcategory) => {
                        const isSubSelected = 
                          selectedCategory === category.id && 
                          selectedSubcategory === subcategory.id;

                        return (
                          <Button
                            key={subcategory.id}
                            variant={isSubSelected ? 'primary' : 'ghost'}
                            size="sm"
                            className={`
                              w-full justify-start text-xs pl-6
                              ${isSubSelected ? '' : 'hover:bg-[rgb(var(--color-surface-2))]'}
                            `}
                            onClick={() => {
                              onCategorySelect(category.id);
                              onSubcategorySelect(subcategory.id);
                            }}
                          >
                            <span className="flex-1">{subcategory.name}</span>
                            {subcategory.count > 0 && (
                              <Badge 
                                variant={isSubSelected ? 'secondary' : 'outline'} 
                                className="ml-auto text-xs scale-90"
                              >
                                {subcategory.count}
                              </Badge>
                            )}
                          </Button>
                        );
                      })}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </ScrollArea>
      </Card>

      {/* Feat Summary Card */}
      <FeatSummary 
        featsData={featsData} 
        availableFeatsCount={availableFeatsCount} 
      />
    </div>
  );
}