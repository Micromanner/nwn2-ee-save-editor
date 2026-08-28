import { Button, Menu, MenuItem, Popover } from '@blueprintjs/core';
import { T } from '../theme';

export interface DropdownSelectItem {
  id: string | number;
  label: string;
}

export interface DropdownSelectProps {
  label?: string;
  value: string;
  items: DropdownSelectItem[];
  onChange: (id: number) => void;
  disabled?: boolean;
  valueColor?: string;
  small?: boolean;
  width?: number | string;
}

export function DropdownSelect({ label, value, items, onChange, disabled, valueColor, small, width }: DropdownSelectProps) {
  const menu = (
    <Menu style={{ maxHeight: 300, overflowY: 'auto' }}>
      {items.map(item => (
        <MenuItem key={item.id} text={item.label} active={item.label === value} onClick={() => onChange(Number(item.id))} />
      ))}
    </Menu>
  );
  return (
    <div style={{ opacity: disabled ? 0.35 : 1, pointerEvents: disabled ? 'none' : undefined, width }}>
      {label && <div className="t-semibold t-sm" style={{ color: T.textMuted, marginBottom: 3 }}>{label}</div>}
      <Popover content={menu} placement="bottom-start" minimal fill disabled={disabled}>
        <Button minimal small={small} rightIcon="caret-down" text={value} fill
          style={{ textAlign: 'left', border: `1px solid ${T.border}`, background: T.surface, color: valueColor }}
          disabled={disabled}
        />
      </Popover>
    </div>
  );
}
