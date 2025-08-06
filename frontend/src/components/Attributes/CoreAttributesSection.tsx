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
  tempValue?: number;
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
    { name: t('attributes.strength'), shortName: 'STR', value: 10, modifier: 0 },
    { name: t('attributes.dexterity'), shortName: 'DEX', value: 10, modifier: 0 },
    { name: t('attributes.constitution'), shortName: 'CON', value: 10, modifier: 0 },
    { name: t('attributes.intelligence'), shortName: 'INT', value: 10, modifier: 0 },
    { name: t('attributes.wisdom'), shortName: 'WIS', value: 10, modifier: 0 },
    { name: t('attributes.charisma'), shortName: 'CHA', value: 10, modifier: 0 },
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
    updateAttribute(index, attributes[index].value + 1);
  };

  const decreaseAttribute = (index: number) => {
    updateAttribute(index, attributes[index].value - 1);
  };

  return (
    <Card variant="container">
      <CardContent className="attribute-section-responsive">
        <h3 className="section-title">{t('attributes.coreAttributes')}</h3>
        <div className="attribute-grid-adaptive">
          {attributes.map((attr, index) => (
            <AttributeCard
              key={attr.shortName}
              name={attr.name}
              shortName={attr.shortName}
              value={attr.value}
              modifier={calculateModifier(attr.value)}
              tempValue={attr.tempValue}
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