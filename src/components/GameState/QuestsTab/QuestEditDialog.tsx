import { Fragment, useEffect, useState } from 'react';
import type { ReactNode } from 'react';
import { Button, Checkbox, Radio, RadioGroup, Spinner, Tag } from '@blueprintjs/core';
import { invoke } from '@tauri-apps/api/core';
import { ParchmentDialog } from '@/components/shared/ParchmentDialog';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useTranslations } from '@/hooks/useTranslations';
import type { TlkResolver } from '@/hooks/useTlkResolver';
import { display } from '@/utils/dataHelpers';
import { conflictingEdits, dialogueText, journalText, questDisplayName } from './utils';
import { PC_LOCAL_TYPE, consoleCommand, describeEngineCall, describeScript, effectBucket, engineCallBucket, formatRawCall, influenceCompanion, isFeatWrite, isItemWrite, isJournalConnector, isMajorEngineCall, isMajorWorldScript, rewardKind, signedDelta } from './scriptDescriptions';
import type { AggregatedModule, ConvoFunctor, NextLine, PendingQuestEdit, QuestPreview, QuestSummary, RouteSummary, SpokenLine } from './types';
import { useModuleNameResolver } from './useModuleNameResolver';
import { useSpeakerNameResolver } from './useSpeakerNameResolver';
import { T } from '../../theme';

const ENTRY_TEXT_LIMIT = 140;

export function QuestEditDialog({ quest, target, pending, tlk, modules, onClose, onApplied }: {
  quest: QuestSummary; target: number; pending: PendingQuestEdit[];
  tlk: TlkResolver; modules: AggregatedModule[];
  onClose: () => void; onApplied: () => void;
}) {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const { resolve: resolveModule } = useModuleNameResolver(modules);
  const speakers = useSpeakerNameResolver(modules, tlk);
  const [route, setRoute] = useState<number | null>(null);
  const [options, setOptions] = useState<RouteOption[] | null>(null);
  const [preview, setPreview] = useState<QuestPreview | null>(null);
  const [replace, setReplace] = useState(false);
  const [showWorld, setShowWorld] = useState(false);
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
        tlk.prime([...grouped.map(o => (o.spoken ?? o).text_strref), ...speakers.strrefs(grouped.map(o => ({ ...(o.spoken ?? o), dlg: o.dlg, module: o.module })))]);
        setOptions(grouped);
        setRoute(grouped.length > 0 ? grouped[0].route : -1);
      })
      .catch(error => { if (active) { setOptions([]); setRoute(-1); handleError(error); } });
    return () => { active = false; };
    // `tlk.prime` is a stable useCallback; the resolver object itself changes every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [quest.tag, target, pending, currentModule, handleError]);

  useEffect(() => {
    if (route == null) return;
    let active = true;
    setPreview(null); setReplace(false); setFailed(false); setLoading(true);
    invoke<QuestPreview>('save_preview_quest_edit', { tag: quest.tag, target, route: route < 0 ? null : route })
      .then(result => {
        if (!active) return;
        const lines = (result.next_steps ?? []).flatMap(step => step.lines);
        tlk.prime([...result.writes.map(write => write.label_strref),
          ...lines.map(line => (line.spoken ?? line).text_strref),
          ...speakers.strrefs(lines.map(line => ({ ...(line.spoken ?? line), dlg: line.dlg, module: line.module })))]);
        setPreview(result);
      })
      .catch(error => { if (active) { setFailed(true); handleError(error); } })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [quest.tag, target, route, refresh, pending, handleError]);

  // Every flag on the route is written: a partial replay is not a state the game can reach.
  const selected = preview ? preview.writes.map((_, index) => index) : [];
  // Gold, XP and influence are written like any other flag but shown with the rewards.
  const rewardWrites = preview ? preview.writes.filter(write => rewardKind(write.key)) : [];
  const itemWrites = preview ? preview.writes.filter(write => isItemWrite(write.key)) : [];
  const influenceWrites = preview ? preview.writes.filter(write => influenceCompanion(write.key.name)) : [];
  const featWrites = preview ? preview.writes.filter(write => isFeatWrite(write.key)) : [];
  const flagWrites = preview ? preview.writes.filter(write => !rewardKind(write.key) && !isItemWrite(write.key) && !isFeatWrite(write.key) && !influenceCompanion(write.key.name)) : [];
  // Blueprint name, TLK name, or the resref when neither resolves.
  const itemName = (write: { name: string; label?: string; label_strref?: number }) =>
    write.label ?? (write.label_strref != null ? tlk.resolve(write.label_strref) : undefined) ?? write.name;
  // TLK name of the feat without its <color>/<b> markup, or "#row" when the table has no strref for it.
  const featName = (write: { name: string; label_strref?: number }) =>
    (write.label_strref != null ? tlk.resolve(write.label_strref)?.replace(/<\/?(color|b|i)[^>]*>/gi, '').trim() : undefined) || `#${write.name}`;
  const featState = (value: string | number | null) => Number(value) ? t('gameState.quests.edit.featOwned') : t('gameState.quests.edit.featMissing');
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

  // Blueprint name first; without one the tag, or for the conversation owner (empty tag) the dialogue file.
  const speakerName = (speaker: string, dlg: string, module: string) =>
    speakers.resolve({ speaker, dlg, module }) || speaker || dlg || t('gameState.quests.edit.npc');
  const spokenLabel = (line: { is_player: boolean; speaker: string; dlg: string; module: string; text_strref: number | null; spoken?: SpokenLine }) => {
    const shown = line.spoken ?? line;
    const text = dialogueText(tlk.resolve(shown.text_strref)) ?? t('gameState.quests.detail.noText');
    return shown.is_player
      ? t('gameState.quests.edit.youSaid', { text })
      : t('gameState.quests.edit.npcSaid', { who: speakerName(shown.speaker, line.dlg, line.module), text });
  };
  const conditionsLine = (option: RouteOption) => {
    const conditions = (option.spoken?.conditions ?? []).map(c => {
      const text = describeScript(c, t) ?? formatRawCall(c);
      return c.negate ? t('gameState.quests.edit.not', { condition: text }) : text;
    });
    if (conditions.length === 0) return null;
    return <div style={{ color: T.textMuted }}>{t('gameState.quests.edit.onlyWhen', { conditions: conditions.join(', ') })}</div>;
  };
  const optionLabel = (option: RouteOption) => {
    const line = spokenLabel(option);
    return option.showModule ? `${line} (${resolveModule(option.module)})` : line;
  };
  const optionOutcome = (option: RouteOption) => {
    const rewards: string[] = [];
    for (const write of option.writes) {
      const kind = rewardKind(write);
      if (kind) rewards.push(t(`gameState.quests.edit.${kind}Outcome`, { after: write.after }));
    }
    for (const write of option.writes) {
      if (isItemWrite(write)) rewards.push(t('gameState.quests.edit.itemOutcome', { item: itemName(write), quantity: write.after }));
    }
    for (const write of option.writes) {
      if (isFeatWrite(write)) rewards.push(t('gameState.quests.edit.featOutcome', { feat: featName(write) }));
    }
    rewards.push(...option.effects.filter(f => effectBucket(f) === 'character').map(f => describeScript(f, t) ?? formatRawCall(f)));
    for (const write of option.writes) {
      const companion = influenceCompanion(write.name);
      if (companion) rewards.push(t('gameState.quests.edit.influenceOutcome', { companion, after: write.after }));
    }
    const count = option.writes.filter(write => !rewardKind(write) && !isItemWrite(write) && !isFeatWrite(write) && !influenceCompanion(write.name)).length;
    if (rewards.length === 0) return count === 0 ? t('gameState.quests.edit.outcome.none') : t('gameState.quests.edit.outcome.flagsOnly', { count });
    return rewards.join(', ') + (count > 0 ? t('gameState.quests.edit.outcome.andFlags', { count }) : '');
  };
  // Columns: label | before | arrow | after | delta | note. Numbers right-aligned so the arrows stack.
  const changeRow = (key: string, label: string, before: string | number | null, after: string | number, options: { delta?: boolean; note?: string } = {}) => {
    const delta = options.delta && typeof before === 'number' && typeof after === 'number' ? signedDelta(before, after) : '';
    return <Fragment key={key}>
      <span style={{ color: T.text }}>{label}</span>
      <span className="t-mono" style={{ color: before == null ? T.textMuted : T.text, textAlign: 'right' }}>{before == null ? t('gameState.quests.edit.unset') : before}</span>
      <span className="t-mono" style={{ color: T.textMuted }}>→</span>
      <span className="t-mono" style={{ color: T.text, textAlign: 'right' }}>{after}</span>
      <span className="t-mono" style={{ color: T.textMuted, textAlign: 'right' }}>{delta}</span>
      <span style={{ color: T.textMuted }}>{options.note ?? ''}</span>
    </Fragment>;
  };
  const rowGap = (key: string) => <div key={key} style={{ gridColumn: '1 / -1', height: 4 }} />;
  const section = (title: string, body: ReactNode) => <div style={{ borderTop: `1px solid ${T.borderLight}`, padding: '10px 0' }}>
    <div className="t-bold" style={{ color: T.textMuted, marginBottom: 6 }}>{title}</div>
    {body}
  </div>;
  const moduleLabel = (module: string | null) => module ? resolveModule(module) : t('gameState.quests.edit.campaign');
  const entryText = (id: number) => journalText(quest.category.entries.find(e => e.id === id)?.text);
  const clip = (text: string | undefined) => text && text.length > ENTRY_TEXT_LIMIT ? `${text.slice(0, ENTRY_TEXT_LIMIT)}…` : text;

  const effects = preview ? splitEffects(preview) : null;
  // What the editor cannot apply, as console lines; the journal is written directly, other quests are set
  // separately, and engine calls decoded out of a wrapper script are covered by the wrapper's own line.
  const consoleLines = preview ? [...new Set(preview.effects
    .filter(e => !isJournalConnector(e.action) && e.reason !== 'journal_action' && e.reason !== 'follow_up_journal'
      && !(e.action.derived_from && e.action.kind === 'custom'))
    .map(e => consoleCommand(e.action)))] : [];
  const targetEntry = quest.category.entries.find(e => e.id === target);
  // Entries are ids, not a state machine; a higher id is the only usable "later" signal.
  const nextSteps = (preview?.next_steps ?? []).filter(step => step.state > target && step.state !== preview?.before_state);

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
            {entryText(target) && <>: {entryText(target)}</>}
          </span>
        </div>
      </div>

      {section(t('gameState.quests.edit.route'), <>
        {options == null && <Spinner size={16} />}
        {options != null && options.length === 0 && <p style={{ margin: 0, color: T.textMuted }}>{t('gameState.quests.edit.noRoutes')}</p>}
        {options != null && options.length === 1 && route === options[0].route && <div style={{ color: T.text }}>
          <p style={{ margin: 0 }}>{optionLabel(options[0])}</p>
          {conditionsLine(options[0])}
        </div>}
        {options != null && options.length > 1 && <>
          <p style={{ margin: '0 0 6px', color: T.textMuted }}>{t('gameState.quests.edit.pickLine')}</p>
          <RadioGroup selectedValue={route ?? -1} onChange={e => setRoute(Number(e.currentTarget.value))} disabled={busy}>
            {options.map(option => <Radio key={option.route} value={option.route}>
              <span>{optionLabel(option)}</span>
              {conditionsLine(option)}
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
      </>)}

      {loading && <Spinner size={24} />}
      {failed && <Button onClick={() => setRefresh(r => r + 1)}>{t('gameState.quests.edit.refresh')}</Button>}
      {preview && effects && <>
        {section(t('gameState.quests.edit.changes'), <>
          <div style={{ display: 'grid', gridTemplateColumns: 'auto auto auto auto auto 1fr', columnGap: 10, rowGap: 4, alignItems: 'baseline' }}>
            {rewardWrites.map((write, i) => changeRow(`reward-${i}`, t(`gameState.quests.edit.${rewardKind(write.key)}Applied`), write.before, write.after, { delta: true }))}
            {itemWrites.map((write, i) => changeRow(`item-${i}`, itemName({ ...write, name: write.key.name }), write.before, write.after, { delta: true }))}
            {influenceWrites.map((write, i) => changeRow(`influence-${i}`, t('gameState.quests.edit.influenceApplied', { companion: influenceCompanion(write.key.name) }), write.before, write.after, { delta: true }))}
            {featWrites.map((write, i) => changeRow(`feat-${i}`, t('gameState.quests.edit.featApplied', { feat: featName({ ...write, name: write.key.name }) }), featState(write.before), featState(write.after), write.derived_from ? { note: t('gameState.quests.edit.derivedFrom', { script: write.derived_from }) } : {}))}
            {effects.character.map((item, i) => <Fragment key={`effect-${i}`}>
              <span style={{ color: T.text }}>{item}</span>
              <span style={{ color: T.textMuted, gridColumn: '2 / 6' }}>{t('gameState.quests.edit.notApplied')}</span>
              <span style={{ color: T.textMuted }}>{t('gameState.quests.edit.rewardsNote')}</span>
            </Fragment>)}
            {(rewardWrites.length + itemWrites.length + influenceWrites.length + featWrites.length + effects.character.length) > 0 && rowGap('gap')}
            {changeRow('journal', t('gameState.quests.edit.journal'), preview.before_state, target, { note: quest.tag })}
            {flagWrites.map((write, index) => {
              const note = [write.key.value_type === PC_LOCAL_TYPE ? t('gameState.quests.edit.pcLocal') : moduleLabel(write.key.module)];
              if (write.derived_from) note.push(t('gameState.quests.edit.derivedFrom', { script: write.derived_from }));
              if (write.line != null) note.push(t('gameState.quests.edit.laterInConversation'));
              return changeRow(`flag-${index}`, write.key.name, write.before, write.after, { note: note.join(' · ') });
            })}
          </div>
          {preview.follow_stop && preview.follow_stop.reason !== 'end' && <p style={{ margin: '6px 0 0', color: T.textMuted }}>
            {t(`gameState.quests.edit.stop.${preview.follow_stop.reason}`, { choices: preview.follow_stop.choices })}
          </p>}
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

        {effects.world.length > 0 && section(t('gameState.quests.edit.world'), <>
          <p style={{ margin: '0 0 6px', color: T.textMuted }}>{t('gameState.quests.edit.worldNote')}</p>
          <Button small minimal onClick={() => setShowWorld(!showWorld)} style={{ marginLeft: -6 }}>
            {showWorld ? t('gameState.quests.edit.worldHide') : t('gameState.quests.edit.worldShow', { count: effects.world.length })}
          </Button>
          {showWorld && <ul style={{ margin: '4px 0 0', paddingLeft: 18, color: T.text }}>
            {effects.world.map((item, i) => <li key={i}>
              {item.text}
              {item.major && <Tag minimal intent="warning" style={{ marginLeft: 6 }}>{t('gameState.quests.edit.worldMajor')}</Tag>}
            </li>)}
          </ul>}
        </>)}

        {consoleLines.length > 0 && section(t('gameState.quests.edit.console'), <>
          <p style={{ margin: '0 0 6px', color: T.textMuted }}>{t('gameState.quests.edit.consoleNote')}</p>
          <pre className="t-mono" style={{ margin: 0, padding: '6px 10px', background: `${T.borderLight}55`, borderRadius: 4, whiteSpace: 'pre-wrap', color: T.text }}>
            {['DebugMode 1', ...consoleLines].join('\n')}
          </pre>
        </>)}

        {section(t('gameState.quests.edit.next'), targetEntry?.final
          ? <p style={{ margin: 0, color: T.textMuted }}>{t('gameState.quests.edit.nextEnds')}</p>
          : nextSteps.length === 0
            ? <p style={{ margin: 0, color: T.textMuted }}>{t('gameState.quests.edit.nextNone')}</p>
            : nextSteps.map(step => <div key={step.state} style={{ marginBottom: 8 }}>
            <div style={{ color: T.text }}>
              <span className="t-semibold">{t('gameState.quests.detail.entry', { n: step.state })}</span>
              {entryText(step.state) && <span style={{ color: T.textMuted }}>: {clip(entryText(step.state))}</span>}
            </div>
            <ul style={{ margin: '2px 0 0', paddingLeft: 18 }}>
              {step.lines.map(line => <li key={line.route} style={{ color: T.text }}>
                {nextLineLabel(line, tlk, t)}
                <span style={{ color: line.gate === 'open' ? T.text : T.textMuted }}> · {t(`gameState.quests.edit.gate.${line.gate}`)}</span>
              </li>)}
            </ul>
          </div>))}

      </>}
    </div>
  </ParchmentDialog>;

  function nextLineLabel(line: NextLine, resolver: TlkResolver, translate: typeof t) {
    const shown = line.spoken ?? line;
    const text = dialogueText(resolver.resolve(shown.text_strref));
    if (!text) return translate('gameState.quests.edit.nextNoText', { dlg: line.module });
    return shown.is_player
      ? translate('gameState.quests.edit.nextSay', { text })
      : translate('gameState.quests.edit.nextTalk', { who: speakerName(shown.speaker, line.dlg, line.module), text });
  }

  /** Everything the game does on the route besides the flag writes, in plain words, split by who it touches. */
  function splitEffects(p: QuestPreview) {
    const character: string[] = [];
    const world: { text: string; major: boolean }[] = [];
    const push = (bucket: 'character' | 'world', text: string, major = false) => {
      if (bucket === 'character') {
        if (!character.includes(text)) character.push(text);
      } else if (!world.some(w => w.text === text)) {
        world.push({ text, major });
      }
    };
    for (const effect of p.effects) {
      const f: ConvoFunctor = effect.action;
      if (isJournalConnector(f) && effect.reason === 'journal_action') continue;
      if (effect.reason !== 'decoded_script') push(effectBucket(f), describeScript(f, t) ?? formatRawCall(f), isMajorWorldScript(f));
      const decoded = f.decoded;
      if (!decoded) continue;
      for (const call of decoded.calls) push(engineCallBucket(call), describeEngineCall(call, t), isMajorEngineCall(call));
      for (const w of decoded.conditional_writes) push('world', t('gameState.quests.edit.decodedConditional', { name: w.name, value: w.value }));
      for (const call of decoded.unresolved) push('world', t('gameState.quests.edit.decodedUnresolved', { call }));
    }
    return { character, world };
  }
}

interface RouteOption extends RouteSummary {
  /** Several offered lines share a speaker across modules: show which module. */
  showModule: boolean;
}

/** Collapse lines with identical outcomes, current module first, cheat dialogues last resort. */
export function groupRoutes(summaries: RouteSummary[], currentModule: string | null): RouteOption[] {
  const playable = summaries.filter(s => !s.debug);
  const pool = playable.length > 0 ? playable : summaries;
  const spoken = (s: RouteSummary) => s.spoken ?? s;
  // Current module first, then lines the player chose over silent NPC branches that
  // reach the same outcome, then authored order.
  const ordered = [...pool].sort((a, b) =>
    Number(b.module === currentModule) - Number(a.module === currentModule)
    || Number(spoken(b).is_player) - Number(spoken(a).is_player)
    || a.route - b.route);
  // The same NPC repeats a conversation in every act's city module; when the outcome
  // is identical the module is noise, so keep one line (the current module's if any).
  const seen = new Map<string, RouteSummary>();
  for (const s of ordered) {
    const key = `${s.is_player}|${s.signature}`;
    if (!seen.has(key)) seen.set(key, s);
  }
  // The same spoken line in the module the player is in wins over its copies elsewhere,
  // even when those copies set different flags: only one of them can be "what you played".
  const spokenKey = (s: RouteSummary) => `${spoken(s).speaker}|${spoken(s).is_player}|${spoken(s).text_strref}`;
  const inCurrent = new Set([...seen.values()].filter(s => s.module === currentModule).map(spokenKey));
  const unique = [...seen.values()].filter(s => s.module === currentModule || !inCurrent.has(spokenKey(s)));
  const modulesPerSpeaker = new Map<string, Set<string>>();
  for (const s of unique) {
    const set = modulesPerSpeaker.get(s.speaker) ?? new Set<string>();
    set.add(s.module);
    modulesPerSpeaker.set(s.speaker, set);
  }
  return unique.map(s => ({ ...s, showModule: (modulesPerSpeaker.get(s.speaker)?.size ?? 0) > 1 }));
}
