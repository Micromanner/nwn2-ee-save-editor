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
    const url = await TauriAPI.getFastAPIBaseURL();
    
    // Validate that we didn't get an unresolved port
    if (url.endsWith(':0')) {
      console.warn('⚠️ DynamicAPI: Got bridge URL with port 0, skipping initialization');
      throw new Error('Backend port not yet resolved');
    }

    this.baseUrl = url;
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
    try {
      const baseUrl = await this.getApiBaseUrl();
      const url = endpoint.startsWith('/') ? `${baseUrl}${endpoint}` : `${baseUrl}/${endpoint}`;

      // Polyfill for cache: 'reload' which might not be supported by the underlying client
      let finalOptions = options;
      if (options?.cache === 'reload' || options?.cache === 'no-cache') {
        const headers = new Headers(options.headers);
        headers.append('Cache-Control', 'no-cache');
        headers.append('Pragma', 'no-cache');
        
        finalOptions = {
          ...options,
          headers
        };
      }

      return await tauriCompatibleFetch(url, finalOptions);
    } catch (error) {
      // If we get a connection error, the backend might have restarted on a new port
      if (error instanceof TypeError && error.message.includes('Failed to fetch')) {
        console.warn('⚠️ DynamicAPI: Connection failed, attempting to re-resolve port...');
        this.reset();
        const baseUrl = await this.getApiBaseUrl();
        const url = endpoint.startsWith('/') ? `${baseUrl}${endpoint}` : `${baseUrl}/${endpoint}`;
        return await tauriCompatibleFetch(url, options);
      }
      throw error;
    }
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