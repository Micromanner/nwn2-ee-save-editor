import { useEffect, useState } from 'react';
import type React from 'react';
import { Button, Checkbox, Radio, RadioGroup, Spinner } from '@blueprintjs/core';
import { invoke } from '@tauri-apps/api/core';
import { ParchmentDialog } from '@/components/shared/ParchmentDialog';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useTranslations } from '@/hooks/useTranslations';
import type { TlkResolver } from '@/hooks/useTlkResolver';
import { display } from '@/utils/dataHelpers';
import { conflictingEdits, dialogueText, questDisplayName } from './utils';
import { describeScript, formatRawCall } from './scriptDescriptions';
import type { AggregatedModule, PendingQuestEdit, QuestPreview, QuestSummary, RouteSummary, TransitionNode } from './types';
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
  const [route, setRoute] = useState<number | null>(null);
  const [options, setOptions] = useState<RouteOption[] | null>(null);
  const [preview, setPreview] = useState<QuestPreview | null>(null);
  const [selected, setSelected] = useState<number[]>([]);
  const [replace, setReplace] = useState(false);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [refresh, setRefresh] = useState(0);
  const [failed, setFailed] = useState(false);
  const currentModule = modules.find(m => m.is_current)?.name ?? null;

  // Lines are grouped by outcome so three phrasings of one moment become one choice;
  // developer cheat conversations are hidden unless nothing else reaches the entry.
  useEffect(() => {
    let active = true;
    setOptions(null);
    invoke<RouteSummary[]>('save_summarize_quest_routes', { tag: quest.tag, target })
      .then(summaries => {
        if (!active) return;
        const grouped = groupRoutes(summaries, currentModule);
        setOptions(grouped);
        setRoute(grouped.length > 0 ? grouped[0].route : -1);
      })
      .catch(error => { if (active) { setOptions([]); setRoute(-1); handleError(error); } });
    return () => { active = false; };
  }, [quest.tag, target, pending, currentModule, handleError]);

  useEffect(() => {
    if (route == null) return;
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

  const optionLabel = (option: RouteOption) => {
    const text = dialogueText(tlk.resolve(option.text_strref)) ?? t('gameState.quests.detail.noText');
    const who = option.is_player ? null : (option.speaker || t('gameState.quests.edit.follow.npc'));
    const line = who ? `${who}: ${text}` : t('gameState.quests.edit.youSaid', { text });
    return option.showModule ? `${line} (${resolveModule(option.module)})` : line;
  };
  const optionOutcome = (option: RouteOption) => option.writes.length === 0
    ? t('gameState.quests.edit.outcome.none')
    : t('gameState.quests.edit.outcome.sets', { flags: option.writes.map(w => `${w.name} = ${display(w.after)}`).join(', ') });
  const followedLine = (node: number | undefined) => node == null ? null : preview?.followed?.find(l => l.node === node) ?? null;
  const lineLabel = (line: { is_player: boolean; speaker: string; text_strref: number | null; has_text: boolean }) => {
    const who = line.is_player ? t('gameState.quests.edit.follow.you') : (line.speaker || t('gameState.quests.edit.follow.npc'));
    const text = line.has_text ? (dialogueText(tlk.resolve(line.text_strref)) ?? t('gameState.quests.detail.noText'))
      : line.is_player ? t('gameState.quests.edit.follow.continue') : t('gameState.quests.edit.follow.silent');
    return `${who}: ${text}`;
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
        {options == null && <Spinner size={16} />}
        {options != null && options.length === 0 && <p style={{ margin: 0, color: T.textMuted }}>{t('gameState.quests.edit.noRoutes')}</p>}
        {options != null && options.length === 1 && route === options[0].route && <div style={{ color: T.text }}>
          <p style={{ margin: 0 }}>{t('gameState.quests.edit.asIf')} <span style={{ fontStyle: 'italic' }}>{optionLabel(options[0])}</span></p>
          <p style={{ margin: '2px 0 0', color: T.textMuted }}>{optionOutcome(options[0])}</p>
        </div>}
        {options != null && options.length > 1 && <>
          <p style={{ margin: '0 0 6px', color: T.textMuted }}>{t('gameState.quests.edit.pickLine')}</p>
          <RadioGroup selectedValue={route ?? -1} onChange={e => setRoute(Number(e.currentTarget.value))} disabled={busy}>
            {options.map(option => <Radio key={option.route} value={option.route}>
              <span style={{ fontStyle: 'italic' }}>{optionLabel(option)}</span>
              <div style={{ color: T.textMuted }}>{optionOutcome(option)}</div>
            </Radio>)}
          </RadioGroup>
        </>}
        {options != null && options.length > 0 && <div style={{ marginTop: 6 }}>
          <Checkbox checked={route === -1} disabled={busy} style={{ margin: 0 }}
            onChange={() => setRoute(route === -1 ? options[0].route : -1)}>
            <span style={{ color: T.textMuted }}>{t('gameState.quests.edit.journalOnly')}</span>
          </Checkbox>
          {route === -1 && <div style={{ color: T.textMuted, paddingLeft: 26 }}>{t('gameState.quests.edit.journalOnlyHint')}</div>}
        </div>}
        {route != null && route >= 0 && transitions[route] && <div style={{ marginTop: 8 }}>
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
              {write.derived_from && <> · {t('gameState.quests.edit.derivedFrom', { script: write.derived_from })}</>}
              {followedLine(write.line) && <> · {t('gameState.quests.edit.follow.fromLine', { line: lineLabel(followedLine(write.line)!).slice(0, 80) })}</>}
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

        {preview.route != null && section(t('gameState.quests.edit.follow.title'), <div style={{ lineHeight: 1.5 }}>
          {(preview.followed ?? []).length === 0
            ? <p style={{ margin: '0 0 6px', color: T.textMuted }}>{t('gameState.quests.edit.follow.none')}</p>
            : (preview.followed ?? []).map(line => <div key={line.node} style={{ color: line.has_text ? T.text : T.textMuted }}>{lineLabel(line)}</div>)}
          {preview.follow_stop && <div style={{ color: T.textMuted, marginTop: 4 }}>
            {t(`gameState.quests.edit.follow.stop.${preview.follow_stop.reason}`, { choices: preview.follow_stop.choices })}
            {preview.follow_stop.reason === 'unknown_condition' && (preview.follow_stop.conditions ?? []).length > 0 && <span className="t-mono"> ({(preview.follow_stop.conditions ?? []).map(formatRawCall).join(', ')})</span>}
          </div>}
        </div>)}

        {preview.effects.length > 0 && section(t('gameState.quests.edit.effects'), preview.effects.map((effect, i) => <div key={i} style={{ marginBottom: 6 }}>
          <div>{describeScript(effect.action, t) ?? <code>{formatRawCall(effect.action)}</code>}{followedLine(effect.line) && <span style={{ color: T.textMuted }}> · {t('gameState.quests.edit.follow.fromLine', { line: lineLabel(followedLine(effect.line)!).slice(0, 80) })}</span>}</div>
          <div style={{ color: T.textMuted }}>{t(`gameState.quests.edit.reason.${effect.reason}`)}</div>
          {effect.action.decoded && <div style={{ color: T.textMuted, paddingLeft: 12 }}>
            {effect.action.decoded.calls.length > 0 && <div>{t('gameState.quests.edit.decodedCalls')}: <span className="t-mono">{effect.action.decoded.calls.join(', ')}</span></div>}
            {effect.action.decoded.conditional_writes.map((w, j) => <div key={j}>{t('gameState.quests.edit.decodedConditional', { name: w.name, value: w.value })}</div>)}
            {effect.action.decoded.unresolved.map((call, j) => <div key={j}>{t('gameState.quests.edit.decodedUnresolved')}: <span className="t-mono">{call}</span></div>)}
          </div>}
        </div>))}

        {section(t('gameState.quests.edit.afterLoad'), <div style={{ color: T.text, lineHeight: 1.5 }}>
          <p style={{ margin: '0 0 6px' }}>{t('gameState.quests.edit.loadExplanation')}</p>
          <p style={{ margin: 0 }}>{t('gameState.quests.edit.stagingExplanation')}</p>
        </div>)}
      </>}
    </div>
  </ParchmentDialog>;
}

interface RouteOption extends RouteSummary {
  /** Several offered lines share a speaker across modules: show which module. */
  showModule: boolean;
}

/** Collapse lines with identical outcomes, current module first, cheat dialogues last resort. */
export function groupRoutes(summaries: RouteSummary[], currentModule: string | null): RouteOption[] {
  const playable = summaries.filter(s => !s.debug);
  const pool = playable.length > 0 ? playable : summaries;
  const ordered = [...pool].sort((a, b) => Number(b.module === currentModule) - Number(a.module === currentModule));
  // The same NPC repeats a conversation in every act's city module; when the outcome
  // is identical the module is noise, so keep one line (the current module's if any).
  const seen = new Map<string, RouteSummary>();
  for (const s of ordered) {
    const key = `${s.speaker}|${s.is_player}|${s.signature}`;
    if (!seen.has(key)) seen.set(key, s);
  }
  const unique = [...seen.values()];
  const modulesPerSpeaker = new Map<string, Set<string>>();
  for (const s of unique) {
    const set = modulesPerSpeaker.get(s.speaker) ?? new Set<string>();
    set.add(s.module);
    modulesPerSpeaker.set(s.speaker, set);
  }
  return unique.map(s => ({ ...s, showModule: (modulesPerSpeaker.get(s.speaker)?.size ?? 0) > 1 }));
}
