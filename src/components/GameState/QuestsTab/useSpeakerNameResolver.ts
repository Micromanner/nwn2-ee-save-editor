import { useCallback, useMemo } from 'react';
import type { TlkResolver } from '@/hooks/useTlkResolver';
import type { AggregatedModule, SpeakerName } from './types';

export interface SpeakerRef { speaker: string; dlg: string; module: string }

interface NameMaps { tags: Map<string, SpeakerName>; conversations: Map<string, SpeakerName> }
export interface SpeakerIndex { byModule: Map<string, NameMaps>; all: NameMaps }

export function buildSpeakerIndex(modules: AggregatedModule[]): SpeakerIndex {
  const byModule = new Map<string, NameMaps>();
  const tagVotes = new Map<string, SpeakerName[]>();
  const conversationVotes = new Map<string, SpeakerName[]>();
  for (const mod of modules) {
    const tags = new Map<string, SpeakerName>();
    const conversations = new Map<string, SpeakerName>();
    for (const name of mod.speaker_names) {
      tags.set(name.tag, name);
      tagVotes.set(name.tag, [...(tagVotes.get(name.tag) ?? []), name]);
      if (name.conversation) {
        if (!conversations.has(name.conversation)) conversations.set(name.conversation, name);
        conversationVotes.set(name.conversation, [...(conversationVotes.get(name.conversation) ?? []), name]);
      }
    }
    byModule.set(mod.name, { tags, conversations });
  }
  return { byModule, all: { tags: majority(tagVotes), conversations: majority(conversationVotes) } };
}

/**
 * A line's speaker tag is looked up in its own module first, then anywhere in the
 * campaign; an empty tag is the conversation owner, found through the creature that
 * owns the `.dlg`.
 */
export function lookupSpeaker(index: SpeakerIndex, { speaker, dlg, module }: SpeakerRef): SpeakerName | undefined {
  const local = index.byModule.get(module);
  if (speaker) {
    const tag = speaker.toLowerCase();
    return local?.tags.get(tag) ?? index.all.tags.get(tag);
  }
  const conversation = dlg.toLowerCase();
  return local?.conversations.get(conversation) ?? index.all.conversations.get(conversation);
}

/**
 * Modules disagree on a shared tag now and then (Highcliff ships a `khelgar`
 * placeholder named after a developer): the name most modules use wins, first seen on a tie.
 */
function majority(votes: Map<string, SpeakerName[]>): Map<string, SpeakerName> {
  const result = new Map<string, SpeakerName>();
  for (const [key, names] of votes) {
    const counts = new Map<string, { name: SpeakerName; count: number }>();
    for (const name of names) {
      const id = `${name.first}|${name.last}|${name.first_strref}|${name.last_strref}`;
      const entry = counts.get(id) ?? { name, count: 0 };
      entry.count += 1;
      counts.set(id, entry);
    }
    let best: { name: SpeakerName; count: number } | undefined;
    for (const entry of counts.values()) if (!best || entry.count > best.count) best = entry;
    if (best) result.set(key, best.name);
  }
  return result;
}

/** Names dialogue speakers from the modules' creature blueprints, resolving TLK names through `tlk`. */
export function useSpeakerNameResolver(modules: AggregatedModule[], tlk: TlkResolver) {
  const index = useMemo(() => buildSpeakerIndex(modules), [modules]);

  const strrefs = useCallback((refs: SpeakerRef[]) => refs.flatMap(ref => {
    const name = lookupSpeaker(index, ref);
    return name ? [name.first_strref, name.last_strref] : [];
  }), [index]);

  /** Display name, or `undefined` when no blueprint names the speaker. */
  const resolve = useCallback((ref: SpeakerRef): string | undefined => {
    const name = lookupSpeaker(index, ref);
    if (!name) return undefined;
    const first = name.first || (name.first_strref !== null ? tlk.resolve(name.first_strref) : undefined) || '';
    const last = name.last || (name.last_strref !== null ? tlk.resolve(name.last_strref) : undefined) || '';
    const full = `${first} ${last}`.trim();
    return full || undefined;
  }, [index, tlk]);

  return { resolve, strrefs };
}
