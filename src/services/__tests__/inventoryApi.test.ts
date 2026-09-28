import { describe, it, expect } from 'vitest';
import { isVanillaSource } from '../inventoryApi';

describe('isVanillaSource', () => {
  it('treats base_game as vanilla', () => {
    expect(isVanillaSource('base_game')).toBe(true);
  });

  it.each(['module', 'campaign', 'override_dir', 'workshop', 'custom_override', 'hak'])('treats %s as override', (source) => {
    expect(isVanillaSource(source)).toBe(false);
  });

  it('treats unknown source strings as override (safe default)', () => {
    expect(isVanillaSource('some_future_source')).toBe(false);
  });
});
