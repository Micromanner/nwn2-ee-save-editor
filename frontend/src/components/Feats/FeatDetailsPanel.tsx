'use client';

import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { Shield, Swords, Sparkles, Sun, Zap } from 'lucide-react';
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
  feat_id?: number;
  label: string;
  name: string;
  type: number;
  protected: boolean;
  custom: boolean;
  description?: string;
  icon?: string;
  prerequisites?: Record<string, unknown>;
  can_take?: boolean;
  missing_requirements?: string[];
  has_feat?: boolean;
  detailed_prerequisites?: DetailedPrerequisites;
}

interface FeatDetailsPanelProps {
  selectedFeat: FeatInfo | null;
  featDetails: FeatInfo | null;
  loadingDetails: boolean;
  onClose: () => void;
  onAdd: (featId: number) => void;
  onRemove: (featId: number) => void;
}

export default function FeatDetailsPanel({ 
  selectedFeat, 
  featDetails, 
  loadingDetails, 
  onClose, 
  onAdd, 
  onRemove 
}: FeatDetailsPanelProps) {
  if (!selectedFeat) return null;
  
  const details = featDetails || selectedFeat;

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
  
  return (
    <Card className="w-96" padding="p-0">
      <div className="p-4">
        <div className="flex items-center justify-between mb-4">
          <h3 className="font-semibold">Feat Details</h3>
          <Button
            size="sm"
            variant="ghost"
            onClick={onClose}
          >
            ×
          </Button>
        </div>
        
        {loadingDetails ? (
          <div className="flex items-center justify-center h-64">
            <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-[rgb(var(--color-primary))]"></div>
          </div>
        ) : (
          <div className="space-y-4">
            {/* Feat Header */}
            <div className="flex items-start gap-3">
              <NWN2Icon icon={`ife_${details.label.toLowerCase()}`} size="lg" />
              <div className="flex-1">
                <h4 className="font-semibold text-lg">{display(details.label)}</h4>
                <Badge variant={getTypeColor(details.type)} className="text-xs mt-1">
                  <span className="flex items-center gap-1">
                    {getTypeIcon(details.type)}
                    {getFeatTypeName(details.type)}
                  </span>
                </Badge>
              </div>
            </div>
            
            {/* Description */}
            {details.description && (
              <div>
                <h5 className="font-medium mb-2">Description</h5>
                <p className="text-sm text-[rgb(var(--color-text-secondary))] whitespace-pre-wrap leading-relaxed">
                  {details.description}
                </p>
              </div>
            )}
            
            {/* Prerequisites */}
            {details.detailed_prerequisites && (
              <div>
                <h5 className="font-medium mb-2">Prerequisites</h5>
                {details.detailed_prerequisites.requirements.length === 0 ? (
                  <p className="text-sm text-[rgb(var(--color-text-muted))]">None</p>
                ) : (
                  <div className="space-y-2">
                    {details.detailed_prerequisites.requirements.map((req, index) => (
                      <div
                        key={index}
                        className={`flex items-center gap-2 text-sm p-2 rounded ${
                          req.met 
                            ? 'bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-300' 
                            : 'bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-300'
                        }`}
                      >
                        <span className={`w-2 h-2 rounded-full ${req.met ? 'bg-green-500' : 'bg-red-500'}`} />
                        <span className="flex-1">{req.description}</span>
                        {req.current_value !== undefined && req.required_value !== undefined && (
                          <span className="text-xs">
                            {req.current_value}/{req.required_value}
                          </span>
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
            
            {/* Action Buttons */}
            <div className="flex gap-2 pt-4 border-t">
              {details.has_feat ? (
                <Button
                  variant="danger"
                  className="flex-1"
                  onClick={() => onRemove(details.id)}
                  disabled={details.protected}
                >
                  {details.protected ? 'Protected' : 'Remove Feat'}
                </Button>
              ) : (
                <Button
                  variant="primary"
                  className="flex-1"
                  onClick={() => onAdd(details.id)}
                  disabled={details.can_take === false}
                >
                  {details.can_take === false ? 'Requirements Not Met' : 'Learn Feat'}
                </Button>
              )}
            </div>
          </div>
        )}
      </div>
    </Card>
  );
}