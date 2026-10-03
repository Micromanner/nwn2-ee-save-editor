import { invoke } from '@tauri-apps/api/core';
import { ask } from '@tauri-apps/plugin-dialog';

export interface SaveFile {
  path: string;
  name: string;
  thumbnail?: string;
  modified?: number;
  character_name?: string;
}

export type SaveMode = 'sp' | 'mp';

// Path Configuration Interfaces
export interface PathInfo {
  path: string | null;
  exists: boolean;
  source: string;
}

export interface PathConfig {
  game_folder: PathInfo;
  documents_folder: PathInfo;
  steam_workshop_folder: PathInfo;
  custom_override_folders: Array<{ path: string; exists: boolean }>;
  custom_module_folders: Array<{ path: string; exists: boolean }>;
  custom_hak_folders: Array<{ path: string; exists: boolean }>;
}

export class TauriAPI {
  // Check for Tauri context
  static async isAvailable(): Promise<boolean> {
    return typeof window !== 'undefined' && '__TAURI__' in window;
  }

  // Initialization
  static async initializeGameData(): Promise<boolean> {
    return await invoke('initialize_game_data');
  }

  static async getInitializationStatus(): Promise<{ step: string; progress: number; message: string }> {
    return await invoke('get_initialization_status');
  }

  // File Operations
  static async selectSaveFile(): Promise<SaveFile> {
    return await invoke('select_save_file');
  }

  static async selectNWN2Directory(): Promise<string> {
    return await invoke('select_nwn2_directory');
  }

  static async findNWN2Saves(saveMode?: SaveMode): Promise<SaveFile[]> {
    return await invoke('find_nwn2_saves', { saveMode });
  }

  static async getSteamWorkshopPath(): Promise<string | null> {
    return await invoke('get_steam_workshop_path');
  }

  static async validateNWN2Installation(path: string): Promise<boolean> {
    return await invoke('validate_nwn2_installation', { path });
  }

  static async getSaveThumbnail(thumbnailPath: string): Promise<string> {
    return await invoke('get_save_thumbnail', { thumbnailPath });
  }

  static async confirmSaveSwitch(message: string, title: string): Promise<boolean> {
    return await ask(
      message,
      {
        title,
        kind: 'warning'
      }
    );
  }

  // Game Launch Operations
  static async detectNWN2Installation(): Promise<string | null> {
    return await invoke('detect_nwn2_installation');
  }

  static async launchNWN2Game(gamePath?: string): Promise<void> {
    return await invoke('launch_nwn2_game', { gamePath });
  }

  static async openFolderInExplorer(folderPath: string): Promise<void> {
    return await invoke('open_folder_in_explorer', { folderPath });
  }

  // Path Configuration
  static async getPathsConfig(): Promise<PathConfig> {
    return await invoke('get_paths_config');
  }

  // Instance method for consistency with the class pattern
  async confirmSaveSwitch(currentSave: string, newSave: string): Promise<boolean> {
    return TauriAPI.confirmSaveSwitch(currentSave, newSave);
  }

  async detectNWN2Installation(): Promise<string | null> {
    return TauriAPI.detectNWN2Installation();
  }

  async launchNWN2Game(gamePath?: string): Promise<void> {
    return TauriAPI.launchNWN2Game(gamePath);
  }

  async openFolderInExplorer(folderPath: string): Promise<void> {
    return TauriAPI.openFolderInExplorer(folderPath);
  }

  
  async getPathsConfig(): Promise<PathConfig> {
    return TauriAPI.getPathsConfig();
  }

  async selectSaveFile(): Promise<SaveFile> {
    return TauriAPI.selectSaveFile();
  }
  
  async findNWN2Saves(saveMode?: SaveMode): Promise<SaveFile[]> {
    return TauriAPI.findNWN2Saves(saveMode);
  }

  // Window Management
  static async openSettingsWindow(): Promise<void> {
    return await invoke('open_settings_window');
  }

  static async closeSettingsWindow(): Promise<void> {
    return await invoke('close_settings_window');
  }
}
