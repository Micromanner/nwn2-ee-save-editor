import { Button } from '@blueprintjs/core';
import { T } from '../theme';

interface VariantStepperProps {
  value: number;
  variants: number[];
  onChange: (value: number) => void;
  disabled?: boolean;
}

export function VariantStepper({ value, variants, onChange, disabled = false }: VariantStepperProps) {
  const empty = variants.length === 0;
  const currentIndex = empty ? -1 : variants.indexOf(value);
  const displayIndex = currentIndex >= 0 ? currentIndex + 1 : 0;
  const total = variants.length;

  const canPrev = !empty && currentIndex > 0;
  const canNext = !empty && currentIndex < variants.length - 1;

  return (
    <div style={{ display: 'inline-flex', alignItems: 'center', gap: 0 }}>
      <Button
        small
        minimal
        icon="chevron-left"
        disabled={disabled || !canPrev}
        onClick={() => canPrev && onChange(variants[currentIndex - 1])}
        style={{ minWidth: 24, minHeight: 24, padding: 0 }}
      />
      <span className="t-semibold" style={{
        minWidth: 48,
        textAlign: 'center',
        color: T.text,
      }}>
        {displayIndex}/{total}
      </span>
      <Button
        small
        minimal
        icon="chevron-right"
        disabled={disabled || !canNext}
        onClick={() => canNext && onChange(variants[currentIndex + 1])}
        style={{ minWidth: 24, minHeight: 24, padding: 0 }}
      />
    </div>
  );
}
