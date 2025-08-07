'use client';

import { useState, useMemo } from 'react';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { ScrollArea } from '@/components/ui/ScrollArea';
import { 
  Grid, 
  List, 
  ChevronRight, 
  ChevronDown,
  Loader2,
  AlertCircle
} from 'lucide-react';
import FeatCard from './FeatCard';
import VirtualizedFeatSection from './VirtualizedFeatSection';
import FeatSearchBar from './FeatSearchBar';
import { useFeatSearch } from '@/hooks/useFeatSearch';
import type { 
  FeatInfo, 
  FeatsState, 
  ViewMode,
  ValidationCache,
  FeatManagementCallbacks
} from './types';

interface FeatContentAreaProps extends FeatManagementCallbacks {
  // Category selection
  selectedCategory: string | null;
  selectedSubcategory: string | null;
  
  // Feat data
  featsData: FeatsState | null;
  categoryFeats: FeatInfo[];
  categoryFeatsLoading: boolean;
  categoryFeatsError: string | null;
  
  // Display options
  viewMode: ViewMode;
  onViewModeChange: (mode: ViewMode) => void;
  
  // Validation
  validationCache?: ValidationCache;
  validatingFeatId?: number | null;
}

export default function FeatContentArea({
  selectedCategory,
  selectedSubcategory,
  featsData,
  categoryFeats,
  categoryFeatsLoading,
  categoryFeatsError,
  viewMode,
  onViewModeChange,
  onDetails,
  onAdd,
  onRemove,
  onValidate,
  validationCache = {},
  validatingFeatId = null,
}: FeatContentAreaProps) {
  const [expandedSections, setExpandedSections] = useState<Set<string>>(
    new Set(['current', 'available'])
  );
  const [categorySearch, setCategorySearch] = useState('');

  // Combine all current feats with deduplication
  const allCurrentFeats = useMemo(() => {
    if (!featsData) return [];
    
    const allFeats = [
      ...featsData.current_feats.protected,
      ...featsData.current_feats.class_feats,
      ...featsData.current_feats.general_feats,
      ...featsData.current_feats.custom_feats,
    ];
    
    // Deduplicate by feat ID
    const uniqueFeats = new Map<number, FeatInfo>();
    allFeats.forEach(feat => {
      uniqueFeats.set(feat.id, feat);
    });
    
    return Array.from(uniqueFeats.values());
  }, [featsData]);

  // Filter out feats the character already has from category feats
  const availableCategoryFeats = useMemo(() => {
    if (!selectedCategory) return [];
    const currentFeatIds = new Set(allCurrentFeats.map(f => f.id));
    return categoryFeats.filter(feat => !currentFeatIds.has(feat.id));
  }, [categoryFeats, allCurrentFeats, selectedCategory]);

  // Use search hook for category feats
  const { searchResults: filteredCategoryFeats } = useFeatSearch(
    availableCategoryFeats,
    categorySearch,
    { 
      keys: ['label', 'name', 'description'],
      threshold: 0.3,
      limit: 200
    }
  );

  // Group current feats by category
  const groupedCurrentFeats = useMemo(() => {
    if (!featsData) return {};
    
    const groups: Record<string, FeatInfo[]> = {
      'Protected': [],
      'Class': [],
      'General': [],
      'Custom': [],
    };
    
    for (const feat of allCurrentFeats) {
      if (feat.protected) {
        groups['Protected'].push(feat);
      } else if (feat.custom) {
        groups['Custom'].push(feat);
      } else if (featsData.current_feats.class_feats.some(f => f.id === feat.id)) {
        groups['Class'].push(feat);
      } else {
        groups['General'].push(feat);
      }
    }
    
    // Remove empty groups
    return Object.fromEntries(
      Object.entries(groups).filter(([, feats]) => feats.length > 0)
    );
  }, [allCurrentFeats, featsData]);

  const toggleSection = (section: string) => {
    const newExpanded = new Set(expandedSections);
    if (newExpanded.has(section)) {
      newExpanded.delete(section);
    } else {
      newExpanded.add(section);
    }
    setExpandedSections(newExpanded);
  };

  // Auto-expand available section when category is selected
  useMemo(() => {
    if (selectedCategory) {
      setExpandedSections(prev => new Set([...prev, 'available']));
    }
  }, [selectedCategory]);

  return (
    <div className="flex-1 flex flex-col gap-4">
      {/* Header with Controls */}
      <Card padding="p-4">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-4">
            <h2 className="text-lg font-semibold">
              {selectedCategory ? 
                `${selectedSubcategory || selectedCategory} Feats` : 
                'All Feats'}
            </h2>
            <div className="text-sm text-[rgb(var(--color-text-secondary))]">
              {selectedCategory ? 
                `${filteredCategoryFeats.length} available` : 
                `${allCurrentFeats.length} current`}
            </div>
          </div>
          
          <div className="flex items-center gap-3">
            {/* View Mode Toggle */}
            <div className="flex rounded-md border border-[rgb(var(--color-border))]">
              <Button
                variant={viewMode === 'grid' ? 'primary' : 'ghost'}
                size="sm"
                className="rounded-r-none border-0"
                onClick={() => onViewModeChange('grid')}
              >
                <Grid className="w-4 h-4" />
              </Button>
              <Button
                variant={viewMode === 'list' ? 'primary' : 'ghost'}
                size="sm"
                className="rounded-l-none border-0"
                onClick={() => onViewModeChange('list')}
              >
                <List className="w-4 h-4" />
              </Button>
            </div>
          </div>
        </div>
        
        {/* Search - only show when browsing a category */}
        {selectedCategory && (
          <FeatSearchBar
            value={categorySearch}
            onChange={setCategorySearch}
            placeholder={`Search ${selectedSubcategory || selectedCategory} feats...`}
            showIcon={true}
            debounceMs={200}
          />
        )}
      </Card>

      {/* Main Content */}
      <Card className="flex-1" padding="p-0">
        <ScrollArea className="h-full p-4">
          {/* Error State */}
          {categoryFeatsError && (
            <div className="mb-4">
              <Card variant="error" padding="p-3">
                <div className="flex items-center gap-2">
                  <AlertCircle className="w-5 h-5 text-error" />
                  <p className="text-error text-sm">{categoryFeatsError}</p>
                </div>
              </Card>
            </div>
          )}

          <div className="space-y-6">
            {/* Available Category Feats Section */}
            {selectedCategory && (
              <div className="space-y-3">
                <div 
                  className="flex items-center justify-between cursor-pointer"
                  onClick={() => toggleSection('available')}
                >
                  <h3 className="text-lg font-semibold flex items-center gap-2">
                    {expandedSections.has('available') ? 
                      <ChevronDown className="w-4 h-4" /> : 
                      <ChevronRight className="w-4 h-4" />}
                    Available {selectedSubcategory || selectedCategory} Feats
                    <Badge variant="secondary">
                      {filteredCategoryFeats.length}
                    </Badge>
                  </h3>
                </div>
                
                {expandedSections.has('available') && (
                  <div className="space-y-4">
                    {categoryFeatsLoading ? (
                      <div className="flex items-center justify-center h-32">
                        <Loader2 className="w-6 h-6 animate-spin text-[rgb(var(--color-primary))]" />
                      </div>
                    ) : (
                      <>
                        {filteredCategoryFeats.length >= 20 ? (
                          <VirtualizedFeatSection
                            feats={filteredCategoryFeats}
                            isActive={false}
                            viewMode={viewMode}
                            maxHeight={500}
                            onDetails={onDetails}
                            onAdd={onAdd}
                            onRemove={onRemove}
                            validationCache={validationCache}
                            validatingFeatId={validatingFeatId}
                            onValidate={onValidate}
                          />
                        ) : filteredCategoryFeats.length > 0 ? (
                          <div className={viewMode === 'grid' ? 
                            'grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-3' : 
                            'space-y-2'}>
                            {filteredCategoryFeats.map((feat) => (
                              <FeatCard 
                                key={`available-${feat.id}`} 
                                feat={feat} 
                                isActive={false} 
                                viewMode={viewMode}
                                onDetails={onDetails}
                                onAdd={onAdd}
                                onRemove={onRemove}
                                validationState={validationCache[feat.id]}
                                isValidating={validatingFeatId === feat.id}
                                onValidate={onValidate}
                              />
                            ))}
                          </div>
                        ) : (
                          <div className="text-center py-8">
                            <p className="text-[rgb(var(--color-text-secondary))]">
                              {categorySearch ? 
                                `No feats found matching "${categorySearch}"` :
                                'No available feats in this category'}
                            </p>
                          </div>
                        )}
                      </>
                    )}
                  </div>
                )}
              </div>
            )}
            
            {/* No category selected message */}
            {!selectedCategory && (
              <div className="text-center py-8">
                <p className="text-[rgb(var(--color-text-secondary))]">
                  Select a category from the sidebar to browse available feats
                </p>
              </div>
            )}
          </div>
        </ScrollArea>
      </Card>
    </div>
  );
}