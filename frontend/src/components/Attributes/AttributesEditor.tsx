'use client';

import { useEffect, useCallback } from 'react';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { useAttributes, AttributeState } from '@/hooks/useAttributes';
import CoreAttributesSection from './CoreAttributesSection';
import VitalStatisticsSection from './VitalStatisticsSection';
import AlignmentSection from './AlignmentSection';
import { Card } from '@/components/ui/Card';

export default function AttributesEditor() {
  const { character } = useCharacterContext();
  const attributesData = useSubsystem<AttributeState>('attributes');
  
  // Load attributes data when component mounts
  useEffect(() => {
    if (character && !attributesData.data && !attributesData.isLoading) {
      attributesData.load();
    }
  }, [character, attributesData]);
  
  // Initialize attribute management with subsystem data
  const {
    attributes,
    stats,
    alignment,
    updateAttribute,
    updateStats,
    updateAlignment
  } = useAttributes(attributesData.data);
  
  // Wrap updateAttribute - no data refresh needed due to backend cache
  const handleAttributeUpdate = useCallback(async (index: number, newValue: number) => {
    await updateAttribute(index, newValue);
  }, [updateAttribute]);

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
      <CoreAttributesSection 
        attributes={attributes}
        onAttributeChange={handleAttributeUpdate}
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