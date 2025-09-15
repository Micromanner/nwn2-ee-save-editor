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
  AlertCircle,
  Info, 
  Swords, 
  Sparkles, 
  Sun, 
  Zap, 
  Shield
} from 'lucide-react';
import GameDataList from '@/components/ui/GameDataList';
import type { GameDataItem } from '@/components/ui/GameDataList';
// Removed VirtualizedFeatSection - using GameDataList for all cases
import FeatSearchBar from './FeatSearchBar';
import { useFeatSearch } from '@/hooks/useFeatSearch';
import NWN2Icon from '@/components/ui/NWN2Icon';
import { display } from '@/utils/dataHelpers';
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

// Helper functions for feat rendering
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

const getTypeColor = (type: number) => {
  switch (type) {
    case 2: return 'destructive'; // Combat
    case 8: return 'secondary'; // Metamagic
    case 16: return 'default'; // Divine
    case 32: return 'outline'; // Epic
    case 64: return 'default'; // Class
    default: return 'default';
  }
};

// Convert FeatInfo to GameDataItem
const featToGameDataItem = (feat: FeatInfo): GameDataItem & FeatInfo => ({
  ...feat,
  name: feat.label,
  isActive: false, // Available feats are not active/learned yet
  icon: `ife_${feat.label.toLowerCase()}`,
});

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
    if (!featsData?.summary) return [];
    
    const allFeats = [
      ...(featsData.summary.protected || []),
      ...(featsData.summary.class_feats || []),
      ...(featsData.summary.general_feats || []),
      ...(featsData.summary.custom_feats || []),
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
      } else if (featsData.summary?.class_feats?.some((f: any) => f.id === feat.id)) {
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

  // Custom renderers for GameDataList
  const renderFeatMain = (item: GameDataItem & FeatInfo) => (
    <div className="game-data-col-main">
      <div className="game-data-icon-wrapper">
        <NWN2Icon icon={item.icon || ''} size="sm" />
      </div>
      <div className="game-data-content">
        <h4 className="game-data-title">
          {display(item.label)}
        </h4>
        <div className="game-data-subtitle">
          <span>{getFeatTypeName(item.type)}</span>
        </div>
      </div>
    </div>
  );

  const renderFeatTags = (item: GameDataItem & FeatInfo) => (
    <div className="flex flex-wrap gap-1">
      <Badge variant={getTypeColor(item.type)} className="text-xs">
        <span className="flex items-center gap-1">
          {getTypeIcon(item.type)}
          {getFeatTypeName(item.type)}
        </span>
      </Badge>
      {item.protected && (
        <Badge variant="outline" className="text-xs">
          <Shield className="w-3 h-3 mr-1" />
          Protected
        </Badge>
      )}
      {item.custom && (
        <Badge variant="secondary" className="text-xs">
          Custom
        </Badge>
      )}
    </div>
  );

  const renderAvailableFeatAction = (item: GameDataItem & FeatInfo) => {
    const validation = validationCache[item.id];
    const isValidating = validatingFeatId === item.id;
    
    return (
      <div className="game-data-col-action">
        <div className="flex items-center gap-2">
          <Button
            size="sm"
            variant="ghost"
            className="text-xs h-7 px-2"
            onClick={() => onDetails(item)}
          >
            <Info className="w-3 h-3" />
          </Button>
          <Button
            size="sm"
            variant="primary"
            className="text-xs h-7 px-2"
            onClick={() => onAdd(item.id)}
            disabled={validation ? !validation.can_take : false}
          >
            {isValidating ? <Loader2 className="w-3 h-3 animate-spin" /> : 'Learn'}
          </Button>
        </div>
      </div>
    );
  };

  const renderCurrentFeatAction = (item: GameDataItem & FeatInfo) => (
    <div className="game-data-col-action">
      <div className="flex items-center gap-2">
        <Button
          size="sm"
          variant="ghost"
          className="text-xs h-7 px-2"
          onClick={() => onDetails(item)}
        >
          <Info className="w-3 h-3" />
        </Button>
        <Button
          size="sm"
          variant="danger"
          className="text-xs h-7 px-2"
          onClick={() => onRemove(item.id)}
          disabled={item.protected}
        >
          Remove
        </Button>
      </div>
    </div>
  );

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
                        {filteredCategoryFeats.length > 0 ? (
                          <GameDataList
                            items={filteredCategoryFeats.map(featToGameDataItem)}
                            renderMain={renderFeatMain}
                            renderTags={renderFeatTags}
                            renderAction={renderAvailableFeatAction}
                            onItemAction={(item) => onAdd(item.id)}
                            actionLabel={() => 'Learn'}
                            actionVariant={() => 'primary'}
                            emptyMessage="No available feats found"
                          />
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