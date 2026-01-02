'use client';

import React, { memo, useState } from 'react';
import { ChevronDown, ChevronUp, Shield, Swords, Sparkles, Sun, Zap, Check, X, AlertCircle } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import NWN2Icon from '@/components/ui/NWN2Icon';
import { display } from '@/utils/dataHelpers';
import type { FeatInfo, Prerequisite } from './types';
import { cn } from '@/lib/utils';

export interface FeatCardProps {
  feat: FeatInfo;
  isOwned: boolean;
  onAdd?: (featId: number) => void;
  onRemove?: (featId: number) => void;
  onLoadDetails?: (feat: FeatInfo) => Promise<FeatInfo | null>;
  isProtected?: boolean;
}

function getFeatTypeInfo(category?: string): { label: string; icon: React.ReactNode; colorClass: string } {
  const label = category || 'General';

  switch (label) {
    case 'General':
      return { label, icon: <Shield className="w-3 h-3" />, colorClass: 'bg-blue-500' };
    case 'Proficiency':
      return { label, icon: <Swords className="w-3 h-3" />, colorClass: 'bg-gray-500' };
    case 'Skill/Save':
      return { label, icon: <Shield className="w-3 h-3" />, colorClass: 'bg-cyan-500' };
    case 'Metamagic':
      return { label, icon: <Sparkles className="w-3 h-3" />, colorClass: 'bg-purple-500' };
    case 'Divine':
      return { label, icon: <Sun className="w-3 h-3" />, colorClass: 'bg-yellow-500' };
    case 'Epic':
      return { label, icon: <Zap className="w-3 h-3" />, colorClass: 'bg-orange-500' };
    case 'Class':
      return { label, icon: <Shield className="w-3 h-3" />, colorClass: 'bg-green-500' };
    case 'Background':
      return { label, icon: <Shield className="w-3 h-3" />, colorClass: 'bg-teal-500' };
    case 'Spellcasting':
      return { label, icon: <Sparkles className="w-3 h-3" />, colorClass: 'bg-indigo-500' };
    case 'History':
      return { label, icon: <Shield className="w-3 h-3" />, colorClass: 'bg-amber-500' };
    case 'Heritage':
      return { label, icon: <Shield className="w-3 h-3" />, colorClass: 'bg-rose-500' };
    case 'Item Creation':
      return { label, icon: <Sparkles className="w-3 h-3" />, colorClass: 'bg-lime-500' };
    case 'Racial':
      return { label, icon: <Shield className="w-3 h-3" />, colorClass: 'bg-pink-500' };
    default:
      return { label: 'General', icon: <Shield className="w-3 h-3" />, colorClass: 'bg-blue-500' };
  }
}

interface ParsedDescription {
  type?: string;
  prerequisite?: string;
  requiredFor?: string;
  specifics?: string;
  use?: string;
  normal?: string;
  special?: string;
}

function stripHtmlTags(text: string): string {
  return text.replace(/<\/?[^>]+(>|$)/g, '');
}

function parseAndCleanDescription(rawDescription: string): ParsedDescription {
  const stripped = stripHtmlTags(rawDescription);

  const sections: ParsedDescription = {};
  const lines = stripped.split('\n').map(l => l.trim()).filter(l => l);

  for (const line of lines) {
    if (line.startsWith('Type of Feat:')) {
      sections.type = line.replace('Type of Feat:', '').trim();
    } else if (line.startsWith('Prerequisite:')) {
      sections.prerequisite = line.replace('Prerequisite:', '').trim();
    } else if (line.startsWith('Required for:')) {
      sections.requiredFor = line.replace('Required for:', '').trim();
    } else if (line.startsWith('Specifics:')) {
      sections.specifics = line.replace('Specifics:', '').trim();
    } else if (line.startsWith('Use:')) {
      sections.use = line.replace('Use:', '').trim();
    } else if (line.startsWith('Normal:')) {
      sections.normal = line.replace('Normal:', '').trim();
    } else if (line.startsWith('Special:')) {
      sections.special = line.replace('Special:', '').trim();
    }
  }

  return sections;
}

function PrerequisiteItem({ prereq }: { prereq: Prerequisite }) {
  const Icon = prereq.met ? Check : X;
  const colorClass = prereq.met ? 'text-[rgb(var(--color-success))]' : 'text-[rgb(var(--color-error))]';

  return (
    <div className="flex items-start gap-2 text-sm">
      <Icon className={cn('w-4 h-4 mt-0.5 flex-shrink-0', colorClass)} />
      <div className="flex-1">
        <span className={prereq.met ? 'text-[rgb(var(--color-text-secondary))]' : 'text-[rgb(var(--color-text-primary))]'}>
          {prereq.description}
        </span>
        {prereq.required_value !== undefined && prereq.current_value !== undefined && (
          <div className="mt-1">
            <div className="flex items-center gap-2">
              <div className="flex-1 bg-[rgb(var(--color-surface-3))] rounded-full h-1.5 overflow-hidden">
                <div
                  className={cn(
                    'h-full transition-all',
                    prereq.met ? 'bg-[rgb(var(--color-success))]' : 'bg-[rgb(var(--color-primary))]'
                  )}
                  style={{ width: `${Math.min((prereq.current_value / prereq.required_value) * 100, 100)}%` }}
                />
              </div>
              <span className="text-xs text-[rgb(var(--color-text-muted))]">
                {prereq.current_value}/{prereq.required_value}
              </span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function FeatCardComponent({ feat, isOwned, onAdd, onRemove, onLoadDetails, isProtected = false }: FeatCardProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [detailedFeat, setDetailedFeat] = useState<FeatInfo | null>(null);
  const [isLoadingDetails, setIsLoadingDetails] = useState(false);

  const typeInfo = getFeatTypeInfo(feat.category);

  const handleToggleExpand = async () => {
    if (!isExpanded && onLoadDetails && !detailedFeat) {
      setIsLoadingDetails(true);
      const details = await onLoadDetails(feat);
      setDetailedFeat(details);
      setIsLoadingDetails(false);
    }
    setIsExpanded(!isExpanded);
  };

  const displayFeat = detailedFeat || feat;
  const hasPrerequisites = displayFeat.detailed_prerequisites?.requirements && displayFeat.detailed_prerequisites.requirements.length > 0;

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
          <NWN2Icon icon={`ife_${feat.label?.toLowerCase() || ''}`} size="lg" />
        </div>

        <div className="flex-1 min-w-0">
          <div className="flex items-start justify-between gap-2 mb-2">
            <div className="flex-1 min-w-0">
              <h3 className="text-base font-semibold text-[rgb(var(--color-text-primary))] truncate">
                {feat.name}
              </h3>
              <div className="flex items-center gap-2 mt-1">
                <Badge className={cn("flex items-center gap-1 text-white", typeInfo.colorClass)}>
                  {typeInfo.icon}
                  {typeInfo.label}
                </Badge>
                {isOwned && (
                  <Badge variant="default" className="bg-[rgb(var(--color-primary))] text-white flex items-center gap-1">
                    <Check className="w-3 h-3" />
                    Active
                  </Badge>
                )}
                {isProtected && (
                  <Badge variant="outline" className="text-[rgb(var(--color-warning))]">
                    Protected
                  </Badge>
                )}
                {feat.custom && (
                  <Badge variant="secondary">Custom</Badge>
                )}
              </div>
            </div>

            <div className="flex items-center gap-2 flex-shrink-0">
              {!isOwned && onAdd && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={(e) => {
                    e.stopPropagation();
                    onAdd(feat.id);
                  }}
                  disabled={feat.can_take === false}
                >
                  Add
                </Button>
              )}
              {isOwned && onRemove && !isProtected && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={(e) => {
                    e.stopPropagation();
                    onRemove(feat.id);
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



          {feat.can_take === false && feat.missing_requirements && feat.missing_requirements.length > 0 && !isExpanded && (
            <div className="mt-2 flex items-start gap-2 text-xs text-[rgb(var(--color-warning))]">
              <AlertCircle className="w-3 h-3 mt-0.5 flex-shrink-0" />
              <span>{feat.missing_requirements[0]}</span>
            </div>
          )}
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
              {displayFeat.description && (() => {
                const parsed = parseAndCleanDescription(displayFeat.description);
                const hasStructuredSections = !!(
                  parsed.specifics ||
                  parsed.requiredFor ||
                  parsed.use ||
                  parsed.normal ||
                  parsed.special
                );

                if (hasStructuredSections) {
                  return (
                    <div className="space-y-3">
                      {parsed.specifics && (
                        <div>
                          <h4 className="text-sm font-semibold text-[rgb(var(--color-text-primary))] mb-1">
                            Effect
                          </h4>
                          <p className="text-sm text-[rgb(var(--color-text-secondary))] leading-relaxed">
                            {parsed.specifics}
                          </p>
                        </div>
                      )}

                      {parsed.requiredFor && (
                        <div>
                          <h4 className="text-sm font-semibold text-[rgb(var(--color-text-primary))] mb-1">
                            Required For
                          </h4>
                          <p className="text-sm text-[rgb(var(--color-text-secondary))]">
                            {parsed.requiredFor}
                          </p>
                        </div>
                      )}

                      {parsed.use && (
                        <div>
                          <h4 className="text-sm font-semibold text-[rgb(var(--color-text-primary))] mb-1">
                            Usage
                          </h4>
                          <p className="text-sm text-[rgb(var(--color-text-secondary))]">
                            {parsed.use}
                          </p>
                        </div>
                      )}

                      {parsed.normal && (
                        <div>
                          <h4 className="text-sm font-semibold text-[rgb(var(--color-text-primary))] mb-1">
                            Normal
                          </h4>
                          <p className="text-sm text-[rgb(var(--color-text-secondary))]">
                            {parsed.normal}
                          </p>
                        </div>
                      )}

                      {parsed.special && (
                        <div>
                          <h4 className="text-sm font-semibold text-[rgb(var(--color-text-primary))] mb-1">
                            Special
                          </h4>
                          <p className="text-sm text-[rgb(var(--color-text-secondary))]">
                            {parsed.special}
                          </p>
                        </div>
                      )}
                    </div>
                  );
                } else {
                  return (
                    <div>
                      <h4 className="text-sm font-semibold text-[rgb(var(--color-text-primary))] mb-2">
                        Description
                      </h4>
                      <p className="text-sm text-[rgb(var(--color-text-secondary))] leading-relaxed whitespace-pre-wrap">
                        {stripHtmlTags(displayFeat.description)}
                      </p>
                    </div>
                  );
                }
              })()}

              {hasPrerequisites && (
                <div>
                  <h4 className="text-sm font-semibold text-[rgb(var(--color-text-primary))] mb-2">
                    Prerequisites
                  </h4>
                  <div className="space-y-2">
                    {displayFeat.detailed_prerequisites!.requirements.map((prereq, idx) => (
                      <PrerequisiteItem key={idx} prereq={prereq} />
                    ))}
                  </div>
                </div>
              )}

              {displayFeat.can_take === false && displayFeat.missing_requirements && displayFeat.missing_requirements.length > 0 && (
                <div>
                  <h4 className="text-sm font-semibold text-[rgb(var(--color-error))] mb-2">
                    Missing Requirements
                  </h4>
                  <ul className="space-y-1">
                    {displayFeat.missing_requirements.map((req, idx) => (
                      <li key={idx} className="text-sm text-[rgb(var(--color-text-secondary))] flex items-start gap-2">
                        <X className="w-4 h-4 text-[rgb(var(--color-error))] flex-shrink-0 mt-0.5" />
                        {req}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </>
          )}
        </div>
      )}
    </Card>
  );
}

export const FeatCard = memo(FeatCardComponent);
