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
  tempValue?: number;
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
  tempValue,
  onIncrease,
  onDecrease,
  onChange,
  min = 3,
  max = 40
}: AttributeCardProps) {
  const [isChanging, setIsChanging] = useState(false);
  const [clickedButton, setClickedButton] = useState<'increase' | 'decrease' | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Determine modifier class for color coding
  const getModifierClass = useCallback(() => {
    if (modifier > 0) return 'positive';
    if (modifier < 0) return 'negative';
    return 'zero';
  }, [modifier]);

  // Handle input change with validation
  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = parseInt(e.target.value) || min;
    const clampedValue = Math.max(min, Math.min(max, newValue));
    
    setIsChanging(true);
    onChange(clampedValue);
    
    // Reset animation state
    setTimeout(() => setIsChanging(false), 200);
  }, [min, max, onChange]);

  // Handle button press with animation
  const handleIncrease = useCallback(() => {
    setIsChanging(true);
    setClickedButton('increase');
    onIncrease();
    setTimeout(() => {
      setIsChanging(false);
      setClickedButton(null);
    }, 200);
  }, [onIncrease]);

  const handleDecrease = useCallback(() => {
    setIsChanging(true);
    setClickedButton('decrease');
    onDecrease();
    setTimeout(() => {
      setIsChanging(false);
      setClickedButton(null);
    }, 200);
  }, [onDecrease]);

  // Handle keyboard navigation
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    switch (e.key) {
      case 'ArrowUp':
        e.preventDefault();
        if (value < max) handleIncrease();
        break;
      case 'ArrowDown':
        e.preventDefault();
        if (value > min) handleDecrease();
        break;
      case '+':
      case '=':
        e.preventDefault();
        if (value < max) handleIncrease();
        break;
      case '-':
        e.preventDefault();
        if (value > min) handleDecrease();
        break;
    }
  }, [value, min, max, handleIncrease, handleDecrease]);

  return (
    <Card 
      variant="container"
      className="attribute-card-responsive"
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
          size="md"
          disabled={value <= min}
          clicked={clickedButton === 'decrease'}
          aria-label={`Decrease ${name}`}
          title={`Decrease ${name} (min: ${min})`}
        >
          −
        </Button>
        
        <input
          ref={inputRef}
          type="number"
          value={value}
          onChange={handleInputChange}
          onKeyDown={handleKeyDown}
          className="attribute-input-responsive"
          min={min}
          max={max}
          aria-label={`${name} value`}
          title={`${name}: ${value} (${formatModifier(modifier)})`}
          aria-describedby={`${shortName}-help`}
        />
        
        <Button
          onClick={handleIncrease}
          variant="outline"
          size="md"
          disabled={value >= max}
          clicked={clickedButton === 'increase'}
          aria-label={`Increase ${name}`}
          title={`Increase ${name} (max: ${max})`}
        >
          +
        </Button>
      </div>

      {tempValue && tempValue !== value && (
        <div 
          className="attribute-temp-value"
          aria-label={`Temporary ${name} modifier`}
          title={`Temporary value: ${tempValue} (${formatModifier(tempValue - value, false)} difference)`}
        >
          Temp: {tempValue} ({formatModifier(tempValue - value, false)})
        </div>
      )}
      
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