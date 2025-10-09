'use client';

import { useState, useCallback, useEffect } from 'react';
import { CharacterAPI } from '@/services/characterApi';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import type { 
  FeatInfo, 
  FeatsState, 
  ValidationCache,
  ValidationState 
} from '@/components/Feats/types';

interface UseFeatManagementOptions {
  autoLoadFeats?: boolean;
  enableValidation?: boolean;
}

interface FeatManagementState {
  // Core data
  featsData: FeatsState | null;
  isLoading: boolean;
  error: string | null;
  
  // Category feats
  categoryFeats: FeatInfo[];
  categoryFeatsLoading: boolean;
  categoryFeatsError: string | null;
  
  // Validation
  validationCache: ValidationCache;
  validatingFeatId: number | null;
  
  // Selected feat details
  selectedFeat: FeatInfo | null;
  featDetails: FeatInfo | null;
  loadingDetails: boolean;
}

interface FeatManagementActions {
  // Loading
  loadFeats: (force?: boolean) => Promise<void>;
  loadCategoryFeats: (category: string, subcategory?: string | null) => Promise<void>;
  loadFeatDetails: (feat: FeatInfo) => Promise<void>;
  
  // Validation
  validateFeat: (featId: number) => Promise<ValidationState | null>;
  clearValidationCache: () => void;
  
  // Management
  addFeat: (featId: number) => Promise<void>;
  removeFeat: (featId: number) => Promise<void>;
  
  // Selection
  selectFeat: (feat: FeatInfo | null) => void;
  clearSelection: () => void;
}

export interface UseFeatManagementReturn extends FeatManagementState, FeatManagementActions {}

export function useFeatManagement(
  options: UseFeatManagementOptions = {}
): UseFeatManagementReturn {
  const { 
    autoLoadFeats = true,
    enableValidation = true 
  } = options;

  const { character, isLoading: characterLoading, error: characterError, invalidateSubsystems } = useCharacterContext();
  const feats = useSubsystem('feats');
  
  // State for category-based feat loading
  const [categoryFeats, setCategoryFeats] = useState<FeatInfo[]>([]);
  const [categoryFeatsLoading, setCategoryFeatsLoading] = useState(false);
  const [categoryFeatsError, setCategoryFeatsError] = useState<string | null>(null);
  
  // State for on-demand feat validation
  const [validationCache, setValidationCache] = useState<ValidationCache>({});
  const [validatingFeatId, setValidatingFeatId] = useState<number | null>(null);
  
  // State for feat details
  const [selectedFeat, setSelectedFeat] = useState<FeatInfo | null>(null);
  const [featDetails, setFeatDetails] = useState<FeatInfo | null>(null);
  const [loadingDetails, setLoadingDetails] = useState(false);
  
  // Get feats data from subsystem
  const featsData = feats.data as FeatsState | null;
  const isLoading = characterLoading || feats.isLoading;
  const error = characterError || feats.error;

  // Auto-load feats data if enabled
  useEffect(() => {
    if (autoLoadFeats && character && !feats.data && !feats.isLoading) {
      feats.load();
    }
  }, [autoLoadFeats, character, feats]);

  // Load feats data
  const loadFeats = useCallback(async (force = false) => {
    if (!character) return;
    await feats.load({ force });
  }, [character, feats]);

  // Load category-specific feats
  const loadCategoryFeats = useCallback(async (
    category: string, 
    subcategory?: string | null
  ) => {
    if (!character?.id) {
      setCategoryFeats([]);
      return;
    }
    
    setCategoryFeatsLoading(true);
    setCategoryFeatsError(null);
    
    try {
      const response = await CharacterAPI.getLegitimateFeats(character.id, {
        category,
        subcategory: subcategory || undefined,
        page: 1,
        limit: 500 // Load more since search is client-side
      });
      
      setCategoryFeats(response.feats);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to load category feats';
      setCategoryFeatsError(errorMessage);
      console.error('Failed to load category feats:', error);
      setCategoryFeats([]);
    } finally {
      setCategoryFeatsLoading(false);
    }
  }, [character?.id]);

  // Load feat details
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

  // Validate a feat (can the character take it?)
  const validateFeat = useCallback(async (featId: number): Promise<ValidationState | null> => {
    if (!character?.id || !enableValidation) return null;
    
    // Check cache first
    if (validationCache[featId]) {
      return validationCache[featId];
    }
    
    // Mark as validating
    setValidatingFeatId(featId);
    
    try {
      // This would be a real API call
      // For now, mock the validation
      const validation: ValidationState = {
        can_take: Math.random() > 0.5, // Mock: 50% chance
        reason: 'Prerequisites not met',
        has_feat: false,
        missing_requirements: ['Level 5 required', 'BAB +3 required']
      };
      
      // In real implementation:
      // const validation = await CharacterAPI.validateFeat(character.id, featId);
      
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
  }, [character?.id, enableValidation, validationCache]);

  // Clear validation cache
  const clearValidationCache = useCallback(() => {
    setValidationCache({});
  }, []);

  // Add a feat to the character
  const addFeat = useCallback(async (featId: number) => {
    if (!character?.id) return;

    try {
      await CharacterAPI.addFeat(character.id, featId);
      // Refresh current feats
      await feats.load({ force: true });
      // Clear validation cache for this feat
      setValidationCache(prev => {
        const newCache = { ...prev };
        delete newCache[featId];
        return newCache;
      });
      // Silently refresh combat subsystem (feats may affect BAB, AC, saves, etc.)
      await invalidateSubsystems(['combat']);
    } catch (error) {
      console.error('Failed to add feat:', error);
      throw error;
    }
  }, [character?.id, feats, invalidateSubsystems]);

  // Remove a feat from the character
  const removeFeat = useCallback(async (featId: number) => {
    if (!character?.id) return;

    try {
      await CharacterAPI.removeFeat(character.id, featId);
      // Refresh current feats
      await feats.load({ force: true });
      // Clear validation cache for this feat
      setValidationCache(prev => {
        const newCache = { ...prev };
        delete newCache[featId];
        return newCache;
      });
      // Silently refresh combat subsystem (feats may affect BAB, AC, saves, etc.)
      await invalidateSubsystems(['combat']);
    } catch (error) {
      console.error('Failed to remove feat:', error);
      throw error;
    }
  }, [character?.id, feats, invalidateSubsystems]);

  // Select a feat
  const selectFeat = useCallback((feat: FeatInfo | null) => {
    setSelectedFeat(feat);
    if (!feat) {
      setFeatDetails(null);
    }
  }, []);

  // Clear selection
  const clearSelection = useCallback(() => {
    setSelectedFeat(null);
    setFeatDetails(null);
  }, []);

  return {
    // State
    featsData,
    isLoading,
    error,
    categoryFeats,
    categoryFeatsLoading,
    categoryFeatsError,
    validationCache,
    validatingFeatId,
    selectedFeat,
    featDetails,
    loadingDetails,
    
    // Actions
    loadFeats,
    loadCategoryFeats,
    loadFeatDetails,
    validateFeat,
    clearValidationCache,
    addFeat,
    removeFeat,
    selectFeat,
    clearSelection,
  };
}

// Hook for managing feat categories and navigation
interface UseFeatNavigationReturn {
  selectedCategory: string | null;
  selectedSubcategory: string | null;
  setSelectedCategory: (category: string | null) => void;
  setSelectedSubcategory: (subcategory: string | null) => void;
  navigateTo: (category: string | null, subcategory: string | null) => void;
  clearNavigation: () => void;
}

export function useFeatNavigation(): UseFeatNavigationReturn {
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [selectedSubcategory, setSelectedSubcategory] = useState<string | null>(null);

  const navigateTo = useCallback((category: string | null, subcategory: string | null) => {
    setSelectedCategory(category);
    setSelectedSubcategory(subcategory);
  }, []);

  const clearNavigation = useCallback(() => {
    setSelectedCategory(null);
    setSelectedSubcategory(null);
  }, []);

  return {
    selectedCategory,
    selectedSubcategory,
    setSelectedCategory,
    setSelectedSubcategory,
    navigateTo,
    clearNavigation,
  };
}