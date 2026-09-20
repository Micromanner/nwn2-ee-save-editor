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

/** Items given and taken add up across quests instead of overwriting each other, so they never conflict. */
export function conflictingEdits(preview: QuestPreview | null, selected: number[], pending: PendingQuestEdit[]) {
  return preview ? pending.filter(p => p.tag !== preview.tag && p.writes.some(old => selected.some(i => {
    const write = preview.writes[i];
    return write && write.key.value_type !== 'item' && write.key.value_type !== 'item_take' && sameVariable(old.key, write.key) && old.after !== write.after;
  }))) : [];
}

export function questStates(quest: QuestSummary, transitions: TransitionNode[]) {
  return [...new Set([...quest.category.entries.map(e => e.id), ...transitions.map(tr => tr.new_state),
    ...(quest.live_state == null ? [] : [quest.live_state])])]
    .sort((a, b) => a - b);
}

export type QuestStatus = 'untouched' | 'active' | 'complete';

export function questStatus(q: QuestSummary): QuestStatus {
  if (q.live_state == null) return 'untouched';
  return q.category.entries.find(e => e.id === q.live_state)?.final ? 'complete' : 'active';
}

/** Journal names carry a toolset prefix like `{1000}` or `{CUT!}` that players never see in-game. */
/**
 * Dialogue text as the game shows it: `{stage directions}` and the leading
 * `|designer note|` markers are hidden in-game.
 */
export function dialogueText(raw: string | null | undefined): string | null {
  if (raw == null) return null;
  const text = raw.replace(/\{[^}]*\}/g, '').replace(/^\s*(\|[^|]*\|\s*)+/, '').trim();
  return text.length > 0 ? text : null;
}

/** Journal entry text as the game shows it: designer notes in braces are never displayed. */
export function journalText(raw: string | null | undefined): string | undefined {
  if (raw == null) return undefined;
  const text = raw.replace(/\{[^}]*\}/g, '').trim();
  return text.length > 0 ? text : undefined;
}

export function questDisplayName(q: QuestSummary) {
  return (q.category.name || '').replace(/^\{[^}]*\}\s*/, '') || q.tag;
}

