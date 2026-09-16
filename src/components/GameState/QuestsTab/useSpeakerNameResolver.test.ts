import { describe, expect, it } from 'vitest';
import { buildSpeakerIndex, lookupSpeaker } from './useSpeakerNameResolver';
import type { AggregatedModule, SpeakerName } from './types';

const name = (tag: string, first: string, conversation = ''): SpeakerName =>
  ({ tag, first, last: '', first_strref: null, last_strref: null, conversation });
const mod = (name: string, speaker_names: SpeakerName[]): AggregatedModule =>
  ({ name, display_name: name, resolved_path: '', resolution_kind: 'install', is_current: false, journal_category_tags: [], speaker_names });

describe('lookupSpeaker', () => {
  const index = buildSpeakerIndex([
    mod('1200', [name('khelgar', 'Alex Foxman')]),
    mod('1300', [name('khelgar', 'Khelgar'), name('hlam', 'Hlam', '10_hlam')]),
    mod('1600', [name('khelgar', 'Khelgar')]),
    mod('1700', [name('hlam', 'Brother Hlam', '10_hlam')]),
  ]);

  it('prefers the blueprint of the line module', () => {
    expect(lookupSpeaker(index, { speaker: 'khelgar', dlg: 'x', module: '1200' })?.first).toBe('Alex Foxman');
  });

  it('falls back to the name most modules agree on', () => {
    expect(lookupSpeaker(index, { speaker: 'KHELGAR', dlg: 'x', module: '1700' })?.first).toBe('Khelgar');
  });

  it('names the conversation owner through the dialogue it owns', () => {
    expect(lookupSpeaker(index, { speaker: '', dlg: '10_hlam', module: '1700' })?.first).toBe('Brother Hlam');
    expect(lookupSpeaker(index, { speaker: '', dlg: '10_hlam', module: '1000' })?.first).toBe('Hlam');
    expect(lookupSpeaker(index, { speaker: '', dlg: 'nobody', module: '1000' })).toBeUndefined();
  });
});
