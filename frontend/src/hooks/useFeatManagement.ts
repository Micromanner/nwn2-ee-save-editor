
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
  featsData: FeatsState | null;
  isLoading: boolean;
  error: string | null;
  categoryFeats: FeatInfo[];
  categoryFeatsLoading: boolean;
  categoryFeatsError: string | null;
  validationCache: ValidationCache;
  validatingFeatId: number | null;
  selectedFeat: FeatInfo | null;
  featDetails: FeatInfo | null;
  loadingDetails: boolean;
}

interface FeatManagementActions {
  loadFeats: (force?: boolean) => Promise<void>;
  loadCategoryFeats: (category: string, subcategory?: string | null) => Promise<void>;
  loadFeatDetails: (feat: FeatInfo) => Promise<void>;
  validateFeat: (featId: number) => Promise<ValidationState | null>;
  clearValidationCache: () => void;
  addFeat: (featId: number) => Promise<void>;
  removeFeat: (featId: number) => Promise<void>;
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
  
  const [categoryFeats, setCategoryFeats] = useState<FeatInfo[]>([]);
  const [categoryFeatsLoading, setCategoryFeatsLoading] = useState(false);
  const [categoryFeatsError, setCategoryFeatsError] = useState<string | null>(null);
  const [validationCache, setValidationCache] = useState<ValidationCache>({});
  const [validatingFeatId, setValidatingFeatId] = useState<number | null>(null);
  const [selectedFeat, setSelectedFeat] = useState<FeatInfo | null>(null);
  const [featDetails, setFeatDetails] = useState<FeatInfo | null>(null);
  const [loadingDetails, setLoadingDetails] = useState(false);
  
  const featsData = feats.data as FeatsState | null;
  const isLoading = characterLoading || feats.isLoading;
  const error = characterError || feats.error;

  useEffect(() => {
    if (autoLoadFeats && character && !feats.data && !feats.isLoading) {
      feats.load();
    }
  }, [autoLoadFeats, character, feats]);

  const loadFeats = useCallback(async (force = false) => {
    if (!character) return;
    await feats.load({ force });
  }, [character, feats]);

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
        limit: 500
      });
      
      setCategoryFeats(response.feats);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to load category feats';
      setCategoryFeatsError(errorMessage);
      setCategoryFeats([]);
    } finally {
      setCategoryFeatsLoading(false);
    }
  }, [character?.id]);

  const loadFeatDetails = useCallback(async (feat: FeatInfo) => {
    if (!character?.id) return;
    
    setSelectedFeat(feat);
    setLoadingDetails(true);
    
    try {
      const details = await CharacterAPI.getFeatDetails(character.id, feat.id);
      setFeatDetails(details);
    } catch {
      setFeatDetails(null);
    } finally {
      setLoadingDetails(false);
    }
  }, [character?.id]);

  const validateFeat = useCallback(async (featId: number): Promise<ValidationState | null> => {
    if (!character?.id || !enableValidation) return null;
    
    if (validationCache[featId]) {
      return validationCache[featId];
    }
    
    setValidatingFeatId(featId);
    
    try {
      const validation: ValidationState = {
        can_take: Math.random() > 0.5,
        reason: 'Prerequisites not met',
        has_feat: false,
        missing_requirements: ['Level 5 required', 'BAB +3 required']
      };
      
      setValidationCache(prev => ({
        ...prev,
        [featId]: validation
      }));
      
      return validation;
    } catch {
      return null;
    } finally {
      setValidatingFeatId(null);
    }
  }, [character?.id, enableValidation, validationCache]);

  const clearValidationCache = useCallback(() => {
    setValidationCache({});
  }, []);

  const addFeat = useCallback(async (featId: number) => {
    if (!character?.id) return;

    try {
      await CharacterAPI.addFeat(character.id, featId);
      await feats.load({ force: true });
      setValidationCache(prev => {
        const newCache = { ...prev };
        delete newCache[featId];
        return newCache;
      });
      await invalidateSubsystems(['combat']);
    } catch (error) {
      throw error;
    }
  }, [character?.id, feats, invalidateSubsystems]);

  const removeFeat = useCallback(async (featId: number) => {
    if (!character?.id) return;

    try {
      await CharacterAPI.removeFeat(character.id, featId);
      await feats.load({ force: true });
      setValidationCache(prev => {
        const newCache = { ...prev };
        delete newCache[featId];
        return newCache;
      });
      await invalidateSubsystems(['combat']);
    } catch (error) {
      throw error;
    }
  }, [character?.id, feats, invalidateSubsystems]);

  const selectFeat = useCallback((feat: FeatInfo | null) => {
    setSelectedFeat(feat);
    if (!feat) {
      setFeatDetails(null);
    }
  }, []);

  const clearSelection = useCallback(() => {
    setSelectedFeat(null);
    setFeatDetails(null);
  }, []);

  return {
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