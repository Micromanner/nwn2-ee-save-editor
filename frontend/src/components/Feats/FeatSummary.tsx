'use client';

import { Card } from '@/components/ui/Card';
import { Info, Shield } from 'lucide-react';
import { display, formatNumber } from '@/utils/dataHelpers';

interface FeatInfo {
  id: number;
  label: string;
  name: string;
  type: number;
  protected: boolean;
  custom: boolean;
}

interface FeatsState {
  current_feats: {
    total: number;
    protected: FeatInfo[];
    class_feats: FeatInfo[];
    general_feats: FeatInfo[];
    custom_feats: FeatInfo[];
  };
  available_feats: FeatInfo[];
}

interface FeatSummaryProps {
  featsData: FeatsState | null;
  availableFeatsCount: number;
}

export default function FeatSummary({ featsData, availableFeatsCount }: FeatSummaryProps) {
  if (!featsData) return null;

  return (
    <Card className="p-4" backgroundColor="rgb(var(--color-surface-1))" shadow="shadow-elevation-2">
      <h4 className="font-semibold text-sm mb-3 flex items-center gap-2">
        <Info className="w-4 h-4 text-[rgb(var(--color-text-muted))]" />
        Feat Summary
      </h4>
      <div className="space-y-3">
        <div className="space-y-1">
          <div className="flex justify-between items-center">
            <span className="text-xs text-[rgb(var(--color-text-muted))]">Current / Available</span>
            <span className="text-sm font-bold">
              {display(featsData.current_feats.total)} / {formatNumber(availableFeatsCount)}
            </span>
          </div>
          <div className="w-full bg-[rgb(var(--color-surface-3))] rounded-full h-2">
            <div 
              className="bg-[rgb(var(--color-primary))] h-2 rounded-full transition-all"
              style={{ 
                width: `${Math.min(100, (featsData.current_feats.total / (featsData.current_feats.total + availableFeatsCount)) * 100)}%` 
              }}
            />
          </div>
        </div>
        
        <div className="space-y-2">
          <div className="flex justify-between items-center">
            <span className="text-xs text-[rgb(var(--color-text-muted))] flex items-center gap-1">
              <Shield className="w-3 h-3" />
              Protected
            </span>
            <span className="text-sm font-semibold">
              {display(featsData.current_feats.protected.length)}
            </span>
          </div>
          <div className="flex justify-between items-center">
            <span className="text-xs text-[rgb(var(--color-text-muted))]">Class Feats</span>
            <span className="text-sm font-semibold">
              {display(featsData.current_feats.class_feats.length)}
            </span>
          </div>
          <div className="flex justify-between items-center">
            <span className="text-xs text-[rgb(var(--color-text-muted))]">General Feats</span>
            <span className="text-sm font-semibold">
              {display(featsData.current_feats.general_feats.length)}
            </span>
          </div>
          {featsData.current_feats.custom_feats.length > 0 && (
            <div className="flex justify-between items-center">
              <span className="text-xs text-[rgb(var(--color-text-muted))]">Custom Content</span>
              <span className="text-sm font-semibold">
                {display(featsData.current_feats.custom_feats.length)}
              </span>
            </div>
          )}
        </div>
      </div>
    </Card>
  );
}