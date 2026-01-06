'use client';

import { useEffect, useCallback } from 'react';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { useAbilityScores, AbilityScoreState } from '@/hooks/useAbilityScores';
import CoreAbilityScoresSection from './CoreAbilityScoresSection';
import VitalStatisticsSection from './VitalStatisticsSection';
import AlignmentSection from './AlignmentSection';
import { Card } from '@/components/ui/Card';

export default function AbilityScoresEditor() {
  const { character } = useCharacterContext();
  const attributesData = useSubsystem('abilityScores');
  
  useEffect(() => {
    if (character?.id && !attributesData.data && !attributesData.isLoading) {
      attributesData.load();
    }
  }, [character?.id, attributesData.data, attributesData.isLoading, attributesData]);
  
  const {
    abilityScores,
    stats,
    alignment,
    updateAbilityScore,
    updateStats,
    updateAlignment,
    pointSummary
  } = useAbilityScores(attributesData.data as AbilityScoreState | null);
  
  const handleAbilityScoreUpdate = useCallback(async (index: number, newValue: number) => {
    await updateAbilityScore(index, newValue);
  }, [updateAbilityScore]);


  if (attributesData.isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-[rgb(var(--color-primary))]"></div>
      </div>
    );
  }

  if (attributesData.error) {
    return (
      <Card variant="error">
        <p className="text-error">{attributesData.error}</p>
      </Card>
    );
  }

  if (!character) {
    return (
      <Card variant="warning">
        <p className="text-muted">No character loaded. Please import a save file to begin.</p>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
       <div className="grid grid-cols-2 gap-3">
         <Card variant="default" padding="sm" className="bg-[rgb(var(--color-surface-1))]">
           <div className="text-center">
             <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase tracking-wider mb-1">Points Spent</div>
             <div className="text-2xl font-bold text-[rgb(var(--color-text-primary))]">
               {pointSummary?.total_spent ?? 0}
             </div>
           </div>
         </Card>
         <Card variant="default" padding="sm" className="bg-[rgb(var(--color-surface-1))]">
           <div className="text-center">
             <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase tracking-wider mb-1">Available Points</div>
             <div className="text-2xl font-bold text-[rgb(var(--color-primary))]">
               {pointSummary?.available ?? 0}
             </div>
           </div>
         </Card>
       </div>

      <CoreAbilityScoresSection 
        abilityScores={abilityScores}
        onAbilityScoreChange={handleAbilityScoreUpdate}
      />
      
      <VitalStatisticsSection 
        stats={stats}
        onStatsChange={updateStats}
      />
      
      <AlignmentSection 
        alignment={alignment}
        onAlignmentChange={updateAlignment}
      />
    </div>
  );
}