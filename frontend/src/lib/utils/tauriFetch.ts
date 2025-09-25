'use client';

export async function tauriCompatibleFetch(
  input: RequestInfo | URL,
  init?: RequestInit
): Promise<Response> {
  const isBrowser = typeof window !== 'undefined';
  const isTauri = isBrowser && ('__TAURI__' in window);

  if (isBrowser) {
    try {
      // In Tauri, prefer the native HTTP client to bypass CORS
      if (isTauri) {
        const mod = await import('@tauri-apps/plugin-http');
        const tauriFetch = mod.fetch as typeof fetch;
        // console.debug('[tauriFetch] Using Tauri plugin-http');
        return await tauriFetch(input as RequestInfo, init);
      }
    } catch (_err) {
      // console.warn('[tauriFetch] Failed to load plugin-http, falling back to window.fetch', _err);
    }
  }

  if (typeof fetch !== 'function') {
    throw new Error('No fetch implementation available');
  }
  // console.debug('[tauriFetch] Using browser fetch');
  return await fetch(input, init);
}
