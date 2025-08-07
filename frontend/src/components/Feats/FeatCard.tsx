'use client';

import { memo } from 'react';
import { Card, CardContent } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { Check, Shield, Swords, Sparkles, Sun, Zap, Info, X, Loader2 } from 'lucide-react';
import { display } from '@/utils/dataHelpers';
import NWN2Icon from '@/components/ui/NWN2Icon';

interface Prerequisite {
  type: 'ability' | 'feat' | 'class' | 'level' | 'bab' | 'spell_level';
  description: string;
  required_value?: number;
  current_value?: number;
  feat_id?: number;
  class_id?: number;
  met: boolean;
}

interface DetailedPrerequisites {
  requirements: Prerequisite[];
  met: string[];
  unmet: string[];
}

interface FeatInfo {
  id: number;
  label: string;
  name: string;
  type: number;
  protected: boolean;
  custom: boolean;
  description?: string;
  icon?: string;
  prerequisites?: {
    abilities: Record<string, number>;
    feats: number[];
    class: number;
    level: number;
    bab: number;
    spell_level: number;
  };
  can_take?: boolean;
  missing_requirements?: string[];
  has_feat?: boolean;
  detailed_prerequisites?: DetailedPrerequisites;
}

interface FeatCardProps {
  feat: FeatInfo;
  isActive?: boolean;
  viewMode: 'grid' | 'list';
  onDetails: (feat: FeatInfo) => void;
  onAdd: (featId: number) => void;
  onRemove: (featId: number) => void;
  validationState?: {
    can_take: boolean;
    reason: string;
    has_feat: boolean;
    missing_requirements: string[];
  };
  isValidating?: boolean;
  onValidate?: (featId: number) => void;
}

function FeatCard({ 
  feat, 
  isActive = false, 
  viewMode, 
  onDetails, 
  onAdd, 
  onRemove,
  validationState,
  isValidating = false,
  onValidate
}: FeatCardProps) {
  // Trigger validation on hover if not already validated
  const handleMouseEnter = () => {
    if (!isActive && !validationState && !isValidating && onValidate) {
      onValidate(feat.id);
    }
  };

  // Get validation status for visual indicators
  const getValidationIcon = () => {
    if (isValidating) {
      return <Loader2 className="w-3 h-3 animate-spin" />;
    }
    if (validationState) {
      if (validationState.has_feat) {
        return <Check className="w-3 h-3 text-green-500" />;
      }
      if (validationState.can_take) {
        return <Check className="w-3 h-3 text-green-500" />;
      }
      return <X className="w-3 h-3 text-red-500" />;
    }
    return null;
  };

  const getValidationTooltip = () => {
    if (validationState && !validationState.can_take && validationState.missing_requirements.length > 0) {
      return validationState.missing_requirements.join(', ');
    }
    return '';
  };

  // Map feat types - based on NWN2 feat types
  const getFeatTypeName = (type: number): string => {
    switch (type) {
      case 1: return 'General';
      case 2: return 'Combat';
      case 8: return 'Metamagic';
      case 16: return 'Divine';
      case 32: return 'Epic';
      case 64: return 'Class';
      default: return 'General';
    }
  };

  const getTypeIcon = (type: number) => {
    switch (type) {
      case 2: return <Swords className="w-4 h-4" />; // Combat
      case 8: return <Sparkles className="w-4 h-4" />; // Metamagic
      case 16: return <Sun className="w-4 h-4" />; // Divine
      case 32: return <Zap className="w-4 h-4" />; // Epic
      case 64: return <Shield className="w-4 h-4" />; // Class
      default: return null;
    }
  };

  const getTypeColor = (type: number) => {
    switch (type) {
      case 2: return 'destructive'; // Combat
      case 8: return 'secondary'; // Metamagic
      case 16: return 'default'; // Divine
      case 32: return 'outline'; // Epic
      case 64: return 'default'; // Class
      default: return 'default';
    }
  };

  if (viewMode === 'list') {
    // Condensed list view
    return (
      <div 
        className={`flex items-center gap-3 px-3 py-2 rounded hover:bg-[rgb(var(--color-surface-2))] transition-colors ${
          isActive ? 'bg-[rgb(var(--color-primary)/0.05)]' : ''
        }`}
        onMouseEnter={handleMouseEnter}
        title={getValidationTooltip()}>
        <NWN2Icon icon={`ife_${feat.label.toLowerCase()}`} size="sm" className="shrink-0" />
        <div className="flex-1 flex items-center gap-3 min-w-0">
          <h4 className="font-medium text-sm text-[rgb(var(--color-text-primary))] w-48 truncate">
            {display(feat.label)}
          </h4>
          <Badge variant={getTypeColor(feat.type)} className="text-xs shrink-0">
            <span className="flex items-center gap-1">
              {getTypeIcon(feat.type)}
              {getFeatTypeName(feat.type)}
            </span>
          </Badge>
          {feat.protected && (
            <Badge variant="outline" className="text-xs shrink-0">
              <Shield className="w-3 h-3 mr-1" />
              Protected
            </Badge>
          )}
          {feat.custom && (
            <Badge variant="secondary" className="text-xs shrink-0">
              Custom
            </Badge>
          )}
          {/* Validation indicator */}
          {!isActive && getValidationIcon() && (
            <div className="flex items-center">
              {getValidationIcon()}
            </div>
          )}
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <Button
            size="sm"
            variant="ghost"
            className="text-xs h-7 px-2"
            onClick={() => onDetails(feat)}
          >
            <Info className="w-3 h-3" />
          </Button>
          {isActive ? (
            <Button
              size="sm"
              variant="danger"
              className="text-xs h-7 px-2"
              onClick={() => onRemove(feat.id)}
              disabled={feat.protected}
            >
              Remove
            </Button>
          ) : (
            <Button
              size="sm"
              variant="primary"
              className="text-xs h-7 px-2"
              onClick={() => onAdd(feat.id)}
              disabled={validationState ? !validationState.can_take : false}
            >
              Learn
            </Button>
          )}
        </div>
      </div>
    );
  }

  // Grid view (card)
  return (
    <Card 
      className={`${isActive ? 'ring-2 ring-[rgb(var(--color-primary)/0.5)]' : ''} 
                  hover:shadow-elevation-3 transition-all`}
      onMouseEnter={handleMouseEnter}
      title={getValidationTooltip()}
    >
      <CardContent className="p-3">
        <div className="flex items-start gap-3">
          <NWN2Icon icon={`ife_${feat.label.toLowerCase()}`} size="md" className="shrink-0" />
          <div className="flex-1">
            <div className="flex items-center gap-2 mb-1">
              <h4 className="font-medium text-sm text-[rgb(var(--color-text-primary))] line-clamp-1">
                {display(feat.label)}
              </h4>
              <Badge variant={getTypeColor(feat.type)} className="text-xs">
                <span className="flex items-center gap-1">
                  {getTypeIcon(feat.type)}
                  {getFeatTypeName(feat.type)}
                </span>
              </Badge>
              {isActive && (
                <Badge variant="default" className="text-xs">
                  <Check className="w-3 h-3" />
                </Badge>
              )}
              {/* Validation indicator */}
              {!isActive && getValidationIcon() && (
                <div className="flex items-center">
                  {getValidationIcon()}
                </div>
              )}
            </div>
            {feat.protected && (
              <div className="flex items-center gap-2 mt-2">
                <Badge variant="outline" className="text-xs">
                  <Shield className="w-3 h-3 mr-1" />
                  Protected
                </Badge>
              </div>
            )}
            {feat.custom && (
              <Badge variant="secondary" className="text-xs">
                Custom Content
              </Badge>
            )}
          </div>
          <div className="flex flex-col items-center gap-1">
            <Button
              size="sm"
              variant="ghost"
              className="text-xs"
              onClick={() => onDetails(feat)}
            >
              <Info className="w-3 h-3" />
            </Button>
            {isActive ? (
              <Button
                size="sm"
                variant="danger"
                className="text-xs"
                onClick={() => onRemove(feat.id)}
                disabled={feat.protected}
              >
                Remove
              </Button>
            ) : (
              <Button
                size="sm"
                variant="primary"
                className="text-xs"
                onClick={() => onAdd(feat.id)}
                disabled={validationState ? !validationState.can_take : false}
              >
                Learn
              </Button>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

export default memo(FeatCard);