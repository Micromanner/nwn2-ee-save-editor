import { invoke } from '@tauri-apps/api/core';

export type BackupReason = 'load' | 'pre_restore';

export interface CharacterChanges {
  name: string;
  ros_name: string | null;
  changes: string[];
}

export interface QuestChange {
  tag: string;
  name: string;
  from_state: number | null;
  to_state: number;
  state_text: string | null;
}

export interface PartyChange {
  ros_name: string;
  name: string | null;
  in_party: boolean;
}

export interface VariableChange {
  module: string | null;
  name: string;
  value: string;
}

export interface SaveRecord {
  saved_at: string;
  characters: CharacterChanges[];
  quests: QuestChange[];
  party: PartyChange[];
  variables: VariableChange[];
}

export interface BackupManifest {
  version: number;
  reason: BackupReason;
  saves: SaveRecord[];
}

export class BackupAPI {
  /** Resolves to true when the restored save was loaded and the backend reloaded it. */
  static async restoreFromBackup(backupPath: string): Promise<boolean> {
    return invoke<boolean>('restore_backup', { backupPath });
  }

  static async deleteBackup(backupPath: string): Promise<boolean> {
    return await invoke<boolean>('delete_backup', { backupPath });
  }

  static async deleteBackups(backupPaths: string[]): Promise<number> {
    return await invoke<number>('delete_backups', { backupPaths });
  }
}
