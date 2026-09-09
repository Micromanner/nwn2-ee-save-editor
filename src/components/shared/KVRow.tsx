import { T } from '../theme';

export function KVRow({ label, value, color }: { label: string; value: React.ReactNode; color?: string }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', padding: '5px 0' }}>
      <span style={{ color: T.textMuted }}>{label}</span>
      <span className="t-semibold" style={{ color: color || T.text }}>{value}</span>
    </div>
  );
}
