/**
 * Enhanced icon URL builder for NWN2 icons with override support
 */

/**
 * Map icon name to full directory path in NWN2 Enhanced Edition
 * @param iconName - Icon name (e.g., "is_magicmissile")
 * @returns Full path (e.g., "evocation/spell/is_magicmissile")
 */
function mapIconNameToPath(iconName: string): string {
  if (!iconName) return iconName;
  
  const prefix = iconName.slice(0, 3).toLowerCase();
  
  // For now, try the icon name as-is first (no directory mapping)
  // This maintains backward compatibility while we test
  return iconName;
  
  // TODO: Add proper directory mapping based on icon prefixes
  // Examples:
  // is_calllightning -> evocation/spell/is_calllightning
  // ife_cm_divinebody -> feats/history/ife_cm_divinebody
  // it_* -> items/category/it_*
}

export interface IconOptions {
  /**
   * Use v2 enhanced icon API with full override support
   * @default true
   */
  useEnhanced?: boolean;
  
  /**
   * Icon category for legacy API (spells, feats, items, etc.)
   * Only used when useEnhanced is false
   */
  category?: string;
}

/**
 * Build icon URL for NWN2 icons
 * @param iconName - Icon name (e.g., "is_magicmissile", "feat_alertness")
 * @param options - Options for icon URL generation
 * @returns Full URL to the icon
 */
export function buildIconUrl(iconName: string, options: IconOptions = {}): string {
  const { useEnhanced = true, category } = options;
  
  if (!iconName) {
    return '';
  }
  
  const apiUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000/api';
  const baseUrl = apiUrl.replace('/api', ''); // Remove /api suffix to get base URL
  
  // Always use the enhanced API now (legacy removed)
  return `${baseUrl}/api/gamedata/icons/${iconName}/`;
}

/**
 * Get icon category from icon name prefix
 * @param iconName - Icon name (e.g., "is_magicmissile")
 * @returns Category name or null
 */
export function getIconCategory(iconName: string): string | null {
  if (!iconName) return null;
  
  const prefix = iconName.slice(0, 3).toLowerCase();
  
  switch (prefix) {
    case 'is_': return 'spells';
    case 'ife_': return 'feats';
    case 'isk_': return 'skills';
    case 'it_': return 'items';
    default: return null;
  }
}

/**
 * Enhanced icon statistics fetcher
 */
export async function fetchIconStats(): Promise<{
  initialized: boolean;
  initializing: boolean;
  statistics: {
    base_count: number;
    override_count: number;
    workshop_count: number;
    hak_count: number;
    module_count: number;
    total_count: number;
    total_size: number;
  };
  format: string;
  mimetype: string;
}> {
  const apiUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000/api';
  const response = await fetch(`${apiUrl}/gamedata/icons/`);
  
  if (!response.ok) {
    throw new Error('Failed to fetch icon statistics');
  }
  
  return response.json();
}

/**
 * Update icon cache with module HAK files
 * @param hakList - List of HAK file names
 */
export async function updateModuleIcons(hakList: string[]): Promise<{
  success: boolean;
  haks_loaded: number;
  statistics: Record<string, unknown>;
}> {
  const apiUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000/api';
  const response = await fetch(`${apiUrl}/gamedata/icons/module/`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ hak_list: hakList }),
  });
  
  if (!response.ok) {
    throw new Error('Failed to update module icons');
  }
  
  return response.json();
}