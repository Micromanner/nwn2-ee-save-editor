import { invoke } from '@tauri-apps/api/core';

export interface BackupInfo {
  path: string;
  folder_name: string;
  timestamp: string;
  display_name: string;
  size_bytes: number;
  original_save: string;
}

export interface BackupsResponse {
  backups: BackupInfo[];
  count: number;
}

export interface RestoreRequest {
  backup_path: string;
  confirm_restore: boolean;
  create_pre_restore_backup: boolean;
}

export interface RestoreResponse {
  success: boolean;
  restored_from: string;
  files_restored: string[];
  pre_restore_backup?: string;
  restore_timestamp: string;
  backups_cleaned_up: number;
}

export interface CleanupResponse {
  cleaned_up: number;
  kept: number;
  errors: string[];
}

export class BackupAPI {
  static async listBackups(characterId: number): Promise<BackupsResponse> {
    const backups = await invoke<any[]>('list_backups');
    const mappedBackups: BackupInfo[] = backups.map(b => ({
      path: b.path,
      folder_name: b.filename || 'unknown',
      timestamp: b.created_at || new Date().toISOString(),
      display_name: b.filename || 'Backup',
      size_bytes: b.size_bytes || 0,
      original_save: "unknown"
    }));

    return {
      backups: mappedBackups,
      count: mappedBackups.length
    };
  }

  static async restoreFromBackup(
    characterId: number,
    request: RestoreRequest
  ): Promise<RestoreResponse> {
    const result = await invoke<any>('restore_backup', {
      backupPath: request.backup_path,
      createPreRestoreBackup: request.create_pre_restore_backup
    });

    return {
      success: result.success,
      restored_from: request.backup_path,
      files_restored: result.files_restored || [],
      pre_restore_backup: result.pre_restore_backup_path,
      restore_timestamp: new Date().toISOString(),
      backups_cleaned_up: 0
    };
  }

  static async cleanupBackups(
    characterId: number,
    keepCount: number = 10
  ): Promise<CleanupResponse> {
    const result = await invoke<any>('cleanup_backups', { keepCount: keepCount });
    return {
      cleaned_up: result.deleted_count || 0,
      kept: result.remaining_count || 0,
      errors: result.errors || []
    };
  }

  static async deleteBackup(backupPath: string): Promise<boolean> {
    return await invoke<boolean>('delete_backup', { backupPath });
  }

  static async deleteBackups(backupPaths: string[]): Promise<number> {
    return await invoke<number>('delete_backups', { backupPaths });
  }

  static formatTimestamp(timestamp: string): string {
    return new Date(timestamp).toLocaleString();
  }
}
