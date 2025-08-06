'use client';

import { useEffect, useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Label } from '@/components/ui/Label';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { Slider } from '@/components/ui/Slider';
import { ScrollArea } from '@/components/ui/ScrollArea';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/Tabs';
import { useTranslations } from '@/hooks/useTranslations';
import { useAppearanceModels, usePortraits, useSoundsets } from '@/lib/api';
import { Skeleton } from '@/components/ui/skeleton';
import { useCharacterContext } from '@/contexts/CharacterContext';

export default function AppearanceEditor() {
  const t = useTranslations();
  const { character, isLoading, error } = useCharacterContext();
  const inDevelopment = true;
  
  // Load data separately for better performance
  const { data: appearanceModels, loading: modelsLoading, error: modelsError } = useAppearanceModels();
  const { data: portraits, loading: portraitsLoading, error: portraitsError } = usePortraits();
  const { data: soundsets, loading: soundsetsLoading, error: soundsetsError } = useSoundsets();
  
  // Local state for all appearance options
  const [appearance, setAppearance] = useState<string>('');
  const [portrait, setPortrait] = useState<string>('');
  const [soundset, setSoundset] = useState<string>('');
  const [bodyType, setBodyType] = useState<number>(0);
  const [hairStyle, setHairStyle] = useState<number>(0);
  const [hairColor, setHairColor] = useState<number>(0);
  const [skinColor, setSkinColor] = useState<number>(0);
  const [headVariation, setHeadVariation] = useState<number>(0);
  const [tattooColor1, setTattooColor1] = useState<number>(0);
  const [tattooColor2, setTattooColor2] = useState<number>(0);
  const [searchQuery, setSearchQuery] = useState<string>('');

  useEffect(() => {
    if (character) {
      setAppearance(character.appearance?.toString() || '');
      setPortrait(character.portrait?.toString() || '');
      setSoundset(character.soundset?.toString() || '');
      setBodyType(character.bodyType || 0);
      setHairStyle(character.hairStyle || 0);
      setHairColor(character.hairColor || 0);
      setSkinColor(character.skinColor || 0);
      setHeadVariation(character.headVariation || 0);
      setTattooColor1(character.tattooColor1 || 0);
      setTattooColor2(character.tattooColor2 || 0);
    }
  }, [character]);

  const handleChange = (field: string, value: string | number) => {
    const numValue = typeof value === 'string' ? parseInt(value, 10) : value;
    
    switch (field) {
      case 'appearance':
        setAppearance(value.toString());
        break;
      case 'portrait':
        setPortrait(value.toString());
        break;
      case 'soundset':
        setSoundset(value.toString());
        break;
      case 'bodyType':
        setBodyType(numValue);
        break;
      case 'hairStyle':
        setHairStyle(numValue);
        break;
      case 'hairColor':
        setHairColor(numValue);
        break;
      case 'skinColor':
        setSkinColor(numValue);
        break;
      case 'headVariation':
        setHeadVariation(numValue);
        break;
      case 'tattooColor1':
        setTattooColor1(numValue);
        break;
      case 'tattooColor2':
        setTattooColor2(numValue);
        break;
    }
    // TODO: Update character through context
  };

  // Filter appearance data based on search
  const filteredAppearance = appearanceModels 
    ? Object.entries(appearanceModels).filter(([, data]) => {
        const item = data as { name?: string; label?: string };
        return (item.name || item.label || '').toLowerCase().includes(searchQuery.toLowerCase());
      })
    : [];

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
        <p className="text-muted">No character loaded. Please import a save file to begin editing appearance.</p>
      </Card>
    );
  }

  if (inDevelopment) {
    return (
      <div className="max-w-6xl mx-auto space-y-6 text-center py-20">
        <h1 className="text-3xl font-bold text-gray-600">Appearance Editor</h1>
        <p className="text-xl text-gray-500">In Development</p>
      </div>
    );
  }

  return (
    <Card className="h-full">
      <CardHeader>
        <CardTitle>{t('appearance.title')}</CardTitle>
        <CardDescription>{t('appearance.description')}</CardDescription>
      </CardHeader>
      <CardContent>
        <Tabs defaultValue="body" className="w-full">
          <TabsList className="grid w-full grid-cols-4">
            <TabsTrigger value="body">{t('appearance.body')}</TabsTrigger>
            <TabsTrigger value="colors">{t('appearance.colors')}</TabsTrigger>
            <TabsTrigger value="portrait">{t('appearance.portrait')}</TabsTrigger>
            <TabsTrigger value="voice">{t('appearance.voice')}</TabsTrigger>
          </TabsList>

          <TabsContent value="body" className="space-y-6">
            {/* Character Model Search */}
            <div className="space-y-4">
              <div className="space-y-2">
                <Label>{t('appearance.characterModel')}</Label>
                <Input
                  placeholder={t('appearance.searchModels')}
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="mb-2"
                />
                <ScrollArea className="h-[200px] border rounded-md p-2">
                  {modelsLoading ? (
                    <div className="space-y-2">
                      <Skeleton className="h-8 w-full" />
                      <Skeleton className="h-8 w-full" />
                      <Skeleton className="h-8 w-full" />
                    </div>
                  ) : modelsError ? (
                    <p className="text-red-500 text-sm">{t('errors.loadingFailed')}</p>
                  ) : (
                    <div className="grid grid-cols-2 gap-2">
                      {filteredAppearance.map(([id, data]) => {
                        const item = data as { name?: string; label?: string };
                        return (
                          <Button
                            key={id}
                            variant={appearance === id ? "primary" : "outline"}
                            size="sm"
                            onClick={() => handleChange('appearance', id)}
                            className="text-left justify-start"
                          >
                            {item.name || item.label}
                          </Button>
                        );
                      })}
                    </div>
                  )}
                </ScrollArea>
              </div>

              {/* Body Type */}
              <div className="space-y-2">
                <Label>{t('appearance.bodyType')}</Label>
                <div className="flex items-center space-x-4">
                  <Slider
                    value={[bodyType]}
                    onValueChange={([value]) => handleChange('bodyType', value)}
                    min={0}
                    max={10}
                    step={1}
                    className="flex-1"
                  />
                  <span className="w-12 text-right">{bodyType}</span>
                </div>
              </div>

              {/* Head Variation */}
              <div className="space-y-2">
                <Label>{t('appearance.headVariation')}</Label>
                <div className="flex items-center space-x-4">
                  <Slider
                    value={[headVariation]}
                    onValueChange={([value]) => handleChange('headVariation', value)}
                    min={0}
                    max={10}
                    step={1}
                    className="flex-1"
                  />
                  <span className="w-12 text-right">{headVariation}</span>
                </div>
              </div>

              {/* Hair Style */}
              <div className="space-y-2">
                <Label>{t('appearance.hairStyle')}</Label>
                <div className="flex items-center space-x-4">
                  <Slider
                    value={[hairStyle]}
                    onValueChange={([value]) => handleChange('hairStyle', value)}
                    min={0}
                    max={20}
                    step={1}
                    className="flex-1"
                  />
                  <span className="w-12 text-right">{hairStyle}</span>
                </div>
              </div>
            </div>
          </TabsContent>

          <TabsContent value="colors" className="space-y-6">
            {/* Skin Color */}
            <div className="space-y-2">
              <Label>{t('appearance.skinColor')}</Label>
              <div className="flex items-center space-x-4">
                <Slider
                  value={[skinColor]}
                  onValueChange={([value]) => handleChange('skinColor', value)}
                  min={0}
                  max={20}
                  step={1}
                  className="flex-1"
                />
                <span className="w-12 text-right">{skinColor}</span>
              </div>
            </div>

            {/* Hair Color */}
            <div className="space-y-2">
              <Label>{t('appearance.hairColor')}</Label>
              <div className="flex items-center space-x-4">
                <Slider
                  value={[hairColor]}
                  onValueChange={([value]) => handleChange('hairColor', value)}
                  min={0}
                  max={50}
                  step={1}
                  className="flex-1"
                />
                <span className="w-12 text-right">{hairColor}</span>
              </div>
            </div>

            {/* Tattoo Colors */}
            <div className="space-y-2">
              <Label>{t('appearance.tattooColor1')}</Label>
              <div className="flex items-center space-x-4">
                <Slider
                  value={[tattooColor1]}
                  onValueChange={([value]) => handleChange('tattooColor1', value)}
                  min={0}
                  max={50}
                  step={1}
                  className="flex-1"
                />
                <span className="w-12 text-right">{tattooColor1}</span>
              </div>
            </div>

            <div className="space-y-2">
              <Label>{t('appearance.tattooColor2')}</Label>
              <div className="flex items-center space-x-4">
                <Slider
                  value={[tattooColor2]}
                  onValueChange={([value]) => handleChange('tattooColor2', value)}
                  min={0}
                  max={50}
                  step={1}
                  className="flex-1"
                />
                <span className="w-12 text-right">{tattooColor2}</span>
              </div>
            </div>
          </TabsContent>

          <TabsContent value="portrait" className="space-y-4">
            <div className="space-y-2">
              <Label>{t('appearance.characterPortrait')}</Label>
              <ScrollArea className="h-[400px] border rounded-md p-4">
                {portraitsLoading ? (
                  <div className="grid grid-cols-4 gap-4">
                    {[...Array(8)].map((_, i) => (
                      <Skeleton key={i} className="aspect-square" />
                    ))}
                  </div>
                ) : portraitsError ? (
                  <p className="text-red-500 text-sm">{t('errors.loadingFailed')}</p>
                ) : (
                  <div className="grid grid-cols-4 gap-4">
                    {portraits && Object.entries(portraits).map(([id, data]) => {
                      const item = data as { name?: string; label?: string };
                      return (
                        <div
                          key={id}
                          className={`cursor-pointer border-2 rounded-md p-2 hover:border-primary transition-colors ${
                            portrait === id ? 'border-primary' : 'border-transparent'
                          }`}
                          onClick={() => handleChange('portrait', id)}
                        >
                          <div className="aspect-square bg-muted rounded-md mb-2 flex items-center justify-center">
                            <span className="text-xs text-muted-foreground">Portrait {id}</span>
                          </div>
                          <p className="text-xs text-center truncate">
                            {item.name || item.label || `Portrait ${id}`}
                          </p>
                        </div>
                      );
                    })}
                  </div>
                )}
              </ScrollArea>
            </div>
          </TabsContent>

          <TabsContent value="voice" className="space-y-4">
            <div className="space-y-2">
              <Label>{t('appearance.voiceSet')}</Label>
              <ScrollArea className="h-[400px] border rounded-md p-4">
                {soundsetsLoading ? (
                  <div className="space-y-2">
                    {[...Array(5)].map((_, i) => (
                      <Skeleton key={i} className="h-16 w-full" />
                    ))}
                  </div>
                ) : soundsetsError ? (
                  <p className="text-red-500 text-sm">{t('errors.loadingFailed')}</p>
                ) : (
                  <div className="space-y-2">
                    {soundsets && Object.entries(soundsets).map(([id, data]) => {
                      const item = data as { name?: string; label?: string; description?: string };
                      return (
                        <div
                          key={id}
                          className={`cursor-pointer border rounded-md p-3 hover:border-primary transition-colors ${
                            soundset === id ? 'border-primary bg-primary/5' : 'border-border'
                          }`}
                          onClick={() => handleChange('soundset', id)}
                        >
                          <p className="font-medium">
                            {item.name || item.label || `Voice Set ${id}`}
                          </p>
                          {item.description && (
                            <p className="text-sm text-muted-foreground">{item.description}</p>
                          )}
                        </div>
                      );
                    })}
                  </div>
                )}
              </ScrollArea>
            </div>
          </TabsContent>
        </Tabs>
      </CardContent>
    </Card>
  );
}