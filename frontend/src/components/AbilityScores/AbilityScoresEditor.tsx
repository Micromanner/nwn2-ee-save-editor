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
  
  // Load attributes data only if missing when component mounts
  useEffect(() => {
    if (character?.id && !attributesData.data && !attributesData.isLoading) {
      attributesData.load(); // Only load if data is missing - no forced refresh on tab switch
    }
  }, [character?.id, attributesData.data, attributesData.isLoading, attributesData]); // Load only when character changes or data is missing
  
  // Initialize ability score management with subsystem data
  const {
    abilityScores,
    stats,
    alignment,
    updateAbilityScore,
    updateStats,
    updateAlignment
  } = useAbilityScores(attributesData.data as AbilityScoreState | null);
  
  // Wrap updateAbilityScore - no data refresh needed due to backend cache
  const handleAbilityScoreUpdate = useCallback(async (index: number, newValue: number) => {
    await updateAbilityScore(index, newValue);
  }, [updateAbilityScore]);

  // Early return for loading/error states
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