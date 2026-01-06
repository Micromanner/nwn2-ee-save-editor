import DynamicAPI from '../utils/dynamicApi';

export function buildIconUrl(iconName: string): string {
  if (!iconName) {
    return '';
  }

  const cachedBase = DynamicAPI.getCachedBaseUrl();
  if (!cachedBase) {
    return '';
  }

  return `${cachedBase}/api/gamedata/icons/${iconName}/`;
}

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
  const response = await DynamicAPI.fetch(`/gamedata/icons/`);
  
  if (!response.ok) {
    throw new Error('Failed to fetch icon statistics');
  }
  
  return response.json();
}

export async function updateModuleIcons(hakList: string[]): Promise<{
  success: boolean;
  haks_loaded: number;
  statistics: Record<string, unknown>;
}> {
  const response = await DynamicAPI.fetch(`/gamedata/icons/module/`, {
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
