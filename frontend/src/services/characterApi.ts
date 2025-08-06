// Character API service for fetching character data from Django backend

export interface CharacterAbilities {
  strength: number;
  dexterity: number;
  constitution: number;
  intelligence: number;
  wisdom: number;
  charisma: number;
}

export interface CharacterSaves {
  fortitude: number;
  reflex: number;
  will: number;
}

export interface CharacterClass {
  name: string;
  level: number;
}

export interface DamageResistance {
  type: string;
  amount: number;
}

export interface CharacterData {
  id?: number;
  name: string;
  race: string;
  subrace?: string;
  gender: string;
  classes: CharacterClass[];
  alignment: string;
  deity: string;
  level: number;
  experience: number;
  hitPoints: number;
  maxHitPoints: number;
  abilities: CharacterAbilities;
  saves: CharacterSaves;
  armorClass: number;
  gold: number;
  location?: string;
  playTime?: string;
  lastSaved?: string;
  portrait?: string;
  customPortrait?: string;
  // Appearance properties
  appearance?: number;
  soundset?: number;
  bodyType?: number;
  hairStyle?: number;
  hairColor?: number;
  skinColor?: number;
  headVariation?: number;
  tattooColor1?: number;
  tattooColor2?: number;
  // Combat stats
  baseAttackBonus: number;
  meleeAttackBonus?: number;
  rangedAttackBonus?: number;
  mainHandDamage?: string;
  offHandDamage?: string;
  // Character progress
  totalSkillPoints?: number;
  availableSkillPoints?: number;
  totalFeats?: number;
  knownSpells?: number;
  // Defenses
  damageResistances?: DamageResistance[];
  damageImmunities?: string[];
  spellResistance?: number;
  // Physical stats
  movementSpeed?: number;
  size?: string;
  initiative?: number;
  // Campaign stats
  completedQuests?: number;
  currentQuests?: number;
  journalEntries?: number;
  companionsRecruited?: number;
  deaths?: number;
  killCount?: number;
  // Campaign info
  campaignName?: string;
  moduleName?: string;
  campaignModules?: string[];
  // Enhanced campaign data
  gameAct?: number;
  difficultyLevel?: number;
  lastSavedTimestamp?: number;
  companionStatus?: Record<string, {name: string, influence: number, status: string, influence_found: boolean}>;
  hiddenStatistics?: Record<string, number>;
  storyMilestones?: Record<string, {name: string, milestones: Array<{description: string, completed: boolean, variable: string}>}>;
  questDetails?: {
    summary: {
      completed_quests: number;
      active_quests: number;
      total_quest_variables: number;
      completed_quest_list: string[];
      active_quest_list: string[];
    };
    categories: Record<string, {
      name: string;
      completed: string[];
      active: string[];
    }>;
    progress_stats: {
      total_completion_rate: number;
      main_story_progress: number;
      companion_progress: number;
      exploration_progress: number;
    };
  };
  // Session & locale data
  detectedLanguage?: string;
  languageId?: number;
  localizationStatus?: string;
  createdAt?: string;
  updatedAt?: string;
}

// Base API URL - assumes Django is running on localhost:8000
const API_BASE_URL = 'http://localhost:8000/api';

export class CharacterAPI {
  // Get character state (comprehensive data)
  static async getCharacterState(characterId: number): Promise<CharacterData> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/character_state/`);
    if (!response.ok) {
      throw new Error(`Failed to fetch character state: ${response.statusText}`);
    }
    
    const data = await response.json();
    return this.mapBackendToFrontend(data);
  }

  // Get character details (basic data)
  static async getCharacterDetails(characterId: number): Promise<CharacterData> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/`);
    if (!response.ok) {
      throw new Error(`Failed to fetch character details: ${response.statusText}`);
    }
    
    const data = await response.json();
    return this.mapBackendToFrontend(data);
  }

  // List all characters
  static async listCharacters(): Promise<CharacterData[]> {
    const response = await fetch(`${API_BASE_URL}/characters/`);
    if (!response.ok) {
      throw new Error(`Failed to list characters: ${response.statusText}`);
    }
    
    const data = await response.json();
    return data.map((char: Record<string, unknown>) => this.mapBackendToFrontend(char));
  }

  // Import character from save game
  static async importCharacter(savePath: string): Promise<CharacterData> {
    const response = await fetch(`${API_BASE_URL}/savegames/import/`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ save_path: savePath }),
    });
    
    if (!response.ok) {
      const errorData = await response.json();
      const errorMessage = errorData.error?.message || `Failed to import character: ${response.statusText}`;
      throw new Error(errorMessage);
    }
    
    const data = await response.json();
    // The API returns the character data directly, not wrapped in a 'character' property
    return this.mapBackendToFrontend(data);
  }

  // Update character data
  static async updateCharacter(characterId: number, updates: Partial<{ first_name: string; last_name: string; [key: string]: unknown }>): Promise<CharacterData> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/`, {
      method: 'PATCH',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(updates),
    });
    
    if (!response.ok) {
      throw new Error(`Failed to update character: ${response.statusText}`);
    }
    
    const data = await response.json();
    return this.mapBackendToFrontend(data);
  }

  // Save character changes to save game
  static async saveCharacter(characterId: number, updates: Record<string, unknown> = {}): Promise<{ success: boolean; changes: any; backup_created: boolean }> {
    const response = await fetch(`${API_BASE_URL}/savegames/${characterId}/update/`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ updates }),
    });
    
    if (!response.ok) {
      const errorData = await response.json();
      const errorMessage = errorData.error?.message || `Failed to save character: ${response.statusText}`;
      throw new Error(errorMessage);
    }
    
    return response.json();
  }

  // Feat management methods
  static async getCharacterFeats(characterId: number, featType?: number): Promise<any> {
    const typeParam = featType !== undefined ? `?type=${featType}` : '';
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/feats/state/${typeParam}`);
    if (!response.ok) {
      throw new Error(`Failed to fetch character feats: ${response.statusText}`);
    }
    return response.json();
  }

  static async getLegitimateFeats(
    characterId: number, 
    options: { 
      featType?: number; 
      page?: number; 
      limit?: number; 
      search?: string; 
    } = {}
  ): Promise<{ feats: any[]; pagination: any; search?: string }> {
    const params = new URLSearchParams();
    if (options.featType !== undefined) params.append('type', options.featType.toString());
    if (options.page !== undefined) params.append('page', options.page.toString());
    if (options.limit !== undefined) params.append('limit', options.limit.toString());
    if (options.search) params.append('search', options.search);
    
    const queryString = params.toString();
    const url = `${API_BASE_URL}/characters/${characterId}/feats/legitimate${queryString ? `?${queryString}` : ''}`;
    
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`Failed to fetch legitimate feats: ${response.statusText}`);
    }
    const data = await response.json();
    return {
      feats: data.legitimate_feats || [],
      pagination: data.pagination || {},
      search: data.search
    };
  }

  static async addFeat(characterId: number, featId: number): Promise<any> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/feats/add/`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ feat_id: featId }),
    });
    
    if (!response.ok) {
      const errorData = await response.json();
      throw new Error(errorData.error || `Failed to add feat: ${response.statusText}`);
    }
    
    return response.json();
  }

  static async removeFeat(characterId: number, featId: number): Promise<any> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/feats/remove/`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ feat_id: featId }),
    });
    
    if (!response.ok) {
      const errorData = await response.json();
      throw new Error(errorData.error || `Failed to remove feat: ${response.statusText}`);
    }
    
    return response.json();
  }

  static async getFeatDetails(characterId: number, featId: number): Promise<any> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/feats/${featId}/details/`);
    if (!response.ok) {
      throw new Error(`Failed to fetch feat details: ${response.statusText}`);
    }
    return response.json();
  }

  // Skills API methods
  static async getSkillsState(characterId: number): Promise<any> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/skills/state/`);
    if (!response.ok) {
      throw new Error(`Failed to fetch skills state: ${response.statusText}`);
    }
    return response.json();
  }

  static async updateSkills(characterId: number, skills: Record<number, number>): Promise<any> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/skills/update/`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ skills }),
    });
    
    if (!response.ok) {
      throw new Error(`Failed to update skills: ${response.statusText}`);
    }
    return response.json();
  }

  static async resetSkills(characterId: number): Promise<any> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/skills/reset/`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
    });
    
    if (!response.ok) {
      throw new Error(`Failed to reset skills: ${response.statusText}`);
    }
    return response.json();
  }

  // Attributes API methods
  static async getAttributesState(characterId: number): Promise<any> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/attributes/`);
    if (!response.ok) {
      throw new Error(`Failed to fetch attributes state: ${response.statusText}`);
    }
    return response.json();
  }

  static async updateAttributes(characterId: number, attributes: Record<string, number>): Promise<any> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/attributes/update/`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ attributes }),
    });
    
    if (!response.ok) {
      throw new Error(`Failed to update attributes: ${response.statusText}`);
    }
    return response.json();
  }

  // Alignment API methods
  static async getAlignment(characterId: number): Promise<{ lawChaos: number; goodEvil: number }> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/alignment/`);
    if (!response.ok) {
      throw new Error(`Failed to fetch alignment: ${response.statusText}`);
    }
    return response.json();
  }

  static async updateAlignment(characterId: number, alignment: { lawChaos: number; goodEvil: number }): Promise<any> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/update_alignment/`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(alignment),
    });
    
    if (!response.ok) {
      throw new Error(`Failed to update alignment: ${response.statusText}`);
    }
    return response.json();
  }

  // Combat stats API methods
  static async updateArmorClass(characterId: number, naturalAC: number): Promise<any> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/combat/update-ac/`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ natural_ac: naturalAC }),
    });
    
    if (!response.ok) {
      throw new Error(`Failed to update armor class: ${response.statusText}`);
    }
    return response.json();
  }

  // Saving throws API methods
  static async updateSavingThrows(characterId: number, saveUpdates: Record<string, number>): Promise<any> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/saves/update/`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ save_bonuses: saveUpdates }),
    });
    
    if (!response.ok) {
      throw new Error(`Failed to update saving throws: ${response.statusText}`);
    }
    return response.json();
  }

  // Map backend data structure to frontend interface
  private static mapBackendToFrontend(backendData: Record<string, unknown>): CharacterData {
    // Handle null or undefined input
    if (!backendData) {
      throw new Error('No character data received from backend');
    }
    
    // Calculate derived values
    const classes = (backendData.classes as Array<Record<string, unknown>>) || [];
    const totalLevel = classes.reduce((sum: number, cls: { class_level?: number; level?: number }) => sum + (cls.class_level || cls.level || 0), 0);
    
    // Map alignment from law_chaos and good_evil
    const alignmentMap: { [key: string]: string } = {
      '0_0': 'True Neutral',
      '0_1': 'Neutral Good',
      '0_2': 'Neutral Evil',
      '1_0': 'Lawful Neutral',
      '1_1': 'Lawful Good',
      '1_2': 'Lawful Evil',
      '2_0': 'Chaotic Neutral',
      '2_1': 'Chaotic Good',
      '2_2': 'Chaotic Evil',
    };
    
    const alignmentKey = `${backendData.law_chaos || 0}_${backendData.good_evil || 0}`;
    const alignment = backendData.alignment || alignmentMap[alignmentKey] || 'True Neutral';

    return {
      id: backendData.id as number | undefined,
      name: `${backendData.first_name || '-'} ${backendData.last_name || ''}`.trim() || 'Unknown',
      race: String(backendData.race_name || backendData.race || 'Unknown'),
      subrace: backendData.subrace_name ? String(backendData.subrace_name) : backendData.subrace ? String(backendData.subrace) : undefined,
      gender: backendData.gender === 0 ? 'Male' : 'Female',
      classes: classes.map((cls) => ({
        name: String(cls.class_name || cls.name || 'Unknown Class'),
        level: Number(cls.class_level || cls.level || 1)
      })),
      alignment: String(alignment),
      deity: String(backendData.deity || 'None'),
      level: Number(backendData.character_level || totalLevel || 1),
      experience: Number(backendData.experience || 0),
      hitPoints: Number(backendData.hit_points || backendData.current_hit_points || 10),
      maxHitPoints: Number(backendData.max_hit_points || 10),
      abilities: {
        strength: Number(backendData.strength || 10),
        dexterity: Number(backendData.dexterity || 10),
        constitution: Number(backendData.constitution || 10),
        intelligence: Number(backendData.intelligence || 10),
        wisdom: Number(backendData.wisdom || 10),
        charisma: Number(backendData.charisma || 10)
      },
      saves: {
        fortitude: Number(backendData.fortitude_save || 0),
        reflex: Number(backendData.reflex_save || 0),
        will: Number(backendData.will_save || 0)
      },
      armorClass: Number(backendData.armor_class || 10),
      gold: Number(backendData.gold || 0),
      location: backendData.current_area ? String(backendData.current_area) : backendData.area_name ? String(backendData.area_name) : backendData.module_name ? String(backendData.module_name) : undefined,
      portrait: backendData.portrait ? String(backendData.portrait) : undefined,
      customPortrait: backendData.custom_portrait ? String(backendData.custom_portrait) : undefined,
      // Combat stats
      baseAttackBonus: Number(backendData.base_attack_bonus || 0),
      // Character progress (from character_state if available)
      totalSkillPoints: backendData.skill_points_total ? Number(backendData.skill_points_total) : undefined,
      availableSkillPoints: backendData.skill_points_available ? Number(backendData.skill_points_available) : undefined,
      totalFeats: (backendData.feats as Array<unknown>)?.length || 0,
      // Physical stats
      movementSpeed: 30, // Default, as it's not in the backend data
      size: 'Medium', // Default, as it's not in the backend data
      initiative: Math.floor(((backendData.dexterity as number) - 10) / 2) || 0,
      // Campaign info from backend
      ...(backendData.campaign_name ? {
        campaignName: String(backendData.campaign_name),
        moduleName: String(backendData.module_name || ''),
        campaignModules: backendData.campaign_modules as string[] || [],
      } : {}),
      // Quest data from globals.xml
      ...(backendData.completed_quests_count !== undefined ? {
        completedQuests: Number(backendData.completed_quests_count),
        currentQuests: Number(backendData.active_quests_count || 0),
        companionInfluence: backendData.companion_influence as Record<string, number> || {},
        unlockedLocations: backendData.unlocked_locations as string[] || [],
      } : {}),
      // Enhanced campaign data
      ...(backendData.game_act !== undefined ? {
        gameAct: Number(backendData.game_act),
        difficultyLevel: Number(backendData.difficulty_level || 1),
        lastSavedTimestamp: Number(backendData.last_saved_timestamp || 0),
        companionStatus: backendData.companion_status as Record<string, {name: string, influence: number, status: string, influence_found: boolean}> || {},
        hiddenStatistics: backendData.hidden_statistics as Record<string, number> || {},
        storyMilestones: backendData.story_milestones as Record<string, {name: string, milestones: Array<{description: string, completed: boolean, variable: string}>}> || {},
        questDetails: backendData.quest_details as any || undefined,
      } : {})
    };
  }
}