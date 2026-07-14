import { Button } from '@blueprintjs/core';
import { T } from '../theme';

interface StepInputProps {
  value: number;
  onValueChange: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  width?: number;
  disabled?: boolean;
  /** Text shown instead of the raw number when `value` is 0 (e.g. '-' for an unset field). */
  zeroDisplay?: string;
}

export function StepInput({
  value,
  onValueChange,
  min = -Infinity,
  max = Infinity,
  step = 1,
  width = 88,
  disabled = false,
  zeroDisplay,
}: StepInputProps) {
  const clamp = (v: number) => Math.max(min, Math.min(max, v));

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const raw = e.target.value;
    if (raw === '' || raw === '-') return;
    const parsed = Number(raw);
    if (!isNaN(parsed)) onValueChange(clamp(parsed));
  };

  const displayValue = value === 0 && zeroDisplay !== undefined ? zeroDisplay : value;

  return (
    <div style={{ display: 'inline-flex', alignItems: 'center', gap: 0, width }}>
      <Button
        small
        minimal
        icon="minus"
        disabled={disabled || value <= min}
        onClick={() => onValueChange(clamp(value - step))}
        className="step-input-btn"
        style={{ minWidth: 22, minHeight: 22, padding: 0 }}
      />
      <input
        type="text"
        inputMode="numeric"
        value={displayValue}
        onChange={handleChange}
        disabled={disabled}
        className="t-lg t-semibold"
        style={{
          flex: 1,
          minWidth: 0,
          textAlign: 'center',
          fontFamily: 'inherit',
          color: T.text,
          background: T.surface,
          border: `1px solid ${T.borderLight}`,
          borderRadius: 3,
          padding: '2px 4px',
          height: 24,
          outline: 'none',
        }}
      />
      <Button
        small
        minimal
        icon="plus"
        disabled={disabled || value >= max}
        onClick={() => onValueChange(clamp(value + step))}
        className="step-input-btn"
        style={{ minWidth: 22, minHeight: 22, padding: 0 }}
      />
    </div>
  );
}
