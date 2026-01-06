import { apiClient } from './client';

export interface PagedResponse<T> {
  count: number;
  next: string | null;
  previous: string | null;
  results: T[];
  page?: number;
  page_size?: number;
}

export interface GameDataItem {
  id: number;
  label: string;
  name: string;
  [key: string]: unknown;
}

export class GameDataService {
  private readonly basePath = '/gamedata';

  private async getDict<T = Record<string, unknown>>(endpoint: string): Promise<T> {
    return apiClient.get<T>(`${this.basePath}/${endpoint}`);
  }

  private async getPaged<T = unknown>(endpoint: string, page?: number): Promise<PagedResponse<T>> {
    const params = page ? `?page=${page}` : '';
    return apiClient.get<PagedResponse<T>>(`${this.basePath}/${endpoint}${params}`);
  }

  appearance = {
    get: () => this.getDict('appearance'),
    getPortraits: () => this.getDict('portraits'),
    getSoundsets: () => this.getDict('soundsets'),
    getAll: () => this.getDict<{
      appearance: Record<string, GameDataItem>;
      portraits: Record<string, unknown>;
      soundsets: Record<string, unknown>;
      gender: Record<string, unknown>;
    }>('appearance_all'),
  };

  races = () => this.getPaged('races');
  subraces = () => this.getPaged('subraces');
  classes = () => this.getPaged('classes');
  genders = () => this.getPaged('genders');
  alignments = () => this.getPaged('alignments');
  deities = () => this.getPaged('deities');
  domains = () => this.getPaged('domains');
  backgrounds = () => this.getPaged('backgrounds');

  feats = async (characterId: number, featType?: number) => {
    const typeParam = featType !== undefined ? `?type=${featType}` : '';
    const response = await apiClient.get<{legitimate_feats: GameDataItem[], total: number}>(`/characters/${characterId}/feats/legitimate${typeParam}`);
    return response.legitimate_feats;
  };
  skills = () => this.getPaged('skills');
  spells = async (characterId: number, filters?: {
    level?: number;
    school?: string;
    search?: string;
  }) => {
    interface SpellResponse {
      id: number;
      name: string;
      icon: string;
      school_id: number;
      school_name: string | null;
      level: number;
      available_classes: string[];
      description: string;
      range: string;
      cast_time: string;
      conjuration_time: string;
      components: string;
      metamagic: string;
      target_type: string;
    }
    
    const params = new URLSearchParams();
    if (filters?.level !== undefined && filters.level !== -1) {
      params.append('level', filters.level.toString());
    }
    if (filters?.school && filters.school !== 'all') {
      params.append('school', filters.school);
    }
    if (filters?.search && filters.search.trim()) {
      params.append('search', filters.search.trim());
    }
    
    const queryString = params.toString();
    const url = `/characters/${characterId}/spells/all${queryString ? `?${queryString}` : ''}`;
    
    const response = await apiClient.get<{spells: SpellResponse[], count: number, total_by_level: Record<string, number>}>(url);
    return response.spells;
  };
  abilities = () => this.getPaged('abilities');

  baseItems = () => this.getPaged('base_items');
  itemProperties = () => this.getPaged('item_properties');

  featCategories = () => this.getPaged('feat_categories');
  spellSchools = () => this.getPaged('spell_schools');
  skillCategories = () => this.getPaged('skill_categories');

  companions = () => this.getPaged('companions');
  packages = () => this.getPaged('packages');
}

export const gameData = new GameDataService();