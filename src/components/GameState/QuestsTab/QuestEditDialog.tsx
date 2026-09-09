import { useEffect, useState } from 'react';
import { Button, Checkbox, Spinner } from '@blueprintjs/core';
import { invoke } from '@tauri-apps/api/core';
import { DropdownSelect } from '@/components/shared/DropdownSelect';
import { ParchmentDialog } from '@/components/shared/ParchmentDialog';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useTranslations } from '@/hooks/useTranslations';
import type { TlkResolver } from '@/hooks/useTlkResolver';
import { display } from '@/utils/dataHelpers';
import { conflictingEdits } from './utils';
import type { AggregatedModule, PendingQuestEdit, QuestPreview, QuestSummary, TransitionNode } from './types';
import { useModuleNameResolver } from './useModuleNameResolver';

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

  return <ParchmentDialog isOpen title={t('gameState.quests.edit.title')} onClose={() => { if (!busy) onClose(); }} width={760}
    footerActions={<>
      <Button intent="primary" loading={busy} disabled={!preview || loading || (conflicts.length > 0 && !replace)} onClick={() => void apply()}>
        {t('gameState.quests.edit.apply')}
      </Button>
    </>}>
    <div style={{ overflowY: 'auto', minHeight: 0 }}>
      <p><strong>{quest.category.name || quest.tag}</strong> — {t('gameState.quests.edit.saved')}: {display(preview?.saved_state)} → {target}</p>
      <p>{quest.category.entries.find(e => e.id === target)?.text}</p>
      <DropdownSelect label={t('gameState.quests.edit.route')} value={choices.find(c => c.id === route)?.label ?? ''}
        items={choices} onChange={setRoute} disabled={busy} />
      {route >= 0 && <p>{display(tlk.resolve(transitions[route]?.text_strref ?? null))}</p>}
      {loading && <Spinner size={24} />}
      {failed && <Button onClick={() => setRefresh(r => r + 1)}>{t('gameState.quests.edit.refresh')}</Button>}
      {preview && <>
        <h4>{t('gameState.quests.edit.writes')}</h4>
        <p>{t('gameState.quests.edit.journalWrite', { tag: quest.tag, before: display(preview.before_state), target })}</p>
        {preview.writes.map((write, index) => <Checkbox key={index} checked={selected.includes(index)} disabled={busy}
          onChange={() => { setReplace(false); setSelected(old => old.includes(index) ? old.filter(i => i !== index) : [...old, index]); }}>
          <strong>{write.key.name}</strong> ({write.key.module ? resolveModule(write.key.module) : t('gameState.quests.edit.campaign')}, {write.key.value_type})
          <div>{t('gameState.quests.edit.saved')}: {display(write.saved)}; {t('gameState.quests.edit.effective')}: {display(write.before)} → {display(write.after)}</div>
        </Checkbox>)}
        {conflicts.length > 0 && <Checkbox checked={replace} onChange={() => setReplace(!replace)} disabled={busy}>
          {t('gameState.quests.edit.replace', { quests: conflicts.map(p => p.tag).join(', ') })}
        </Checkbox>}
        {conflicts.map(edit => <details key={edit.tag}>
          <summary>{edit.tag}: {edit.target} → {display(edit.saved_state)}</summary>
          {edit.writes.map((write, i) => <p key={i}>{write.key.module ? resolveModule(write.key.module) : t('gameState.quests.edit.campaign')} / {write.key.name}: {display(write.after)}</p>)}
        </details>)}
        {preview.effects.length > 0 && <>
          <h4>{t('gameState.quests.edit.effects')}</h4>
          {preview.effects.map((effect, i) => <div key={i} style={{ marginBottom: 8 }}>
            <code>{effect.action.script}({effect.action.params.map(p => JSON.stringify(p)).join(', ')})</code>
            <div>{t(`gameState.quests.edit.reason.${effect.reason}`)}</div>
          </div>)}
        </>}
        <h4>{t('gameState.quests.edit.afterLoad')}</h4>
        <p>{t('gameState.quests.edit.loadExplanation')}</p>
        <p>{t('gameState.quests.edit.stagingExplanation')}</p>
      </>}
    </div>
  </ParchmentDialog>;
}
