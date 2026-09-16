import type { ConvoFunctor } from './types';

/**
 * Positional parameter names for the stock NWN2 `ga_*` / `gc_*` conversation
 * scripts (signatures taken from the game's scripts.zip). Each entry maps to
 * an i18n message under `gameState.quests.scripts.<script>` that interpolates
 * these names. Scripts outside this table are shown as raw calls.
 */
const STOCK_SCRIPTS: Record<string, string[]> = {
  ga_give_quest_xp: ['quest'],
  ga_give_partial_quest_xp: ['quest', 'percent'],
  ga_give_xp: ['xp'],
  ga_global_int: ['name', 'change'],
  ga_global_string: ['name', 'change'],
  ga_global_float: ['name', 'change'],
  ga_gint_max: ['name', 'change'],
  ga_gint_min: ['name', 'change'],
  ga_local_int: ['name', 'change', 'target'],
  ga_local_string: ['name', 'change', 'target'],
  ga_local_float: ['name', 'change', 'target'],
  ga_journal: ['quest', 'entry'],
  ga_mapnote: ['tag', 'active'],
  ga_give_gold: ['amount'],
  ga_take_gold: ['amount'],
  ga_give_item: ['item', 'quantity'],
  ga_take_item: ['item', 'quantity'],
  ga_destroy: ['tag'],
  ga_destroy_item: ['item'],
  ga_faction_join: ['target', 'faction'],
  ga_jump_players: ['destination'],
  ga_jump: ['destination', 'target'],
  ga_area_transition: ['destination'],
  ga_travel: ['destination', 'target'],
  ga_roster_party_add: ['companion'],
  ga_party_add: ['companion'],
  ga_roster_party_remove: ['companion'],
  ga_roster_selectable: ['companion', 'selectable'],
  ga_roster_add_blueprint: ['companion'],
  ga_henchman_add: ['target'],
  ga_henchman_remove: ['target'],
  ga_open_store: ['store'],
  ga_start_convo: ['target', 'conversation'],
  ga_alignment: ['amount', 'axis'],
  ga_influence: ['companion', 'change'],
  ka_influence_magnitude: ['companion', 'magnitude'],
  ka_influence_set: ['companion', 'value'],
  ga_heal_pc: ['percent'],
  ga_heal_party: ['percent'],
  ga_give_feat: ['target', 'feat'],
  ga_remove_feat: ['target', 'feat'],
  ga_setplotflag: ['target', 'flag'],
  ga_setimmortal: ['target', 'flag'],
  ga_door_open: ['tag'],
  ga_door_close: ['tag'],
  ga_lock: ['tag', 'flag'],
  ga_death: ['tag'],
  ga_attack: ['attacker'],
  ga_play_sound: ['sound'],
  ga_fade_to_black: [],
  ga_fade_from_black: [],
  ga_time_advance: ['hours', 'minutes'],
  ga_load_mod: ['module'],
  ga_end_game: [],
  gc_global_int: ['name', 'check'],
  gc_global_string: ['name', 'check'],
  gc_global_float: ['name', 'check'],
  gc_local_int: ['name', 'check', 'target'],
  gc_local_string: ['name', 'check', 'target'],
  gc_local_float: ['name', 'check', 'target'],
  gc_journal_entry: ['quest', 'check'],
  gc_check_gold: ['gold'],
  gc_check_item: ['item'],
  gc_item_count: ['item', 'check'],
  gc_check_level: ['level'],
  gc_check_race: ['target', 'race'],
  gc_check_race_pc: ['race'],
  gc_class_level: ['class', 'check', 'target'],
  gc_is_in_party: ['companion'],
  gc_is_male: [],
  gc_is_female: [],
  gc_align_good: [],
  gc_align_evil: [],
  gc_align_lawful: [],
  gc_align_chaotic: [],
  gc_dead: ['tag'],
  gc_has_feat: ['target', 'feat'],
  gc_skill_rank: ['skill', 'rank'],
  gc_skill_dc: ['skill', 'dc'],
  gc_roster_available: ['companion'],
  gc_roster_selectable: ['companion'],
  gc_henchman: ['target'],
  gc_rand_1of: ['max'],
  gc_true: [],
  gc_false: [],
  gc_equipped: ['item', 'target'],
  gc_check_stats: ['stat', 'value'],
  gc_background: ['background'],
  gc_deity: ['deity'],
  gc_num_comps: ['check'],
  gc_is_night: [],
};

/** Stock scripts whose effect lands on the player or the party rather than the world. */
const CHARACTER_SCRIPTS = new Set([
  'ga_give_xp', 'ga_give_quest_xp', 'ga_give_partial_quest_xp', 'ga_give_gold', 'ga_take_gold',
  'ga_give_item', 'ga_take_item', 'ga_destroy_item', 'ga_alignment', 'ga_influence',
  'ka_influence_magnitude', 'ka_influence_set',
  'ga_give_feat', 'ga_remove_feat', 'ga_heal_pc', 'ga_heal_party', 'ga_roster_party_add',
  'ga_party_add', 'ga_roster_party_remove', 'ga_roster_selectable', 'ga_henchman_add', 'ga_henchman_remove',
]);

/** Engine calls decoded out of module wrapper scripts that touch the player or party. */
const CHARACTER_ENGINE_CALLS = new Set([
  'adjustalignment', 'givegoldtocreature', 'takegoldfromcreature', 'givexptocreature',
  'createitemonobject', 'adjustinfluence', 'addhenchman', 'removehenchman',
]);

export type EffectBucket = 'character' | 'world';

const INFLUENCE_PREFIX = '00_nInfluence';

/** `ga_influence` companion numbers (OC `kinc_companion.GetCompanionTagByNumber`). */
const OC_COMPANION_TAGS: Record<number, string> = {
  1: 'khelgar', 2: 'neeshka', 3: 'elanee', 4: 'qara', 5: 'sand', 6: 'grobnar', 7: 'casavir',
  8: 'bishop', 9: 'shandra', 10: 'construct', 11: 'zhjaeve', 12: 'ammon_jerro', 13: 'npc_bevil',
};

const COMPANION_NAMES: Record<string, string> = {
  ammon_jerro: 'Ammon Jerro', npc_bevil: 'Bevil', dove: 'Kaelyn the Dove', oneofmany: 'One of Many',
};

function companionName(tag: string) {
  return COMPANION_NAMES[tag] ?? tag.charAt(0).toUpperCase() + tag.slice(1);
}

/** MotB `kinc_companion.GetCompanionTagByCode`: the first letter of the code, "one..." is One of Many. */
function motbCompanionTag(code: string) {
  const lower = code.trim().toLowerCase();
  if (lower.startsWith('one')) return 'oneofmany';
  return { s: 'safiya', g: 'gann', o: 'okku', d: 'dove' }[lower.charAt(0)] ?? code;
}

/** The companion behind an influence global (`00_nInfluencekhelgar` -> "Khelgar"), null for other variables. */
export function influenceCompanion(variable: string): string | null {
  if (!variable.startsWith(INFLUENCE_PREFIX)) return null;
  return companionName(variable.slice(INFLUENCE_PREFIX.length));
}

/** Whether a dialogue action changes the player/party or the game world. */
export function effectBucket(f: ConvoFunctor): EffectBucket {
  return CHARACTER_SCRIPTS.has(f.script.toLowerCase()) ? 'character' : 'world';
}

/** Same split for a raw engine call string from a decoded script (`SetLocked` or `10_x("a", 1)`). */
export function engineCallBucket(call: string): EffectBucket {
  return CHARACTER_ENGINE_CALLS.has(engineCallName(call)) ? 'character' : 'world';
}

function engineCallName(call: string) {
  return call.split('(')[0].trim().toLowerCase();
}

/** Plain-language sentence for a decoded engine call; the raw call when it has arguments or is unknown. */
export function describeEngineCall(call: string, t: (key: string) => string): string {
  const name = engineCallName(call);
  if (call.includes('(') || !CHARACTER_ENGINE_CALLS.has(name)) return call;
  return t(`gameState.quests.engineCalls.${name}`);
}

/** The engine-internal functor the graph builder synthesizes for the journal write itself. */
export function isJournalConnector(f: ConvoFunctor) {
  return f.kind === 'journal' || f.script === '__connector_quest_tuple__';
}

function formatChange(raw: string) {
  if (raw === '++') return '+1';
  if (raw === '--') return '-1';
  if (/^[+\-<>!=]/.test(raw)) return raw;
  return `= ${raw}`;
}

function formatTarget(raw: string, t: (key: string) => string) {
  if (raw === '' || raw === '$OWNER') return t('gameState.quests.scriptTarget.owner');
  if (raw === '$PC_SPEAKER') return t('gameState.quests.scriptTarget.player');
  return raw;
}

export function formatRawCall(f: ConvoFunctor) {
  return `${f.script}(${f.params.map(p => (typeof p === 'string' ? JSON.stringify(p) : String(p))).join(', ')})`;
}

/** Plain-language sentence for a stock script, or null when the script is not in the table. */
export function describeScript(
  f: ConvoFunctor,
  t: (key: string, values?: Record<string, unknown>) => string,
): string | null {
  const script = f.script.toLowerCase();
  const names = STOCK_SCRIPTS[script];
  if (!names) return null;
  const values: Record<string, string> = {};
  names.forEach((name, i) => {
    const raw = f.params[i];
    const text = raw == null ? '' : String(raw);
    if (name === 'change' || name === 'check') values[name] = formatChange(text);
    else if (name === 'target') values[name] = formatTarget(text, t);
    else if (name === 'companion' && script === 'ga_influence') values[name] = companionName(OC_COMPANION_TAGS[Number(text)] ?? text);
    else if (name === 'companion' && script.startsWith('ka_influence')) values[name] = companionName(motbCompanionTag(text));
    else values[name] = text;
  });
  return t(`gameState.quests.scripts.${script}`, values);
}
