
import React, { memo, useState } from 'react';
import { ChevronDown, ChevronUp, Sparkles, Check } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/Select';
import NWN2Icon from '@/components/ui/NWN2Icon';
import { display } from '@/utils/dataHelpers';
import { parseSpellDescription } from '@/utils/spellParser';
import { getSchoolIcon } from './SpellSections';
import type { SpellInfo } from './types';
import { cn } from '@/lib/utils';

export interface SpellCardProps {
  spell: SpellInfo;
  isOwned: boolean;
  onAdd?: (spellId: number, classIndex: number, spellLevel: number) => void;
  onRemove?: (spellId: number, classIndex: number, spellLevel: number) => void;
  onLoadDetails?: (spell: SpellInfo) => Promise<SpellInfo | null>;
  casterClasses: Array<{index: number; name: string; class_id: number; can_edit_spells: boolean}>;
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

  const editableClasses = casterClasses.filter(cls => cls.can_edit_spells);
  const [selectedClassIndex, setSelectedClassIndex] = useState<number>(
    editableClasses.length > 0 ? editableClasses[0].index : 0
  );

  const spellClassCanEdit = spell.class_id !== undefined
    ? casterClasses.find(cls => cls.class_id === spell.class_id)?.can_edit_spells ?? false
    : false;

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
                {spell.is_domain_spell && (
                  <Badge variant="outline" className="text-amber-500 border-amber-500">
                    Domain
                  </Badge>
                )}
                {isOwned && (
                  <Badge variant="default" className="bg-[rgb(var(--color-primary))] text-white flex items-center gap-1">
                    <Check className="w-3 h-3" />
                    {spell.memorized_count && spell.memorized_count > 1
                      ? `x${spell.memorized_count}`
                      : 'Active'}
                  </Badge>
                )}
              </div>
            </div>

            <div className="flex items-center gap-2 flex-shrink-0">
              {!isOwned && onAdd && editableClasses.length > 0 && (
                <div className="flex items-center gap-2">
                  <div onClick={(e) => e.stopPropagation()}>
                    <Select
                      value={selectedClassIndex.toString()}
                      onValueChange={(value) => setSelectedClassIndex(Number(value))}
                      disabled={editableClasses.length === 1}
                    >
                      <SelectTrigger className="w-[120px] h-8 text-xs">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {editableClasses.map((cls) => (
                          <SelectItem key={cls.index} value={cls.index.toString()}>
                            {cls.name}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                   <Button
                    variant="outline"
                    size="sm"
                    onClick={(e) => {
                      e.stopPropagation();
                      onAdd(spell.id, selectedClassIndex, spell.level);
                    }}
                  >
                    Add
                  </Button>
                </div>
              )}
              {isOwned && onRemove && spellClassCanEdit && (
                 <Button
                  variant="ghost"
                  size="sm"
                  onClick={(e) => {
                    e.stopPropagation();
                    const classIndex = casterClasses.find(cls => cls.class_id === spell.class_id)?.index ?? selectedClassIndex;
                    onRemove(spell.id, classIndex, spell.level);
                  }}
                >
                  Remove
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

          {!isLoadingDetails && displaySpell.description && (
            <div className="px-4">
              <p className="text-sm text-[rgb(var(--color-text-secondary))] leading-relaxed whitespace-pre-wrap">
                {stripHtmlTags(displaySpell.description)}
              </p>
            </div>
          )}
        </div>
      )}
    </Card>
  );
}

export const SpellCard = memo(SpellCardComponent);
