'use client';

import { useState, useMemo, useEffect, useCallback } from 'react';
import { Card } from '@/components/ui/Card';
import { AlertCircle } from 'lucide-react';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { CharacterAPI } from '@/services/characterApi';
import { useFeatSearch } from '@/hooks/useFeatSearch';
import { FeatNavBar, type FeatTab } from './FeatNavBar';
import { FeatTabContent } from './FeatTabContent';
import type { FeatInfo, FeatsState } from './types';
import { useToast } from '@/contexts/ToastContext';

export default function FeatsEditor() {
  const { 
    character, 
    isLoading: characterLoading, 
    error: characterError, 
    invalidateSubsystems,
    totalFeats,
    setTotalFeats
  } = useCharacterContext();
  const feats = useSubsystem('feats');
  const { showToast } = useToast();

  const [activeTab, setActiveTab] = useState<FeatTab>('my-feats');
  const [searchTerm, setSearchTerm] = useState('');
  const [sortBy, setSortBy] = useState('name');
  const [selectedTypes, setSelectedTypes] = useState<Set<number>>(new Set());
  const [showAvailableOnly, setShowAvailableOnly] = useState(false);
  const [availableFeats, setAvailableFeats] = useState<FeatInfo[]>([]);
  const [,setAvailableFeatsLoading] = useState(false);
  const [availableFeatsError, setAvailableFeatsError] = useState<string | null>(null);
  const [currentPage, setCurrentPage] = useState(1);
  const [, setAvailableTotal] = useState(0); 
  const [hasNext, setHasNext] = useState(false);
  const [hasPrevious, setHasPrevious] = useState(false);
  const FEATS_PER_PAGE = 50;

  const featsData = feats.data as FeatsState | null;
  const isLoading = characterLoading || feats.isLoading;
  const error = characterError || feats.error || availableFeatsError;

  useEffect(() => {
    if (character?.id && !feats.data && !feats.isLoading) {
      feats.load();
    }
  }, [character?.id, feats.data, feats.isLoading, feats]);

  useEffect(() => {
    setCurrentPage(1);
  }, [activeTab, searchTerm, selectedTypes, showAvailableOnly]);

  useEffect(() => {
    const loadAvailableFeats = async () => {
      if (!character?.id) {
        return;
      }

      setAvailableFeatsLoading(true);
      setAvailableFeatsError(null);

      try {
        const featTypeBitmask = selectedTypes.size > 0
          ? Array.from(selectedTypes).reduce((acc, type) => acc | type, 0)
          : undefined;

        if (showAvailableOnly) {
           const response = await CharacterAPI.getAvailableFeats(character.id, featTypeBitmask);
           setAvailableFeats(response.available_feats);
           setAvailableTotal(response.total);
           setTotalFeats(response.total);
           setHasNext(false); 
           setHasPrevious(false);
        } else {
           const response = await CharacterAPI.getLegitimateFeats(character.id, {
             page: currentPage,
             limit: FEATS_PER_PAGE,
             featType: featTypeBitmask,
             search: (searchTerm && searchTerm.length >= 3) ? searchTerm : undefined,
           });

           setAvailableFeats(response.feats);
           setTotalFeats(response.pagination.total);
           setAvailableTotal(response.pagination.total);
           setHasNext(response.pagination.has_next);
           setHasPrevious(response.pagination.has_previous);
        }

      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Failed to load available feats';
        setAvailableFeatsError(errorMessage);
      } finally {
        setAvailableFeatsLoading(false);
      }
    };

    loadAvailableFeats();
  }, [character?.id, activeTab, currentPage, FEATS_PER_PAGE, selectedTypes, searchTerm, showAvailableOnly, setTotalFeats]);


  const allMyFeats = useMemo(() => {
    if (!featsData?.summary) return [];

    const allFeats = [
      ...(featsData.summary.protected || []),
      ...(featsData.summary.class_feats || []),
      ...(featsData.summary.general_feats || []),
      ...(featsData.summary.custom_feats || []),
      ...(featsData.summary.background_feats || []),
      ...(featsData.summary.domain_feats || []),
    ];

    const uniqueFeats = new Map<number, FeatInfo>();
    allFeats.forEach(feat => {
      uniqueFeats.set(feat.id, feat);
    });

    return Array.from(uniqueFeats.values());
  }, [featsData]);

  const ownedFeatIds = useMemo(() => {
    return new Set(allMyFeats.map(f => f.id));
  }, [allMyFeats]);

  const protectedFeatIds = useMemo(() => {
    if (!featsData?.summary) return new Set<number>();
    return new Set((featsData.summary.protected || []).map(f => f.id));
  }, [featsData]);

  const filterAndSortFeats = useCallback((feats: FeatInfo[]) => {
    let filtered = [...feats];

    if (selectedTypes.size > 0) {
      filtered = filtered.filter(feat => {
        return Array.from(selectedTypes).some(type => (feat.type & type) !== 0);
      });
    }

    filtered.sort((a, b) => {
      switch (sortBy) {
        case 'name':
          return a.name.localeCompare(b.name);
        case 'type':
          return a.type - b.type;
        case 'level':
          return 0;
        default:
          return 0;
      }
    });

    return filtered;
  }, [selectedTypes, sortBy]);

  const { searchResults: searchedMyFeats } = useFeatSearch(allMyFeats, searchTerm);
  const filteredMyFeats = useMemo(() => filterAndSortFeats(searchedMyFeats), [searchedMyFeats, filterAndSortFeats]);

  const filteredAvailableFeats = useMemo(() => {
    let listToFilter = availableFeats;
    
    // Filter out owned feats
    listToFilter = listToFilter.filter(feat => !ownedFeatIds.has(feat.id));

    if (showAvailableOnly && searchTerm) {
       const lowerSearch = searchTerm.toLowerCase();
       listToFilter = listToFilter.filter(feat => 
          feat.name.toLowerCase().includes(lowerSearch) || 
          feat.label?.toLowerCase().includes(lowerSearch)
       );
    }

    return listToFilter.sort((a, b) => {
      switch (sortBy) {
        case 'name':
          return a.name.localeCompare(b.name);
        case 'type':
          return a.type - b.type;
        case 'level':
          return 0;
        default:
          return 0;
      }
    });
  }, [availableFeats, ownedFeatIds, sortBy, showAvailableOnly, searchTerm]);

  const finalAvailableFeats = filteredAvailableFeats;

  const handleAddFeat = useCallback(async (featId: number) => {
    if (!character?.id) return;

    try {
      const response = await CharacterAPI.addFeat(character.id, featId);
      await feats.load({ force: true });
      await invalidateSubsystems(['combat', 'abilityScores']);

      if (response.message && (response.message.includes('feats:') || response.message.includes('abilities:'))) {
        showToast(response.message, 'success', 6000);
      } else {
        showToast('Feat added successfully', 'success');
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to add feat';
      showToast(errorMessage, 'error');
    }
  }, [character?.id, feats, invalidateSubsystems, showToast]);

  const handleRemoveFeat = useCallback(async (featId: number) => {
    if (!character?.id) return;

    try {
      await CharacterAPI.removeFeat(character.id, featId);
      await feats.load({ force: true });
      await invalidateSubsystems(['combat']);
      showToast('Feat removed successfully', 'success');
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to remove feat';
      showToast(errorMessage, 'error');
    }
  }, [character?.id, feats, invalidateSubsystems, showToast]);

  const handleLoadFeatDetails = useCallback(async (feat: FeatInfo): Promise<FeatInfo | null> => {
    if (!character?.id) return null;

    try {
      const details = await CharacterAPI.getFeatDetails(character.id, feat.id);
      return details;
    } catch {
      return null;
    }
  }, [character?.id]);

  const totalPages = useMemo(() => {
    return Math.ceil(totalFeats / FEATS_PER_PAGE);
  }, [totalFeats, FEATS_PER_PAGE]);

  const filteredCount = useMemo(() => {
    if (activeTab === 'my-feats') return filteredMyFeats.length;
    if (activeTab === 'all-feats') return totalFeats;
    return 0;
  }, [activeTab, filteredMyFeats.length, totalFeats]);

  const handlePageChange = useCallback((newPage: number) => {
    setCurrentPage(newPage);
  }, []);

  if (isLoading && !featsData) {
    return (
      <div className="flex flex-col gap-4">
        <FeatNavBar
          activeTab={activeTab}
          onTabChange={setActiveTab}
          searchTerm={searchTerm}
          onSearchChange={setSearchTerm}
          sortBy={sortBy}
          onSortChange={setSortBy}
          selectedTypes={selectedTypes}
          onTypesChange={setSelectedTypes}
          myFeatsCount={0}
          availableFeatsCount={totalFeats}
          filteredCount={0}
          currentPage={1}
          totalPages={1}
          hasNext={false}
          hasPrevious={false}
          onPageChange={() => {}}
          showAvailableOnly={showAvailableOnly}
          onAvailableOnlyChange={setShowAvailableOnly}
        />
        <div className="flex items-center justify-center h-64 bg-[rgb(var(--color-surface-1))] border border-[rgb(var(--color-surface-border))] rounded-lg">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-[rgb(var(--color-primary))]"></div>
        </div>
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
    <div className="flex flex-col h-full">
      <div className="sticky top-0 z-10 mb-4">
        <FeatNavBar
          activeTab={activeTab}
          onTabChange={setActiveTab}
          searchTerm={searchTerm}
          onSearchChange={setSearchTerm}
          sortBy={sortBy}
          onSortChange={setSortBy}
          selectedTypes={selectedTypes}
          onTypesChange={setSelectedTypes}
          myFeatsCount={allMyFeats.length}
          availableFeatsCount={totalFeats}
          filteredCount={filteredCount}
          currentPage={currentPage}
          totalPages={totalPages}
          hasNext={hasNext}
          hasPrevious={hasPrevious}
          onPageChange={handlePageChange}
          showAvailableOnly={showAvailableOnly}
          onAvailableOnlyChange={setShowAvailableOnly}
        />
      </div>

      <FeatTabContent
        activeTab={activeTab}
        myFeats={filteredMyFeats}
        allFeats={finalAvailableFeats}
        ownedFeatIds={ownedFeatIds}
        protectedFeatIds={protectedFeatIds}
        onAddFeat={handleAddFeat}
        onRemoveFeat={handleRemoveFeat}
        onLoadFeatDetails={handleLoadFeatDetails}
        currentPage={currentPage}
        totalPages={totalPages}
        hasNext={hasNext}
        hasPrevious={hasPrevious}
        onPageChange={handlePageChange}
      />
    </div>
  );
}
