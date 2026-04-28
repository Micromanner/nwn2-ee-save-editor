import { useMemo } from 'react';
import { HTMLTable } from '@blueprintjs/core';
import { T } from '../../theme';
import { useTranslations } from '@/hooks/useTranslations';
import type { ConvoFunctor, LiveModuleVar, SaveGraph, TransitionNode, XmlData } from './types';

interface Reference {
  name: string;
  declared: (string | number | boolean)[];
}

function paramAsPrimitive(v: unknown): string | number | boolean | undefined {
  if (typeof v === 'string' || typeof v === 'number' || typeof v === 'boolean') return v;
  return undefined;
}

function extractRef(f: ConvoFunctor): { name: string; value?: string | number | boolean } | null {
  const rawName = f.params[0];
  if (typeof rawName !== 'string' || rawName.length === 0) return null;
  const value = f.params.length > 1 ? paramAsPrimitive(f.params[1]) : undefined;
  return { name: rawName, value };
}

function collectByName(map: Map<string, Set<string | number | boolean>>, f: ConvoFunctor) {
  const ref = extractRef(f);
  if (!ref) return;
  const set = map.get(ref.name) ?? new Set();
  if (ref.value !== undefined) set.add(ref.value);
  map.set(ref.name, set);
}

function finalize(map: Map<string, Set<string | number | boolean>>): Reference[] {
  return [...map.entries()]
    .map(([name, set]) => ({ name, declared: [...set] }))
    .sort((a, b) => a.name.localeCompare(b.name));
}

function splitReferences(transitions: TransitionNode[]): { globals: Reference[]; locals: Reference[] } {
  const globalsByName = new Map<string, Set<string | number | boolean>>();
  const localsByName = new Map<string, Set<string | number | boolean>>();
  for (const tr of transitions) {
    for (const source of [tr.co_authored_globals, tr.co_authored_locals, tr.gating_conditions]) {
      for (const f of source) {
        if (isGlobal(f.kind)) collectByName(globalsByName, f);
        else if (f.kind === 'module_local') collectByName(localsByName, f);
      }
    }
  }
  return { globals: finalize(globalsByName), locals: finalize(localsByName) };
}

function liveGlobal(name: string, globals: XmlData): string | number | boolean | null {
  if (name in globals.integers) return globals.integers[name];
  if (name in globals.booleans) return globals.booleans[name];
  if (name in globals.floats) return globals.floats[name];
  if (name in globals.strings) return globals.strings[name];
  return null;
}

function liveLocal(name: string, vars: LiveModuleVar[]): string | number | boolean | null {
  const hit = vars.find(v => v.name === name);
  if (!hit) return null;
  return hit.value.value;
}

function isGlobal(kind: ConvoFunctor['kind']): boolean {
  return kind === 'global_int' || kind === 'global_string' || kind === 'global_float' || kind === 'global_bool';
}

export function LiveOverlay({
  transitions, graph,
}: {
  transitions: TransitionNode[];
  graph: SaveGraph;
}) {
  const t = useTranslations();

  const { globals, locals } = useMemo(() => splitReferences(transitions), [transitions]);

  if (globals.length === 0 && locals.length === 0) {
    return <div style={{ color: T.textMuted, fontStyle: 'italic' }}>{t('gameState.quests.detail.none')}</div>;
  }

  const renderRow = (
    ref: Reference,
    live: string | number | boolean | null,
  ) => {
    const liveStr = live == null ? null : String(live);
    const mismatches = live != null && ref.declared.length > 0 && !ref.declared.some(d => String(d) === liveStr);
    return (
      <tr key={ref.name}>
        <td className="t-mono">{ref.name}</td>
        <td className="t-mono">
          {live == null ? (
            <span style={{ color: T.textMuted, fontStyle: 'italic' }}>{t('gameState.quests.detail.unset')}</span>
          ) : (
            <span style={{ color: mismatches ? T.negative : T.text }}>{liveStr}</span>
          )}
          {mismatches && (
            <span style={{ color: T.textMuted, marginLeft: 6 }}>
              ({t('gameState.quests.detail.expected')}: {ref.declared.map(String).join(', ')})
            </span>
          )}
        </td>
      </tr>
    );
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {globals.length > 0 && (
        <div>
          <div className="t-semibold" style={{ color: T.textMuted, marginBottom: 4 }}>
            {t('gameState.quests.detail.liveOverlayGlobals')}
          </div>
          <HTMLTable compact bordered style={{ width: '100%' }}>
            <tbody>
              {globals.map(ref => renderRow(ref, liveGlobal(ref.name, graph.globals)))}
            </tbody>
          </HTMLTable>
        </div>
      )}
      {locals.length > 0 && (
        <div>
          <div className="t-semibold" style={{ color: T.textMuted, marginBottom: 4 }}>
            {t('gameState.quests.detail.liveOverlayLocals')}
          </div>
          <HTMLTable compact bordered style={{ width: '100%' }}>
            <tbody>
              {locals.map(ref => renderRow(ref, liveLocal(ref.name, graph.current_module_variables)))}
            </tbody>
          </HTMLTable>
        </div>
      )}
    </div>
  );
}
