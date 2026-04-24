import { useCallback, useReducer, useRef } from 'react';
import { gameStateAPI } from '@/services/gameStateApi';

type TlkEntry = { state: 'pending' } | { state: 'resolved'; value: string } | { state: 'error' };

export interface TlkResolver {
  resolve: (strRef: number | null | undefined) => string | undefined;
  prime: (strRefs: readonly (number | null | undefined)[]) => void;
}

export function useTlkResolver(): TlkResolver {
  const cacheRef = useRef<Map<number, TlkEntry>>(new Map());
  const [, forceUpdate] = useReducer((x: number) => x + 1, 0);

  const flush = useCallback(async (batch: number[]) => {
    try {
      const result = await gameStateAPI.getTlkStrings(batch);
      for (const strRef of batch) {
        const text = result[strRef];
        cacheRef.current.set(strRef, text !== undefined
          ? { state: 'resolved', value: text }
          : { state: 'error' });
      }
    } catch {
      for (const strRef of batch) {
        cacheRef.current.set(strRef, { state: 'error' });
      }
    }
    forceUpdate();
  }, []);

  const prime = useCallback((strRefs: readonly (number | null | undefined)[]) => {
    const batch: number[] = [];
    for (const strRef of strRefs) {
      if (strRef == null) continue;
      if (cacheRef.current.has(strRef)) continue;
      cacheRef.current.set(strRef, { state: 'pending' });
      batch.push(strRef);
    }
    if (batch.length > 0) {
      void flush(batch);
    }
  }, [flush]);

  const resolve = useCallback((strRef: number | null | undefined): string | undefined => {
    if (strRef == null) return undefined;
    const entry = cacheRef.current.get(strRef);
    return entry?.state === 'resolved' ? entry.value : undefined;
  }, []);

  return { resolve, prime };
}
