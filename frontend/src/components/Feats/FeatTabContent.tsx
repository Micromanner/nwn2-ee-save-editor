'use client';

import React, { memo } from 'react';
import { ScrollArea } from '@/components/ui/ScrollArea';
import { Button } from '@/components/ui/Button';
import { FeatCard } from './FeatCard';
import type { FeatInfo } from './types';
import type { FeatTab } from './FeatNavBar';
import { cn } from '@/lib/utils';

export interface FeatTabContentProps {
  activeTab: FeatTab;

  myFeats: FeatInfo[];
  allFeats: FeatInfo[];

  ownedFeatIds: Set<number>;
  protectedFeatIds: Set<number>;

  onAddFeat: (featId: number) => void;
  onRemoveFeat: (featId: number) => void;
  onLoadFeatDetails: (feat: FeatInfo) => Promise<FeatInfo | null>;

  currentPage: number;
  totalPages: number;
  hasNext: boolean;
  hasPrevious: boolean;
  onPageChange: (page: number) => void;
}

function MyFeatsTabComponent({
  feats,
  protectedFeatIds,
  onRemove,
  onLoadDetails,
}: {
  feats: FeatInfo[];
  protectedFeatIds: Set<number>;
  onRemove: (featId: number) => void;
  onLoadDetails: (feat: FeatInfo) => Promise<FeatInfo | null>;
}) {
  if (feats.length === 0) {
    return (
      <div className="flex items-center justify-center h-64">
        <p className="text-[rgb(var(--color-text-muted))]">
          No feats found. Try adjusting your filters.
        </p>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 gap-3">
      {feats.map((feat) => (
        <FeatCard
          key={feat.id}
          feat={feat}
          isOwned={true}
          isProtected={protectedFeatIds.has(feat.id)}
          onRemove={onRemove}
          onLoadDetails={onLoadDetails}
        />
      ))}
    </div>
  );
}

const MyFeatsTab = memo(MyFeatsTabComponent);

function AvailableFeatsTabComponent({
  feats,
  ownedFeatIds,
  onAdd,
  onLoadDetails,
}: {
  feats: FeatInfo[];
  ownedFeatIds: Set<number>;
  onAdd: (featId: number) => void;
  onLoadDetails: (feat: FeatInfo) => Promise<FeatInfo | null>;
}) {
  if (feats.length === 0) {
    return (
      <div className="flex items-center justify-center h-64">
        <p className="text-[rgb(var(--color-text-muted))]">
          No available feats found. Try adjusting your filters.
        </p>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 gap-3">
      {feats.map((feat) => (
        <FeatCard
          key={feat.id}
          feat={feat}
          isOwned={ownedFeatIds.has(feat.id)}
          onAdd={onAdd}
          onLoadDetails={onLoadDetails}
        />
      ))}
    </div>
  );
}

const AvailableFeatsTab = memo(AvailableFeatsTabComponent);

function FeatTabContentComponent({
  activeTab,
  myFeats,
  allFeats,
  ownedFeatIds,
  protectedFeatIds,
  onAddFeat,
  onRemoveFeat,
  onLoadFeatDetails,
  currentPage,
  totalPages,
  hasNext,
  hasPrevious,
  onPageChange,
}: FeatTabContentProps) {

  return (
    <div className="flex flex-col flex-1 bg-[rgb(var(--color-surface-1))] border border-[rgb(var(--color-surface-border))] rounded-lg overflow-hidden">
      <ScrollArea className="flex-1">
        <div className="p-4">
          <div className={cn('transition-opacity duration-200', activeTab === 'my-feats' ? 'block' : 'hidden')}>
            <MyFeatsTab
              feats={myFeats}
              protectedFeatIds={protectedFeatIds}
              onRemove={onRemoveFeat}
              onLoadDetails={onLoadFeatDetails}
            />
          </div>

          <div className={cn('transition-opacity duration-200', activeTab === 'all-feats' ? 'block' : 'hidden')}>
            <AvailableFeatsTab
              feats={allFeats}
              ownedFeatIds={ownedFeatIds}
              onAdd={onAddFeat}
              onLoadDetails={onLoadFeatDetails}
            />
          </div>
        </div>
      </ScrollArea>

      {activeTab === 'all-feats' && totalPages > 1 && (
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

export const FeatTabContent = memo(FeatTabContentComponent);
