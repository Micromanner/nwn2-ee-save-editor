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
  influence_var: string;
  source: string;
}

export interface CompanionInfluenceResponse {
  companions: Record<string, CompanionInfluenceData>;
}

export interface CampaignVariablesResponse {
  integers: Record<string, number>;
  booleans: Record<string, number>;
  strings: Record<string, string>;
  floats: Record<string, number>;
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

export interface CampaignBackupInfo {
  filename: string;
  path: string;
  size_bytes: number;
  /** Unix seconds. */
  created_at: number;
}

export interface ModuleSummary {
  id: string;
  name: string;
  is_current: boolean;
}

export interface ModuleInfo {
  module_name: string;
  area_name: string;
  area_name_strref: number;
  campaign: string;
  entry_area: string;
  entry_area_strref: number;
  module_description: string;
  campaign_id: string;
  hak_list: string[];
  custom_tlk: string;
  current_module: string;
  game_year: number;
  game_month: number;
  game_day: number;
  game_hour: number;
}

export interface ModuleVariablesResponse {
  integers: Record<string, number>;
  strings: Record<string, string>;
  floats: Record<string, number>;
}

export interface QuestGraphProgress {
  step: 'idle' | 'starting' | 'campaign' | 'modules' | 'journal' | 'globals' | 'ready' | 'error';
  progress: number;
  message: string;
}

export class GameStateAPI {
  async getCompanionInfluence(): Promise<CompanionInfluenceResponse> {
    const companions = await invoke<Record<string, CompanionInfluenceData>>('get_companion_influence');
    return { companions };
  }

  async getCampaignVariables(): Promise<CampaignVariablesResponse> {
    return invoke<CampaignVariablesResponse>('get_campaign_variables');
  }

  async getCampaignSettings(): Promise<CampaignSettingsResponse> {
    const [info] = await invoke<[ModuleInfo, ModuleVariablesResponse]>('get_module_info');
    if (!info.campaign_id) {
      throw new Error('No campaign associated with this save');
    }
    const settings = await invoke<CampaignSettingsResponse>('get_campaign_settings', {
      campaignId: info.campaign_id,
    });
    return settings;
  }

  async updateCampaignSettings(settings: CampaignSettingsResponse): Promise<void> {
    await invoke('update_campaign_settings', { settings });
  }

  async batchUpdateCampaignVariables(
    updates: Array<[string, string, string]>,
  ): Promise<void> {
    await invoke('batch_update_campaign_variables', { updates });
  }

  async getCampaignBackups(campaignId: string): Promise<CampaignBackupInfo[]> {
    return invoke<CampaignBackupInfo[]>('list_campaign_backups', { campaignId });
  }

  async restoreCampaignFromBackup(backupPath: string, campaignId: string): Promise<void> {
    await invoke('restore_campaign_backup', { backupPath, campaignId });
  }

  async getCampaignVariableBackups(): Promise<CampaignBackupInfo[]> {
    return invoke<CampaignBackupInfo[]>('list_campaign_variable_backups');
  }

  async restoreCampaignVariableBackup(backupPath: string): Promise<void> {
    await invoke('restore_campaign_variable_backup', { backupPath });
  }

  /** Backups of `moduleId`, or of the current module when omitted. */
  async getModuleBackups(moduleId?: string | null): Promise<CampaignBackupInfo[]> {
    return invoke<CampaignBackupInfo[]>('list_module_backups', { moduleId: moduleId ?? null });
  }

  async restoreModuleBackup(backupPath: string): Promise<void> {
    await invoke('restore_module_backup', { backupPath });
  }

  async listModules(): Promise<ModuleSummary[]> {
    return invoke<ModuleSummary[]>('list_modules');
  }

  async getModuleById(moduleId: string): Promise<ModuleInfo & {variables: ModuleVariablesResponse}> {
    const [info, variables] = await invoke<[ModuleInfo, ModuleVariablesResponse]>('get_module_info_by_id', { moduleId });
    return { ...info, variables };
  }

  async getModuleVariables(moduleId?: string | null): Promise<ModuleVariablesResponse> {
    const [, variables] = moduleId
      ? await invoke<[ModuleInfo, ModuleVariablesResponse]>('get_module_info_by_id', { moduleId })
      : await invoke<[ModuleInfo, ModuleVariablesResponse]>('get_module_info');
    return variables;
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

  async getSaveQuestGraph(): Promise<SaveGraph> {
    return invoke<SaveGraph>('save_get_quest_graph');
  }

  async getSaveQuestTransitions(tag: string): Promise<TransitionNode[]> {
    return invoke<TransitionNode[]>('save_get_quest_transitions', { tag });
  }

  async getQuestGraphProgress(): Promise<QuestGraphProgress> {
    return invoke<QuestGraphProgress>('save_get_quest_graph_progress');
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
