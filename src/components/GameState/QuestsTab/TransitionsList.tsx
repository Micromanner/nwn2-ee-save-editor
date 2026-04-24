import { useMemo } from 'react';
import { T } from '../../theme';
import { useTranslations } from '@/hooks/useTranslations';
import type { TlkResolver } from '@/hooks/useTlkResolver';
import type { ConvoFunctor, QuestAggregate, TransitionNode } from './types';

function formatParam(param: unknown): string {
  if (param == null) return 'null';
  if (typeof param === 'string') return JSON.stringify(param);
  if (typeof param === 'number' || typeof param === 'boolean') return String(param);
  return JSON.stringify(param);
}

function FunctorRow({ f }: { f: ConvoFunctor }) {
  return (
    <div style={{ display: 'flex', gap: 6, alignItems: 'baseline', color: T.text }}>
      <span style={{
        fontSize: 11, padding: '1px 6px', borderRadius: 2,
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
      <div className="t-semibold" style={{ color: T.textMuted, fontSize: 12 }}>{title}</div>
      {items.length === 0 ? (
        <div style={{ color: T.textMuted, fontStyle: 'italic', fontSize: 12, marginLeft: 8 }}>
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

export function TransitionsList({ quest, tlk }: { quest: QuestAggregate; tlk: TlkResolver }) {
  const t = useTranslations();

  const groups = useMemo(() => {
    const byState = new Map<number, TransitionNode[]>();
    for (const tr of quest.transitions) {
      const list = byState.get(tr.new_state) ?? [];
      list.push(tr);
      byState.set(tr.new_state, list);
    }
    const stateOrder = [...byState.keys()].sort((a, b) => a - b);
    return stateOrder.map(state => ({
      state,
      transitions: byState.get(state)!,
      entryText: quest.category.entries.find(e => e.id === state),
    }));
  }, [quest]);

  if (quest.transitions.length === 0) {
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
          {g.transitions.map((tr, i) => {
            const lineText = tlk.resolve(tr.text_strref);
            return (
              <div
                key={`${tr.module}-${tr.dlg}-${tr.node}-${i}`}
                style={{ padding: '8px 10px', borderTop: i === 0 ? 'none' : `1px dashed ${T.borderLight}` }}
              >
                <div className="t-mono" style={{ fontSize: 12, color: T.textMuted }}>
                  {tr.module} / {tr.dlg} / {tr.node}
                </div>
                <div style={{ color: T.text, marginTop: 2 }}>
                  {lineText ?? <span style={{ color: T.textMuted, fontStyle: 'italic' }}>{t('gameState.quests.detail.noText')}</span>}
                </div>
                <FunctorList title={t('gameState.quests.detail.coGlobals')} items={tr.co_authored_globals} />
                <FunctorList title={t('gameState.quests.detail.coLocals')} items={tr.co_authored_locals} />
                <FunctorList title={t('gameState.quests.detail.gatingConditions')} items={tr.gating_conditions} />
              </div>
            );
          })}
        </div>
      ))}
    </div>
  );
}
