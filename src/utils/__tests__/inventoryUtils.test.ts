import { describe, it, expect } from 'vitest';
import {
  INVENTORY_COLS,
  INVENTORY_ROWS,
  mapSlotName,
  parseInventoryGrid,
  resolveEquipSlot,
} from '../inventoryUtils';
import type { FullEquippedItem } from '@/lib/bindings';

describe('mapSlotName', () => {
  it('maps helmet to head', () => {
    expect(mapSlotName('helmet')).toBe('head');
  });

  it('maps l ring to left_ring', () => {
    expect(mapSlotName('l ring')).toBe('left_ring');
  });

  it('returns null for unknown slots', () => {
    expect(mapSlotName('unknown')).toBeNull();
  });

  it('is case-insensitive', () => {
    expect(mapSlotName('Helmet')).toBe('head');
  });
});

describe('parseInventoryGrid', () => {
  it('returns INVENTORY_COLS * INVENTORY_ROWS null slots for null data', () => {
    const result = parseInventoryGrid(null);
    expect(result).toHaveLength(INVENTORY_COLS * INVENTORY_ROWS);
    expect(result.every(s => s === null)).toBe(true);
  });

  it('places items at correct indices', () => {
    const data = {
      inventory: [
        {
          index: 0, name: 'Sword', base_item: 1, base_item_name: 'Longsword',
          is_custom: false, category: 'weapon', stack_size: 1, enhancement: 0,
          identified: true, plot: false, cursed: false, stolen: false,
          default_slot: null, charges: null, description: '', weight: 3, value: 15,
          base_ac: null, equippable_slots: [], decoded_properties: [], item: {},
        },
      ],
      equipped: [],
      gold: 0,
      encumbrance: { total_weight: 0, heavy_load: 150, status: 'normal' },
    };
    const result = parseInventoryGrid(data as any);
    expect(result[0]).not.toBeNull();
    expect(result[0]!.name).toBe('Sword');
    expect(result[1]).toBeNull();
  });
});

describe('resolveEquipSlot', () => {
  it('returns default_slot when not occupied', () => {
    const result = resolveEquipSlot('chest', ['chest', 'head'], []);
    expect(result).toBe('chest');
  });

  it('falls back to empty slot when default is occupied', () => {
    const equipped = [{ slot: 'left_ring' }] as FullEquippedItem[];
    const result = resolveEquipSlot('left_ring', ['left_ring', 'right_ring'], equipped);
    expect(result).toBe('right_ring');
  });

  it('returns default when no empty alternative exists', () => {
    const equipped = [{ slot: 'left_ring' }, { slot: 'right_ring' }] as FullEquippedItem[];
    const result = resolveEquipSlot('left_ring', ['left_ring', 'right_ring'], equipped);
    expect(result).toBe('left_ring');
  });
});
