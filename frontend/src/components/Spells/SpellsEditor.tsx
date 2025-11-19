'use client';

import { useState, useMemo, useEffect, useCallback } from 'react';
import { Card } from '@/components/ui/Card';
import { AlertCircle } from 'lucide-react';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { CharacterAPI } from '@/services/characterApi';
import { useSpellSearch } from '@/hooks/useSpellSearch';
import { SpellNavBar, type SpellTab } from './SpellNavBar';
import { SpellTabContent } from './SpellTabContent';
import type { SpellInfo, SpellsState, SpellcastingClass } from './types';
import { useToast } from '@/contexts/ToastContext';

export default function SpellsEditor() {
  const { character, isLoading: characterLoading, error: characterError, invalidateSubsystems } = useCharacterContext();
  const spells = useSubsystem('spells');
  const { showToast } = useToast();

  const [activeTab, setActiveTab] = useState<SpellTab>('my-spells');
  const [searchTerm, setSearchTerm] = useState('');
  const [sortBy, setSortBy] = useState('name');
  const [selectedSchools, setSelectedSchools] = useState<Set<string>>(new Set());
  const [selectedLevels, setSelectedLevels] = useState<Set<number>>(new Set());

  const [availableSpells, setAvailableSpells] = useState<SpellInfo[]>([]);
  const [_availableSpellsLoading, setAvailableSpellsLoading] = useState(false); // eslint-disable-line @typescript-eslint/no-unused-vars
  const [availableSpellsError, setAvailableSpellsError] = useState<string | null>(null);
  const [currentPage, setCurrentPage] = useState(1);
  const [totalSpells, setTotalSpells] = useState(0);
  const [hasNext, setHasNext] = useState(false);
  const [hasPrevious, setHasPrevious] = useState(false);
  const SPELLS_PER_PAGE = 50;

  const spellsData = spells.data as SpellsState | null;
  const isLoading = characterLoading || spells.isLoading;
  const error = characterError || spells.error || availableSpellsError;

  useEffect(() => {
    if (character?.id && !spells.data && !spells.isLoading) {
      spells.load();
    }
  }, [character?.id, spells.data, spells.isLoading, spells]);

  useEffect(() => {
    setCurrentPage(1);
  }, [activeTab, searchTerm, selectedSchools, selectedLevels]);

  useEffect(() => {
    const loadAvailableSpells = async () => {
      if (!character?.id || activeTab !== 'all-spells') {
        return;
      }

      setAvailableSpellsLoading(true);
      setAvailableSpellsError(null);

      try {
        const schools = selectedSchools.size > 0
          ? Array.from(selectedSchools).join(',')
          : undefined;

        const levels = selectedLevels.size > 0
          ? Array.from(selectedLevels).join(',')
          : undefined;

        const response = await CharacterAPI.getLegitimateSpells(character.id, {
          page: currentPage,
          limit: SPELLS_PER_PAGE,
          schools,
          levels,
          search: (searchTerm && searchTerm.length >= 3) ? searchTerm : undefined,
        });

        setAvailableSpells(response.spells);
        setTotalSpells(response.pagination.total);
        setHasNext(response.pagination.has_next);
        setHasPrevious(response.pagination.has_previous);
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Failed to load available spells';
        setAvailableSpellsError(errorMessage);
      } finally {
        setAvailableSpellsLoading(false);
      }
    };

    loadAvailableSpells();
  }, [character?.id, activeTab, currentPage, SPELLS_PER_PAGE, selectedSchools, selectedLevels, searchTerm]);

  const casterClasses = useMemo(() => {
    if (!spellsData?.spellcasting_classes) return [];
    return spellsData.spellcasting_classes.map((cls: SpellcastingClass) => ({
      index: cls.index,
      name: cls.class_name,
    }));
  }, [spellsData]);

  const allMySpells = useMemo(() => {
    if (!spellsData?.memorized_spells || !character?.id) return [];

    const uniqueSpellIds = new Set(spellsData.memorized_spells.map(s => s.spell_id));

    return availableSpells.filter(spell => uniqueSpellIds.has(spell.id));
  }, [spellsData?.memorized_spells, character?.id, availableSpells]);

  const ownedSpellIds = useMemo(() => {
    return new Set(allMySpells.map(s => s.id));
  }, [allMySpells]);

  const filterAndSortSpells = useCallback((spells: SpellInfo[]) => {
    let filtered = [...spells];

    if (selectedSchools.size > 0) {
      filtered = filtered.filter(spell => {
        return spell.school_name && selectedSchools.has(spell.school_name);
      });
    }

    if (selectedLevels.size > 0) {
      filtered = filtered.filter(spell => selectedLevels.has(spell.level));
    }

    filtered.sort((a, b) => {
      switch (sortBy) {
        case 'name':
          return a.name.localeCompare(b.name);
        case 'level':
          return a.level - b.level;
        case 'school':
          return (a.school_name || '').localeCompare(b.school_name || '');
        default:
          return 0;
      }
    });

    return filtered;
  }, [selectedSchools, selectedLevels, sortBy]);

  const { searchResults: searchedMySpells } = useSpellSearch(allMySpells, searchTerm);
  const filteredMySpells = useMemo(() => filterAndSortSpells(searchedMySpells), [searchedMySpells, filterAndSortSpells]);

  const filteredAvailableSpells = useMemo(() => {
    const notOwned = availableSpells.filter(spell => !ownedSpellIds.has(spell.id));

    return notOwned.sort((a, b) => {
      switch (sortBy) {
        case 'name':
          return a.name.localeCompare(b.name);
        case 'level':
          return a.level - b.level;
        case 'school':
          return (a.school_name || '').localeCompare(b.school_name || '');
        default:
          return 0;
      }
    });
  }, [availableSpells, ownedSpellIds, sortBy]);

  const finalAvailableSpells = filteredAvailableSpells;

  const handleAddSpell = useCallback(async (spellId: number, classIndex: number) => {
    if (!character?.id) return;

    try {
      const response = await CharacterAPI.manageSpell(character.id, 'add', spellId, classIndex);
      await spells.load({ force: true });
      await invalidateSubsystems(['combat']);

      showToast(response.message || 'Spell learned successfully', 'success');
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to learn spell';
      showToast(errorMessage, 'error');
      console.error('Failed to learn spell:', error);
    }
  }, [character?.id, spells, invalidateSubsystems, showToast]);

  const handleRemoveSpell = useCallback(async (spellId: number, classIndex: number) => {
    if (!character?.id) return;

    try {
      const response = await CharacterAPI.manageSpell(character.id, 'remove', spellId, classIndex);
      await spells.load({ force: true });
      await invalidateSubsystems(['combat']);
      showToast(response.message || 'Spell removed successfully', 'success');
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to remove spell';
      showToast(errorMessage, 'error');
      console.error('Failed to remove spell:', error);
    }
  }, [character?.id, spells, invalidateSubsystems, showToast]);

  const totalPages = useMemo(() => {
    return Math.ceil(totalSpells / SPELLS_PER_PAGE);
  }, [totalSpells, SPELLS_PER_PAGE]);

  const filteredCount = useMemo(() => {
    if (activeTab === 'my-spells') return filteredMySpells.length;
    if (activeTab === 'all-spells') return totalSpells;
    return 0;
  }, [activeTab, filteredMySpells.length, totalSpells]);

  const handlePageChange = useCallback((newPage: number) => {
    setCurrentPage(newPage);
  }, []);

  if (isLoading && !spellsData) {
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

  if (!character || !spellsData) {
    return (
      <Card variant="warning">
        <p className="text-muted">No character loaded. Please import a save file to begin.</p>
      </Card>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="sticky top-0 z-10 mb-4">
        <SpellNavBar
          activeTab={activeTab}
          onTabChange={setActiveTab}
          searchTerm={searchTerm}
          onSearchChange={setSearchTerm}
          sortBy={sortBy}
          onSortChange={setSortBy}
          selectedSchools={selectedSchools}
          onSchoolsChange={setSelectedSchools}
          selectedLevels={selectedLevels}
          onLevelsChange={setSelectedLevels}
          mySpellsCount={allMySpells.length}
          availableSpellsCount={totalSpells}
          filteredCount={filteredCount}
          currentPage={currentPage}
          totalPages={totalPages}
          hasNext={hasNext}
          hasPrevious={hasPrevious}
          onPageChange={handlePageChange}
        />
      </div>

      <SpellTabContent
        activeTab={activeTab}
        mySpells={filteredMySpells}
        allSpells={finalAvailableSpells}
        ownedSpellIds={ownedSpellIds}
        onAddSpell={handleAddSpell}
        onRemoveSpell={handleRemoveSpell}
        currentPage={currentPage}
        totalPages={totalPages}
        hasNext={hasNext}
        hasPrevious={hasPrevious}
        onPageChange={handlePageChange}
        casterClasses={casterClasses}
      />
    </div>
  );
}