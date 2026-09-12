import { useEffect, useState } from 'react';
import type React from 'react';
import { Button, Checkbox, Spinner } from '@blueprintjs/core';
import { invoke } from '@tauri-apps/api/core';
import { DropdownSelect } from '@/components/shared/DropdownSelect';
import { ParchmentDialog } from '@/components/shared/ParchmentDialog';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useTranslations } from '@/hooks/useTranslations';
import type { TlkResolver } from '@/hooks/useTlkResolver';
import { display } from '@/utils/dataHelpers';
import { conflictingEdits, questDisplayName } from './utils';
import { describeScript, formatRawCall } from './scriptDescriptions';
import type { AggregatedModule, PendingQuestEdit, QuestPreview, QuestSummary, TransitionNode } from './types';
import { useModuleNameResolver } from './useModuleNameResolver';
import { T } from '../../theme';
import { PlainList } from './TransitionsList';

export function QuestEditDialog({ quest, target, transitions, pending, tlk, modules, onClose, onApplied }: {
  quest: QuestSummary; target: number; transitions: TransitionNode[]; pending: PendingQuestEdit[];
  tlk: TlkResolver; modules: AggregatedModule[];
  onClose: () => void; onApplied: () => void;
}) {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const { resolve: resolveModule } = useModuleNameResolver(modules);
  const [route, setRoute] = useState(-1);
  const [preview, setPreview] = useState<QuestPreview | null>(null);
  const [selected, setSelected] = useState<number[]>([]);
  const [replace, setReplace] = useState(false);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [refresh, setRefresh] = useState(0);
  const [failed, setFailed] = useState(false);
  const choices = [{ id: -1, label: t('gameState.quests.edit.journalOnly') }, ...transitions.flatMap((tr, i) =>
    tr.new_state === target ? [{ id: i, label: `${(tlk.resolve(tr.text_strref) ?? t('gameState.quests.detail.noText')).slice(0, 120)} (${resolveModule(tr.module)} / ${tr.dlg} / ${tr.node})` }] : [])];

  useEffect(() => {
    let active = true;
    setPreview(null); setSelected([]); setReplace(false); setFailed(false); setLoading(true);
    invoke<QuestPreview>('save_preview_quest_edit', { tag: quest.tag, target, route: route < 0 ? null : route })
      .then(result => { if (active) setPreview(result); })
      .catch(error => { if (active) { setFailed(true); handleError(error); } })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [quest.tag, target, route, refresh, pending, handleError]);

  const conflicts = conflictingEdits(preview, selected, pending);
  const apply = async () => {
    if (!preview || busy) return;
    setBusy(true);
    try {
      await invoke('save_stage_quest_edit', { tag: quest.tag, target, route: preview.route,
        revision: preview.revision, selected, replace });
      window.dispatchEvent(new Event('quest-edits-changed'));
      onApplied(); onClose();
    } catch (error) { handleError(error); setRefresh(r => r + 1); }
    finally { setBusy(false); }
  };

  const section = (title: string, body: React.ReactNode) => <div style={{ borderTop: `1px solid ${T.borderLight}`, padding: '10px 0' }}>
    <div className="t-bold" style={{ color: T.textMuted, marginBottom: 6 }}>{title}</div>
    {body}
  </div>;
  const moduleLabel = (module: string | null) => module ? resolveModule(module) : t('gameState.quests.edit.campaign');
  const targetText = quest.category.entries.find(e => e.id === target)?.text;

  return <ParchmentDialog isOpen title={t('gameState.quests.edit.title')} onClose={() => { if (!busy) onClose(); }} width={760}
    footerActions={<>
      <Button intent="primary" loading={busy} disabled={!preview || loading || (conflicts.length > 0 && !replace)} onClick={() => void apply()}>
        {t('gameState.quests.edit.apply')}
      </Button>
    </>}>
    <div style={{ overflowY: 'auto', minHeight: 0 }}>
      <div style={{ paddingBottom: 10 }}>
        <div className="t-bold" style={{ color: T.text, marginBottom: 8 }}>{questDisplayName(quest)}</div>
        <div style={{ display: 'grid', gridTemplateColumns: 'auto 1fr', columnGap: 16, rowGap: 4, alignItems: 'baseline' }}>
          <span style={{ color: T.textMuted }}>{t('gameState.quests.edit.saved')}</span>
          <span className="t-semibold" style={{ color: T.text }}>{t('gameState.quests.detail.entry', { n: display(preview?.saved_state) })}</span>
          <span style={{ color: T.textMuted }}>{t('gameState.quests.edit.newEntry')}</span>
          <span style={{ color: T.text }}>
            <span className="t-semibold">{t('gameState.quests.detail.entry', { n: target })}</span>
            {targetText && <>: {targetText}</>}
          </span>
        </div>
      </div>

      {section(t('gameState.quests.edit.route'), <>
        <DropdownSelect value={choices.find(c => c.id === route)?.label ?? ''} items={choices} onChange={setRoute} disabled={busy} />
        {route >= 0 && transitions[route] && <div style={{ marginTop: 8 }}>
          <p style={{ fontStyle: 'italic', color: T.textMuted }}>{display(tlk.resolve(transitions[route].text_strref))}</p>
          <PlainList title={t('gameState.quests.detail.alsoHappens')} items={[...transitions[route].actions, ...transitions[route].co_authored_locals]} />
          <PlainList title={t('gameState.quests.detail.requires')} items={transitions[route].gating_conditions} />
        </div>}
      </>)}

      {loading && <Spinner size={24} />}
      {failed && <Button onClick={() => setRefresh(r => r + 1)}>{t('gameState.quests.edit.refresh')}</Button>}
      {preview && <>
        {section(t('gameState.quests.edit.writes'), <>
          <div style={{ display: 'flex', gap: 10, padding: '4px 0' }}>
            <span style={{ color: T.textMuted, minWidth: 140 }}>{t('gameState.quests.edit.journal')}</span>
            <span className="t-mono" style={{ color: T.text }}>{display(preview.before_state)} → {target}</span>
          </div>
          {preview.writes.map((write, index) => <Checkbox key={index} checked={selected.includes(index)} disabled={busy} style={{ margin: '4px 0' }}
            onChange={() => { setReplace(false); setSelected(old => old.includes(index) ? old.filter(i => i !== index) : [...old, index]); }}>
            <span className="t-semibold">{write.key.name}</span>
            <span className="t-mono" style={{ marginLeft: 10, color: T.text }}>{display(write.before)} → {display(write.after)}</span>
            <div style={{ color: T.textMuted }}>
              {moduleLabel(write.key.module)} · {write.key.value_type} · {t('gameState.quests.edit.saved')}: {display(write.saved)}
            </div>
          </Checkbox>)}
          {conflicts.length > 0 && <div style={{ marginTop: 8, padding: '8px 12px', background: `${T.gold}14`, border: `1px solid ${T.gold}66`, borderRadius: 4 }}>
            <Checkbox checked={replace} onChange={() => setReplace(!replace)} disabled={busy} style={{ margin: 0 }}>
              {t('gameState.quests.edit.replace', { quests: conflicts.map(p => p.tag).join(', ') })}
            </Checkbox>
            {conflicts.map(edit => <div key={edit.tag} style={{ color: T.textMuted, marginTop: 4 }}>
              <span className="t-semibold">{edit.tag}</span> <span className="t-mono">{display(edit.saved_state)} → {edit.target}</span>
              {edit.writes.map((write, i) => <div key={i}>{moduleLabel(write.key.module)} / {write.key.name}: {display(write.after)}</div>)}
            </div>)}
          </div>}
        </>)}

        {preview.effects.length > 0 && section(t('gameState.quests.edit.effects'), preview.effects.map((effect, i) => <div key={i} style={{ marginBottom: 6 }}>
          <div>{describeScript(effect.action, t) ?? <code>{formatRawCall(effect.action)}</code>}</div>
          <div style={{ color: T.textMuted }}>{t(`gameState.quests.edit.reason.${effect.reason}`)}</div>
        </div>))}

        {section(t('gameState.quests.edit.afterLoad'), <div style={{ color: T.text, lineHeight: 1.5 }}>
          <p style={{ margin: '0 0 6px' }}>{t('gameState.quests.edit.loadExplanation')}</p>
          <p style={{ margin: 0 }}>{t('gameState.quests.edit.stagingExplanation')}</p>
        </div>)}
      </>}
    </div>
  </ParchmentDialog>;
}
