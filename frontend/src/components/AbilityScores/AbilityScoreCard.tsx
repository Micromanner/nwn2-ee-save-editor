'use client';

import { formatModifier } from '@/utils/dataHelpers';
import { useState, useCallback, useRef } from 'react';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';

interface AbilityScoreCardProps {
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
  onIncrease: () => void;
  onDecrease: () => void;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
}

export default function AbilityScoreCard({
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
}: AbilityScoreCardProps) {
  const [clickedButton, setClickedButton] = useState<'increase' | 'decrease' | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const getModifierClass = useCallback(() => {
    if (modifier > 0) return 'positive';
    if (modifier < 0) return 'negative';
    return 'zero';
  }, [modifier]);

  const getValueClass = useCallback((value: number) => {
    if (value > 0) return 'positive';
    if (value < 0) return 'negative';
    return 'zero';
  }, []);

  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = parseInt(e.target.value) || min;
    const clampedValue = Math.max(min, Math.min(max, newValue));
    
    onChange(clampedValue);
  }, [min, max, onChange]);

  const handleIncrease = useCallback(() => {
    setClickedButton('increase');
    onIncrease();
    
    setTimeout(() => {
      setClickedButton(null);
    }, 200);
  }, [onIncrease]);

  const handleDecrease = useCallback(() => {
    setClickedButton('decrease');
    onDecrease();
    
    setTimeout(() => {
      setClickedButton(null);
    }, 200);
  }, [onDecrease]);

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
      </div>

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
        <div className="breakdown-row breakdown-base">
          <span className="breakdown-label">Raw Base:</span>
          <div className="breakdown-controls">
            <Button
              onClick={handleDecrease}
              variant="outline"
              size="xs"
              disabled={(baseValue !== undefined ? baseValue : value) <= min}
              clicked={clickedButton === 'decrease'}
              aria-label={`Decrease ${name}`}
              title={`Decrease ${name} (min: ${min})`}
              className="breakdown-button"
            >
              −
            </Button>
            
            <input
              ref={inputRef}
              type="number"
              value={baseValue !== undefined ? baseValue : value}
              onChange={handleInputChange}
              onKeyDown={handleKeyDown}
              className="breakdown-input"
              min={min}
              max={max}
              aria-label={`${name} base value`}
              title={`${name} base: ${baseValue !== undefined ? baseValue : value}, effective: ${value} (${formatModifier(modifier)})`}
              aria-describedby={`${shortName}-help`}
            />
            
            <Button
              onClick={handleIncrease}
              variant="outline"
              size="xs"
              disabled={(baseValue !== undefined ? baseValue : value) >= max}
              clicked={clickedButton === 'increase'}
              aria-label={`Increase ${name}`}
              title={`Increase ${name} (max: ${max})`}
              className="breakdown-button"
            >
              +
            </Button>
          </div>
        </div>
        
        {breakdown && (
          <>
            <div className="breakdown-row">
              <span className="breakdown-label">Level-ups:</span>
              <div className="breakdown-value-container">
                <span className={`breakdown-value ${getValueClass(breakdown.levelUp)}`}>
                  {formatModifier(breakdown.levelUp)}
                </span>
              </div>
            </div>
            <div className="breakdown-row">
              <span className="breakdown-label">Racial:</span>
              <div className="breakdown-value-container">
                <span className={`breakdown-value ${getValueClass(breakdown.racial)}`}>
                  {formatModifier(breakdown.racial)}
                </span>
              </div>
            </div>
            <div className="breakdown-row">
              <span className="breakdown-label">Equipment:</span>
              <div className="breakdown-value-container">
                <span className={`breakdown-value ${getValueClass(breakdown.equipment)}`}>
                  {formatModifier(breakdown.equipment)}
                </span>
              </div>
            </div>
            {breakdown.enhancement !== 0 && (
              <div className="breakdown-row">
                <span className="breakdown-label">Enhancement:</span>
                <div className="breakdown-value-container">
                  <span className={`breakdown-value ${getValueClass(breakdown.enhancement)}`}>
                    {formatModifier(breakdown.enhancement)}
                  </span>
                </div>
              </div>
            )}
            {breakdown.temporary !== 0 && (
              <div className="breakdown-row">
                <span className="breakdown-label">Temporary:</span>
                <div className="breakdown-value-container">
                  <span className={`breakdown-value ${getValueClass(breakdown.temporary)}`}>
                    {formatModifier(breakdown.temporary)}
                  </span>
                </div>
              </div>
            )}
            <hr className="breakdown-divider" />
            <div className="breakdown-row breakdown-effective-row">
              <span className="breakdown-label">Effective:</span>
              <div className="breakdown-value-container">
                <span className="breakdown-value breakdown-effective">{value}</span>
              </div>
            </div>
            <div className="breakdown-row">
              <span className="breakdown-label">Modifier:</span>
              <div className="breakdown-value-container">
                <span className={`breakdown-value ${getModifierClass()}`}>
                  {formatModifier(modifier)}
                </span>
              </div>
            </div>
          </>
        )}
      </div>

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