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
        return await tauriFetch(input as RequestInfo, init);
      }
    } catch {
    }
  }

  if (typeof fetch !== 'function') {
    throw new Error('No fetch implementation available');
  }
  return await fetch(input, init);
}
