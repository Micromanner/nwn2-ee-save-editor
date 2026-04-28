import { useCallback, useMemo } from 'react';
import type { AggregatedModule } from './types';

export function useModuleNameResolver(modules: AggregatedModule[]) {
  const map = useMemo(() => {
    const m = new Map<string, string>();
    for (const mod of modules) {
      if (mod.display_name) m.set(mod.name, mod.display_name);
    }
    return m;
  }, [modules]);

  const resolve = useCallback((id: string) => map.get(id) || id, [map]);
  const resolveMany = useCallback(
    (ids: string[]) => ids.map(id => map.get(id) || id).join(', '),
    [map],
  );

  return { resolve, resolveMany };
}
