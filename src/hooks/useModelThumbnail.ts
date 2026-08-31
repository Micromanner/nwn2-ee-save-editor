import { useState, useEffect } from 'react';
import { renderModelThumbnail } from '@/components/ModelViewer/thumbnailRenderer';
import type { TintColors } from '@/components/ModelViewer/materials';
import type { ModelData } from '@/components/ModelViewer/types';

// Thumbnail cache + strictly sequential render queue over the shared offscreen
// renderer (same LRU/pending-dedup shape as useIcon). Callers supply the model
// fetch and a cache key encoding everything the image depends on, so any
// feature (items now, character parts later) can plug in.

const THUMBNAIL_CACHE_LIMIT = 200;

const thumbnailCache = new Map<string, string>();
const pendingRequests = new Map<string, Promise<string>>();

// Live consumers per cache key. A queued job whose key has no subscriber left
// by the time it is dequeued (dialog closed, item switched) is skipped instead
// of paying its fetch + render.
const subscribers = new Map<string, number>();

function subscribe(key: string): void {
  subscribers.set(key, (subscribers.get(key) ?? 0) + 1);
}

function unsubscribe(key: string): void {
  const next = (subscribers.get(key) ?? 0) - 1;
  if (next <= 0) subscribers.delete(key);
  else subscribers.set(key, next);
}

function cachePut(key: string, dataUrl: string): void {
  if (thumbnailCache.has(key)) {
    thumbnailCache.delete(key);
  } else if (thumbnailCache.size >= THUMBNAIL_CACHE_LIMIT) {
    const oldest = thumbnailCache.keys().next().value;
    if (oldest !== undefined) thumbnailCache.delete(oldest);
  }
  thumbnailCache.set(key, dataUrl);
}

function cacheGet(key: string): string | undefined {
  const value = thumbnailCache.get(key);
  if (value !== undefined) {
    thumbnailCache.delete(key);
    thumbnailCache.set(key, value);
  }
  return value;
}

const queue: Array<() => Promise<void>> = [];
let draining = false;

function enqueue(job: () => Promise<void>): void {
  queue.push(job);
  if (!draining) void drain();
}

async function drain(): Promise<void> {
  draining = true;
  while (queue.length > 0) {
    await queue.shift()!();
  }
  draining = false;
}

export function fetchModelThumbnail(
  cacheKey: string,
  fetchModel: () => Promise<ModelData>,
  tints: TintColors,
): Promise<string> {
  const cached = cacheGet(cacheKey);
  if (cached !== undefined) return Promise.resolve(cached);

  const pending = pendingRequests.get(cacheKey);
  if (pending) return pending;

  const request = new Promise<string>((resolve) => {
    enqueue(async () => {
      try {
        if (!subscribers.has(cacheKey)) {
          resolve('');
          return;
        }
        const model = await fetchModel();
        const url = await renderModelThumbnail(model, tints);
        cachePut(cacheKey, url);
        resolve(url);
      } catch (err) {
        console.warn(`[thumbnail] Failed to render '${cacheKey}':`, err);
        resolve('');
      } finally {
        pendingRequests.delete(cacheKey);
      }
    });
  });

  pendingRequests.set(cacheKey, request);
  return request;
}

/// null = still rendering, '' = failed/no model, else a PNG data URL.
export function useModelThumbnail(
  cacheKey: string,
  fetchModel: () => Promise<ModelData>,
  tints: TintColors,
): string | null {
  const [dataUrl, setDataUrl] = useState<string | null>(() => cacheGet(cacheKey) ?? null);

  useEffect(() => {
    const cached = cacheGet(cacheKey);
    if (cached !== undefined) {
      setDataUrl(cached);
      return;
    }

    subscribe(cacheKey);
    let cancelled = false;
    setDataUrl(null);
    fetchModelThumbnail(cacheKey, fetchModel, tints).then((url) => {
      if (!cancelled) setDataUrl(url);
    });

    return () => {
      cancelled = true;
      unsubscribe(cacheKey);
    };
    // fetchModel/tints are captured per key; cacheKey encodes everything the
    // thumbnail depends on, so it is the sole invalidation trigger.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cacheKey]);

  return dataUrl;
}
