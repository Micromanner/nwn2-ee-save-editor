// Utility functions for safe data access with fallbacks

/**
 * Safely access nested object properties with a fallback value
 * @param obj The object to access
 * @param path The property path (e.g., 'user.profile.name')
 * @param fallback The fallback value if property doesn't exist
 */
export function get<T = unknown>(obj: Record<string, unknown>, path: string, fallback: T): T {
  const keys = path.split('.');
  let result: unknown = obj;
  
  for (const key of keys) {
    if (result == null || typeof result !== 'object') {
      return fallback;
    }
    result = (result as Record<string, unknown>)[key];
  }
  
  return (result ?? fallback) as T;
}

/**
 * Format a value with fallback for display
 * @param value The value to format
 * @param fallback The fallback string (default: '-')
 */
export function display(value: unknown, fallback = '-'): string {
  if (value === null || value === undefined || value === '') {
    return fallback;
  }
  
  if (typeof value === 'boolean') {
    return value ? 'Yes' : 'No';
  }
  
  if (typeof value === 'number') {
    return value.toString();
  }
  
  return String(value);
}

/**
 * Format a number with optional prefix (e.g., '+5' for positive numbers)
 */
export function formatModifier(value: number | null | undefined, showPositive = true): string {
  if (value == null) return '-';
  
  if (value > 0 && showPositive) {
    return `+${value}`;
  }
  
  return value.toString();
}

/**
 * Format a percentage value
 */
export function formatPercent(value: number | null | undefined, decimals = 0): string {
  if (value == null) return '-';
  
  return `${value.toFixed(decimals)}%`;
}

/**
 * Format large numbers with commas
 */
export function formatNumber(value: number | null | undefined): string {
  if (value == null) return '-';
  
  return value.toLocaleString();
}

/**
 * Get ability modifier from ability score
 */
export function getAbilityModifier(abilityScore: number): number {
  return Math.floor((abilityScore - 10) / 2);
}

/**
 * Format ability score with modifier
 */
export function formatAbilityScore(score: number | null | undefined): { score: string; modifier: string } {
  if (score == null) {
    return { score: '-', modifier: '-' };
  }
  
  const modifier = getAbilityModifier(score);
  return {
    score: score.toString(),
    modifier: formatModifier(modifier)
  };
}

/**
 * Join array values with fallback
 */
export function joinArray(arr: unknown[] | null | undefined, separator = ', ', fallback = '-'): string {
  if (!arr || arr.length === 0) return fallback;
  
  return arr.join(separator);
}

/**
 * Get length of array with fallback
 */
export function countArray(arr: unknown[] | null | undefined, fallback = 0): number {
  if (!arr) return fallback;
  return arr.length;
}

/**
 * Format time duration (in seconds) to readable format
 */
export function formatDuration(seconds: number | null | undefined): string {
  if (seconds == null) return '-';
  
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;
  
  return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
}

/**
 * Safe wrapper for rendering values in components
 */
export function safe<T>(value: T | null | undefined, fallback: T | string = '-'): T | string {
  return value ?? fallback;
}