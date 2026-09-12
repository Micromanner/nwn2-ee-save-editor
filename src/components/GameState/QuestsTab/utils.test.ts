import { describe, expect, it } from 'vitest';
import { conflictingEdits, questStates } from './utils';
import type { PendingQuestEdit, QuestPreview, QuestSummary, TransitionNode, VariableWrite } from './types';

const write = (module: string | null, after: number): VariableWrite => ({
  key: { module, name: 'progress', value_type: 'int' }, saved: 1, before: 1, after, action_index: 0,
});
const pending = (tag: string, writes: VariableWrite[]): PendingQuestEdit => ({
  tag, target: 20, saved_state: 1, source: null, writes,
});
const preview: QuestPreview = { revision: 'abc', tag: 'target', target: 20, saved_state: 1,
  before_state: 1, route: 0, writes: [write('module_a', 5)], effects: [] };

describe('quest edit conflicts', () => {
  it('only replaces selected, differing writes to the same scoped variable', () => {
    const edits = [pending('conflict', [write('module_a', 2)]), pending('equal', [write('module_a', 5)]),
      pending('other module', [write('module_b', 2)]), pending('global', [write(null, 2)]),
      pending('target', [write('module_a', 2)])];
    expect(conflictingEdits(preview, [], edits)).toEqual([]);
    expect(conflictingEdits(preview, [0], edits).map(p => p.tag)).toEqual(['conflict']);
  });
  it('keeps separate variable types separate', () => {
    const differentType = write('module_a', 2);
    differentType.key.value_type = 'float';
    expect(conflictingEdits(preview, [0], [pending('float', [differentType])])).toEqual([]);
  });
});

describe('quest states', () => {
  it('includes unknown saved progress and transition-only outcomes', () => {
    const quest: QuestSummary = { tag: 'q', live_state: 77, defined_in: [], transition_count: 1,
      category: { tag: 'q', name: 'Quest', priority: '', xp: 0, source: '', entries: [{ id: 20, text: 'Final', final: true }] } };
    const transition: TransitionNode = { module: 'm', dlg: 'd', node: 0, new_state: 10,
      text_strref: null, actions: [], co_authored_globals: [], co_authored_locals: [], gating_conditions: [] };
    expect(questStates(quest, [transition, transition])).toEqual([10, 20, 77]);
    expect(quest.live_state).toBe(77);
  });
});
