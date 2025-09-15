import React from 'react';
import { Badge } from '@/components/ui/Badge';
import NWN2Icon from '@/components/ui/NWN2Icon';
import GameDataList, { GameDataItem } from '@/components/ui/GameDataList';
import { getSchoolIcon } from './SpellSections';
import { display } from '@/utils/dataHelpers';
import { parseSpellDescription, getSpellMetaTags } from '@/utils/spellParser';

interface Spell extends GameDataItem {
  icon: string;
  icon_url?: string;
  school_id: number;
  school_name: string | null;
  level: number;
  available_classes: string[];
  
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
  
  // Computed properties for compatibility
  school: string | null;
  innate_level: number;
}

interface SpellListProps {
  spells: Spell[];
  isLoading: boolean;
  onToggleLearned: (spellId: number) => void;
  defaultExpandedLevels?: number[];
}

export default function SpellList({
  spells,
  isLoading,
  onToggleLearned,
  defaultExpandedLevels = [0, 1]
}: SpellListProps) {

  // Convert spells to have isActive property and sort by level, then name
  const gameDataSpells = spells
    .map(spell => ({
      ...spell,
      isActive: spell.isLearned
    }))
    .sort((a, b) => {
      // Sort by level first (treating null/undefined as -1)
      const levelA = a.innate_level ?? -1;
      const levelB = b.innate_level ?? -1;
      if (levelA !== levelB) {
        // Put unavailable (-1) at the end
        if (levelA === -1 && levelB !== -1) return 1;
        if (levelB === -1 && levelA !== -1) return -1;
        return levelA - levelB;
      }
      // Then sort by name
      return a.name.localeCompare(b.name);
    });

  // Table renderers for flat table layout
  const renderMain = (spell: Spell) => (
    <div className="game-data-col-main">
      <div className="game-data-content">
        <h4 className="game-data-title">
          {spell.name}
        </h4>
        <div className="game-data-subtitle">
          {spell.school_name && (
            <div className="spell-school-icon shrink-0" title={spell.school_name}>
              {getSchoolIcon(spell.school_name, 'sm')}
            </div>
          )}
          <span>{spell.school_name || 'Unknown School'}</span>
        </div>
      </div>
    </div>
  );

  const renderLevel = (spell: Spell) => {
    const level = spell.innate_level ?? -1;
    const levelText = level === 0 ? 'C' : level === -1 ? '—' : level.toString();
    return (
      <div className="game-data-col-level game-data-table-cell primary">
        {levelText}
      </div>
    );
  };

  const renderRange = (spell: Spell) => {
    const parsed = parseSpellDescription(spell.description);
    return (
      <div className="game-data-col-range game-data-table-cell secondary">
        {display(parsed.range) || display(spell.range) || '-'}
      </div>
    );
  };

  const renderDuration = (spell: Spell) => {
    const parsed = parseSpellDescription(spell.description);
    return (
      <div className="game-data-col-duration game-data-table-cell secondary">
        {display(parsed.duration) || '-'}
      </div>
    );
  };

  const renderSave = (spell: Spell) => {
    const parsed = parseSpellDescription(spell.description);
    const hasSave = parsed.save && parsed.save.toLowerCase() !== 'none';
    return (
      <div className={`game-data-col-save game-data-table-cell ${hasSave ? 'primary has-save' : 'muted'}`}>
        {hasSave ? parsed.save : 'None'}
      </div>
    );
  };

  const renderClasses = (spell: Spell) => (
    <div className="game-data-col-classes">
      <div className="game-data-tags">
        {spell.available_classes.slice(0, 2).map((cls, idx) => (
          <span key={idx} className="game-data-tag">
            {cls}
          </span>
        ))}
        {spell.available_classes.length > 2 && (
          <span className="game-data-tag secondary">
            +{spell.available_classes.length - 2}
          </span>
        )}
      </div>
    </div>
  );

  return (
    <GameDataList
      items={gameDataSpells}
      isLoading={isLoading}
      onItemAction={onToggleLearned}
      actionLabel={(spell) => spell.isLearned ? 'Learned' : 'Learn'}
      actionVariant={(spell) => spell.isLearned ? 'primary' : 'outline'}
      showHeader={true}
      headerLabels={{
        main: 'Spell',
        level: 'Lvl',
        range: 'Range',
        duration: 'Duration',
        save: 'Save',
        tags: 'Classes',
        action: 'Action'
      }}
      renderMain={renderMain}
      renderLevel={renderLevel}
      renderRange={renderRange}
      renderDuration={renderDuration}
      renderSave={renderSave}
      renderTags={renderClasses}
      emptyMessage="No spells found"
    />
  );
}