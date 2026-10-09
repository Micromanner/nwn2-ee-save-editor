
import { useState, useCallback, useEffect } from 'react';
import { CharacterAPI, FeatActionResponse } from '@/services/characterApi';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { useTranslations } from '@/hooks/useTranslations';
import type { FeatInfo } from '@/lib/bindings';
import type { FeatsState } from '@/components/Feats/types';

const FEAT_DEPENDENT_SUBSYSTEMS = ['combat', 'abilityScores', 'saves', 'skills', 'classes'] as const;

interface UseFeatManagementOptions {
  autoLoadFeats?: boolean;
}

interface FeatManagementState {
  featsData: FeatsState | null;
  isLoading: boolean;
  error: string | null;
  selectedFeat: FeatInfo | null;
  featDetails: FeatInfo | null;
  loadingDetails: boolean;
}

interface FeatManagementActions {
  loadFeats: (force?: boolean) => Promise<void>;
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

  const loadFeatDetails = useCallback(async (feat: FeatInfo) => {
    if (!character?.id) return;
    
    setSelectedFeat(feat);
    setLoadingDetails(true);
    
    try {
      const details = await CharacterAPI.getFeatDetails(feat.id);
      setFeatDetails(details);
    } catch {
      setFeatDetails(null);
    } finally {
      setLoadingDetails(false);
    }
  }, [character?.id]);

  const addFeat = useCallback(async (featId: number): Promise<FeatActionResponse> => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    const response = await CharacterAPI.addFeat(featId);
    await Promise.all([
      feats.load({ force: true }),
      invalidateSubsystems([...FEAT_DEPENDENT_SUBSYSTEMS]),
      refreshCharacter(),
    ]);
    return response;
  }, [t, character?.id, feats, invalidateSubsystems, refreshCharacter]);

  const removeFeat = useCallback(async (featId: number): Promise<void> => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    await CharacterAPI.removeFeat(featId);
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
    selectedFeat,
    featDetails,
    loadingDetails,
    loadFeats,
    loadFeatDetails,
    addFeat,
    removeFeat,
    selectFeat,
    clearSelection,
  };
}