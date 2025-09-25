import { TauriAPI } from '../tauri-api';
import { tauriCompatibleFetch } from './tauriFetch';

console.log('📦 DynamicAPI: Module loaded/parsed');

/**
 * Dynamic API utility that gets the backend URL from Tauri
 * instead of using hardcoded environment variables
 */
class DynamicAPI {
  private static baseUrl: string | null = null;
  private static isInitialized = false;

  /**
   * Initialize the API by getting the base URL from Tauri
   */
  static async initialize(): Promise<void> {
    if (this.isInitialized) {
      console.log('DynamicAPI already initialized with:', this.baseUrl);
      return;
    }

    console.log('🔧 DynamicAPI: Starting initialization...');
    
    // Get the base URL from Tauri
    this.baseUrl = await TauriAPI.getFastAPIBaseURL();
    console.log('✅ DynamicAPI: Successfully got base URL from Tauri:', this.baseUrl);
    this.isInitialized = true;
  }

  /**
   * Synchronously return the cached base URL if available, else null.
   */
  static getCachedBaseUrl(): string | null {
    return this.baseUrl;
  }

  /**
   * Get the API base URL (with /api suffix)
   */
  static async getApiBaseUrl(): Promise<string> {
    console.log('🔧 DynamicAPI: getApiBaseUrl called, initialized:', this.isInitialized);
    await this.initialize();
    console.log('🔧 DynamicAPI: After initialize, baseUrl:', this.baseUrl);
    return `${this.baseUrl}/api`;
  }

  /**
   * Get the server base URL (without /api suffix)
   */
  static async getBaseUrl(): Promise<string> {
    await this.initialize();
    return this.baseUrl!;
  }

  /**
   * Make a fetch request with dynamic URL
   */
  static async fetch(
    endpoint: string,
    options?: RequestInit
  ): Promise<Response> {
    const baseUrl = await this.getApiBaseUrl();
    const url = endpoint.startsWith('/') ? `${baseUrl}${endpoint}` : `${baseUrl}/${endpoint}`;
    return tauriCompatibleFetch(url, options);
  }

  /**
   * Reset the initialization state (useful for testing)
   */
  static reset(): void {
    this.baseUrl = null;
    this.isInitialized = false;
  }
}

export default DynamicAPI;