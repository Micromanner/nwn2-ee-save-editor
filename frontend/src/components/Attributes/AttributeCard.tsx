'use client';

import { formatModifier } from '@/utils/dataHelpers';
import { useState, useCallback, useRef } from 'react';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';

interface AttributeCardProps {
  name: string;
  shortName: string;
  value: number;
  modifier: number;
  baseValue?: number;
  breakdown?: {
    racial: number;
    equipment: number;
  };
  onIncrease: () => void;
  onDecrease: () => void;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
}

export default function AttributeCard({
  name,
  shortName,
  value,
  modifier,
  baseValue,
  breakdown,
  onIncrease,
  onDecrease,
  onChange,
  min = 3,
  max = 40
}: AttributeCardProps) {
  const [clickedButton, setClickedButton] = useState<'increase' | 'decrease' | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Determine modifier class for color coding
  const getModifierClass = useCallback(() => {
    if (modifier > 0) return 'positive';
    if (modifier < 0) return 'negative';
    return 'zero';
  }, [modifier]);

  // Helper function to get value modifier class
  const getValueClass = useCallback((value: number) => {
    if (value > 0) return 'positive';
    if (value < 0) return 'negative';
    return 'zero';
  }, []);

  // Handle input change with validation
  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = parseInt(e.target.value) || min;
    const clampedValue = Math.max(min, Math.min(max, newValue));
    
    onChange(clampedValue);
  }, [min, max, onChange]);

  // Handle button press with animation
  const handleIncrease = useCallback(() => {
    setClickedButton('increase');
    onIncrease();
    
    // Clear visual feedback
    setTimeout(() => {
      setClickedButton(null);
    }, 200);
  }, [onIncrease]);

  const handleDecrease = useCallback(() => {
    setClickedButton('decrease');
    onDecrease();
    
    // Clear visual feedback
    setTimeout(() => {
      setClickedButton(null);
    }, 200);
  }, [onDecrease]);

  // Handle keyboard navigation
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    const currentValue = baseValue !== undefined ? baseValue : value;
    switch (e.key) {
      case 'ArrowUp':
        e.preventDefault();
        if (currentValue < max) handleIncrease();
        break;
      case 'ArrowDown':
        e.preventDefault();
        if (currentValue > min) handleDecrease();
        break;
      case '+':
      case '=':
        e.preventDefault();
        if (currentValue < max) handleIncrease();
        break;
      case '-':
        e.preventDefault();
        if (currentValue > min) handleDecrease();
        break;
    }
  }, [value, baseValue, min, max, handleIncrease, handleDecrease]);

  return (
    <Card 
      variant="interactive"
      className="flex flex-col h-full"
      role="group"
      aria-labelledby={`${shortName}-label`}
    >
      <div className="attribute-header-responsive">
        <span 
          id={`${shortName}-label`}
          className="attribute-name-responsive"
          title={`${name} (${shortName})`}
        >
          {name}
        </span>
        <span 
          className={`attribute-modifier-responsive ${getModifierClass()}`}
          aria-label={`${name} modifier: ${formatModifier(modifier)}`}
          title={`Modifier: ${formatModifier(modifier)}`}
        >
          {formatModifier(modifier)}
        </span>
      </div>

      <div className="attribute-controls-mobile">
        <Button
          onClick={handleDecrease}
          variant="outline"
          size="sm"
          disabled={(baseValue !== undefined ? baseValue : value) <= min}
          clicked={clickedButton === 'decrease'}
          aria-label={`Decrease ${name}`}
          title={`Decrease ${name} (min: ${min})`}
        >
          −
        </Button>
        
        <input
          ref={inputRef}
          type="number"
          value={baseValue !== undefined ? baseValue : value}
          onChange={handleInputChange}
          onKeyDown={handleKeyDown}
          className="attribute-input-responsive"
          min={min}
          max={max}
          aria-label={`${name} base value`}
          title={`${name} base: ${baseValue !== undefined ? baseValue : value}, effective: ${value} (${formatModifier(modifier)})`}
          aria-describedby={`${shortName}-help`}
        />
        
        <Button
          onClick={handleIncrease}
          variant="outline"
          size="sm"
          disabled={(baseValue !== undefined ? baseValue : value) >= max}
          clicked={clickedButton === 'increase'}
          aria-label={`Increase ${name}`}
          title={`Increase ${name} (max: ${max})`}
        >
          +
        </Button>
      </div>

      {/* Always show breakdown */}
      <div 
        className="attribute-breakdown"
        role="region"
        aria-labelledby={`${shortName}-breakdown-label`}
      >
        <div 
          id={`${shortName}-breakdown-label`}
          className="sr-only"
        >
          {name} breakdown details
        </div>
        <div className="breakdown-row">
          <span className="breakdown-label">Base:</span>
          <span className="breakdown-value">{baseValue !== undefined ? baseValue : value}</span>
        </div>
        {breakdown && (
          <>
            <div className="breakdown-row">
              <span className="breakdown-label">Racial:</span>
              <span className={`breakdown-value ${getValueClass(breakdown.racial)}`}>
                {formatModifier(breakdown.racial)}
              </span>
            </div>
            <div className="breakdown-row">
              <span className="breakdown-label">Equipment:</span>
              <span className={`breakdown-value ${getValueClass(breakdown.equipment)}`}>
                {formatModifier(breakdown.equipment)}
              </span>
            </div>
            <hr className="breakdown-divider" />
            <div className="breakdown-row breakdown-total">
              <span className="breakdown-label">Effective:</span>
              <span className="breakdown-value">{value}</span>
            </div>
          </>
        )}
      </div>
      
      {/* Hidden help text for screen readers */}
      <div 
        id={`${shortName}-help`} 
        className="sr-only"
        aria-hidden="true"
      >
        Use arrow keys or +/- to adjust value. Range: {min} to {max}
      </div>
    </Card>
  );
}