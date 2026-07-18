import { useRef, useCallback } from 'react';
import { Tooltip } from '@blueprintjs/core';
import { T } from '../theme';
import type { TintChannel } from '@/lib/bindings';

interface ColorPickerProps {
  label: string;
  value: TintChannel;
  onChange: (value: TintChannel) => void;
  disabled?: boolean;
  disabledReason?: string;
}

function tintToHex(c: TintChannel): string {
  const hex = (n: number) => n.toString(16).padStart(2, '0');
  return `#${hex(c.r)}${hex(c.g)}${hex(c.b)}`;
}

function hexToTint(hex: string, a: number): TintChannel {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return { r, g, b, a };
}

export function ColorPicker({ label, value, onChange, disabled, disabledReason }: ColorPickerProps) {
  const debounceRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  const handleChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const hex = e.target.value;
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      onChange(hexToTint(hex, value.a));
    }, 300);
  }, [onChange, value.a]);

  const picker = (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, opacity: disabled ? 0.4 : 1 }}>
      <input
        type="color"
        value={tintToHex(value)}
        onChange={handleChange}
        disabled={disabled}
        style={{
          width: 28,
          height: 28,
          padding: 0,
          border: `1px solid ${T.borderLight}`,
          borderRadius: 3,
          cursor: disabled ? 'not-allowed' : 'pointer',
          background: 'none',
        }}
      />
      <span className="t-base" style={{ color: T.textMuted }}>{label}</span>
    </div>
  );

  if (disabled && disabledReason) {
    return <Tooltip content={disabledReason} placement="right">{picker}</Tooltip>;
  }
  return picker;
}
