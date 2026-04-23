import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

const ICON_CACHE_LIMIT = 500;
const MAX_CONCURRENT = 8;

const iconCache = new Map<string, string>();
const failedIcons = new Set<string>();
const pendingRequests = new Map<string, Promise<string>>();

let inFlight = 0;
const queue: Array<() => void> = [];

function acquireSlot(): Promise<void> {
  if (inFlight < MAX_CONCURRENT) {
    inFlight++;
    return Promise.resolve();
  }
  return new Promise((resolve) => {
    queue.push(() => {
      inFlight++;
      resolve();
    });
  });
}

function releaseSlot(): void {
  inFlight--;
  const next = queue.shift();
  if (next) next();
}

function cachePut(resref: string, dataUrl: string): void {
  if (iconCache.has(resref)) {
    iconCache.delete(resref);
  } else if (iconCache.size >= ICON_CACHE_LIMIT) {
    const oldest = iconCache.keys().next().value;
    if (oldest !== undefined) iconCache.delete(oldest);
  }
  iconCache.set(resref, dataUrl);
}

function cacheGet(resref: string): string | undefined {
  const value = iconCache.get(resref);
  if (value !== undefined) {
    iconCache.delete(resref);
    iconCache.set(resref, value);
  }
  return value;
}

export async function fetchIcon(resref: string): Promise<string> {
  const cached = cacheGet(resref);
  if (cached) return cached;

  if (failedIcons.has(resref)) return '';

  const pending = pendingRequests.get(resref);
  if (pending) return pending;

  const request = (async () => {
    await acquireSlot();
    try {
      const dataUrl = await invoke<string>('get_icon_png', { name: resref });
      cachePut(resref, dataUrl);
      return dataUrl;
    } catch (err) {
      failedIcons.add(resref);
      console.warn(`[icon] Failed to load '${resref}':`, err);
      return '';
    } finally {
      releaseSlot();
      pendingRequests.delete(resref);
    }
  })();

  pendingRequests.set(resref, request);
  return request;
}

export function useIcon(resref: string | null | undefined): string {
  const [dataUrl, setDataUrl] = useState<string>(() => {
    if (!resref) return '';
    return cacheGet(resref) || '';
  });

  useEffect(() => {
    if (!resref) {
      setDataUrl('');
      return;
    }

    const cached = cacheGet(resref);
    if (cached) {
      setDataUrl(cached);
      return;
    }

    let cancelled = false;
    fetchIcon(resref).then((url) => {
      if (!cancelled) setDataUrl(url);
    });

    return () => { cancelled = true; };
  }, [resref]);

  return dataUrl;
}
