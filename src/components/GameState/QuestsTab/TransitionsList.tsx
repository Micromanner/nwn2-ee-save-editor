import { useMemo, useState } from 'react';
import { Icon } from '@blueprintjs/core';
import { T } from '../../theme';
import { useTranslations } from '@/hooks/useTranslations';
import type { TlkResolver } from '@/hooks/useTlkResolver';
import type { ConvoFunctor, QuestSummary, TransitionNode } from './types';

function formatParam(param: unknown): string {
  if (param == null) return 'null';
  if (typeof param === 'string') return JSON.stringify(param);
  if (typeof param === 'number' || typeof param === 'boolean') return String(param);
  return JSON.stringify(param);
}

function functorKey(f: ConvoFunctor): string {
  return `${f.kind}|${f.script}|${JSON.stringify(f.params)}`;
}

function transitionSignature(tr: TransitionNode): string {
  return [
    tr.text_strref ?? 'null',
    tr.co_authored_globals.map(functorKey).join(';'),
    tr.co_authored_locals.map(functorKey).join(';'),
    tr.gating_conditions.map(functorKey).join(';'),
  ].join('::');
}

function FunctorRow({ f }: { f: ConvoFunctor }) {
  return (
    <div style={{ display: 'flex', gap: 6, alignItems: 'baseline', color: T.text }}>
      <span style={{
        padding: '1px 6px', borderRadius: 2,
        background: f.kind === 'custom' ? `${T.textMuted}22` : `${T.accent}18`,
        color: f.kind === 'custom' ? T.textMuted : T.accent,
      }}>
        {f.kind}
      </span>
      <span className="t-mono" style={{ color: T.text }}>{f.script}</span>
      <span className="t-mono" style={{ color: T.textMuted }}>
        ({f.params.map(formatParam).join(', ')})
      </span>
    </div>
  );
}

function FunctorList({ title, items }: { title: string; items: ConvoFunctor[] }) {
  const t = useTranslations();
  return (
    <div style={{ marginTop: 4 }}>
      <div className="t-semibold" style={{ color: T.textMuted }}>{title}</div>
      {items.length === 0 ? (
        <div style={{ color: T.textMuted, fontStyle: 'italic', marginLeft: 8 }}>
          {t('gameState.quests.detail.none')}
        </div>
      ) : (
        <div style={{ marginLeft: 8, display: 'flex', flexDirection: 'column', gap: 2 }}>
          {items.map((f, i) => <FunctorRow key={i} f={f} />)}
        </div>
      )}
    </div>
  );
}

interface SubGroup {
  signature: string;
  representative: TransitionNode;
  members: TransitionNode[];
}

function TransitionGroup({
  group, isFirst, tlk,
}: {
  group: SubGroup;
  isFirst: boolean;
  tlk: TlkResolver;
}) {
  const t = useTranslations();
  const [expanded, setExpanded] = useState(false);
  const rep = group.representative;
  const isMulti = group.members.length > 1;
  const lineText = tlk.resolve(rep.text_strref);

  return (
    <div style={{ padding: '8px 10px', borderTop: isFirst ? 'none' : `1px dashed ${T.borderLight}` }}>
      {!isMulti && (
        <div className="t-mono" style={{ color: T.textMuted }}>
          {rep.module} / {rep.dlg} / {rep.node}
        </div>
      )}
      <div style={{ display: 'flex', gap: 8, alignItems: 'baseline', marginTop: isMulti ? 0 : 2 }}>
        <span style={{ color: T.text, flex: 1 }}>
          {lineText ?? <span style={{ color: T.textMuted, fontStyle: 'italic' }}>{t('gameState.quests.detail.noText')}</span>}
        </span>
        {isMulti && (
          <span
            className="t-mono"
            style={{
              padding: '1px 6px', borderRadius: 2,
              background: `${T.accent}18`, color: T.accent, flexShrink: 0,
            }}
          >
            &times;{group.members.length}
          </span>
        )}
      </div>
      <FunctorList title={t('gameState.quests.detail.coGlobals')} items={rep.co_authored_globals} />
      <FunctorList title={t('gameState.quests.detail.coLocals')} items={rep.co_authored_locals} />
      <FunctorList title={t('gameState.quests.detail.gatingConditions')} items={rep.gating_conditions} />
      {isMulti && (
        <>
          <button
            type="button"
            onClick={() => setExpanded(e => !e)}
            style={{
              marginTop: 6, background: 'transparent', border: 'none', padding: '2px 0',
              color: T.textMuted, cursor: 'pointer', display: 'flex', alignItems: 'center',
              gap: 4,
            }}
          >
            <Icon icon={expanded ? 'chevron-down' : 'chevron-right'} size={12} />
            {expanded
              ? t('gameState.quests.detail.hideSources')
              : t('gameState.quests.detail.showSources', { count: group.members.length })}
          </button>
          {expanded && (
            <div style={{ marginTop: 4, marginLeft: 16, display: 'flex', flexDirection: 'column', gap: 2 }}>
              {group.members.map((m, i) => (
                <div
                  key={`${m.module}-${m.dlg}-${m.node}-${i}`}
                  className="t-mono"
                  style={{ color: T.textMuted }}
                >
                  {m.module} / {m.dlg} / {m.node}
                </div>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}

export function TransitionsList({
  quest, transitions, tlk,
}: {
  quest: QuestSummary;
  transitions: TransitionNode[];
  tlk: TlkResolver;
}) {
  const t = useTranslations();

  const groups = useMemo(() => {
    const byState = new Map<number, TransitionNode[]>();
    for (const tr of transitions) {
      const list = byState.get(tr.new_state) ?? [];
      list.push(tr);
      byState.set(tr.new_state, list);
    }
    const stateOrder = [...byState.keys()].sort((a, b) => a - b);
    return stateOrder.map(state => {
      const stateTransitions = byState.get(state)!;
      const bySignature = new Map<string, SubGroup>();
      for (const tr of stateTransitions) {
        const sig = transitionSignature(tr);
        const existing = bySignature.get(sig);
        if (existing) {
          existing.members.push(tr);
        } else {
          bySignature.set(sig, { signature: sig, representative: tr, members: [tr] });
        }
      }
      return {
        state,
        subGroups: [...bySignature.values()],
        entryText: quest.category.entries.find(e => e.id === state),
      };
    });
  }, [transitions, quest.category.entries]);

  if (transitions.length === 0) {
    return (
      <div style={{ color: T.textMuted, fontStyle: 'italic' }}>
        {t('gameState.quests.detail.noTransitionsLiveOnly')}
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {groups.map(g => (
        <div key={g.state} style={{ border: `1px solid ${T.borderLight}`, borderRadius: 3 }}>
          <div style={{
            padding: '6px 10px', borderBottom: `1px solid ${T.borderLight}`,
            background: `${T.accent}08`, display: 'flex', gap: 8, alignItems: 'baseline',
          }}>
            <span className="t-semibold" style={{ color: T.accent }}>
              {t('gameState.quests.detail.groupPrefix')} <span className="t-mono">{g.state}</span>
            </span>
            {g.entryText && (
              <>
                <span style={{ color: T.text }}>&mdash; {g.entryText.text}</span>
                {g.entryText.final && <span style={{ color: T.positive }}>({t('gameState.quests.detail.final')})</span>}
              </>
            )}
          </div>
          {g.subGroups.map((sub, i) => (
            <TransitionGroup
              key={sub.signature}
              group={sub}
              isFirst={i === 0}
              tlk={tlk}
            />
          ))}
        </div>
      ))}
    </div>
  );
}
