'use client';

import { useState, useMemo, useEffect, useCallback } from 'react';
import { Card } from '@/components/ui/Card';
import { AlertCircle } from 'lucide-react';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { CharacterAPI } from '@/services/characterApi';
import { useFeatNavigation } from '@/hooks/useFeatManagement';
import { useIconPreloader } from '@/hooks/useIconPreloader';

// Import new components
import FeatCategorySidebar from './FeatCategorySidebar';
import FeatBreadcrumbs from './FeatBreadcrumbs';
import FeatContentArea from './FeatContentArea';
import MyFeatsView from './MyFeatsView';
import FeatDetailsPanel from './FeatDetailsPanel';

// Import types
import type { 
  FeatInfo, 
  FeatsState, 
  ViewMode,
  ValidationState
} from './types';
import { FEAT_CATEGORIES as CATEGORIES } from './types';

export default function FeatsEditor() {
  const { character, isLoading: characterLoading, error: characterError } = useCharacterContext();
  const feats = useSubsystem('feats');
  
  // Use custom hooks for feat management
  const navigation = useFeatNavigation();
  const [viewMode, setViewMode] = useState<ViewMode>('grid');
  const [showMyFeats, setShowMyFeats] = useState(true);
  const [globalSearch, setGlobalSearch] = useState('');
  
  // State for category-based feat loading
  const [categoryFeats, setCategoryFeats] = useState<FeatInfo[]>([]);
  const [categoryFeatsLoading, setCategoryFeatsLoading] = useState(false);
  const [categoryFeatsError, setCategoryFeatsError] = useState<string | null>(null);
  
  // State for feat details and validation
  const [selectedFeat, setSelectedFeat] = useState<FeatInfo | null>(null);
  const [featDetails, setFeatDetails] = useState<FeatInfo | null>(null);
  const [loadingDetails, setLoadingDetails] = useState(false);
  
  // State for on-demand feat validation
  const [validationCache, setValidationCache] = useState<Record<number, ValidationState>>({});
  const [validatingFeatId, setValidatingFeatId] = useState<number | null>(null);
  
  // Get feats data from subsystem
  const featsData = feats.data as FeatsState | null;
  const isLoading = characterLoading || feats.isLoading || categoryFeatsLoading;
  const error = characterError || feats.error || categoryFeatsError;

  // Load feats data only if character exists and data hasn't been loaded
  useEffect(() => {
    if (character && !feats.data && !feats.isLoading) {
      feats.load();
    }
  }, [character, feats]);

  // Load category feats when category or subcategory changes
  useEffect(() => {
    const loadCategoryFeats = async () => {
      if (!character?.id || !navigation.selectedCategory) {
        setCategoryFeats([]);
        return;
      }
      
      setCategoryFeatsLoading(true);
      setCategoryFeatsError(null);
      
      try {
        const response = await CharacterAPI.getLegitimateFeats(character.id, {
          category: navigation.selectedCategory,
          subcategory: navigation.selectedSubcategory || undefined,
          page: 1,
          limit: 500 // Load more feats since search is client-side now
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
  }, [character?.id, navigation.selectedCategory, navigation.selectedSubcategory]);

  // Callbacks for feat management
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

  const validateFeat = useCallback(async (featId: number) => {
    if (!character?.id) return null;
    
    // Check cache first
    if (validationCache[featId]) {
      return validationCache[featId];
    }
    
    // Mark as validating
    setValidatingFeatId(featId);
    
    try {
      const validation = await CharacterAPI.validateFeat(character.id, featId);
      
      // Cache the result
      setValidationCache(prev => ({
        ...prev,
        [featId]: validation
      }));
      
      return validation;
    } catch (error) {
      console.error('Failed to validate feat:', error);
      return null;
    } finally {
      setValidatingFeatId(null);
    }
  }, [character?.id, validationCache]);

  const handleAddFeat = useCallback(async (featId: number) => {
    if (!character?.id) return;
    
    try {
      await CharacterAPI.addFeat(character.id, featId);
      // Refresh current feats and clear validation cache for this feat
      await feats.load(true);
      setValidationCache(prev => {
        const newCache = { ...prev };
        delete newCache[featId];
        return newCache;
      });
    } catch (error) {
      console.error('Failed to add feat:', error);
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
    }
  }, [character?.id, feats]);

  // Combine all feats for available count calculation
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

  // Calculate available feats count
  const availableFeatsCount = useMemo(() => {
    const currentFeatIds = new Set(allCurrentFeats.map(f => f.id));
    return categoryFeats.filter(feat => !currentFeatIds.has(feat.id)).length;
  }, [categoryFeats, allCurrentFeats]);

  // Preload icons for visible feats
  const visibleFeatIcons = useMemo(() => {
    const icons: string[] = [];
    
    // Add icons from current feats (first 20)
    allCurrentFeats.slice(0, 20).forEach(feat => {
      icons.push(`ife_${feat.label.toLowerCase()}`);
    });
    
    // Add icons from category feats (first 20)
    categoryFeats.slice(0, 20).forEach(feat => {
      icons.push(`ife_${feat.label.toLowerCase()}`);
    });
    
    return icons;
  }, [allCurrentFeats, categoryFeats]);

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

  return (
    <div className="flex flex-col gap-4 h-full">
      {/* Breadcrumbs */}
      <FeatBreadcrumbs
        category={navigation.selectedCategory}
        subcategory={navigation.selectedSubcategory}
        categories={CATEGORIES}
        onNavigate={navigation.navigateTo}
        totalFeats={showMyFeats ? allCurrentFeats.length : availableFeatsCount}
      />

      <div className="flex gap-4 flex-1">
        {/* Sidebar */}
        <FeatCategorySidebar
          categories={CATEGORIES}
          selectedCategory={navigation.selectedCategory}
          selectedSubcategory={navigation.selectedSubcategory}
          onCategorySelect={navigation.setSelectedCategory}
          onSubcategorySelect={navigation.setSelectedSubcategory}
          featsData={featsData}
          availableFeatsCount={availableFeatsCount}
          showMyFeats={true}
          onMyFeatsClick={() => {
            setShowMyFeats(true);
            navigation.clearNavigation();
          }}
          globalSearch={globalSearch}
          onGlobalSearchChange={setGlobalSearch}
        />

        {/* Main Content Area */}
        {showMyFeats ? (
          <MyFeatsView
            featsData={featsData}
            viewMode={viewMode}
            onViewModeChange={setViewMode}
            onDetails={loadFeatDetails}
            onAdd={handleAddFeat}
            onRemove={handleRemoveFeat}
            onValidate={validateFeat}
            validationCache={validationCache}
            validatingFeatId={validatingFeatId}
          />
        ) : (
          <FeatContentArea
            selectedCategory={navigation.selectedCategory}
            selectedSubcategory={navigation.selectedSubcategory}
            featsData={featsData}
            categoryFeats={categoryFeats}
            categoryFeatsLoading={categoryFeatsLoading}
            categoryFeatsError={categoryFeatsError}
            viewMode={viewMode}
            onViewModeChange={setViewMode}
            onDetails={loadFeatDetails}
            onAdd={handleAddFeat}
            onRemove={handleRemoveFeat}
            onValidate={validateFeat}
            validationCache={validationCache}
            validatingFeatId={validatingFeatId}
          />
        )}
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