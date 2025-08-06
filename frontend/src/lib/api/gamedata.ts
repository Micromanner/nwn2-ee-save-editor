import { apiClient } from './client';

// Response types
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

// Service class for all game data endpoints
export class GameDataService {
  private readonly basePath = '/gamedata';

  // Simple endpoints that return dictionaries
  private async getDict<T = Record<string, unknown>>(endpoint: string): Promise<T> {
    return apiClient.get<T>(`${this.basePath}/${endpoint}/`);
  }

  // Paged endpoints that return results arrays
  private async getPaged<T = unknown>(endpoint: string, page?: number): Promise<PagedResponse<T>> {
    const params = page ? `?page=${page}` : '';
    return apiClient.get<PagedResponse<T>>(`${this.basePath}/${endpoint}/${params}`);
  }

  // Appearance endpoints
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

  // Character creation endpoints
  races = () => this.getPaged('races');
  subraces = () => this.getPaged('subraces');
  classes = () => this.getPaged('classes');
  genders = () => this.getPaged('genders');
  alignments = () => this.getPaged('alignments');
  deities = () => this.getPaged('deities');
  domains = () => this.getPaged('domains');
  backgrounds = () => this.getPaged('backgrounds');
  
  // Character progression endpoints
  feats = async (characterId: number, featType?: number) => {
    const typeParam = featType !== undefined ? `?type=${featType}` : '';
    const response = await apiClient.get<{legitimate_feats: GameDataItem[], total: number}>(`/characters/${characterId}/feats/legitimate/${typeParam}`);
    return response.legitimate_feats;
  };
  skills = () => this.getPaged('skills');
  spells = async (characterId: number) => {
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
    const response = await apiClient.get<{spells: SpellResponse[], count: number, total_by_level: Record<string, number>}>(`/characters/${characterId}/spells/all/`);
    return response.spells;
  };
  abilities = () => this.getPaged('abilities');
  
  // Item endpoints
  baseItems = () => this.getPaged('base_items');
  itemProperties = () => this.getPaged('item_properties');
  
  // Category endpoints
  featCategories = () => this.getPaged('feat_categories');
  spellSchools = () => this.getPaged('spell_schools');
  skillCategories = () => this.getPaged('skill_categories');
  
  // Companion endpoints
  companions = () => this.getPaged('companions');
  packages = () => this.getPaged('packages');
}

export const gameData = new GameDataService();