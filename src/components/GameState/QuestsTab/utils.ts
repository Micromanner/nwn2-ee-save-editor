import type { PendingQuestEdit, QuestPreview, QuestSummary, TransitionNode, VariableKey } from './types';

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

export function sameVariable(a: VariableKey, b: VariableKey) {
  return a.module === b.module && a.name === b.name && a.value_type === b.value_type;
}

export function conflictingEdits(preview: QuestPreview | null, selected: number[], pending: PendingQuestEdit[]) {
  return preview ? pending.filter(p => p.tag !== preview.tag && p.writes.some(old => selected.some(i => {
    const write = preview.writes[i];
    return write && sameVariable(old.key, write.key) && old.after !== write.after;
  }))) : [];
}

export function questStates(quest: QuestSummary, transitions: TransitionNode[]) {
  return [...new Set([...quest.category.entries.map(e => e.id), ...transitions.map(tr => tr.new_state),
    ...(quest.live_state == null ? [] : [quest.live_state]), ...(quest.pending_state == null ? [] : [quest.pending_state])])]
    .sort((a, b) => a - b);
}

export type QuestStatus = 'untouched' | 'active' | 'complete';

export function questStatus(q: QuestSummary): QuestStatus {
  if (q.live_state == null) return 'untouched';
  return q.category.entries.find(e => e.id === q.live_state)?.final ? 'complete' : 'active';
}

/** Journal names carry a toolset prefix like `{1000}` or `{CUT!}` that players never see in-game. */
export function questDisplayName(q: QuestSummary) {
  return (q.category.name || '').replace(/^\{[^}]*\}\s*/, '') || q.tag;
}

