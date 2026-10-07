import { invoke } from '@tauri-apps/api/core';

export interface PathInfo {
  path: string | null;
  exists: boolean;
  source: string;
}

export interface CustomFolderInfo {
  path: string;
  exists: boolean;
}

export interface PathConfig {
  game_folder: PathInfo;
  documents_folder: PathInfo;
  steam_workshop_folder: PathInfo;
  localvault_folder: PathInfo;
  custom_override_folders: CustomFolderInfo[];
  custom_module_folders: CustomFolderInfo[];
  custom_hak_folders: CustomFolderInfo[];
}

export interface PathsResponse {
  paths: PathConfig;
}

export interface PathUpdateResponse {
  success: boolean;
  message: string;
  paths: PathConfig;
}

export class PathService {
  async getConfig(): Promise<PathsResponse> {
    const paths = await invoke<PathConfig>('get_paths_config');
    return { paths };
  }

  async setGameFolder(path: string): Promise<PathUpdateResponse> {
    return await invoke<PathUpdateResponse>('set_game_folder', { path });
  }

  async setDocumentsFolder(path: string): Promise<PathUpdateResponse> {
    return await invoke<PathUpdateResponse>('set_documents_folder', { path });
  }

  async setSteamWorkshopFolder(path: string): Promise<PathUpdateResponse> {
    return await invoke<PathUpdateResponse>('set_steam_workshop_folder', { path });
  }

  async addOverrideFolder(path: string): Promise<PathUpdateResponse> {
    return await invoke<PathUpdateResponse>('add_override_folder', { path });
  }

  async removeOverrideFolder(path: string): Promise<PathUpdateResponse> {
    return await invoke<PathUpdateResponse>('remove_override_folder', { path });
  }

  async resetGameFolder(): Promise<PathUpdateResponse> {
    return await invoke<PathUpdateResponse>('reset_game_folder');
  }

  async resetDocumentsFolder(): Promise<PathUpdateResponse> {
    return await invoke<PathUpdateResponse>('reset_documents_folder');
  }

  async resetSteamWorkshopFolder(): Promise<PathUpdateResponse> {
    return await invoke<PathUpdateResponse>('reset_steam_workshop_folder');
  }

  async addHakFolder(path: string): Promise<PathUpdateResponse> {
    return await invoke<PathUpdateResponse>('add_hak_folder', { path });
  }

  async removeHakFolder(path: string): Promise<PathUpdateResponse> {
    return await invoke<PathUpdateResponse>('remove_hak_folder', { path });
  }
}

export const pathService = new PathService();
