// Utility functions for safe data access with fallbacks

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
 * Format large numbers with commas
 */
export function formatNumber(value: number | null | undefined): string {
  if (value == null) return '-';
  
  return value.toLocaleString();
}

/**
 * Safely parse unknown value to number with fallback
 */
export function safeToNumber(value: unknown, defaultValue: number = 0): number {
  if (typeof value === 'number') return value;
  if (typeof value === 'string') {
    const parsed = parseFloat(value);
    return isNaN(parsed) ? defaultValue : parsed;
  }
  return defaultValue;
}