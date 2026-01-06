'use client';

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { useTranslations } from '@/hooks/useTranslations';
import { Badge } from '@/components/ui/Badge';
import { Users2, Heart, Sword, Shield } from 'lucide-react';
import { useState } from 'react';
import { useCharacterContext } from '@/contexts/CharacterContext';

interface Companion {
  id: string;
  name: string;
  race: string;
  class: string;
  level: number;
  hitPoints: { current: number; max: number };
  armorClass: number;
  portrait?: string;
  influence?: number;
}

interface CompanionsViewProps {
  onLoadCompanion?: (companionName: string) => void;
  currentCharacterName?: string;
}

export default function CompanionsView({ onLoadCompanion, currentCharacterName }: CompanionsViewProps) {
  const t = useTranslations();
  const { character, isLoading, error } = useCharacterContext();
  const inDevelopment = true;
  const companions: Companion[] = [];
  const hasUnsavedChanges = true;
  const loadCompanion = (companion: Companion) => {
    if (onLoadCompanion) {
      onLoadCompanion(companion.name);
    }
  };
  const [companionToLoad, setCompanionToLoad] = useState<Companion | null>(null);

  const handleLoadCompanion = (companion: Companion) => {
    if (hasUnsavedChanges) {
      setCompanionToLoad(companion);
    } else {
      loadCompanion(companion);
    }
  };

  const confirmLoadCompanion = () => {
    if (companionToLoad) {
      loadCompanion(companionToLoad);
      setCompanionToLoad(null);
    }
  };

  // Mock data for now - will be replaced with actual companion data
  const mockCompanions: Companion[] = [
    {
      id: 'neeshka',
      name: 'Neeshka',
      race: 'Tiefling',
      class: 'Rogue',
      level: 8,
      hitPoints: { current: 45, max: 52 },
      armorClass: 18,
      influence: 75,
    },
    {
      id: 'khelgar',
      name: 'Khelgar Ironfist',
      race: 'Shield Dwarf',
      class: 'Fighter',
      level: 8,
      hitPoints: { current: 89, max: 96 },
      armorClass: 22,
      influence: 60,
    },
    {
      id: 'sand',
      name: 'Sand',
      race: 'Moon Elf',
      class: 'Wizard',
      level: 8,
      hitPoints: { current: 28, max: 32 },
      armorClass: 14,
      influence: 45,
    },
  ];

  const displayCompanions = companions.length > 0 ? companions : mockCompanions;

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-[rgb(var(--color-primary))]"></div>
      </div>
    );
  }

  if (error) {
    return (
      <Card variant="error">
        <p className="text-error">{error}</p>
      </Card>
    );
  }

  if (!character) {
    return (
      <Card variant="warning">
        <p className="text-muted">No character loaded. Please import a save file to view companions.</p>
      </Card>
    );
  }

  if (inDevelopment) {
    return (
      <div className="max-w-6xl mx-auto space-y-6 text-center py-20">
        <h1 className="text-3xl font-bold text-gray-600">Companions</h1>
        <p className="text-xl text-gray-500">In Development</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-2">
        <Users2 className="h-6 w-6" />
        <h2 className="text-2xl font-bold">{t('navigation.companions')}</h2>
        <Badge variant="secondary">{displayCompanions.length}</Badge>
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {displayCompanions.map((companion) => (
          <Card key={companion.id} className="relative overflow-hidden">
            <CardHeader className="pb-3">
              <div className="flex items-start gap-3">
                <div className="h-12 w-12 rounded-full bg-[rgb(var(--color-surface-3))] flex items-center justify-center text-lg font-semibold">
                  {companion.name.charAt(0)}
                </div>
                <div className="flex-1">
                  <CardTitle className="text-lg">{companion.name}</CardTitle>
                  <CardDescription>
                    {companion.race} {companion.class} • Level {companion.level}
                  </CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="space-y-2 text-sm">
                <div className="flex items-center justify-between">
                  <span className="flex items-center gap-1 text-muted-foreground">
                    <Heart className="h-3 w-3" />
                    HP
                  </span>
                  <span className="font-medium">
                    {companion.hitPoints.current}/{companion.hitPoints.max}
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="flex items-center gap-1 text-muted-foreground">
                    <Shield className="h-3 w-3" />
                    AC
                  </span>
                  <span className="font-medium">{companion.armorClass}</span>
                </div>
                {companion.influence !== undefined && (
                  <div className="flex items-center justify-between">
                    <span className="flex items-center gap-1 text-muted-foreground">
                      <Sword className="h-3 w-3" />
                      Influence
                    </span>
                    <span className="font-medium">{companion.influence}%</span>
                  </div>
                )}
              </div>
              
              <Button 
                onClick={() => handleLoadCompanion(companion)}
                className="w-full"
                variant={companion.name === currentCharacterName ? "primary" : "secondary"}
                disabled={companion.name === currentCharacterName}
              >
                {companion.name === currentCharacterName ? 'Loaded' : 'Load Companion'}
              </Button>
            </CardContent>
          </Card>
        ))}
      </div>

      {companionToLoad && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50">
          <Card className="max-w-md">
            <CardHeader>
              <CardTitle>Unsaved Changes</CardTitle>
              <CardDescription>
                You have unsaved changes to the current character. Loading {companionToLoad.name} will discard these changes. Do you want to continue?
              </CardDescription>
            </CardHeader>
            <CardContent className="flex gap-2 justify-end">
              <Button variant="outline" onClick={() => setCompanionToLoad(null)}>Cancel</Button>
              <Button onClick={confirmLoadCompanion}>Load {companionToLoad.name}</Button>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}