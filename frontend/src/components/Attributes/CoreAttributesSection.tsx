'use client';

import { useState } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import AttributeCard from './AttributeCard';

interface Attribute {
  name: string;
  shortName: string;
  value: number;
  modifier: number;
  baseValue?: number;
  breakdown?: {
    racial: number;
    equipment: number;
  };
}

interface CoreAttributesSectionProps {
  attributes?: Attribute[];
  onAttributeChange?: (index: number, value: number) => void;
}

export default function CoreAttributesSection({ 
  attributes: externalAttributes,
  onAttributeChange 
}: CoreAttributesSectionProps) {
  const t = useTranslations();
  
  // Default attributes if none provided
  const [internalAttributes, setInternalAttributes] = useState<Attribute[]>([
    { name: t('abilities.strength'), shortName: 'STR', value: 10, modifier: 0 },
    { name: t('abilities.dexterity'), shortName: 'DEX', value: 10, modifier: 0 },
    { name: t('abilities.constitution'), shortName: 'CON', value: 10, modifier: 0 },
    { name: t('abilities.intelligence'), shortName: 'INT', value: 10, modifier: 0 },
    { name: t('abilities.wisdom'), shortName: 'WIS', value: 10, modifier: 0 },
    { name: t('abilities.charisma'), shortName: 'CHA', value: 10, modifier: 0 },
  ]);

  // Use external attributes if provided, otherwise use internal state
  const attributes = externalAttributes || internalAttributes;

  const calculateModifier = (value: number): number => {
    return Math.floor((value - 10) / 2);
  };

  const updateAttribute = (index: number, newValue: number) => {
    const clampedValue = Math.max(3, Math.min(50, newValue));
    const newModifier = calculateModifier(clampedValue);

    if (onAttributeChange) {
      // If external handler provided, use it
      onAttributeChange(index, clampedValue);
    } else {
      // Otherwise update internal state
      const newAttributes = [...internalAttributes];
      newAttributes[index].value = clampedValue;
      newAttributes[index].modifier = newModifier;
      setInternalAttributes(newAttributes);
    }
  };

  const increaseAttribute = (index: number) => {
    // Use baseValue if available, otherwise fall back to value
    const currentValue = attributes[index].baseValue !== undefined 
      ? attributes[index].baseValue 
      : attributes[index].value;
    const newValue = currentValue + 1;
    
    // Double check bounds before calling update
    if (newValue <= 50) {
      updateAttribute(index, newValue);
    }
  };

  const decreaseAttribute = (index: number) => {
    // Use baseValue if available, otherwise fall back to value
    const currentValue = attributes[index].baseValue !== undefined
      ? attributes[index].baseValue
      : attributes[index].value;
    const newValue = currentValue - 1;
    
    // Double check bounds before calling update
    if (newValue >= 3) {
      updateAttribute(index, newValue);
    }
  };

  return (
    <Card variant="container">
      <CardContent className="attribute-section-responsive">
        <h3 className="section-title">{t('abilities.title')}</h3>
        <div className="attribute-grid-adaptive">
          {attributes.map((attr, index) => (
            <AttributeCard
              key={attr.shortName}
              name={attr.name}
              shortName={attr.shortName}
              value={attr.value}
              modifier={calculateModifier(attr.value)}
              baseValue={attr.baseValue}
              breakdown={attr.breakdown}
              onIncrease={() => increaseAttribute(index)}
              onDecrease={() => decreaseAttribute(index)}
              onChange={(value) => updateAttribute(index, value)}
              min={3}
              max={50}
            />
          ))}
        </div>
      </CardContent>
    </Card>
  );
}