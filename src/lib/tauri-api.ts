import { invoke } from '@tauri-apps/api/core';
import { ask } from '@tauri-apps/plugin-dialog';
import type { PathConfig } from '@/lib/api/paths';

export interface SaveFile {
  path: string;
  name: string;
  thumbnail: string | null;
  modified: number | null;
  character_name: string | null;
}

export type SaveMode = 'sp' | 'mp';

export class TauriAPI {
  static async isAvailable(): Promise<boolean> {
    return typeof window !== 'undefined' && '__TAURI__' in window;
  }

  static async initializeGameData(): Promise<boolean> {
    return await invoke('initialize_game_data');
  }

  static async getInitializationStatus(): Promise<{ step: string; progress: number; message: string }> {
    return await invoke('get_initialization_status');
  }

  static async findNWN2Saves(saveMode?: SaveMode): Promise<SaveFile[]> {
    return await invoke('find_nwn2_saves', { saveMode });
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

  static async launchNWN2Game(gamePath?: string): Promise<void> {
    return await invoke('launch_nwn2_game', { gamePath });
  }

  static async openFolderInExplorer(folderPath: string): Promise<void> {
    return await invoke('open_folder_in_explorer', { folderPath });
  }

  static async getPathsConfig(): Promise<PathConfig> {
    return await invoke('get_paths_config');
  }
}
