import { useEffect, useRef } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import type { PathConfig } from '@/lib/api/paths';

export const PATHS_CHANGED_EVENT = 'paths-changed';

export function usePathsChangedEvent(
  callback: (paths: PathConfig) => void,
): void {
  const callbackRef = useRef(callback);
  callbackRef.current = callback;

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;

    listen<PathConfig>(PATHS_CHANGED_EVENT, (event) => {
      callbackRef.current(event.payload);
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);
}
