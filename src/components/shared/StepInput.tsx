import { useEffect, useRef, useState } from 'react';
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
  // Values keep as many decimals as `step` has, so an integer step rounds typed
  // fractions and a fractional step avoids float noise like 0.30000000000000004.
  const decimals = String(step).split('.')[1]?.length ?? 0;
  const clamp = (v: number) => Math.max(min, Math.min(max, Number(v.toFixed(decimals))));

  // Typed text is held locally and committed on blur/Enter so a value like
  // "18" produces one history entry, not one per keystroke.
  const [draft, setDraft] = useState<string | null>(null);
  useEffect(() => setDraft(null), [value]);

  const commitDraft = () => {
    if (draft === null) return;
    const parsed = Number(draft);
    if (draft !== '' && draft !== '-' && !isNaN(parsed) && clamp(parsed) !== value) onValueChange(clamp(parsed));
    setDraft(null);
  };

  // Commit a pending edit if the input unmounts before blur/Enter fires (e.g.
  // typing a value then switching tabs), so it isn't silently discarded. The
  // ref is reassigned every render so the cleanup always sees the latest
  // draft/value/onValueChange instead of the ones from the first render.
  const commitDraftRef = useRef(commitDraft);
  commitDraftRef.current = commitDraft;
  useEffect(() => () => commitDraftRef.current(), []);

  const displayValue = draft ?? (value === 0 && zeroDisplay !== undefined ? zeroDisplay : value);

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
        inputMode={decimals > 0 ? 'decimal' : 'numeric'}
        value={displayValue}
        onChange={e => setDraft(e.target.value)}
        onBlur={commitDraft}
        onKeyDown={e => { if (e.key === 'Enter') e.currentTarget.blur(); }}
        disabled={disabled}
        className="t-semibold"
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
