import { useState, useEffect, useCallback } from 'react';
import { gameData, type PagedResponse } from './gamedata';

// Generic hook for loading game data
export function useGameData<T = unknown>(
  fetcher: () => Promise<T>,
  deps: unknown[] = []
) {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let cancelled = false;

    const loadData = async () => {
      try {
        setLoading(true);
        const result = await fetcher();
        if (!cancelled) {
          setData(result);
          setError(null);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err : new Error('Unknown error'));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    loadData();

    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  return { data, loading, error };
}

// Specific hooks for common data
export function useAppearanceData() {
  return useGameData(() => gameData.appearance.getAll());
}

// Separate hooks for individual appearance data
export function useAppearanceModels() {
  return useGameData(() => gameData.appearance.get());
}

export function usePortraits() {
  return useGameData(() => gameData.appearance.getPortraits());
}

export function useSoundsets() {
  return useGameData(() => gameData.appearance.getSoundsets());
}

export function useRaces() {
  return useGameData(() => gameData.races());
}

export function useClasses() {
  return useGameData(() => gameData.classes());
}

export function useFeats(characterId: number, featType?: number) {
  return useGameData(() => gameData.feats(characterId, featType), [characterId, featType]);
}

export function useSpells(characterId?: number, filters?: {
  level?: number;
  school?: string;
  search?: string;
}) {
  return useGameData(() => 
    characterId ? gameData.spells(characterId, filters) : Promise.resolve([]), 
    [characterId, filters?.level, filters?.school, filters?.search]
  );
}

// Hook for paginated data with automatic loading
export function usePaginatedData<T>(
  fetcher: (page: number) => Promise<PagedResponse<T>>
) {
  const [items, setItems] = useState<T[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [hasMore, setHasMore] = useState(true);
  const [page, setPage] = useState(1);

  const loadMore = useCallback(async () => {
    if (loading || !hasMore) return;

    try {
      setLoading(true);
      const response = await fetcher(page);
      setItems(prev => [...prev, ...response.results]);
      setHasMore(response.next !== null);
      setPage(prev => prev + 1);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Unknown error'));
    } finally {
      setLoading(false);
    }
  }, [fetcher, loading, hasMore, page]);

  useEffect(() => {
    loadMore();
  }, [loadMore]); // Load first page on mount

  return { items, loading, error, hasMore, loadMore };
}