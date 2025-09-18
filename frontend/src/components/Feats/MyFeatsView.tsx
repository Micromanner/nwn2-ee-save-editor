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
  Shield,
  BookOpen,
  Sparkles,
  Package,
  Info, 
  Swords, 
  Sun, 
  Zap
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

interface MyFeatsViewProps extends FeatManagementCallbacks {
  featsData: FeatsState;
  viewMode: ViewMode;
  onViewModeChange: (mode: ViewMode) => void;
  validationCache?: ValidationCache;
  validatingFeatId?: number | null;
}

type SortOption = 'name' | 'type' | 'level' | 'recent';
type FilterOption = 'all' | 'protected' | 'class' | 'general' | 'custom';

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
  isActive: true, // All feats in MyFeatsView are active/learned
  icon: `ife_${feat.label.toLowerCase()}`,
});

export default function MyFeatsView({
  featsData,
  viewMode,
  onViewModeChange,
  onDetails,
  // onAdd prop removed as unused
  onRemove,
  // onValidate prop removed as unused
  // validationCache prop removed as unused
  // validatingFeatId prop removed as unused
}: MyFeatsViewProps) {
  const [expandedSections, setExpandedSections] = useState<Set<string>>(
    new Set(['protected', 'class', 'general'])
  );
  const [searchTerm, setSearchTerm] = useState('');
  const [sortBy, setSortBy] = useState<SortOption>('name');
  const [filterBy, setFilterBy] = useState<FilterOption>('all');

  // Combine all feats with deduplication
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

  // Filter feats based on filter option
  const filteredFeats = useMemo(() => {
    if (!featsData?.summary) return [];
    
    switch (filterBy) {
      case 'protected':
        return featsData.summary.protected || [];
      case 'class':
        return featsData.summary.class_feats || [];
      case 'general':
        return featsData.summary.general_feats || [];
      case 'custom':
        return featsData.summary.custom_feats || [];
      default:
        return allCurrentFeats;
    }
  }, [filterBy, featsData, allCurrentFeats]);

  // Use search hook for filtering
  const { searchResults } = useFeatSearch(
    filteredFeats,
    searchTerm,
    { 
      keys: ['label', 'name', 'description'],
      threshold: 0.3
    }
  );

  // Sort feats
  const sortedFeats = useMemo(() => {
    const featsToSort = [...searchResults];
    
    switch (sortBy) {
      case 'name':
        return featsToSort.sort((a, b) => a.label.localeCompare(b.label));
      case 'type':
        return featsToSort.sort((a, b) => a.type - b.type);
      case 'level':
        // Sort by prerequisites if available
        return featsToSort.sort((a, b) => {
          const aLevel = typeof a.prerequisites?.level === 'number' ? a.prerequisites.level : 0;
          const bLevel = typeof b.prerequisites?.level === 'number' ? b.prerequisites.level : 0;
          return aLevel - bLevel;
        });
      case 'recent':
        // For now, just reverse the order (newest first)
        return featsToSort.reverse();
      default:
        return featsToSort;
    }
  }, [searchResults, sortBy]);

  // Group feats by category for organized display
  const groupedFeats = useMemo(() => {
    const groups: Record<string, FeatInfo[]> = {
      'Protected': [],
      'Class': [],
      'General': [],
      'Custom': [],
    };
    
    for (const feat of sortedFeats) {
      if (feat.protected) {
        groups['Protected'].push(feat);
      } else if (feat.custom) {
        groups['Custom'].push(feat);
      } else if (featsData.summary?.class_feats?.some(f => f.id === feat.id)) {
        groups['Class'].push(feat);
      } else {
        groups['General'].push(feat);
      }
    }
    
    // Remove empty groups
    return Object.fromEntries(
      Object.entries(groups).filter(([, feats]) => feats.length > 0)
    );
  }, [sortedFeats, featsData]);

  const toggleSection = (section: string) => {
    const newExpanded = new Set(expandedSections);
    if (newExpanded.has(section)) {
      newExpanded.delete(section);
    } else {
      newExpanded.add(section);
    }
    setExpandedSections(newExpanded);
  };

  const getSectionIcon = (section: string) => {
    switch (section) {
      case 'Protected':
        return <Shield className="w-4 h-4" />;
      case 'Class':
        return <BookOpen className="w-4 h-4" />;
      case 'General':
        return <Package className="w-4 h-4" />;
      case 'Custom':
        return <Sparkles className="w-4 h-4" />;
      default:
        return null;
    }
  };

  const getSectionColor = (section: string) => {
    switch (section) {
      case 'Protected':
        return 'text-amber-500';
      case 'Class':
        return 'text-blue-500';
      case 'General':
        return 'text-green-500';
      case 'Custom':
        return 'text-purple-500';
      default:
        return '';
    }
  };

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

  const renderFeatAction = (item: GameDataItem & FeatInfo) => (
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

  return (
    <div className="flex flex-col gap-4 h-full">
      {/* Header with Controls */}
      <Card padding="p-4">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h2 className="text-lg font-semibold">My Feats</h2>
            <p className="text-sm text-[rgb(var(--color-text-secondary))] mt-1">
              {featsData.summary?.total || 0} acquired feats
            </p>
          </div>
          
          <div className="flex items-center gap-3">
            {/* Filter Dropdown */}
            <select
              value={filterBy}
              onChange={(e) => setFilterBy(e.target.value as FilterOption)}
              className="px-3 py-1.5 text-xs border border-[rgb(var(--color-border))] rounded-md bg-[rgb(var(--color-surface-1))] focus:outline-none focus:ring-2 focus:ring-[rgb(var(--color-primary))]/20"
            >
              <option value="all">All Feats</option>
              <option value="protected">Protected Only</option>
              <option value="class">Class Feats</option>
              <option value="general">General Feats</option>
              <option value="custom">Custom Content</option>
            </select>

            {/* Sort Dropdown */}
            <select
              value={sortBy}
              onChange={(e) => setSortBy(e.target.value as SortOption)}
              className="px-3 py-1.5 text-xs border border-[rgb(var(--color-border))] rounded-md bg-[rgb(var(--color-surface-1))] focus:outline-none focus:ring-2 focus:ring-[rgb(var(--color-primary))]/20"
            >
              <option value="name">Sort by Name</option>
              <option value="type">Sort by Type</option>
              <option value="level">Sort by Level</option>
              <option value="recent">Recently Added</option>
            </select>

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

        {/* Search Bar */}
        <FeatSearchBar
          value={searchTerm}
          onChange={setSearchTerm}
          placeholder="Search your feats..."
          showIcon={true}
          debounceMs={200}
        />
      </Card>

      {/* Feats Display */}
      <Card className="flex-1" padding="p-0">
        <ScrollArea className="h-full p-4">
          <div className="space-y-6">
            {/* Show filtered view when filter is active */}
            {filterBy !== 'all' ? (
              <div className="space-y-3">
                <h3 className="text-lg font-semibold flex items-center gap-2">
                  {getSectionIcon(filterBy === 'protected' ? 'Protected' : 
                                   filterBy === 'class' ? 'Class' :
                                   filterBy === 'general' ? 'General' : 'Custom')}
                  <span>Filtered Results</span>
                  <Badge variant="secondary">{sortedFeats.length}</Badge>
                </h3>
                
                <GameDataList
                  items={sortedFeats.map(featToGameDataItem)}
                  renderMain={renderFeatMain}
                  renderTags={renderFeatTags}
                  renderAction={renderFeatAction}
                  onItemAction={(item) => onRemove(item.id)}
                  actionLabel={() => 'Remove'}
                  actionVariant={() => 'primary'}
                  emptyMessage="No feats found in this filter"
                />
              </div>
            ) : (
              // Show grouped view when no filter
              Object.entries(groupedFeats).map(([section, sectionFeats]) => (
                <div key={section} className="space-y-3">
                  <div 
                    className="flex items-center justify-between cursor-pointer group"
                    onClick={() => toggleSection(section)}
                  >
                    <h3 className="text-lg font-semibold flex items-center gap-2">
                      {expandedSections.has(section) ? 
                        <ChevronDown className="w-4 h-4" /> : 
                        <ChevronRight className="w-4 h-4" />
                      }
                      <span className={getSectionColor(section)}>
                        {getSectionIcon(section)}
                      </span>
                      <span>{section} Feats</span>
                      <Badge variant="secondary">{sectionFeats.length}</Badge>
                    </h3>
                  </div>
                  
                  {expandedSections.has(section) && (
                    <GameDataList
                      items={sectionFeats.map(featToGameDataItem)}
                      renderMain={renderFeatMain}
                      renderTags={renderFeatTags}
                      renderAction={renderFeatAction}
                      onItemAction={(item) => onRemove(item.id)}
                      actionLabel={() => 'Remove'}
                      actionVariant={() => 'primary'}
                      emptyMessage={`No ${section.toLowerCase()} feats found`}
                    />
                  )}
                </div>
              ))
            )}

            {/* No results message */}
            {sortedFeats.length === 0 && (
              <div className="text-center py-8">
                <p className="text-[rgb(var(--color-text-secondary))]">
                  {searchTerm ? 
                    `No feats found matching "${searchTerm}"` : 
                    'No feats in this category'}
                </p>
              </div>
            )}
          </div>
        </ScrollArea>
      </Card>
    </div>
  );
}