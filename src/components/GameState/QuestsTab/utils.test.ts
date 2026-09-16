import { describe, expect, it } from 'vitest';
import { conflictingEdits, dialogueText, questStates } from './utils';
import { groupRoutes } from './QuestEditDialog';
import type { RouteSummary, PendingQuestEdit, QuestPreview, QuestSummary, TransitionNode, VariableWrite } from './types';

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
      text_strref: null, actions: [], co_authored_globals: [], co_authored_locals: [], gating_conditions: [],
      is_player: false, debug: false, speaker: '', children: [], follow_up: [] };
    expect(questStates(quest, [transition, transition])).toEqual([10, 20, 77]);
    expect(quest.live_state).toBe(77);
  });
});

describe('groupRoutes', () => {
  const line = (over: Partial<RouteSummary>): RouteSummary => ({ route: 0, module: 'a1', dlg: 'd', speaker: '', is_player: true, text_strref: 1,
    debug: false, signature: 'x=1|end', writes: [], effects: [], write_count: 1, followed_count: 0, ...over });

  it('collapses same-outcome lines, puts the current module first, hides cheat dialogues', () => {
    const grouped = groupRoutes([
      line({ route: 0, module: 'mq', signature: 'x=1|end' }),
      line({ route: 1, module: 'a1', signature: 'x=1|end' }),
      line({ route: 2, module: 'a1', signature: 'x=1|end', text_strref: 2 }),
      line({ route: 3, module: 'a1', signature: 'x=2|end' }),
      line({ route: 4, module: 'a1', debug: true }),
    ], 'a1');
    // Route 0 (other module) has the same outcome as route 1 and is folded into it.
    expect(grouped.map(o => o.route)).toEqual([1, 3]);
    expect(grouped.map(o => o.showModule)).toEqual([false, false]);
  });

  it('shows the module only when the same speaker has different outcomes per module', () => {
    const grouped = groupRoutes([
      line({ route: 0, module: 'a1', signature: 'x=1|end', text_strref: 1 }),
      line({ route: 1, module: 'mq', signature: 'x=2|end', text_strref: 2 }),
    ], 'a1');
    expect(grouped.map(o => o.showModule)).toEqual([true, true]);
  });

  it('prefers the current module when the same spoken line exists elsewhere with other flags', () => {
    const grouped = groupRoutes([
      line({ route: 0, module: 'mq', signature: 'faulk=2|end', text_strref: 5 }),
      line({ route: 1, module: 'a1', signature: 'faulk=2;sal=2|end', text_strref: 5 }),
      line({ route: 2, module: 'mq', signature: 'other=1|end', text_strref: 9 }),
    ], 'a1');
    expect(grouped.map(o => o.route)).toEqual([1, 2]);
  });

  it('keeps the line the player chose over a silent branch with the same outcome', () => {
    const grouped = groupRoutes([
      line({ route: 0, module: 'a1', signature: 'x=1|end', text_strref: null, spoken: { node: 1, is_player: false, speaker: 'caleb', text_strref: 3, conditions: [] } }),
      line({ route: 1, module: 'a1', signature: 'x=1|end', text_strref: 5, spoken: { node: 2, is_player: true, speaker: '', text_strref: 5, conditions: [] } }),
    ], 'a1');
    expect(grouped.map(o => o.route)).toEqual([1]);
  });

  it('labels a silent reply by the line the player saw', () => {
    const grouped = groupRoutes([
      line({ route: 0, module: 'mq', signature: 'x=1|end', text_strref: null, spoken: { node: 3, is_player: true, speaker: '', text_strref: 7, conditions: [] } }),
      line({ route: 1, module: 'a1', signature: 'x=2|end', text_strref: null, spoken: { node: 4, is_player: true, speaker: '', text_strref: 7, conditions: [] } }),
    ], 'a1');
    expect(grouped.map(o => o.route)).toEqual([1]);
  });

  it('falls back to cheat dialogues when nothing else reaches the entry', () => {
    const grouped = groupRoutes([line({ route: 4, debug: true })], null);
    expect(grouped.map(o => o.route)).toEqual([4]);
    expect(grouped[0].showModule).toBe(false);
  });
});

describe('dialogueText', () => {
  it('hides designer notes and stage directions the game never shows', () => {
    expect(dialogueText('| Good Response |No reward is necessary.')).toBe('No reward is necessary.');
    expect(dialogueText('|Player helped at Fort Locke|{Very happy}Well, look who it is.')).toBe('Well, look who it is.');
    expect(dialogueText('{Cormick nods}')).toBeNull();
    expect(dialogueText(null)).toBeNull();
  });
});
