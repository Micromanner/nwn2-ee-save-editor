import { apiClient } from './client';

// Path configuration types
export interface PathInfo {
  path: string | null;
  exists: boolean;
  auto_detected: boolean;
}

export interface CustomFolderInfo {
  path: string;
  exists: boolean;
}

export interface PathConfig {
  game_folder: PathInfo;
  documents_folder: PathInfo;
  steam_workshop_folder: PathInfo;
  custom_override_folders: CustomFolderInfo[];
  custom_module_folders: CustomFolderInfo[];
  custom_hak_folders: CustomFolderInfo[];
}

export interface PathsResponse {
  paths: PathConfig;
}

export interface AutoDetectResponse {
  game_installations: string[];
  documents_folder: string | null;
  steam_workshop: string | null;
  current_paths: PathConfig;
}

export interface PathUpdateResponse {
  success: boolean;
  message: string;
  paths: PathConfig;
}

export interface ErrorResponse {
  error: string;
}

// Service class for path management endpoints
export class PathService {
  private readonly basePath = '/gamedata/paths';

  // Get current path configuration
  async getConfig(): Promise<PathsResponse> {
    return apiClient.get<PathsResponse>(`${this.basePath}`);
  }

  // Set main game folder
  async setGameFolder(path: string): Promise<PathUpdateResponse> {
    return apiClient.post<PathUpdateResponse>(`${this.basePath}/set-game/?path=${encodeURIComponent(path)}`);
  }

  // Set documents folder
  async setDocumentsFolder(path: string): Promise<PathUpdateResponse> {
    return apiClient.post<PathUpdateResponse>(`${this.basePath}/set-documents/?path=${encodeURIComponent(path)}`);
  }

  // Set Steam workshop folder
  async setSteamWorkshopFolder(path: string): Promise<PathUpdateResponse> {
    return apiClient.post<PathUpdateResponse>(`${this.basePath}/set-steam-workshop/?path=${encodeURIComponent(path)}`);
  }

  // Custom override folders
  async addOverrideFolder(path: string): Promise<PathUpdateResponse> {
    return apiClient.post<PathUpdateResponse>(`${this.basePath}/add-override/?path=${encodeURIComponent(path)}`);
  }

  async removeOverrideFolder(path: string): Promise<PathUpdateResponse> {
    return apiClient.post<PathUpdateResponse>(`${this.basePath}/remove-override/?path=${encodeURIComponent(path)}`);
  }

  // Reset paths to auto-discovery
  async resetGameFolder(): Promise<PathUpdateResponse> {
    return apiClient.post<PathUpdateResponse>(`${this.basePath}/reset-game/`);
  }

  async resetDocumentsFolder(): Promise<PathUpdateResponse> {
    return apiClient.post<PathUpdateResponse>(`${this.basePath}/reset-documents/`);
  }

  async resetSteamWorkshopFolder(): Promise<PathUpdateResponse> {
    return apiClient.post<PathUpdateResponse>(`${this.basePath}/reset-steam-workshop/`);
  }

  // Custom HAK folders
  async addHakFolder(path: string): Promise<PathUpdateResponse> {
    return apiClient.post<PathUpdateResponse>(`${this.basePath}/add-hak/?path=${encodeURIComponent(path)}`);
  }

  async removeHakFolder(path: string): Promise<PathUpdateResponse> {
    return apiClient.post<PathUpdateResponse>(`${this.basePath}/remove-hak/?path=${encodeURIComponent(path)}`);
  }

  // Auto-detect paths - not used, paths are auto-detected at startup
  async autoDetect(): Promise<AutoDetectResponse> {
    return apiClient.get<AutoDetectResponse>(`${this.basePath}/auto-detect/`);
  }
}

export const pathService = new PathService();