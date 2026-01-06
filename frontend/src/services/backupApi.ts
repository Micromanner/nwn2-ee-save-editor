const API_BASE = process.env.NODE_ENV === 'development' 
  ? 'http://localhost:8000/api' 
  : '/api';

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
    const response = await fetch(`${API_BASE}/savegame/${characterId}/backups`);
    
    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to list backups: ${error}`);
    }
    
    return response.json();
  }

  static async restoreFromBackup(
    characterId: number, 
    request: RestoreRequest
  ): Promise<RestoreResponse> {
    const response = await fetch(`${API_BASE}/savegame/${characterId}/restore`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });
    
    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to restore backup: ${error}`);
    }
    
    return response.json();
  }

  static async cleanupBackups(
    characterId: number, 
    keepCount: number = 10
  ): Promise<CleanupResponse> {
    const response = await fetch(
      `${API_BASE}/savegame/${characterId}/cleanup-backups?keep_count=${keepCount}`,
      {
        method: 'POST',
      }
    );
    
    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to cleanup backups: ${error}`);
    }
    
    return response.json();
  }

  static formatTimestamp(timestamp: string): string {
    return new Date(timestamp).toLocaleString();
  }

  static formatSize(sizeBytes: number): string {
    if (sizeBytes < 1024) return `${sizeBytes} B`;
    if (sizeBytes < 1024 * 1024) return `${(sizeBytes / 1024).toFixed(1)} KB`;
    if (sizeBytes < 1024 * 1024 * 1024) return `${(sizeBytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(sizeBytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }
}