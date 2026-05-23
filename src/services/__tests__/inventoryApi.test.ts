import { describe, it, expect } from 'vitest';
import { isVanillaSource, VANILLA_SOURCES } from '../inventoryApi';

describe('isVanillaSource', () => {
  it('treats Base Game as vanilla', () => {
    expect(isVanillaSource('Base Game')).toBe(true);
  });

  it('treats Expansion as vanilla', () => {
    expect(isVanillaSource('Expansion')).toBe(true);
  });

  it('treats HAK Pack as override', () => {
    expect(isVanillaSource('HAK Pack')).toBe(false);
  });

  it('treats Steam Workshop as override', () => {
    expect(isVanillaSource('Steam Workshop')).toBe(false);
  });

  it('treats Override Directory as override', () => {
    expect(isVanillaSource('Override Directory')).toBe(false);
  });

  it('treats Custom Override as override', () => {
    expect(isVanillaSource('Custom Override')).toBe(false);
  });

  it('treats Module as override', () => {
    expect(isVanillaSource('Module')).toBe(false);
  });

  it('treats Campaign as override', () => {
    expect(isVanillaSource('Campaign')).toBe(false);
  });

  it('treats unknown source strings as override (safe default)', () => {
    expect(isVanillaSource('Some Future Source')).toBe(false);
  });

  it('exposes VANILLA_SOURCES as a Set with exactly two entries', () => {
    expect(VANILLA_SOURCES.size).toBe(2);
    expect(VANILLA_SOURCES.has('Base Game')).toBe(true);
    expect(VANILLA_SOURCES.has('Expansion')).toBe(true);
  });
});
