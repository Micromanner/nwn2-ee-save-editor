'use client';

import React, { memo } from 'react';
import { ScrollArea } from '@/components/ui/ScrollArea';
import { Button } from '@/components/ui/Button';
import { SpellCard } from './SpellCard';
import type { SpellInfo } from './types';
import type { SpellTab } from './SpellNavBar';
import { cn } from '@/lib/utils';

export interface SpellTabContentProps {
  activeTab: SpellTab;

  mySpells: SpellInfo[];
  allSpells: SpellInfo[];

  ownedSpellIds: Set<number>;

  onAddSpell: (spellId: number, classIndex: number) => void;
  onRemoveSpell: (spellId: number, classIndex: number) => void;
  onLoadSpellDetails?: (spell: SpellInfo) => Promise<SpellInfo | null>;

  currentPage: number;
  totalPages: number;
  hasNext: boolean;
  hasPrevious: boolean;
  onPageChange: (page: number) => void;

  casterClasses: Array<{index: number; name: string}>;
}

function MySpellsTabComponent({
  spells,
  onRemove,
  onLoadDetails,
  casterClasses,
}: {
  spells: SpellInfo[];
  onRemove: (spellId: number, classIndex: number) => void;
  onLoadDetails?: (spell: SpellInfo) => Promise<SpellInfo | null>;
  casterClasses: Array<{index: number; name: string}>;
}) {
  if (spells.length === 0) {
    return (
      <div className="flex items-center justify-center h-64">
        <p className="text-[rgb(var(--color-text-muted))]">
          No spells found. Try adjusting your filters.
        </p>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 gap-3">
      {spells.map((spell) => (
        <SpellCard
          key={`${spell.id}-${spell.level}`}
          spell={spell}
          isOwned={true}
          onRemove={onRemove}
          onLoadDetails={onLoadDetails}
          casterClasses={casterClasses}
        />
      ))}
    </div>
  );
}

const MySpellsTab = memo(MySpellsTabComponent);

function AvailableSpellsTabComponent({
  spells,
  ownedSpellIds,
  onAdd,
  onLoadDetails,
  casterClasses,
}: {
  spells: SpellInfo[];
  ownedSpellIds: Set<number>;
  onAdd: (spellId: number, classIndex: number) => void;
  onLoadDetails?: (spell: SpellInfo) => Promise<SpellInfo | null>;
  casterClasses: Array<{index: number; name: string}>;
}) {
  if (spells.length === 0) {
    return (
      <div className="flex items-center justify-center h-64">
        <p className="text-[rgb(var(--color-text-muted))]">
          No available spells found. Try adjusting your filters.
        </p>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 gap-3">
      {spells.map((spell) => (
        <SpellCard
          key={`${spell.id}-${spell.level}`}
          spell={spell}
          isOwned={ownedSpellIds.has(spell.id)}
          onAdd={onAdd}
          onLoadDetails={onLoadDetails}
          casterClasses={casterClasses}
        />
      ))}
    </div>
  );
}

const AvailableSpellsTab = memo(AvailableSpellsTabComponent);

function SpellTabContentComponent({
  activeTab,
  mySpells,
  allSpells,
  ownedSpellIds,
  onAddSpell,
  onRemoveSpell,
  onLoadSpellDetails,
  currentPage,
  totalPages,
  hasNext,
  hasPrevious,
  onPageChange,
  casterClasses,
}: SpellTabContentProps) {

  return (
    <div className="flex flex-col flex-1 bg-[rgb(var(--color-surface-1))] border border-[rgb(var(--color-surface-border))] rounded-lg overflow-hidden">
      <ScrollArea className="flex-1">
        <div className="p-4">
          <div className={cn('transition-opacity duration-200', activeTab === 'my-spells' ? 'block' : 'hidden')}>
            <MySpellsTab
              spells={mySpells}
              onRemove={onRemoveSpell}
              onLoadDetails={onLoadSpellDetails}
              casterClasses={casterClasses}
            />
          </div>

          <div className={cn('transition-opacity duration-200', activeTab === 'all-spells' ? 'block' : 'hidden')}>
            <AvailableSpellsTab
              spells={allSpells}
              ownedSpellIds={ownedSpellIds}
              onAdd={onAddSpell}
              onLoadDetails={onLoadSpellDetails}
              casterClasses={casterClasses}
            />
          </div>
        </div>
      </ScrollArea>

      {activeTab === 'all-spells' && totalPages > 1 && (
        <div className="border-t border-[rgb(var(--color-surface-border))] bg-[rgb(var(--color-surface-1))] px-4 py-3">
          <div className="flex items-center justify-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => onPageChange(currentPage - 1)}
              disabled={!hasPrevious}
            >
              Previous
            </Button>
            <span className="text-sm text-[rgb(var(--color-text-primary))] px-2">
              Page {currentPage} of {totalPages}
            </span>
            <Button
              variant="outline"
              size="sm"
              onClick={() => onPageChange(currentPage + 1)}
              disabled={!hasNext}
            >
              Next
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

export const SpellTabContent = memo(SpellTabContentComponent);
