import DynamicAPI from '../lib/utils/dynamicApi';

export interface CompanionInfluenceData {
  name: string;
  influence: number | null;
  recruitment: string;
  source: string;
}

export interface CompanionInfluenceResponse {
  companions: Record<string, CompanionInfluenceData>;
}

export interface UpdateCompanionInfluenceRequest {
  companion_id: string;
  new_influence: number;
}

export interface UpdateCompanionInfluenceResponse {
  success: boolean;
  companion_id: string;
  old_influence: number;
  new_influence: number;
  message: string;
  has_unsaved_changes: boolean;
}

export interface QuestVariable {
  name: string;
  value: number | string;
  type: string;
  category?: string;
}

export interface QuestGroup {
  prefix: string;
  name: string;
  variables: QuestVariable[];
  completed_count: number;
  active_count: number;
  total_count: number;
}

export interface QuestDetailsResponse {
  groups: QuestGroup[];
  total_quests: number;
  completed_quests: number;
  active_quests: number;
  unknown_quests: number;
  completion_rate: number;
}

export interface UpdateQuestVariableRequest {
  variable_name: string;
  value: number | string;
  variable_type: string;
}

export interface UpdateQuestVariableResponse {
  success: boolean;
  variable_name: string;
  old_value: number | string;
  new_value: number | string;
  message: string;
  has_unsaved_changes: boolean;
}

export interface BatchQuestUpdate {
  variable_name: string;
  value: number | string;
  variable_type: string;
}

export interface BatchUpdateQuestsResponse {
  success: boolean;
  updated_count: number;
  failed_count: number;
  updates: Array<{
    variable_name: string;
    success: boolean;
    error?: string;
  }>;
  message: string;
  has_unsaved_changes: boolean;
}

export interface CampaignVariable {
  variable_name: string;
  value: number | string;
  variable_type: string;
}

export interface CampaignVariablesResponse {
  integers: Record<string, number>;
  strings: Record<string, string>;
  floats: Record<string, number>;
  total_count: number;
}

export interface UpdateCampaignVariableRequest {
  variable_name: string;
  value: number | string;
  variable_type: string;
}

export interface UpdateCampaignVariableResponse {
  success: boolean;
  variable_name: string;
  old_value: number | string;
  new_value: number | string;
  message: string;
  has_unsaved_changes: boolean;
}

export interface CampaignSettingsResponse {
  campaign_file_path: string;
  guid: string;
  display_name: string;
  description: string;
  level_cap: number;
  xp_cap: number;
  companion_xp_weight: number;
  henchman_xp_weight: number;
  attack_neutrals: number;
  auto_xp_award: number;
  journal_sync: number;
  no_char_changing: number;
  use_personal_reputation: number;
  start_module: string;
  module_names: string[];
}

export interface UpdateCampaignSettingsRequest {
  level_cap?: number;
  xp_cap?: number;
  companion_xp_weight?: number;
  henchman_xp_weight?: number;
  attack_neutrals?: number;
  auto_xp_award?: number;
  journal_sync?: number;
  no_char_changing?: number;
  use_personal_reputation?: number;
}

export interface UpdateCampaignSettingsResponse {
  success: boolean;
  updated_fields: string[];
  warning: string;
}

export interface QuestProgressData {
  variable: string;
  category: string;
  name: string;
  description?: string;
  current_stage: number;
  is_completed: boolean;
  xp: number;
  source: string;
  type_hint?: string;
}

export interface QuestProgressResponse {
  quests: QuestProgressData[];
  total_count: number;
}

export interface PlotVariableData {
  name: string;
  display_name?: string;
  description?: string;
  value: number | string;
  type: string;
  has_definition: boolean;
  category?: string;
  quest_text?: string;
  type_hint?: string;
}

export interface PlotVariablesResponse {
  quest_variables: PlotVariableData[];
  unknown_variables: PlotVariableData[];
  total_count: number;
}

export interface KnownQuestValue {
  value: number;
  description: string;
  is_completed: boolean;
}

export interface QuestInfoData {
  category: string;
  category_name: string;
  entry_id: number;
  quest_name: string;
  current_stage_text: string;
  xp: number;
}

export interface EnrichedQuestData {
  variable_name: string;
  current_value: number;
  variable_type: string;
  quest_info: QuestInfoData | null;
  known_values: KnownQuestValue[];
  confidence: 'high' | 'medium' | 'low';
  source: string;
  is_completed: boolean;
  is_active: boolean;
}

export interface UnmappedVariableData {
  variable_name: string;
  display_name: string;
  current_value: number | string;
  variable_type: string;
  category: string;
}

export interface QuestStats {
  total: number;
  completed: number;
  active: number;
  unmapped: number;
}

export interface DialogueCacheInfo {
  cached: boolean;
  version?: string;
  generated_at?: string;
  dialogue_count: number;
  mapping_count: number;
  campaign_name: string;
}

export interface EnrichedQuestsResponse {
  quests: EnrichedQuestData[];
  unmapped_variables: UnmappedVariableData[];
  stats: QuestStats;
  cache_info: DialogueCacheInfo;
}

export interface ModuleInfo {
  module_name: string;
  area_name: string;
  campaign: string;
  entry_area: string;
  module_description: string;
  current_module?: string;
}

export interface ModuleVariablesResponse {
  integers: Record<string, number>;
  strings: Record<string, string>;
  floats: Record<string, number>;
  total_count: number;
}

export interface UpdateModuleVariableRequest {
  variable_name: string;
  value: number | string;
  variable_type: string;
}

export interface UpdateModuleVariableResponse {
  success: boolean;
  variable_name: string;
  old_value: number | string;
  new_value: number | string;
  message: string;
  has_unsaved_changes: boolean;
}

export class GameStateAPI {
  async getCompanionInfluence(characterId: number): Promise<CompanionInfluenceResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/companion-influence`,
      {
        method: 'GET',
      }
    );

    if (!response.ok) {
      throw new Error(`Failed to fetch companion influence: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

  async updateCompanionInfluence(
    characterId: number,
    companionId: string,
    newInfluence: number
  ): Promise<UpdateCompanionInfluenceResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/companion-influence/update`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          companion_id: companionId,
          new_influence: newInfluence,
        }),
      }
    );

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({ detail: response.statusText }));
      throw new Error(errorData.detail || `Failed to update companion influence: ${response.status}`);
    }

    return response.json();
  }

  async getQuestDetails(characterId: number): Promise<QuestDetailsResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/quests/details`,
      {
        method: 'GET',
      }
    );

    if (!response.ok) {
      throw new Error(`Failed to fetch quest details: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

  async updateQuestVariable(
    characterId: number,
    variableName: string,
    value: number | string,
    variableType: string
  ): Promise<UpdateQuestVariableResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/quests/variable/update`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          variable_name: variableName,
          value: value,
          variable_type: variableType,
        }),
      }
    );

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({ detail: response.statusText }));
      throw new Error(errorData.detail || `Failed to update quest variable: ${response.status}`);
    }

    return response.json();
  }

  async batchUpdateQuests(
    characterId: number,
    updates: BatchQuestUpdate[]
  ): Promise<BatchUpdateQuestsResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/quests/batch-update`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ updates }),
      }
    );

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({ detail: response.statusText }));
      throw new Error(errorData.detail || `Failed to batch update quests: ${response.status}`);
    }

    return response.json();
  }

  async getCampaignVariables(characterId: number): Promise<CampaignVariablesResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/campaign/variables`,
      {
        method: 'GET',
      }
    );

    if (!response.ok) {
      throw new Error(`Failed to fetch campaign variables: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

  async updateCampaignVariable(
    characterId: number,
    variableName: string,
    value: number | string,
    variableType: string
  ): Promise<UpdateCampaignVariableResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/campaign/variable/update`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          variable_name: variableName,
          value: value,
          variable_type: variableType,
        }),
      }
    );

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({ detail: response.statusText }));
      throw new Error(errorData.detail || `Failed to update campaign variable: ${response.status}`);
    }

    return response.json();
  }

  async getCampaignSettings(characterId: number): Promise<CampaignSettingsResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/campaign/settings`,
      {
        method: 'GET',
      }
    );

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({ detail: response.statusText }));
      throw new Error(errorData.detail || `Failed to fetch campaign settings: ${response.status}`);
    }

    return response.json();
  }

  async updateCampaignSettings(
    characterId: number,
    settings: UpdateCampaignSettingsRequest
  ): Promise<UpdateCampaignSettingsResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/campaign/settings`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(settings),
      }
    );

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({ detail: response.statusText }));
      throw new Error(errorData.detail || `Failed to update campaign settings: ${response.status}`);
    }

    return response.json();
  }

  async getModuleInfo(characterId: number): Promise<ModuleInfo> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/campaign-info`,
      {
        method: 'GET',
      }
    );

    if (!response.ok) {
      throw new Error(`Failed to fetch module info: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    return {
      module_name: data.module_name,
      area_name: data.area_name,
      campaign: data.campaign,
      entry_area: data.entry_area,
      module_description: data.module_description,
      current_module: data.current_module
    };
  }

  async getAllModules(characterId: number): Promise<{modules: Array<{id: string, name: string, campaign: string, variable_count: number, is_current: boolean}>, current_module: string}> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/modules`,
      {
        method: 'GET',
      }
    );

    if (!response.ok) {
      throw new Error(`Failed to fetch modules: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

  async getModuleById(characterId: number, moduleId: string): Promise<ModuleInfo & {variables: ModuleVariablesResponse}> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/modules/${moduleId}`,
      {
        method: 'GET',
      }
    );

    if (!response.ok) {
      throw new Error(`Failed to fetch module ${moduleId}: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

  async getModuleVariables(characterId: number): Promise<ModuleVariablesResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/module/variables`,
      {
        method: 'GET',
      }
    );

    if (!response.ok) {
      throw new Error(`Failed to fetch module variables: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

  async updateModuleVariable(
    characterId: number,
    variableName: string,
    value: number | string,
    variableType: string,
    moduleId?: string
  ): Promise<UpdateModuleVariableResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/module/variable/update`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          variable_name: variableName,
          value: value,
          variable_type: variableType,
          module_id: moduleId,
        }),
      }
    );

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({ detail: response.statusText }));
      throw new Error(errorData.detail || `Failed to update module variable: ${response.status}`);
    }

    return response.json();
  }

  async getQuestProgress(characterId: number): Promise<QuestProgressResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/quests/progress`,
      {
        method: 'GET',
      }
    );

    if (!response.ok) {
      throw new Error(`Failed to fetch quest progress: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

  async getPlotVariables(characterId: number): Promise<PlotVariablesResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/quests/plot-variables`,
      {
        method: 'GET',
      }
    );

    if (!response.ok) {
      throw new Error(`Failed to fetch plot variables: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

  async getEnrichedQuests(characterId: number): Promise<EnrichedQuestsResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/quests/enriched`,
      {
        method: 'GET',
      }
    );

    if (!response.ok) {
      throw new Error(`Failed to fetch enriched quests: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }
}

export const gameStateAPI = new GameStateAPI();
