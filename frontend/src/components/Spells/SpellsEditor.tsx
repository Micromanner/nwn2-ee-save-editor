'use client';

import { useState, useMemo, useEffect, useCallback } from 'react';
import { Card } from '@/components/ui/Card';
import { AlertCircle, Info } from 'lucide-react';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { CharacterAPI } from '@/services/characterApi';
import { useSpellSearch } from '@/hooks/useSpellSearch';
import { SpellNavBar, type SpellTab } from './SpellNavBar';
import { SpellTabContent } from './SpellTabContent';
import type { SpellInfo, SpellsState, SpellcastingClass, KnownSpell } from './types';
import { useToast } from '@/contexts/ToastContext';
import { useTranslations } from '@/hooks/useTranslations';

export default function SpellsEditor() {
  const {
    character,
    isLoading: characterLoading,
    error: characterError,
    invalidateSubsystems,
    totalSpells,
    setTotalSpells
  } = useCharacterContext();
  const spells = useSubsystem('spells');
  const { showToast } = useToast();
  const t = useTranslations();

  const [activeTab, setActiveTab] = useState<SpellTab>('my-spells');
  const [searchTerm, setSearchTerm] = useState('');
  const [sortBy, setSortBy] = useState('name');
  const [selectedClasses, setSelectedClasses] = useState<Set<string>>(new Set());
  const [selectedSchools, setSelectedSchools] = useState<Set<string>>(new Set());
  const [selectedLevels, setSelectedLevels] = useState<Set<number>>(new Set());

  const [availableSpells, setAvailableSpells] = useState<SpellInfo[]>([]);
  const [, setAvailableSpellsLoading] = useState(false);
  const [availableSpellsError, setAvailableSpellsError] = useState<string | null>(null);
  const [currentPage, setCurrentPage] = useState(1);
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
  }, [activeTab, searchTerm, selectedClasses, selectedSchools, selectedLevels]);

  useEffect(() => {
    const loadAvailableSpells = async () => {
      if (!character?.id) {
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

        const classId = selectedClasses.size === 1
          ? Number(Array.from(selectedClasses)[0])
          : undefined;

        const response = await CharacterAPI.getLegitimateSpells(character.id, {
          page: currentPage,
          limit: SPELLS_PER_PAGE,
          schools,
          levels,
          search: (searchTerm && searchTerm.length >= 3) ? searchTerm : undefined,
          class_id: classId,
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
  }, [character?.id, activeTab, currentPage, SPELLS_PER_PAGE, selectedClasses, selectedSchools, selectedLevels, searchTerm, setTotalSpells]);

  const casterClasses = useMemo(() => {
    if (!spellsData?.spellcasting_classes) return [];
    return spellsData.spellcasting_classes.map((cls: SpellcastingClass) => ({
      index: cls.index,
      name: cls.class_name,
      class_id: cls.class_id,
      can_edit_spells: cls.can_edit_spells,
    }));
  }, [spellsData]);

  const hasEditableClasses = useMemo(() => {
    return casterClasses.some(cls => cls.can_edit_spells);
  }, [casterClasses]);

  const readOnlyClassNames = useMemo(() => {
    return casterClasses
      .filter(cls => !cls.can_edit_spells)
      .map(cls => cls.name);
  }, [casterClasses]);

  const availableClassFilters = useMemo(() => {
    if (!spellsData?.spellcasting_classes) return [];
    return spellsData.spellcasting_classes.map((cls: SpellcastingClass) => ({
      name: cls.class_name,
      value: cls.class_id.toString(),
    }));
  }, [spellsData]);

  /* Known Spells */
  const allMySpells = useMemo((): SpellInfo[] => {
    if (!spellsData?.known_spells) return [];

    return spellsData.known_spells.map((ks: KnownSpell) => ({
      id: ks.spell_id,
      name: ks.name,
      level: ks.level,
      icon: ks.icon,
      school_name: ks.school_name,
      description: ks.description,
      class_id: ks.class_id,
      available_classes: [],
      is_domain_spell: ks.is_domain_spell,
    })) as SpellInfo[];
  }, [spellsData?.known_spells]);

  /* Prepared Spells */
  const preparedSpells = useMemo((): SpellInfo[] => {
    if (!spellsData?.memorized_spells) return [];

    const spellMap = new Map<string, { spell: SpellInfo; count: number }>();

    for (const ms of spellsData.memorized_spells) {
      const key = `${ms.spell_id}-${ms.level}-${ms.class_id}`;
      const existing = spellMap.get(key);

      if (existing) {
        existing.count++;
      } else {
        spellMap.set(key, {
            spell: {
                id: ms.spell_id,
                name: ms.name,
                level: ms.level,
                icon: ms.icon,
                school_name: ms.school_name,
                description: ms.description,
                class_id: ms.class_id,
                available_classes: [],
            },
            count: 1,
        });
      }
    }

    return Array.from(spellMap.values()).map(({ spell, count }) => ({
      ...spell,
      memorized_count: count,
    }));
  }, [spellsData?.memorized_spells]);

  const ownedSpellIds = useMemo(() => new Set(allMySpells.map(s => s.id)), [allMySpells]);

  const filterAndSortSpells = useCallback((spells: SpellInfo[]) => {
    let filtered = [...spells];

    if (selectedClasses.size > 0) {
      const classIds = new Set(Array.from(selectedClasses).map(c => Number(c)));
      filtered = filtered.filter(spell => 
        spell.class_id !== undefined && classIds.has(spell.class_id)
      );
    }

    if (selectedSchools.size > 0) {
      filtered = filtered.filter(spell => 
        spell.school_name && selectedSchools.has(spell.school_name)
      );
    }

    if (selectedLevels.size > 0) {
      filtered = filtered.filter(spell => selectedLevels.has(spell.level));
    }

    filtered.sort((a, b) => {
      switch (sortBy) {
        case 'name': return a.name.localeCompare(b.name);
        case 'level': return a.level - b.level;
        case 'school': return (a.school_name || '').localeCompare(b.school_name || '');
        default: return 0;
      }
    });

    return filtered;
  }, [selectedClasses, selectedSchools, selectedLevels, sortBy]);

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

  const handleAddSpell = useCallback(async (spellId: number, classIndex: number, spellLevel: number) => {
    if (!character?.id) return;

    try {
      const response = await CharacterAPI.manageSpell(character.id, 'add', spellId, classIndex, spellLevel);
      await spells.load({ force: true });
      await invalidateSubsystems(['combat']);

      showToast(response.message || 'Spell learned successfully', 'success');
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to learn spell';
      showToast(errorMessage, 'error');
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
    }
  }, [character?.id, spells, invalidateSubsystems, showToast]);

  const totalPages = useMemo(() => {
    return Math.ceil(totalSpells / SPELLS_PER_PAGE);
  }, [totalSpells, SPELLS_PER_PAGE]);

  // Apply search/filter to prepared spells
  const { searchResults: searchedPreparedSpells } = useSpellSearch(preparedSpells, searchTerm);
  const filteredPreparedSpells = useMemo(() => filterAndSortSpells(searchedPreparedSpells), [searchedPreparedSpells, filterAndSortSpells]);

  const filteredCount = useMemo(() => {
    if (activeTab === 'my-spells') return filteredMySpells.length;
    if (activeTab === 'prepared') return filteredPreparedSpells.length;
    if (activeTab === 'all-spells') return totalSpells;
    return 0;
  }, [activeTab, filteredMySpells.length, filteredPreparedSpells.length, totalSpells]);

  const handlePageChange = useCallback((newPage: number) => {
    setCurrentPage(newPage);
  }, []);

  if (isLoading && !spellsData) {
    return (
      <div className="flex flex-col gap-4">
        <SpellNavBar
          activeTab={activeTab}
          onTabChange={setActiveTab}
          searchTerm={searchTerm}
          onSearchChange={setSearchTerm}
          sortBy={sortBy}
          onSortChange={setSortBy}
          selectedClasses={selectedClasses}
          onClassesChange={setSelectedClasses}
          selectedSchools={selectedSchools}
          onSchoolsChange={setSelectedSchools}
          selectedLevels={selectedLevels}
          onLevelsChange={setSelectedLevels}
          mySpellsCount={0}
          preparedSpellsCount={0}
          availableSpellsCount={totalSpells}
          filteredCount={0}
          currentPage={1}
          totalPages={1}
          hasNext={false}
          hasPrevious={false}
          onPageChange={() => {}}
          availableClasses={availableClassFilters}
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
          selectedClasses={selectedClasses}
          onClassesChange={setSelectedClasses}
          selectedSchools={selectedSchools}
          onSchoolsChange={setSelectedSchools}
          selectedLevels={selectedLevels}
          onLevelsChange={setSelectedLevels}
          mySpellsCount={allMySpells.length}
          preparedSpellsCount={preparedSpells.length}
          availableSpellsCount={totalSpells}
          filteredCount={filteredCount}
          currentPage={currentPage}
          totalPages={totalPages}
          hasNext={hasNext}
          hasPrevious={hasPrevious}
          onPageChange={handlePageChange}
          availableClasses={availableClassFilters}
        />
      </div>

      {readOnlyClassNames.length > 0 && (
        <div className="mb-4 flex items-start gap-3 p-3 bg-[rgb(var(--color-surface-2))] border border-[rgb(var(--color-surface-border))] rounded-lg">
          <Info className="w-5 h-5 text-[rgb(var(--color-text-muted))] flex-shrink-0 mt-0.5" />
          <p className="text-sm text-[rgb(var(--color-text-secondary))]">
            {hasEditableClasses ? (
              <>
                <span className="font-medium text-[rgb(var(--color-text-primary))]">
                  {readOnlyClassNames.join(', ')}
                </span>
                {' '}{t('spells.readOnlyClassNotice')}
              </>
            ) : (
              t('spells.noEditableClasses')
            )}
          </p>
        </div>
      )}

      <SpellTabContent
        activeTab={activeTab}
        mySpells={filteredMySpells}
        preparedSpells={filteredPreparedSpells}
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