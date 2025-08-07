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
  Package
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

interface MyFeatsViewProps extends FeatManagementCallbacks {
  featsData: FeatsState;
  viewMode: ViewMode;
  onViewModeChange: (mode: ViewMode) => void;
  validationCache?: ValidationCache;
  validatingFeatId?: number | null;
}

type SortOption = 'name' | 'type' | 'level' | 'recent';
type FilterOption = 'all' | 'protected' | 'class' | 'general' | 'custom';

export default function MyFeatsView({
  featsData,
  viewMode,
  onViewModeChange,
  onDetails,
  onAdd,
  onRemove,
  onValidate,
  validationCache = {},
  validatingFeatId = null,
}: MyFeatsViewProps) {
  const [expandedSections, setExpandedSections] = useState<Set<string>>(
    new Set(['protected', 'class', 'general'])
  );
  const [searchTerm, setSearchTerm] = useState('');
  const [sortBy, setSortBy] = useState<SortOption>('name');
  const [filterBy, setFilterBy] = useState<FilterOption>('all');

  // Combine all feats with deduplication
  const allCurrentFeats = useMemo(() => {
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

  // Filter feats based on filter option
  const filteredFeats = useMemo(() => {
    switch (filterBy) {
      case 'protected':
        return featsData.current_feats.protected;
      case 'class':
        return featsData.current_feats.class_feats;
      case 'general':
        return featsData.current_feats.general_feats;
      case 'custom':
        return featsData.current_feats.custom_feats;
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
          const aLevel = a.prerequisites?.level || 0;
          const bLevel = b.prerequisites?.level || 0;
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

  return (
    <div className="flex flex-col gap-4 h-full">
      {/* Header with Controls */}
      <Card padding="p-4">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h2 className="text-lg font-semibold">My Feats</h2>
            <p className="text-sm text-[rgb(var(--color-text-secondary))] mt-1">
              {featsData.current_feats.total} acquired feats
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
                
                {sortedFeats.length >= 20 ? (
                  <VirtualizedFeatSection
                    feats={sortedFeats}
                    isActive={true}
                    viewMode={viewMode}
                    maxHeight={500}
                    onDetails={onDetails}
                    onAdd={onAdd}
                    onRemove={onRemove}
                    validationCache={validationCache}
                    validatingFeatId={validatingFeatId}
                    onValidate={onValidate}
                  />
                ) : (
                  <div className={viewMode === 'grid' ? 
                    'grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-3' : 
                    'space-y-2'
                  }>
                    {sortedFeats.map((feat) => (
                      <FeatCard
                        key={feat.id}
                        feat={feat}
                        isActive={true}
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
                )}
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
                    <>
                      {sectionFeats.length >= 20 ? (
                        <VirtualizedFeatSection
                          feats={sectionFeats}
                          isActive={true}
                          viewMode={viewMode}
                          maxHeight={400}
                          onDetails={onDetails}
                          onAdd={onAdd}
                          onRemove={onRemove}
                          validationCache={validationCache}
                          validatingFeatId={validatingFeatId}
                          onValidate={onValidate}
                        />
                      ) : (
                        <div className={viewMode === 'grid' ? 
                          'grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-3' : 
                          'space-y-2'
                        }>
                          {sectionFeats.map((feat) => (
                            <FeatCard
                              key={`${section}-${feat.id}`}
                              feat={feat}
                              isActive={true}
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
                      )}
                    </>
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