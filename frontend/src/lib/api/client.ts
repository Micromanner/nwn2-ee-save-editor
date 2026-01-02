import { tauriCompatibleFetch } from '../utils/tauriFetch';
import DynamicAPI from '../utils/dynamicApi';

export class ApiClient {
  private cache: Map<string, { data: unknown; timestamp: number }>;
  private cacheTimeout: number = 5 * 60 * 1000; // 5 minutes

  constructor() {
    this.cache = new Map();
  }

  private async fetchWithCache<T>(endpoint: string, options?: RequestInit): Promise<T> {
    const cacheKey = `${endpoint}${JSON.stringify(options)}`;
    const cached = this.cache.get(cacheKey);
    
    if (cached && Date.now() - cached.timestamp < this.cacheTimeout) {
      return cached.data as T;
    }

    const baseUrl = await DynamicAPI.getApiBaseUrl();
    const response = await tauriCompatibleFetch(`${baseUrl}${endpoint}`, options);
    if (!response.ok) {
      throw new Error(`API Error: ${response.status} ${response.statusText}`);
    }
    
    const data = await response.json();
    this.cache.set(cacheKey, { data, timestamp: Date.now() });
    return data;
  }

  async get<T>(endpoint: string, options?: RequestInit): Promise<T> {
    return this.fetchWithCache<T>(endpoint, options);
  }

  async post<T>(endpoint: string, data?: unknown): Promise<T> {
    const baseUrl = await DynamicAPI.getApiBaseUrl();
    const response = await tauriCompatibleFetch(`${baseUrl}${endpoint}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: data ? JSON.stringify(data) : undefined,
    });

    if (!response.ok) {
      throw new Error(`API Error: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

  async delete<T>(endpoint: string, data?: unknown): Promise<T> {
    const baseUrl = await DynamicAPI.getApiBaseUrl();
    const response = await tauriCompatibleFetch(`${baseUrl}${endpoint}`, {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
      body: data ? JSON.stringify(data) : undefined,
    });

    if (!response.ok) {
      throw new Error(`API Error: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

  clearCache() {
    this.cache.clear();
  }
}

export const apiClient = new ApiClient();
