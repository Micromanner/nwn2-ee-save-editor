'use client';

import { useState, useMemo, useEffect, useCallback } from 'react';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { ScrollArea } from '@/components/ui/ScrollArea';
import { Badge } from '@/components/ui/Badge';
import { Grid, List, ChevronRight, ChevronDown, AlertCircle } from 'lucide-react';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { CharacterAPI } from '@/services/characterApi';
import FeatCard from './FeatCard';
import FeatDetailsPanel from './FeatDetailsPanel';
import FeatSummary from './FeatSummary';
import VirtualizedFeatSection from './VirtualizedFeatSection';
import { useIconPreloader } from '@/hooks/useIconPreloader';

interface Prerequisite {
  type: 'ability' | 'feat' | 'class' | 'level' | 'bab' | 'spell_level';
  description: string;
  required_value?: number;
  current_value?: number;
  feat_id?: number;
  class_id?: number;
  met: boolean;
}

interface DetailedPrerequisites {
  requirements: Prerequisite[];
  met: string[];
  unmet: string[];
}

interface FeatInfo {
  id: number;
  label: string;
  name: string;
  type: number;
  protected: boolean;
  custom: boolean;
  description?: string;
  icon?: string;
  prerequisites?: {
    abilities: Record<string, number>;
    feats: number[];
    class: number;
    level: number;
    bab: number;
    spell_level: number;
  };
  can_take?: boolean;
  missing_requirements?: string[];
  has_feat?: boolean;
  detailed_prerequisites?: DetailedPrerequisites;
}

interface FilterState {
  activeOnly: boolean;
}

interface CategoryInfo {
  id: string;
  name: string;
  count: number;
  subcategories?: { id: string; name: string; count: number }[];
}

// Mock categories based on NWN2 official structure
const FEAT_CATEGORIES: CategoryInfo[] = [
  { id: 'general', name: 'General', count: 0 },
  { id: 'combat', name: 'Combat', count: 0 },
  {
    id: 'class',
    name: 'Class',
    count: 0,
    subcategories: [
      { id: 'barbarian', name: 'Barbarian', count: 0 },
      { id: 'bard', name: 'Bard', count: 0 },
      { id: 'cleric', name: 'Cleric', count: 0 },
      { id: 'druid', name: 'Druid', count: 0 },
      { id: 'fighter', name: 'Fighter', count: 0 },
      { id: 'monk', name: 'Monk', count: 0 },
      { id: 'paladin', name: 'Paladin', count: 0 },
      { id: 'ranger', name: 'Ranger', count: 0 },
      { id: 'rogue', name: 'Rogue', count: 0 },
      { id: 'sorcerer', name: 'Sorcerer', count: 0 },
      { id: 'wizard', name: 'Wizard', count: 0 },
      // Add more as needed
    ],
  },
  {
    id: 'race',
    name: 'Race',
    count: 0,
    subcategories: [
      { id: 'human', name: 'Human', count: 0 },
      { id: 'elf', name: 'Elf [All]', count: 0 },
      { id: 'dwarf', name: 'Dwarf [All]', count: 0 },
      { id: 'halfling', name: 'Halfling [All]', count: 0 },
      // Add more as needed
    ],
  },
  { id: 'epic', name: 'Epic', count: 0 },
  { id: 'divine', name: 'Divine', count: 0 },
  { id: 'metamagic', name: 'Metamagic', count: 0 },
  { id: 'item_creation', name: 'Item Creation', count: 0 },
  { id: 'skills_saves', name: 'Skills & Saves', count: 0 },
  { id: 'spellcasting', name: 'Spellcasting', count: 0 },
];

interface FeatsState {
  current_feats: {
    total: number;
    protected: FeatInfo[];
    class_feats: FeatInfo[];
    general_feats: FeatInfo[];
    custom_feats: FeatInfo[];
  };
  available_feats?: FeatInfo[]; // Optional - no longer loaded from /feats/state/
}

export default function FeatsEditor() {
  const { character, isLoading: characterLoading, error: characterError } = useCharacterContext();
  const feats = useSubsystem('feats');
  
  // State for category-based feat loading
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [selectedSubcategory, setSelectedSubcategory] = useState<string | null>(null);
  const [categoryFeats, setCategoryFeats] = useState<FeatInfo[]>([]);
  const [categoryFeatsLoading, setCategoryFeatsLoading] = useState(false);
  const [categoryFeatsError, setCategoryFeatsError] = useState<string | null>(null);
  const [categorySearch, setCategorySearch] = useState('');
  const [onlyAvailable, setOnlyAvailable] = useState(true); // Show only obtainable feats by default

  // Load feats data only if character exists and data hasn't been loaded
  useEffect(() => {
    if (character && !feats.data && !feats.isLoading) {
      feats.load();
    }
  }, [character, feats.data, feats.isLoading]);
  
  // Auto-expand the CategoryFeats section when a category is selected
  useEffect(() => {
    if (selectedCategory) {
      setExpandedCategories(prev => new Set([...prev, 'CategoryFeats']));
    }
  }, [selectedCategory]);

  // Load category feats when category, subcategory, or availability changes
  useEffect(() => {
    const loadCategoryFeats = async () => {
      if (!character?.id || !selectedCategory) {
        setCategoryFeats([]);
        return;
      }
      
      setCategoryFeatsLoading(true);
      setCategoryFeatsError(null);
      
      try {
        // Use real API with category-based loading
        const response = await CharacterAPI.getLegitimateFeats(character.id, {
          category: selectedCategory,
          subcategory: selectedSubcategory || undefined,
          onlyAvailable: onlyAvailable,
          page: 1,
          limit: 100 // Load more initially for better UX
        });
        
        setCategoryFeats(response.feats);
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Failed to load category feats';
        setCategoryFeatsError(errorMessage);
        console.error('Failed to load category feats:', error);
      } finally {
        setCategoryFeatsLoading(false);
      }
    };

    loadCategoryFeats();
  }, [character?.id, selectedCategory, selectedSubcategory, onlyAvailable]);

  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');
  const [expandedCategories, setExpandedCategories] = useState<Set<string>>(new Set());
  const [selectedFeat, setSelectedFeat] = useState<FeatInfo | null>(null);
  const [featDetails, setFeatDetails] = useState<FeatInfo | null>(null);
  const [loadingDetails, setLoadingDetails] = useState(false);
  
  // Get feats data from subsystem
  const featsData = feats.data as FeatsState | null;
  const isLoading = characterLoading || feats.isLoading || categoryFeatsLoading;
  const error = characterError || feats.error || categoryFeatsError;
  
  const [filters, setFilters] = useState<FilterState>({
    activeOnly: false,
  });

  // Memoized callbacks to prevent re-renders
  const handleCategorySelect = useCallback((categoryId: string) => {
    setSelectedCategory(categoryId);
    setSelectedSubcategory(null);
    setCategorySearch('');
  }, []);

  const handleSubcategorySelect = useCallback((subcategoryId: string) => {
    setSelectedSubcategory(subcategoryId);
    setCategorySearch('');
  }, []);

  const handleCategorySearch = useCallback((search: string) => {
    setCategorySearch(search);
  }, []);

  const loadFeatDetails = useCallback(async (feat: FeatInfo) => {
    if (!character?.id) return;
    
    setSelectedFeat(feat);
    setLoadingDetails(true);
    
    try {
      const details = await CharacterAPI.getFeatDetails(character.id, feat.id);
      setFeatDetails(details);
    } catch (error) {
      console.error('Failed to load feat details:', error);
      setFeatDetails(null);
    } finally {
      setLoadingDetails(false);
    }
  }, [character?.id]);

  const handleAddFeat = useCallback(async (featId: number) => {
    if (!character?.id) return;
    
    try {
      await CharacterAPI.addFeat(character.id, featId);
      // Refresh current feats
      await feats.load(true);
    } catch (error) {
      console.error('Failed to add feat:', error);
      // You might want to show a toast notification here
    }
  }, [character?.id, feats]);

  const handleRemoveFeat = useCallback(async (featId: number) => {
    if (!character?.id) return;
    
    try {
      await CharacterAPI.removeFeat(character.id, featId);
      // Refresh current feats
      await feats.load(true);
    } catch (error) {
      console.error('Failed to remove feat:', error);
      // You might want to show a toast notification here
    }
  }, [character?.id, feats]);

  // Combine all feats for display with deduplication
  const allCurrentFeats = useMemo(() => {
    if (!featsData) return [];
    
    const allFeats = [
      ...featsData.current_feats.protected,
      ...featsData.current_feats.class_feats,
      ...featsData.current_feats.general_feats,
      ...featsData.current_feats.custom_feats,
    ];
    
    // Deduplicate by feat ID
    const uniqueFeats = new Map();
    allFeats.forEach(feat => {
      uniqueFeats.set(feat.id, feat);
    });
    
    return Array.from(uniqueFeats.values());
  }, [featsData]);


  // Show current feats (no complex filtering needed with categories)
  const filteredCurrentFeats = allCurrentFeats;

  const filteredCategoryFeats = useMemo(() => {
    // Filter out feats the character already has
    const currentFeatIds = new Set(allCurrentFeats.map(f => f.id));
    const availableFeats = categoryFeats.filter(feat => !currentFeatIds.has(feat.id));
    
    return availableFeats.filter(feat => {
      // Category search filter only
      if (categorySearch && 
          !feat.label.toLowerCase().includes(categorySearch.toLowerCase()) &&
          !feat.name.toLowerCase().includes(categorySearch.toLowerCase())) {
        return false;
      }
      return true;
    });
  }, [categoryFeats, allCurrentFeats, categorySearch]);

  // Group feats by category for display with mutually exclusive categories
  const groupedCurrentFeats = useMemo(() => {
    if (!featsData) return {};
    
    const groups: Record<string, FeatInfo[]> = {
      'Protected': [],
      'Class': [],
      'General': [],
      'Custom': [],
    };
    
    // Categorize each feat with priority order to avoid duplicates
    for (const feat of filteredCurrentFeats) {
      if (feat.protected) {
        groups['Protected'].push(feat);
      } else if (feat.custom) {
        groups['Custom'].push(feat);
      } else if (featsData.current_feats.class_feats.includes(feat)) {
        groups['Class'].push(feat);
      } else if (featsData.current_feats.general_feats.includes(feat)) {
        groups['General'].push(feat);
      } else {
        // Default to General if not categorized elsewhere
        groups['General'].push(feat);
      }
    }
    
    // Remove empty groups
    return Object.fromEntries(
      Object.entries(groups).filter(([, feats]) => feats.length > 0)
    );
  }, [filteredCurrentFeats, featsData]);

  // Preload icons based on virtualization strategy
  const visibleFeatIcons = useMemo(() => {
    const visibleFeats: FeatInfo[] = [];
    
    // For current feats - preload first 10 items from each category (virtualized or not)
    Object.entries(groupedCurrentFeats).forEach(([, categoryFeats]) => {
      visibleFeats.push(...categoryFeats.slice(0, 10));
    });
    
    // For category feats - only preload if list is small or first 10 for large lists
    if (filteredCategoryFeats.length < 20) {
      // Small list - preload all
      visibleFeats.push(...filteredCategoryFeats);
    } else {
      // Large list (virtualized) - only preload first 10
      visibleFeats.push(...filteredCategoryFeats.slice(0, 10));
    }
    
    return visibleFeats.map(feat => `ife_${feat.label.toLowerCase()}`);
  }, [groupedCurrentFeats, filteredCategoryFeats]);

  // Preload icons only for visible feats
  useIconPreloader(visibleFeatIcons, {
    enabled: true,
    batchSize: 10,
    delay: 100,
  });

  // Early return for loading/error states
  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-[rgb(var(--color-primary))]"></div>
      </div>
    );
  }

  if (error) {
    return (
      <Card variant="error">
        <div className="flex items-center gap-2">
          <AlertCircle className="w-5 h-5 text-error" />
          <p className="text-error">{error}</p>
        </div>
      </Card>
    );
  }

  if (!character || !featsData) {
    return (
      <Card variant="warning">
        <p className="text-muted">No character loaded. Please import a save file to begin.</p>
      </Card>
    );
  }

  const toggleCategory = (category: string) => {
    const newExpanded = new Set(expandedCategories);
    if (newExpanded.has(category)) {
      newExpanded.delete(category);
    } else {
      newExpanded.add(category);
    }
    setExpandedCategories(newExpanded);
  };




  return (
    <div className="flex gap-4">
      {/* Category Sidebar */}
      <div className="w-64 flex flex-col gap-4">
        <FeatSummary 
          featsData={featsData} 
          availableFeatsCount={filteredCategoryFeats.length} 
        />
        
        {/* Category Selection */}
        <Card padding="p-4">
          <h3 className="text-sm font-semibold mb-3">Browse by Category</h3>
          <div className="space-y-2">
            {FEAT_CATEGORIES.map((category) => (
              <div key={category.id}>
                <Button
                  variant={selectedCategory === category.id ? 'primary' : 'ghost'}
                  size="sm"
                  className="w-full justify-start text-left"
                  onClick={() => handleCategorySelect(category.id)}
                >
                  {category.name}
                  {selectedCategory === category.id && category.subcategories && (
                    <ChevronDown className="w-4 h-4 ml-auto" />
                  )}
                  {selectedCategory !== category.id && category.subcategories && (
                    <ChevronRight className="w-4 h-4 ml-auto" />
                  )}
                </Button>
                
                {/* Subcategories */}
                {selectedCategory === category.id && category.subcategories && (
                  <div className="ml-4 mt-2 space-y-1">
                    {category.subcategories.map((subcategory) => (
                      <Button
                        key={subcategory.id}
                        variant={selectedSubcategory === subcategory.id ? 'primary' : 'ghost'}
                        size="sm"
                        className="w-full justify-start text-xs"
                        onClick={() => handleSubcategorySelect(subcategory.id)}
                      >
                        {subcategory.name}
                      </Button>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        </Card>
      </div>

      {/* Main Content */}
      <div className="flex-1 flex flex-col gap-4">
        {/* Header with Search and Controls */}
        <Card padding="p-4">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-4">
              <h2 className="text-lg font-semibold">
                {selectedCategory ? `${selectedSubcategory || selectedCategory} Feats` : 'Character Feats'}
              </h2>
              <div className="text-sm text-[rgb(var(--color-text-secondary))]">
                {selectedCategory ? `${filteredCategoryFeats.length} available` : `${filteredCurrentFeats.length} current`}
              </div>
            </div>
            <div className="flex items-center gap-3">
              {/* Show Only Obtainable Feats Toggle */}
              {selectedCategory && (
                <Button
                  variant={onlyAvailable ? 'primary' : 'ghost'}
                  size="sm"
                  onClick={() => setOnlyAvailable(!onlyAvailable)}
                  className="text-xs"
                  title={onlyAvailable ? 'Showing only feats you can take' : 'Showing all feats in category'}
                >
                  {onlyAvailable ? 'Obtainable Only' : 'Show All'}
                </Button>
              )}
              
              {/* Current Feats Only Toggle */}
              {!selectedCategory && (
                <Button
                  variant={filters.activeOnly ? 'primary' : 'ghost'}
                  size="sm"
                  onClick={() => setFilters(prev => ({ ...prev, activeOnly: !prev.activeOnly }))}
                  className="text-xs"
                >
                  Current Only
                </Button>
              )}
              
              {/* View Mode Toggle */}
              <div className="flex rounded-md">
                <Button
                  variant={viewMode === 'grid' ? 'primary' : 'ghost'}
                  size="sm"
                  className="rounded-r-none"
                  onClick={() => setViewMode('grid')}
                >
                  <Grid className="w-4 h-4" />
                </Button>
                <Button
                  variant={viewMode === 'list' ? 'primary' : 'ghost'}
                  size="sm"
                  className="rounded-l-none"
                  onClick={() => setViewMode('list')}
                >
                  <List className="w-4 h-4" />
                </Button>
              </div>
            </div>
          </div>
          
          {/* Search - only show when browsing a category */}
          {selectedCategory && (
            <div className="flex items-center gap-2">
              <input
                type="text"
                placeholder={`Search ${selectedSubcategory || selectedCategory} feats...`}
                value={categorySearch}
                onChange={(e) => handleCategorySearch(e.target.value)}
                className="flex-1 px-3 py-2 border border-[rgb(var(--color-border))] rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-[rgb(var(--color-primary))]/20 focus:border-[rgb(var(--color-primary))]"
              />
              {categorySearch && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => handleCategorySearch('')}
                  className="text-xs"
                >
                  Clear
                </Button>
              )}
            </div>
          )}
        </Card>

        {/* Current Feats */}
        <Card className="flex-1" padding="p-0">
          <ScrollArea className="h-full p-4">
            <div className="space-y-6">
              {/* Current Feats Section - only show when no category is browsing */}
              {!selectedCategory && Object.entries(groupedCurrentFeats).map(([category, categoryFeats]) => (
                <div key={category} className="space-y-3">
                  <div 
                    className="flex items-center justify-between cursor-pointer"
                    onClick={() => toggleCategory(category)}
                  >
                    <h3 className="text-lg font-semibold text-[rgb(var(--color-text-primary))] flex items-center gap-2">
                      {expandedCategories.has(category) ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
                      {category}
                      <Badge variant="secondary">
                        {categoryFeats.length}
                      </Badge>
                    </h3>
                  </div>
                  
                  {expandedCategories.has(category) && (
                    <>
                      {categoryFeats.length >= 20 ? (
                        <VirtualizedFeatSection
                          feats={categoryFeats}
                          isActive={true}
                          viewMode={viewMode}
                          maxHeight={400}
                          onDetails={loadFeatDetails}
                          onAdd={handleAddFeat}
                          onRemove={handleRemoveFeat}
                        />
                      ) : (
                        <div className={viewMode === 'grid' ? 'grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-3' : 'space-y-2'}>
                          {categoryFeats.map((feat, index) => (
                            <FeatCard 
                              key={`current-${feat.id}-${index}`} 
                              feat={feat} 
                              isActive={true} 
                              viewMode={viewMode}
                              onDetails={loadFeatDetails}
                              onAdd={handleAddFeat}
                              onRemove={handleRemoveFeat}
                            />
                          ))}
                        </div>
                      )}
                    </>
                  )}
                </div>
              ))}
              
              {/* Show current feats in a different way when browsing categories */}
              {selectedCategory && (
                <div className="space-y-3">
                  <h3 className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">Current Feats</h3>
                  <div className={viewMode === 'grid' ? 'grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-3' : 'space-y-2'}>
                    {allCurrentFeats.slice(0, 6).map((feat, index) => (
                      <FeatCard
                        key={`current-compact-${feat.id}-${index}`}
                        feat={feat}
                        isActive={true}
                        viewMode={viewMode}
                        onDetails={loadFeatDetails}
                        onAdd={handleAddFeat}
                        onRemove={handleRemoveFeat}
                      />
                    ))}
                  </div>
                  {allCurrentFeats.length > 6 && (
                    <p className="text-sm text-[rgb(var(--color-text-secondary))] text-center">
                      and {allCurrentFeats.length - 6} more...
                    </p>
                  )}
                </div>
              )}
              
              {/* Category Feats Section - only show when a category is selected */}
              {selectedCategory && filteredCategoryFeats.length > 0 && !filters.activeOnly && (
                <div className="space-y-3">
                  
                  <div 
                    className="flex items-center justify-between cursor-pointer"
                    onClick={() => toggleCategory('CategoryFeats')}
                  >
                    <h3 className="text-lg font-semibold text-[rgb(var(--color-text-primary))] flex items-center gap-2">
                      {expandedCategories.has('CategoryFeats') ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
                      {selectedSubcategory || selectedCategory} Feats
                      <Badge variant="secondary">
                        {filteredCategoryFeats.length}
                      </Badge>
                    </h3>
                  </div>
                  
                  {expandedCategories.has('CategoryFeats') && (
                    <div className="space-y-4">
                      {categoryFeatsLoading ? (
                        <div className="flex items-center justify-center h-32">
                          <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-[rgb(var(--color-primary))]"></div>
                        </div>
                      ) : (
                        <>
                          {filteredCategoryFeats.length >= 20 ? (
                            <VirtualizedFeatSection
                              feats={filteredCategoryFeats}
                              isActive={false}
                              viewMode={viewMode}
                              maxHeight={500}
                              onDetails={loadFeatDetails}
                              onAdd={handleAddFeat}
                              onRemove={handleRemoveFeat}
                            />
                          ) : (
                            <div className={viewMode === 'grid' ? 'grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-3' : 'space-y-2'}>
                              {filteredCategoryFeats.map((feat, index) => (
                                <FeatCard 
                                  key={`category-${feat.id}-${index}`} 
                                  feat={feat} 
                                  isActive={false} 
                                  viewMode={viewMode}
                                  onDetails={loadFeatDetails}
                                  onAdd={handleAddFeat}
                                  onRemove={handleRemoveFeat}
                                />
                              ))}
                            </div>
                          )}
                        </>
                      )}
                      
                      {/* Category Info */}
                      {filteredCategoryFeats.length > 0 && (
                        <div className="text-center text-sm text-[rgb(var(--color-text-secondary))]">
                          {categorySearch ? `Showing ${filteredCategoryFeats.length} matching feats` : `${filteredCategoryFeats.length} feats in this category`}
                        </div>
                      )}
                    </div>
                  )}
                </div>
              )}
              
              {/* No category selected message */}
              {!selectedCategory && !filters.activeOnly && (
                <div className="text-center py-8">
                  <p className="text-[rgb(var(--color-text-secondary))]">
                    Select a category from the sidebar to browse available feats
                  </p>
                </div>
              )}
              
              {/* No results when searching */}
              {selectedCategory && filteredCategoryFeats.length === 0 && categorySearch && (
                <div className="text-center py-8">
                  <p className="text-[rgb(var(--color-text-secondary))]">
                    No feats found matching &quot;{categorySearch}&quot;
                  </p>
                </div>
              )}
            </div>
          </ScrollArea>
        </Card>
      </div>

      {/* Feat Details Panel */}
      {selectedFeat && (
        <FeatDetailsPanel
          selectedFeat={selectedFeat}
          featDetails={featDetails}
          loadingDetails={loadingDetails}
          onClose={() => {
            setSelectedFeat(null);
            setFeatDetails(null);
          }}
          onAdd={handleAddFeat}
          onRemove={handleRemoveFeat}
        />
      )}
    </div>
  );
}