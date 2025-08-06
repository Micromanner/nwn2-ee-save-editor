'use client';

import React, { useState, useMemo } from 'react';
import { AlertCircle } from 'lucide-react';
import { useSpells } from '@/lib/api/hooks';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { spellSections, viewModeConfig } from './SpellSections';
import SpellFilters from './SpellFilters';
import SpellHeader from './SpellHeader';
import SpellList from './SpellList';
import { Card } from '@/components/ui/Card';

interface Spell {
  id: number;
  name: string;
  icon: string;      // Icon name from 2DA
  icon_url?: string; // URL to actual icon image
  school_id: number; // School ID from backend
  school_name: string | null; // School name from backend
  level: number;     // Spell level
  available_classes: string[]; // Classes that can use this spell
  
  // Spell details from backend
  description: string;
  range: string;
  cast_time: string;
  conjuration_time: string;
  components: string;
  metamagic: string;
  target_type: string;
  
  // Client-side properties for UI state
  isLearned?: boolean;
  isFavorite?: boolean;
  
  // Computed properties for compatibility
  school: string | null;
  innate_level: number;
}

interface SpellSlot {
  level: number;
  total: number;
}


export default function SpellsEditor() {
  // Get character context to check if character is loaded
  const { character, isLoading: characterLoading, error: characterError } = useCharacterContext();
  
  const [viewMode, setViewMode] = useState<'grid' | 'list'>(viewModeConfig.defaultMode as 'grid' | 'list');
  const [learnedSpells, setLearnedSpells] = useState<Set<number>>(new Set());
  
  // Fetch game spell data (filtered character spells) - only if character exists
  const { data: gameSpellsData, loading: gameSpellsLoading, error: spellsError } = useSpells(character?.id);
  
  // Note: We now get spell data directly from the new filtered endpoint
  // No need to use the subsystem since we're getting both available spells and character spell state
  // from the same optimized endpoint

  const [spellSlots] = useState<SpellSlot[]>([
    { level: 0, total: 4 },
    { level: 1, total: 3 },
    { level: 2, total: 2 },
    { level: 3, total: 1 },
    { level: 4, total: 1 },
  ]);

  // TODO: Get character's known spells from character data or separate endpoint
  // For now, all spells are shown as available to learn

  // Merge game spells with character spell data
  const knownSpells = useMemo<Spell[]>(() => {
    if (!gameSpellsData || !Array.isArray(gameSpellsData)) return [];
    
    return gameSpellsData.map(spell => ({
      ...spell,
      // Map compatibility fields for existing filter logic
      innate_level: spell.level,
      school: spell.school_name,
      // Add UI state
      isLearned: learnedSpells.has(spell.id),
    }));
  }, [gameSpellsData, learnedSpells]);

  const [filters, setFilters] = useState({
    search: '',
    level: 'all' as number | 'all',
    school: 'all',
    onlyLearned: false,
  });

  // Get unique schools from spell data
  const schools = useMemo(() => {
    const schoolSet = new Set<string>();
    knownSpells.forEach(spell => {
      if (spell.school) schoolSet.add(spell.school);
    });
    return Array.from(schoolSet).sort();
  }, [knownSpells]);

  const filteredSpells = useMemo(() => {
    return knownSpells.filter(spell => {
      if (filters.search && !spell.name.toLowerCase().includes(filters.search.toLowerCase())) {
        return false;
      }
      if (filters.level !== 'all' && filters.level !== spell.innate_level) return false;
      if (filters.school !== 'all' && spell.school !== filters.school) return false;
      if (filters.onlyLearned && !spell.isLearned) return false;
      
      return true;
    });
  }, [knownSpells, filters]);
  
  const isLoading = characterLoading || gameSpellsLoading;
  const error = characterError || spellsError;

  // Early return for loading state
  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
      </div>
    );
  }

  // Early return for error state
  if (error) {
    return (
      <Card variant="error">
        <div className="flex items-center gap-2">
          <AlertCircle className="w-5 h-5 text-error" />
          <p className="text-error">{error?.toString()}</p>
        </div>
      </Card>
    );
  }

  // Early return for no character loaded
  if (!character) {
    return (
      <Card variant="warning">
        <p className="text-muted">No character loaded. Please import a save file to begin.</p>
      </Card>
    );
  }

  const toggleSpellLearning = (spellId: number) => {
    setLearnedSpells(prev => {
      const newSet = new Set(prev);
      if (newSet.has(spellId)) {
        newSet.delete(spellId);
      } else {
        newSet.add(spellId);
      }
      return newSet;
    });
  };

  // Create sections dynamically based on configuration
  const renderSection = (section: typeof spellSections[0]) => {
    switch (section.component) {
      case 'filters':
        return (
          <SpellFilters
            key={section.id}
            filters={filters}
            onFilterChange={setFilters}
            schools={schools}
            spellSlots={spellSlots}
            showResetButton={section.props?.showResetButton}
          />
        );
      case 'header':
        return (
          <SpellHeader
            key={section.id}
            filteredCount={filteredSpells.length}
            totalCount={knownSpells.length}
            learnedCount={filteredSpells.filter(s => s.isLearned).length}
            viewMode={viewMode}
            onViewModeChange={setViewMode}
          />
        );
      case 'spellList':
        return (
          <SpellList
            key={section.id}
            spells={filteredSpells}
            viewMode={viewMode}
            isLoading={isLoading}
            onToggleLearned={toggleSpellLearning}
            defaultExpandedLevels={section.props?.defaultExpandedLevels}
          />
        );
      default:
        return null;
    }
  };

  return (
    <div className="flex gap-4">
      {/* Filters Sidebar */}
      {renderSection(spellSections.find(s => s.component === 'filters')!)}
      
      {/* Main Content */}
      <div className="flex-1 flex flex-col gap-4">
        {/* Header */}
        <Card padding="p-4">
          {renderSection(spellSections.find(s => s.component === 'header')!)}
        </Card>
        
        {/* Spells List */}
        <Card className="flex-1" padding="p-0">
          {renderSection(spellSections.find(s => s.component === 'spellList')!)}
        </Card>
      </div>
    </div>
  );
}