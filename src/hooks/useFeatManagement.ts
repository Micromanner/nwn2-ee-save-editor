
import { useState, useCallback, useEffect } from 'react';
import { CharacterAPI, FeatActionResponse } from '@/services/characterApi';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { useTranslations } from '@/hooks/useTranslations';
import type { FeatInfo, FeatsState } from '@/components/Feats/types';

const FEAT_DEPENDENT_SUBSYSTEMS = ['combat', 'abilityScores', 'saves', 'skills', 'classes'] as const;

interface UseFeatManagementOptions {
  autoLoadFeats?: boolean;
}

interface FeatManagementState {
  featsData: FeatsState | null;
  isLoading: boolean;
  error: string | null;
  categoryFeats: FeatInfo[];
  categoryFeatsLoading: boolean;
  categoryFeatsError: string | null;
  selectedFeat: FeatInfo | null;
  featDetails: FeatInfo | null;
  loadingDetails: boolean;
}

interface FeatManagementActions {
  loadFeats: (force?: boolean) => Promise<void>;
  loadCategoryFeats: (category: string, subcategory?: string | null) => Promise<void>;
  loadFeatDetails: (feat: FeatInfo) => Promise<void>;
  addFeat: (featId: number) => Promise<FeatActionResponse>;
  removeFeat: (featId: number) => Promise<void>;
  selectFeat: (feat: FeatInfo | null) => void;
  clearSelection: () => void;
}

export interface UseFeatManagementReturn extends FeatManagementState, FeatManagementActions {}

export function useFeatManagement(
  options: UseFeatManagementOptions = {}
): UseFeatManagementReturn {
  const t = useTranslations();
  const { autoLoadFeats = true } = options;

  const { character, isLoading: characterLoading, error: characterError, invalidateSubsystems, refreshCharacter } = useCharacterContext();
  const feats = useSubsystem('feats');
  
  const [categoryFeats, setCategoryFeats] = useState<FeatInfo[]>([]);
  const [categoryFeatsLoading, setCategoryFeatsLoading] = useState(false);
  const [categoryFeatsError, setCategoryFeatsError] = useState<string | null>(null);
  const [selectedFeat, setSelectedFeat] = useState<FeatInfo | null>(null);
  const [featDetails, setFeatDetails] = useState<FeatInfo | null>(null);
  const [loadingDetails, setLoadingDetails] = useState(false);
  
  const featsData = feats.data as FeatsState | null;
  const isLoading = characterLoading || feats.isLoading;
  const error = characterError || feats.error;

  useEffect(() => {
    if (autoLoadFeats && character && !feats.data && !feats.isLoading && !feats.error) {
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
      const errorMessage = error instanceof Error ? error.message : t('errors.loadingFailed');
      setCategoryFeatsError(errorMessage);
      setCategoryFeats([]);
    } finally {
      setCategoryFeatsLoading(false);
    }
  }, [t, character?.id]);

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

  const addFeat = useCallback(async (featId: number): Promise<FeatActionResponse> => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    const response = await CharacterAPI.addFeat(character.id, featId);
    await Promise.all([
      feats.load({ force: true }),
      invalidateSubsystems([...FEAT_DEPENDENT_SUBSYSTEMS]),
      refreshCharacter(),
    ]);
    return response;
  }, [t, character?.id, feats, invalidateSubsystems, refreshCharacter]);

  const removeFeat = useCallback(async (featId: number): Promise<void> => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    await CharacterAPI.removeFeat(character.id, featId);
    await Promise.all([
      feats.load({ force: true }),
      invalidateSubsystems([...FEAT_DEPENDENT_SUBSYSTEMS]),
      refreshCharacter(),
    ]);
  }, [t, character?.id, feats, invalidateSubsystems, refreshCharacter]);

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
    selectedFeat,
    featDetails,
    loadingDetails,
    loadFeats,
    loadCategoryFeats,
    loadFeatDetails,
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