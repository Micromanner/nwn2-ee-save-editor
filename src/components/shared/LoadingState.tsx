import { Spinner } from '@blueprintjs/core';
import { T } from '@/components/theme';

type Variant = 'page' | 'inline' | 'overlay';

interface LoadingStateProps {
  /// `page` fills its container, `inline` is a single row, `overlay` floats over a positioned parent.
  variant?: Variant;
  /// Names what is being loaded, e.g. "Loading inventory..."; omit to show only the spinner.
  text?: string;
  /// Fixed height for `page` when the parent has no height of its own (dialog bodies).
  height?: number | string;
}

const SIZE: Record<Variant, number> = { page: 36, inline: 16, overlay: 36 };

export function LoadingState({ variant = 'page', text, height }: LoadingStateProps) {
  const spinner = <Spinner className="nwn-spinner" size={SIZE[variant]} />;

  if (variant === 'inline') {
    return (
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '8px 0', color: T.textMuted, fontSize: 12 }}>
        {spinner}
        {text && <span>{text}</span>}
      </div>
    );
  }

  const centered: React.CSSProperties = {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 12,
  };

  if (variant === 'overlay') {
    return (
      <div style={{ ...centered, position: 'absolute', inset: 0, zIndex: 10, background: 'rgba(30, 30, 34, 0.35)', color: '#e8dcc8', fontSize: 12, letterSpacing: 0.4 }}>
        {spinner}
        {text && <span>{text}</span>}
      </div>
    );
  }

  return (
    <div style={{ ...centered, flex: 1, width: '100%', height: height ?? '100%', minHeight: height ? undefined : 160, color: T.textMuted, fontSize: 12, letterSpacing: 0.4 }}>
      {spinner}
      {text && <span>{text}</span>}
    </div>
  );
}
