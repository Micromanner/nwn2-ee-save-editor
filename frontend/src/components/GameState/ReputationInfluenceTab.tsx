'use client';

import { useState, useEffect } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { useTranslations } from '@/hooks/useTranslations';
import { Slider } from '@/components/ui/Slider';
import { Input } from '@/components/ui/Input';
import { Label } from '@/components/ui/Label';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { gameStateAPI, CompanionInfluenceData } from '@/services/gameStateApi';
import { display, formatModifier } from '@/utils/dataHelpers';

interface CompanionState {
  [key: string]: number | null;
}


export default function ReputationInfluenceTab() {
  const t = useTranslations();
  const { character } = useCharacterContext();
  const characterId = character?.id;

  const [companions, setCompanions] = useState<Record<string, CompanionInfluenceData>>({});
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  const [localCompanionInfluence, setLocalCompanionInfluence] = useState<CompanionState>({});

  useEffect(() => {
    if (!characterId) return;

    const fetchData = async () => {
      setIsLoading(true);
      setError(null);

      try {
        const companionResponse = await gameStateAPI.getCompanionInfluence(characterId);
        setCompanions(companionResponse.companions);

        const companionInfluence: CompanionState = {};
        Object.entries(companionResponse.companions).forEach(([id, data]) => {
          companionInfluence[id] = data.influence;
        });
        setLocalCompanionInfluence(companionInfluence);
      } catch (err) {
        console.error('Failed to load companion influence:', err);
      }

      setIsLoading(false);
    };

    fetchData();
  }, [characterId]);

  const handleCompanionInfluenceChange = (companionId: string, value: number[]) => {
    setLocalCompanionInfluence((prev) => ({
      ...prev,
      [companionId]: value[0],
    }));
  };

  const handleCompanionInfluenceInputChange = (companionId: string, value: string) => {
    const numValue = parseInt(value, 10);
    if (!isNaN(numValue)) {
      setLocalCompanionInfluence((prev) => ({
        ...prev,
        [companionId]: numValue,
      }));
    }
  };


  const handleSaveChanges = async () => {
    if (!characterId) return;

    setIsSaving(true);
    setError(null);

    try {
      const updatePromises: Promise<unknown>[] = [];

      Object.entries(localCompanionInfluence).forEach(([companionId, influence]) => {
        const originalInfluence = companions[companionId]?.influence;
        if (influence !== null && influence !== originalInfluence) {
          updatePromises.push(
            gameStateAPI.updateCompanionInfluence(characterId, companionId, influence)
          );
        }
      });

      await Promise.all(updatePromises);

      // Refresh data after save
      try {
        const companionResponse = await gameStateAPI.getCompanionInfluence(characterId);
        setCompanions(companionResponse.companions);
      } catch (err) {
        console.error('Failed to refresh companion data:', err);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : t('errors.loadingFailed'));
    } finally {
      setIsSaving(false);
    }
  };

  const getRecruitmentBadgeVariant = (status: string) => {
    switch (status) {
      case 'recruited':
        return 'default';
      case 'met':
        return 'secondary';
      case 'not_recruited':
      default:
        return 'outline';
    }
  };

  const hasChanges = Object.entries(localCompanionInfluence).some(
    ([id, value]) => value !== null && value !== companions[id]?.influence
  );

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <div className="text-[rgb(var(--color-text-muted))]">
          Loading...
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center py-8">
        <div className="text-[rgb(var(--color-error))]">
          {error}
        </div>
      </div>
    );
  }

  const companionEntries = Object.entries(companions);

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>{t('gameState.reputation.companionInfluence')}</CardTitle>
          <CardDescription>
            {t('gameState.reputation.companionInfluence')}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {companionEntries.length === 0 ? (
            <div className="text-center text-[rgb(var(--color-text-muted))] py-8">
              {t('gameState.reputation.noCompanions')}
            </div>
          ) : (
            <div className="space-y-6">
              {companionEntries.map(([companionId, companion]) => {
                const currentInfluence = localCompanionInfluence[companionId] ?? companion.influence ?? 0;
                return (
                  <div key={companionId} className="space-y-2">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <Label className="font-medium">{display(companion.name)}</Label>
                        <Badge variant={getRecruitmentBadgeVariant(companion.recruitment)}>
                          {companion.recruitment}
                        </Badge>
                      </div>
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium">{t('gameState.reputation.influence')}:</span>
                        <Input
                          type="number"
                          value={currentInfluence ?? ''}
                          onChange={(e) => handleCompanionInfluenceInputChange(companionId, e.target.value)}
                          className="w-20 h-8 text-right"
                          min={-100}
                          max={100}
                        />
                      </div>
                    </div>
                    <Slider
                      value={[currentInfluence ?? 0]}
                      onValueChange={(value) => handleCompanionInfluenceChange(companionId, value)}
                      min={-100}
                      max={100}
                      step={1}
                      className="w-full"
                    />
                    <div className="flex justify-between text-xs text-[rgb(var(--color-text-muted))]">
                      <span>-100</span>
                      <span>{formatModifier(currentInfluence ?? 0)}</span>
                      <span>+100</span>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>

      {companionEntries.length > 0 && (
        <div className="flex justify-end">
          <Button
            onClick={handleSaveChanges}
            disabled={!hasChanges || isSaving}
            className="min-w-32"
          >
            {isSaving ? t('actions.save') + '...' : t('actions.save')}
          </Button>
        </div>
      )}
    </div>
  );
}
