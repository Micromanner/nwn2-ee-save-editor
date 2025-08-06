import { invoke } from '@tauri-apps/api/core';
import { ask } from '@tauri-apps/plugin-dialog';

export interface SaveFile {
  path: string;
  name: string;
  thumbnail?: string;
}

// The TauriAPI class remains the same, as its methods are correctly defined.
export class TauriAPI {
  // Check for Tauri context
  static async isAvailable(): Promise<boolean> {
    return typeof window !== 'undefined' && '__TAURI__' in window;
  }

  // Django Sidecar Management
  static async startDjangoServer(): Promise<string> {
    return await invoke('start_django_sidecar');
  }

  static async stopDjangoServer(): Promise<string> {
    return await invoke('stop_django_sidecar');
  }

  static async checkDjangoHealth(): Promise<boolean> {
    return await invoke('check_django_health');
  }

  // File Operations
  static async selectSaveFile(): Promise<SaveFile> {
    return await invoke('select_save_file');
  }

  static async selectNWN2Directory(): Promise<string> {
    return await invoke('select_nwn2_directory');
  }

  static async findNWN2Saves(): Promise<SaveFile[]> {
    return await invoke('find_nwn2_saves');
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

  static async confirmSaveSwitch(currentSave: string, newSave: string): Promise<boolean> {
    return await ask(
      `You have a character loaded from "${currentSave}". Switching to "${newSave}" will replace the current character data.\n\nMake sure to save any changes before switching.\n\nDo you want to continue?`,
      {
        title: 'Switch Save File?',
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

  // Window Management
  static async openSettingsWindow(): Promise<void> {
    return await invoke('open_settings_window');
  }

  static async closeSettingsWindow(): Promise<void> {
    return await invoke('close_settings_window');
  }

  // Note: Desktop authentication is now handled automatically by Django middleware
  // No explicit session initialization needed
}

