import { useState, useEffect } from 'react';
import { renderModelThumbnail, type ThumbnailFraming } from '@/components/ModelViewer/thumbnailRenderer';
import type { TintColors } from '@/components/ModelViewer/materials';
import type { ModelData } from '@/components/ModelViewer/types';
import type { PreviewPose } from '@/lib/bindings';

// Thumbnail cache + render queue over the shared offscreen renderer (same
// LRU/pending-dedup shape as useIcon). Rendering is strictly sequential (the
// scene/canvas are shared); model fetches for the next jobs run ahead so the
// backend's MDB parsing overlaps the GPU work. Callers supply the model fetch
// and a cache key encoding everything the image depends on, so any feature
// (items now, character parts later) can plug in.

const THUMBNAIL_CACHE_LIMIT = 200;
// Fetches in flight beyond the job currently rendering.
const PREFETCH_AHEAD = 2;

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

interface Job {
  key: string;
  fetchModel: () => Promise<ModelData>;
  tints: TintColors;
  framing: ThumbnailFraming;
  pose: PreviewPose | null | undefined;
  resolve: (url: string) => void;
  // Set once the fetch has been started ahead of the job's turn.
  model?: Promise<ModelData>;
  // Consumer is on screen; such jobs run before the rest of the queue.
  visible: boolean;
}

const queue: Job[] = [];
let draining = false;

function enqueue(job: Job): void {
  queue.push(job);
  if (!draining) void drain();
}

function prefetch(): void {
  const limit = Math.min(PREFETCH_AHEAD, queue.length);
  for (let i = 0; i < limit; i++) {
    const job = queue[i];
    if (job.model || !subscribers.has(job.key)) continue;
    job.model = job.fetchModel();
    // The rejection is reported when the job awaits it; without a handler
    // here it would surface as unhandled in the meantime.
    job.model.catch(() => {});
  }
}

async function drain(): Promise<void> {
  draining = true;
  while (queue.length > 0) {
    const job = queue.shift()!;
    prefetch();
    try {
      if (!subscribers.has(job.key)) {
        job.resolve('');
        continue;
      }
      const model = await (job.model ?? job.fetchModel());
      const url = await renderModelThumbnail(model, job.tints, job.framing, job.pose);
      cachePut(job.key, url);
      job.resolve(url);
    } catch (err) {
      console.warn(`[thumbnail] Failed to render '${job.key}':`, err);
      job.resolve('');
    } finally {
      pendingRequests.delete(job.key);
    }
  }
  draining = false;
}

/// Moves a queued thumbnail ahead of every job whose consumer is not on
/// screen, behind the ones that are (earlier-visible tiles keep their turn).
export function setThumbnailVisible(cacheKey: string, visible: boolean): void {
  const idx = queue.findIndex((j) => j.key === cacheKey);
  if (idx < 0) return;
  const job = queue[idx];
  if (job.visible === visible) return;
  job.visible = visible;
  if (!visible) return;
  queue.splice(idx, 1);
  let insert = 0;
  while (insert < queue.length && queue[insert].visible) insert++;
  queue.splice(insert, 0, job);
  prefetch();
}

export function fetchModelThumbnail(
  cacheKey: string,
  fetchModel: () => Promise<ModelData>,
  tints: TintColors,
  framing: ThumbnailFraming = 'model',
  pose?: PreviewPose | null,
): Promise<string> {
  const cached = cacheGet(cacheKey);
  if (cached !== undefined) return Promise.resolve(cached);

  const pending = pendingRequests.get(cacheKey);
  if (pending) return pending;

  const request = new Promise<string>((resolve) => {
    enqueue({ key: cacheKey, fetchModel, tints, framing, pose, resolve, visible: false });
  });

  pendingRequests.set(cacheKey, request);
  return request;
}

/// null = still rendering, '' = failed/no model, else a PNG data URL.
export function useModelThumbnail(
  cacheKey: string,
  fetchModel: () => Promise<ModelData>,
  tints: TintColors,
  framing: ThumbnailFraming = 'model',
  pose?: PreviewPose | null,
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
    fetchModelThumbnail(cacheKey, fetchModel, tints, framing, pose).then((url) => {
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
