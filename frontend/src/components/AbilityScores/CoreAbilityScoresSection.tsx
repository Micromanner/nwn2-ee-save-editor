
import { useState } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import AbilityScoreCard from './AbilityScoreCard';

interface AbilityScore {
  name: string;
  shortName: string;
  value: number;
  modifier: number;
  baseValue?: number;
  breakdown?: {
    levelUp: number;
    racial: number;
    equipment: number;
    enhancement: number;
    temporary: number;
  };
}

interface CoreAbilityScoresSectionProps {
  abilityScores?: AbilityScore[];
  onAbilityScoreChange?: (index: number, value: number) => void;
}

export default function CoreAbilityScoresSection({ 
  abilityScores: externalAbilityScores,
  onAbilityScoreChange 
}: CoreAbilityScoresSectionProps) {
  const t = useTranslations();
  
  const [internalAbilityScores, setInternalAbilityScores] = useState<AbilityScore[]>([
    { name: t('abilityScores.strength'), shortName: 'STR', value: 10, modifier: 0 },
    { name: t('abilityScores.dexterity'), shortName: 'DEX', value: 10, modifier: 0 },
    { name: t('abilityScores.constitution'), shortName: 'CON', value: 10, modifier: 0 },
    { name: t('abilityScores.intelligence'), shortName: 'INT', value: 10, modifier: 0 },
    { name: t('abilityScores.wisdom'), shortName: 'WIS', value: 10, modifier: 0 },
    { name: t('abilityScores.charisma'), shortName: 'CHA', value: 10, modifier: 0 },
  ]);

  const abilityScores = externalAbilityScores || internalAbilityScores;

  const calculateModifier = (value: number): number => {
    return Math.floor((value - 10) / 2);
  };

  const updateAbilityScore = (index: number, newValue: number) => {
    const clampedValue = Math.max(3, Math.min(50, newValue));

    if (onAbilityScoreChange) {
      onAbilityScoreChange(index, clampedValue);
    } else {
      const newAbilityScores = [...internalAbilityScores];
      newAbilityScores[index].value = clampedValue;
      newAbilityScores[index].modifier = calculateModifier(clampedValue);
      setInternalAbilityScores(newAbilityScores);
    }
  };

  const increaseAbilityScore = (index: number) => {
    const currentValue = abilityScores[index].baseValue ?? abilityScores[index].value;
    const newValue = currentValue + 1;
    if (newValue <= 50) {
      updateAbilityScore(index, newValue);
    }
  };

  const decreaseAbilityScore = (index: number) => {
    const currentValue = abilityScores[index].baseValue ?? abilityScores[index].value;
    const newValue = currentValue - 1;
    if (newValue >= 3) {
      updateAbilityScore(index, newValue);
    }
  };

  return (
    <Card variant="container">
      <CardContent className="attribute-section-responsive">
        <h3 className="section-title">{t('abilityScores.title')}</h3>
        <div className="attribute-grid-adaptive">
          {abilityScores.map((attr, index) => (
            <AbilityScoreCard
              key={attr.shortName}
              name={attr.name}
              shortName={attr.shortName}
              value={attr.value}
              modifier={attr.modifier}
              baseValue={attr.baseValue}
              breakdown={attr.breakdown}
              onIncrease={() => increaseAbilityScore(index)}
              onDecrease={() => decreaseAbilityScore(index)}
              onChange={(value) => updateAbilityScore(index, value)}
              min={3}
              max={50}
            />
          ))}
        </div>
      </CardContent>
    </Card>
  );
}