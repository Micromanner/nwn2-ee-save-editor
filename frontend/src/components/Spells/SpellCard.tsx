'use client';

import React, { memo, useState } from 'react';
import { ChevronDown, ChevronUp, Sparkles } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import NWN2Icon from '@/components/ui/NWN2Icon';
import { display } from '@/utils/dataHelpers';
import { parseSpellDescription } from '@/utils/spellParser';
import { getSchoolIcon } from './SpellSections';
import type { SpellInfo } from './types';
import { cn } from '@/lib/utils';

export interface SpellCardProps {
  spell: SpellInfo;
  isOwned: boolean;
  onAdd?: (spellId: number, classIndex: number) => void;
  onRemove?: (spellId: number, classIndex: number) => void;
  onLoadDetails?: (spell: SpellInfo) => Promise<SpellInfo | null>;
  casterClasses: Array<{index: number; name: string}>;
}

function getSchoolColorClass(schoolName?: string): string {
  if (!schoolName) return 'bg-gray-500';

  const school = schoolName.toLowerCase();

  if (school.includes('abjuration')) return 'bg-blue-500';
  if (school.includes('conjuration')) return 'bg-purple-500';
  if (school.includes('divination')) return 'bg-cyan-500';
  if (school.includes('enchantment')) return 'bg-pink-500';
  if (school.includes('evocation')) return 'bg-red-500';
  if (school.includes('illusion')) return 'bg-indigo-500';
  if (school.includes('necromancy')) return 'bg-gray-600';
  if (school.includes('transmutation')) return 'bg-green-500';
  if (school.includes('universal')) return 'bg-yellow-500';

  return 'bg-gray-500';
}

function stripHtmlTags(text: string): string {
  return text.replace(/<\/?[^>]+(>|$)/g, '');
}

function SpellCardComponent({
  spell,
  isOwned,
  onAdd,
  onRemove,
  onLoadDetails,
  casterClasses
}: SpellCardProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [detailedSpell, setDetailedSpell] = useState<SpellInfo | null>(null);
  const [isLoadingDetails, setIsLoadingDetails] = useState(false);
  const [selectedClassIndex, setSelectedClassIndex] = useState<number>(
    casterClasses.length > 0 ? casterClasses[0].index : 0
  );

  const handleToggleExpand = async () => {
    if (!isExpanded && onLoadDetails && !detailedSpell) {
      setIsLoadingDetails(true);
      const details = await onLoadDetails(spell);
      setDetailedSpell(details);
      setIsLoadingDetails(false);
    }
    setIsExpanded(!isExpanded);
  };

  const displaySpell = detailedSpell || spell;
  const schoolColorClass = getSchoolColorClass(displaySpell.school_name);
  const levelText = displaySpell.level === 0 ? 'Cantrip' : `Level ${displaySpell.level}`;

  const parsedDescription = parseSpellDescription(displaySpell.description || '');

  return (
    <Card
      variant="interactive"
      className={cn(
        'transition-all duration-200',
        isOwned && 'border-[rgb(var(--color-primary)/0.3)]'
      )}
    >
      <div className="flex items-start gap-3">
        <div className="flex-shrink-0">
          {displaySpell.icon ? (
            <NWN2Icon icon={displaySpell.icon} size="lg" />
          ) : (
            <div className="w-10 h-10 rounded bg-[rgb(var(--color-surface-2))] flex items-center justify-center">
              <Sparkles className="w-5 h-5 text-[rgb(var(--color-text-muted))]" />
            </div>
          )}
        </div>

        <div className="flex-1 min-w-0">
          <div className="flex items-start justify-between gap-2 mb-2">
            <div className="flex-1 min-w-0">
              <h3 className="text-base font-semibold text-[rgb(var(--color-text-primary))] truncate">
                {spell.name}
              </h3>
              <div className="flex items-center gap-2 mt-1 flex-wrap">
                <Badge className={cn("flex items-center gap-1 text-white", schoolColorClass)}>
                  {displaySpell.school_name && getSchoolIcon(displaySpell.school_name, 'sm')}
                  {displaySpell.school_name || 'Unknown'}
                </Badge>
                <Badge variant="secondary">
                  {levelText}
                </Badge>
                {isOwned && (
                  <Badge variant="default" className="bg-[rgb(var(--color-primary))] text-white">
                    Known
                  </Badge>
                )}
              </div>
            </div>

            <div className="flex items-center gap-2 flex-shrink-0">
              {!isOwned && onAdd && casterClasses.length > 0 && (
                <div className="flex items-center gap-2">
                  {casterClasses.length > 1 && (
                    <select
                      value={selectedClassIndex}
                      onChange={(e) => setSelectedClassIndex(Number(e.target.value))}
                      className="text-xs px-2 py-1 rounded bg-[rgb(var(--color-surface-2))] border border-[rgb(var(--color-surface-border))] text-[rgb(var(--color-text-primary))]"
                      onClick={(e) => e.stopPropagation()}
                    >
                      {casterClasses.map((cls) => (
                        <option key={cls.index} value={cls.index}>
                          {cls.name}
                        </option>
                      ))}
                    </select>
                  )}
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={(e) => {
                      e.stopPropagation();
                      onAdd(spell.id, selectedClassIndex);
                    }}
                  >
                    Learn
                  </Button>
                </div>
              )}
              {isOwned && onRemove && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={(e) => {
                    e.stopPropagation();
                    onRemove(spell.id, selectedClassIndex);
                  }}
                >
                  Unlearn
                </Button>
              )}
              <Button
                variant="icon-interactive"
                size="icon"
                onClick={handleToggleExpand}
              >
                {isExpanded ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
              </Button>
            </div>
          </div>

          {displaySpell.description && (
            <p className="text-sm text-[rgb(var(--color-text-secondary))] line-clamp-2">
              {display(stripHtmlTags(displaySpell.description).split('\n')[0])}
            </p>
          )}

          <div className="flex items-center gap-3 mt-2 text-xs text-[rgb(var(--color-text-muted))]">
            {parsedDescription.range && (
              <div>
                <span className="font-medium">Range:</span> {parsedDescription.range}
              </div>
            )}
            {parsedDescription.duration && (
              <div>
                <span className="font-medium">Duration:</span> {parsedDescription.duration}
              </div>
            )}
            {parsedDescription.save && parsedDescription.save.toLowerCase() !== 'none' && (
              <div>
                <span className="font-medium">Save:</span> {parsedDescription.save}
              </div>
            )}
            {parsedDescription.spellResistance && (
              <div>
                <span className="font-medium">SR:</span> {parsedDescription.spellResistance}
              </div>
            )}
          </div>
        </div>
      </div>

      {isExpanded && (
        <div className="mt-4 pt-4 border-t border-[rgb(var(--color-surface-border))] space-y-4">
          {isLoadingDetails && (
            <div className="flex items-center justify-center py-4">
              <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-[rgb(var(--color-primary))]"></div>
            </div>
          )}

          {!isLoadingDetails && (
            <>
              {displaySpell.description && (
                <div>
                  <h4 className="text-sm font-semibold text-[rgb(var(--color-text-primary))] mb-2">
                    Description
                  </h4>
                  <p className="text-sm text-[rgb(var(--color-text-secondary))] leading-relaxed whitespace-pre-wrap">
                    {stripHtmlTags(displaySpell.description)}
                  </p>
                </div>
              )}

              <div className="grid grid-cols-2 gap-4">
                {displaySpell.range && (
                  <div>
                    <h5 className="text-xs font-semibold text-[rgb(var(--color-text-primary))] mb-1">
                      Range
                    </h5>
                    <p className="text-sm text-[rgb(var(--color-text-secondary))]">
                      {display(parsedDescription.range) || display(displaySpell.range)}
                    </p>
                  </div>
                )}

                {displaySpell.cast_time && (
                  <div>
                    <h5 className="text-xs font-semibold text-[rgb(var(--color-text-primary))] mb-1">
                      Casting Time
                    </h5>
                    <p className="text-sm text-[rgb(var(--color-text-secondary))]">
                      {display(displaySpell.cast_time)}
                    </p>
                  </div>
                )}

                {parsedDescription.duration && (
                  <div>
                    <h5 className="text-xs font-semibold text-[rgb(var(--color-text-primary))] mb-1">
                      Duration
                    </h5>
                    <p className="text-sm text-[rgb(var(--color-text-secondary))]">
                      {parsedDescription.duration}
                    </p>
                  </div>
                )}

                {displaySpell.components && (
                  <div>
                    <h5 className="text-xs font-semibold text-[rgb(var(--color-text-primary))] mb-1">
                      Components
                    </h5>
                    <p className="text-sm text-[rgb(var(--color-text-secondary))]">
                      {display(displaySpell.components)}
                    </p>
                  </div>
                )}

                {parsedDescription.save && (
                  <div>
                    <h5 className="text-xs font-semibold text-[rgb(var(--color-text-primary))] mb-1">
                      Saving Throw
                    </h5>
                    <p className="text-sm text-[rgb(var(--color-text-secondary))]">
                      {parsedDescription.save}
                    </p>
                  </div>
                )}

                {parsedDescription.spellResistance && (
                  <div>
                    <h5 className="text-xs font-semibold text-[rgb(var(--color-text-primary))] mb-1">
                      Spell Resistance
                    </h5>
                    <p className="text-sm text-[rgb(var(--color-text-secondary))]">
                      {parsedDescription.spellResistance}
                    </p>
                  </div>
                )}
              </div>

              {displaySpell.available_classes && displaySpell.available_classes.length > 0 && (
                <div>
                  <h5 className="text-xs font-semibold text-[rgb(var(--color-text-primary))] mb-2">
                    Available To
                  </h5>
                  <div className="flex flex-wrap gap-1">
                    {displaySpell.available_classes.map((cls, idx) => (
                      <Badge key={idx} variant="secondary" className="text-xs">
                        {cls}
                      </Badge>
                    ))}
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      )}
    </Card>
  );
}

export const SpellCard = memo(SpellCardComponent);
