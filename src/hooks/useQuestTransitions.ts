import { useCallback, useReducer, useRef } from 'react';
import { gameStateAPI, type TransitionNode } from '@/services/gameStateApi';

type Entry =
  | { state: 'pending' }
  | { state: 'resolved'; transitions: TransitionNode[] }
  | { state: 'error'; message: string };

export interface QuestTransitionsResolver {
  get: (tag: string) => Entry | undefined;
  load: (tag: string) => void;
}

export function useQuestTransitions(): QuestTransitionsResolver {
  const cacheRef = useRef<Map<string, Entry>>(new Map());
  const [, forceUpdate] = useReducer((x: number) => x + 1, 0);

  const load = useCallback((tag: string) => {
    if (cacheRef.current.has(tag)) return;
    cacheRef.current.set(tag, { state: 'pending' });
    (async () => {
      try {
        const transitions = await gameStateAPI.getSaveQuestTransitions(tag);
        cacheRef.current.set(tag, { state: 'resolved', transitions });
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        cacheRef.current.set(tag, { state: 'error', message });
      }
      forceUpdate();
    })();
  }, []);

  const get = useCallback((tag: string) => cacheRef.current.get(tag), []);

  return { get, load };
}
