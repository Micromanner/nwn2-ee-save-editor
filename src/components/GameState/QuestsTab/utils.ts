import type { QuestSummary } from './types';

export type QuestSource = 'normal' | 'live_only' | 'transition_only';

export function effectiveSource(q: QuestSummary): QuestSource {
  const s = q.category.source;
  if (s === 'live_only' || s === 'transition_only') return s;
  return 'normal';
}

export function selectedRowStyle(isSelected: boolean, accent: string) {
  return isSelected
    ? { background: `${accent}14`, borderLeft: `3px solid ${accent}` }
    : {};
}
