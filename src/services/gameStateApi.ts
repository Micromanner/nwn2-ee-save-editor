import { invoke } from '@tauri-apps/api/core';

export type {
  SaveGraph,
  QuestSummary,
  TransitionNode,
  JournalCategory,
  JournalEntry,
  ConvoFunctor,
  FunctorKind,
  AggregatedModule,
  CampaignSummary,
  LiveModuleVar,
  ModuleVarValue,
  OrphanNote,
  OrphanKind,
  ResolutionKind,
  XmlData,
} from '@/components/GameState/QuestsTab/types';
import type { SaveGraph, TransitionNode } from '@/components/GameState/QuestsTab/types';

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
  attack_neutrals: boolean;
  auto_xp_award: boolean;
  journal_sync: boolean;
  no_char_changing: boolean;
  use_personal_reputation: boolean;
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

export interface CampaignBackupInfo {
  filename: string;
  path: string;
  size_bytes: number;
  created: string;
}

export interface CampaignBackupsResponse {
  backups: CampaignBackupInfo[];
  campaign_name: string | null;
  campaign_guid: string | null;
}

export interface RestoreCampaignResponse {
  success: boolean;
  restored_from: string;
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

export interface ModuleSummary {
  id: string;
  name: string;
  is_current: boolean;
}

export interface ModuleInfo {
  module_name: string;
  area_name: string;
  campaign: string;
  entry_area: string;
  module_description: string;
  campaign_id: string;
  current_module?: string;
  game_year?: number;
  game_month?: number;
  game_day?: number;
  game_hour?: number;
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
    const companions = await invoke<Record<string, CompanionInfluenceData>>('get_companion_influence');
    return { companions };
  }

  async updateCompanionInfluence(
    characterId: number,
    companionId: string,
    newInfluence: number
  ): Promise<UpdateCompanionInfluenceResponse> {
    await invoke('update_companion_influence', { companionId, newInfluence });
    return {
        success: true,
        companion_id: companionId,
        old_influence: 0,
        new_influence: newInfluence,
        message: "Updated",
        has_unsaved_changes: true
    };
  }

  async getQuestDetails(characterId: number): Promise<QuestDetailsResponse> {
    console.warn("getQuestDetails not implemented in Rust backend");
    return {
        groups: [],
        total_quests: 0,
        completed_quests: 0,
        active_quests: 0,
        unknown_quests: 0,
        completion_rate: 0
    };
  }

  async updateQuestVariable(
    characterId: number,
    variableName: string,
    value: number | string,
    variableType: string
  ): Promise<UpdateQuestVariableResponse> {
    console.warn("updateQuestVariable not implemented");
    return {
         success: false,
         variable_name: variableName,
         old_value: value,
         new_value: value,
         message: "Not implemented",
         has_unsaved_changes: false
    };
  }

  async batchUpdateQuests(
    characterId: number,
    updates: BatchQuestUpdate[]
  ): Promise<BatchUpdateQuestsResponse> {
    console.warn("batchUpdateQuests not implemented");
    return {
         success: false,
         updated_count: 0,
         failed_count: updates.length,
         updates: [],
         message: "Not implemented",
         has_unsaved_changes: false
    };
  }

  async getCampaignVariables(characterId: number): Promise<CampaignVariablesResponse> {
    const data = await invoke<{
      integers: Record<string, number>;
      floats: Record<string, number>;
      strings: Record<string, string>;
    }>('get_campaign_variables');
    const total_count = Object.keys(data.integers).length
      + Object.keys(data.floats).length
      + Object.keys(data.strings).length;
    return {
      integers: data.integers,
      floats: data.floats,
      strings: data.strings,
      total_count,
    };
  }

  async updateCampaignVariable(
    characterId: number,
    variableName: string,
    value: number | string,
    variableType: string
  ): Promise<UpdateCampaignVariableResponse> {
    await invoke('update_campaign_variable', {
      variableName,
      value: String(value),
      variableType,
    });
    return {
      success: true,
      variable_name: variableName,
      old_value: value,
      new_value: value,
      message: 'Updated',
      has_unsaved_changes: true,
    };
  }

  async getCampaignSettings(characterId: number): Promise<CampaignSettingsResponse> {
    const [info] = await invoke<[ModuleInfo, ModuleVariablesResponse]>('get_module_info');
    if (!info.campaign_id) {
      throw new Error('No campaign associated with this save');
    }
    const settings = await invoke<CampaignSettingsResponse>('get_campaign_settings', {
      campaignId: info.campaign_id,
    });
    return settings;
  }

  async updateCampaignSettings(
    characterId: number,
    settings: CampaignSettingsResponse
  ): Promise<void> {
    await invoke('update_campaign_settings', { settings });
  }

  async batchUpdateCampaignVariables(
    updates: Array<[string, string, string]>,
  ): Promise<void> {
    await invoke('batch_update_campaign_variables', { updates });
  }

  async getCampaignBackups(campaignId: string): Promise<CampaignBackupsResponse> {
    try {
        const backups = await invoke<any[]>('list_campaign_backups', { campaignId });
        return {
            backups: backups.map(b => ({
                filename: b.filename,
                path: b.path,
                size_bytes: b.size_bytes || 0,
                created: b.created_at ? new Date(b.created_at * 1000).toISOString() : new Date().toISOString()
            })),
            campaign_name: null,
            campaign_guid: campaignId
        };
    } catch (e) {
        console.warn("list_campaign_backups failed", e);
        return { backups: [], campaign_name: null, campaign_guid: null };
    }
  }

  async getSaveBackups(): Promise<CampaignBackupsResponse> {
    try {
        const backups = await invoke<any[]>('list_backups');
        return {
            backups: backups.map(b => ({
                filename: b.filename || b.path.split(/[\\/]/).pop(),
                path: b.path,
                size_bytes: b.size_bytes || 0,
                created: b.created_at ? new Date(b.created_at * 1000).toISOString() : new Date().toISOString()
            })),
            campaign_name: "Current Session",
            campaign_guid: null
        };
    } catch (e) {
        console.warn("list_backups failed", e);
        return { backups: [], campaign_name: null, campaign_guid: null };
    }
  }

  async createBackup(): Promise<void> {
    await invoke('create_backup');
  }

  async restoreModulesFromBackup(backupPath: string): Promise<RestoreCampaignResponse> {
    try {
      await invoke('restore_modules_from_backup', { backupPath });
      return { success: true, restored_from: backupPath };
    } catch (e) {
      throw new Error(String(e));
    }
  }

  async restoreCampaignFromBackup(
    backupPath: string,
    campaignId: string
  ): Promise<RestoreCampaignResponse> {
      try {
          await invoke('restore_campaign_backup', { backupPath, campaignId });
          return { success: true, restored_from: backupPath };
      } catch (e) {
          throw new Error(String(e));
      }
  }

  async getCampaignVariableBackups(): Promise<CampaignBackupsResponse> {
    try {
      const backups = await invoke<any[]>('list_campaign_variable_backups');
      return {
        backups: backups.map(b => ({
          filename: b.filename,
          path: b.path,
          size_bytes: b.size_bytes || 0,
          created: b.created_at ? new Date(b.created_at * 1000).toISOString() : new Date().toISOString()
        })),
        campaign_name: null,
        campaign_guid: null
      };
    } catch (e) {
      console.warn('list_campaign_variable_backups failed', e);
      return { backups: [], campaign_name: null, campaign_guid: null };
    }
  }

  async restoreCampaignVariableBackup(backupPath: string): Promise<RestoreCampaignResponse> {
    try {
      await invoke('restore_campaign_variable_backup', { backupPath });
      return { success: true, restored_from: backupPath };
    } catch (e) {
      throw new Error(String(e));
    }
  }

  async getModuleBackups(): Promise<CampaignBackupsResponse> {
    try {
      const backups = await invoke<any[]>('list_module_backups');
      return {
        backups: backups.map(b => ({
          filename: b.filename,
          path: b.path,
          size_bytes: b.size_bytes || 0,
          created: b.created_at ? new Date(b.created_at * 1000).toISOString() : new Date().toISOString()
        })),
        campaign_name: null,
        campaign_guid: null
      };
    } catch (e) {
      console.warn('list_module_backups failed', e);
      return { backups: [], campaign_name: null, campaign_guid: null };
    }
  }

  async restoreModuleBackup(backupPath: string): Promise<RestoreCampaignResponse> {
    try {
      await invoke('restore_module_backup', { backupPath });
      return { success: true, restored_from: backupPath };
    } catch (e) {
      throw new Error(String(e));
    }
  }

  async getModuleInfo(characterId: number): Promise<ModuleInfo> {
    const [info] = await invoke<[ModuleInfo, ModuleVariablesResponse]>('get_module_info');
    return info;
  }

  async listModules(): Promise<ModuleSummary[]> {
    return invoke<ModuleSummary[]>('list_modules');
  }

  async getAllModules(characterId: number): Promise<{modules: Array<{id: string, name: string, campaign: string, variable_count: number, is_current: boolean}>, current_module: string}> {
    const summaries = await this.listModules();
    const current = summaries.find(m => m.is_current);
    return {
      modules: summaries.map(m => ({ id: m.id, name: m.name, campaign: '', variable_count: 0, is_current: m.is_current })),
      current_module: current?.id || (summaries.length > 0 ? summaries[0].id : ''),
    };
  }

  async getModuleById(characterId: number, moduleId: string): Promise<ModuleInfo & {variables: ModuleVariablesResponse}> {
    const [info, variables] = await invoke<[ModuleInfo, ModuleVariablesResponse]>('get_module_info_by_id', { moduleId });
    const totalCount = Object.keys(variables.integers).length + Object.keys(variables.strings).length + Object.keys(variables.floats).length;
    return {
      ...info,
      variables: {
        ...variables,
        total_count: totalCount
      }
    };
  }

  async getModuleVariables(characterId: number): Promise<ModuleVariablesResponse> {
    const [, variables] = await invoke<[ModuleInfo, ModuleVariablesResponse]>('get_module_info');
    const totalCount = Object.keys(variables.integers).length + Object.keys(variables.strings).length + Object.keys(variables.floats).length;
    return {
      ...variables,
      total_count: totalCount
    };
  }

  async updateModuleVariable(
    characterId: number,
    variableName: string,
    value: number | string,
    variableType: string,
    moduleId?: string
  ): Promise<UpdateModuleVariableResponse> {
    await invoke('update_module_variable', {
      variableName,
      value: String(value),
      variableType,
      moduleId: moduleId ?? null,
    });
    return {
      success: true,
      variable_name: variableName,
      old_value: value,
      new_value: value,
      message: 'Updated',
      has_unsaved_changes: true,
    };
  }

  async batchUpdateModuleVariables(
    updates: Array<[string, string, string]>,
    moduleId?: string
  ): Promise<void> {
    await invoke('batch_update_module_variables', {
      updates,
      moduleId: moduleId ?? null,
    });
  }

  async getQuestProgress(characterId: number): Promise<QuestProgressResponse> {
      return { quests: [], total_count: 0 };
  }

  async getPlotVariables(characterId: number): Promise<PlotVariablesResponse> {
      return { quest_variables: [], unknown_variables: [], total_count: 0 };
  }

  async getEnrichedQuests(characterId: number): Promise<EnrichedQuestsResponse> {
      return {
          quests: [],
          unmapped_variables: [],
          stats: { total: 0, completed: 0, active: 0, unmapped: 0 },
          cache_info: { cached: false, dialogue_count: 0, mapping_count: 0, campaign_name: "" }
      };
  }

  async getSaveQuestGraph(): Promise<SaveGraph> {
    return invoke<SaveGraph>('save_get_quest_graph');
  }

  async getSaveQuestTransitions(tag: string): Promise<TransitionNode[]> {
    return invoke<TransitionNode[]>('save_get_quest_transitions', { tag });
  }

  async getTlkStrings(strRefs: number[]): Promise<Record<number, string>> {
    if (strRefs.length === 0) return {};
    const result = await invoke<Record<string, string>>('tlk_get_strings', { strRefs });
    const out: Record<number, string> = {};
    for (const [k, v] of Object.entries(result)) {
      out[Number(k)] = v;
    }
    return out;
  }
}

export const gameStateAPI = new GameStateAPI();
