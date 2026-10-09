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
  ga_xp_award_2da_entry: ['award'],
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
  adjustalignment: ['alignment', 'shift'],
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
  'ga_give_xp', 'ga_give_quest_xp', 'ga_give_partial_quest_xp', 'ga_xp_award_2da_entry', 'ga_give_gold', 'ga_take_gold',
  'ga_give_item', 'ga_take_item', 'ga_destroy_item', 'ga_alignment', 'adjustalignment', 'ga_influence',
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

/** `names`: roster `RosCharName` by lowercased roster name, preferred when the save has one. */
export function companionName(tag: string, names?: Record<string, string>) {
  return names?.[tag.toLowerCase()] || COMPANION_NAMES[tag] || tag.charAt(0).toUpperCase() + tag.slice(1);
}

/** MotB `kinc_companion.GetCompanionTagByCode`: the first two letters of the code, lowercased. */
function motbCompanionTag(code: string) {
  const codes: Record<string, string> = { sa: 'safiya', ga: 'gann', ok: 'okku', do: 'dove', on: 'oneofmany' };
  return codes[code.slice(0, 2).toLowerCase()] ?? code;
}

/** The companion behind an influence global (`00_nInfluencekhelgar` -> "Khelgar"), null for other variables. */
export function influenceCompanion(variable: string): string | null {
  if (!variable.startsWith(INFLUENCE_PREFIX)) return null;
  return companionName(variable.slice(INFLUENCE_PREFIX.length));
}

/** `VariableKey.value_type` of writes that land on the player character (gold, XP) rather than a story flag. */
export const CHARACTER_TYPE = 'character';
/** `VariableKey.value_type` of an item handed to the player: `name` is the blueprint resref, the value the quantity. */
export const ITEM_TYPE = 'item';

export function isItemWrite(key: { value_type: string }) {
  return key.value_type === ITEM_TYPE;
}

/** `VariableKey.value_type` of items taken out of the player's backpack: `name` is the item tag, the value the units left. */
export const ITEM_TAKE_TYPE = 'item_take';

export function isItemTakeWrite(key: { value_type: string }) {
  return key.value_type === ITEM_TAKE_TYPE;
}

/** A story flag: every write the dialog does not show as a reward, item, feat, party, companion or area change. */
export function isPlainFlag(key: { name: string; value_type: string }) {
  return !rewardKind(key) && !isItemWrite(key) && !isItemTakeWrite(key) && !isFeatWrite(key) && !isPartyWrite(key) && !isCompanionWrite(key) && !isAreaWrite(key) && !influenceCompanion(key.name);
}

/** `VariableKey.value_type` of a door's lock in the saved area: `name` is the door tag, the value 1 (locked) or 0. */
export const DOOR_LOCK_TYPE = 'door_lock';
/** `VariableKey.value_type` of a map pin in the saved area: `name` is the waypoint tag, the value 1 (shown) or 0. */
export const MAP_NOTE_TYPE = 'map_note';

export function isAreaWrite(key: { value_type: string }) {
  return key.value_type === DOOR_LOCK_TYPE || key.value_type === MAP_NOTE_TYPE;
}

/** `VariableKey.value_type` of a feat on the player: `name` is the feat.2da row, the value 1 (owned) or 0. */
export const FEAT_TYPE = 'feat';

export function isFeatWrite(key: { value_type: string }) {
  return key.value_type === FEAT_TYPE;
}

/** `VariableKey.value_type` of a companion's party membership: `name` is the roster name, the value 1 (in party) or 0 (benched). */
export const PARTY_TYPE = 'party';
/** `VariableKey.value_type` of a companion's party-screen lock: `name` is the roster name, the value 1 (selectable) or 0 (locked). */
export const SELECTABLE_TYPE = 'selectable';

export function isPartyWrite(key: { value_type: string }) {
  return key.value_type === PARTY_TYPE || key.value_type === SELECTABLE_TYPE;
}

/** `VariableKey.value_type` of an alignment field on a companion: `name` is `<roster name>/<field>`. */
export const COMPANION_CHARACTER_TYPE = 'companion_character';
/** `VariableKey.value_type` of a feat on a companion: `name` is `<roster name>/<feat.2da row>`, the value 1 (owned) or 0. */
export const COMPANION_FEAT_TYPE = 'companion_feat';
/** `VariableKey.value_type` of an item handed to a companion: `name` is `<roster name>/<blueprint resref>`, the value the quantity. */
export const COMPANION_ITEM_TYPE = 'companion_item';
/** `VariableKey.value_type` of items taken from a companion's backpack: `name` is `<roster name>/<item tag>`, the value the units left. */
export const COMPANION_ITEM_TAKE_TYPE = 'companion_item_take';

export function isCompanionWrite(key: { value_type: string }) {
  return [COMPANION_CHARACTER_TYPE, COMPANION_FEAT_TYPE, COMPANION_ITEM_TYPE, COMPANION_ITEM_TAKE_TYPE].includes(key.value_type);
}

export function isCompanionItemWrite(key: { value_type: string }) {
  return key.value_type === COMPANION_ITEM_TYPE || key.value_type === COMPANION_ITEM_TAKE_TYPE;
}

/** Write kinds that move a quantity: two quests writing one both apply, so they never conflict. */
export function accumulates(key: { value_type: string }) {
  return [CHARACTER_TYPE, COMPANION_CHARACTER_TYPE, ITEM_TYPE, ITEM_TAKE_TYPE, COMPANION_ITEM_TYPE, COMPANION_ITEM_TAKE_TYPE].includes(key.value_type);
}

/** The companion and the field or feat row of a companion write. */
export function companionKey(name: string, names?: Record<string, string>): { companion: string; field: string } {
  const slash = name.indexOf('/');
  if (slash < 0) return { companion: companionName(name, names), field: '' };
  return { companion: companionName(name.slice(0, slash), names), field: name.slice(slash + 1) };
}

export type RewardKind = 'gold' | 'xp' | 'goodEvil' | 'lawChaos';

/** Which player field a write changes, null for story flags and influence. */
export function rewardKind(key: { name: string; value_type: string }): RewardKind | null {
  if (key.value_type !== CHARACTER_TYPE) return null;
  if (key.name === 'Gold') return 'gold';
  if (key.name === 'Experience') return 'xp';
  if (key.name === 'GoodEvil') return 'goodEvil';
  if (key.name === 'LawfulChaotic') return 'lawChaos';
  return null;
}

/** `VariableKey.value_type` of an integer local on the player character (e.g. the share of quest XP already paid). */
export const PC_LOCAL_TYPE = 'pc_local';
/** PC local the engine keeps a quest's journal entry in, followed by the quest tag. */
export const JOURNAL_ENTRY_PREFIX = 'NW_JOURNAL_ENTRY';

/** World actions a player would notice missing: party, faction, deaths, area and module changes. */
const MAJOR_WORLD_SCRIPTS = new Set([
  'ga_destroy', 'ga_death', 'ga_attack', 'ga_faction_join', 'ga_jump', 'ga_jump_players', 'ga_load_mod', 'ga_end_game',
  'ga_roster_party_add', 'ga_party_add', 'ga_roster_party_remove', 'ga_roster_selectable', 'ga_roster_add_blueprint',
  'ga_henchman_add', 'ga_henchman_remove', 'ga_start_convo',
]);
const MAJOR_ENGINE_CALLS = new Set([
  'destroyobject', 'changetostandardfaction', 'changefaction', 'addhenchman', 'removehenchman',
  'actionstartconversation', 'jumptoobject', 'jumptolocation', 'actionjumptoobject', 'actionjumptolocation',
]);

export function isMajorWorldScript(f: ConvoFunctor) {
  return MAJOR_WORLD_SCRIPTS.has(f.script.toLowerCase());
}

export function isMajorEngineCall(call: string) {
  return MAJOR_ENGINE_CALLS.has(engineCallName(call));
}

/**
 * Actions that only play out the moment (cutscene, sound, movement, a fight, travel) and leave
 * no lasting state behind that later dialogue depends on.
 */
const MOMENT_SCRIPTS = new Set([
  'ga_fade', 'ga_fade_to_black', 'ga_fade_from_black', 'ga_play_custom_animation', 'ga_play_animation', 'ga_move',
  'ga_face_target', 'ga_play_sound', 'ga_sound_object_play', 'ga_sound_object_stop', 'ga_start_convo',
  'ga_conversation_self', 'ga_party_freeze', 'ga_party_unfreeze', 'ga_heal_pc', 'ga_heal_party', 'ga_open_store',
  'ga_time_advance', 'ga_attack', 'ga_jump', 'ga_jump_players', 'ga_jump_party_in_formation', 'ga_area_transition',
  'ga_travel', 'ga_load_mod', 'ga_start_mod', 'ga_end_game',
]);
const MOMENT_SCRIPT_PREFIXES = ['ga_music_', 'ga_camera_'];
const MOMENT_ENGINE_CALLS = new Set([
  'fadetoblack', 'fadefromblack', 'playcustomanimation', 'actionplayanimation', 'playanimation', 'actionmovetoobject',
  'actionmovetolocation', 'actionforcemovetoobject', 'actionforcemovetolocation', 'setfacing', 'setfacingpoint',
  'actionspeakstring', 'speakstring', 'playsound', 'actionstartconversation', 'actionattack', 'jumptoobject',
  'jumptolocation', 'actionjumptoobject', 'actionjumptolocation', 'startnewmodule', 'loadnewmodule',
]);

/** Whether skipping this console line only skips a cutscene moment, so running it is usually not needed. */
export function isMomentAction(f: ConvoFunctor) {
  const script = f.script.toLowerCase();
  if (MOMENT_SCRIPTS.has(script) || MOMENT_SCRIPT_PREFIXES.some(prefix => script.startsWith(prefix))) return true;
  const decoded = f.decoded;
  if (!decoded || decoded.calls.length === 0 || decoded.conditional_writes.length > 0 || decoded.unresolved.length > 0) return false;
  return decoded.calls.every(call => MOMENT_ENGINE_CALLS.has(engineCallName(call)));
}

/** The in-game console line that runs a dialogue action on the player: `rs script(a,b)`, no quotes, no spaces. */
export function consoleCommand(f: ConvoFunctor) {
  return `rs ${f.script}(${f.params.map(p => String(p)).join(',')})`;
}

/** "+500" / "-120" for a reward delta. */
export function signedDelta(before: string | number | null, after: string | number) {
  const delta = Number(after) - Number(before ?? 0);
  return delta >= 0 ? `+${delta}` : String(delta);
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

const ALIGNMENT_CONSTANTS: Record<string, number> = {
  ALIGNMENT_ALL: 0, ALIGNMENT_NEUTRAL: 1, ALIGNMENT_LAWFUL: 2, ALIGNMENT_CHAOTIC: 3, ALIGNMENT_GOOD: 4, ALIGNMENT_EVIL: 5,
};

/** NWScript object variables are named `oKhelgar`, `oPC`: the name without its type prefix. */
function objectVariableName(raw: string) {
  const m = /^o([A-Z]\w*)$/.exec(raw.trim());
  return m ? m[1] : raw.trim();
}

/**
 * Plain-language sentence for a decoded engine call: known calls without arguments get their
 * generic text, `AdjustAlignment(target, ALIGNMENT_X, n)` on someone other than the player is
 * spelled out, everything else is shown raw.
 */
export function describeEngineCall(call: string, t: (key: string, values?: Record<string, unknown>) => string): string {
  const name = engineCallName(call);
  if (!CHARACTER_ENGINE_CALLS.has(name)) return call;
  if (!call.includes('(')) return t(`gameState.quests.engineCalls.${name}`);
  if (name === 'adjustalignment') {
    const args = call.slice(call.indexOf('(') + 1, call.lastIndexOf(')')).split(',').map(a => a.trim());
    const alignment = ALIGNMENT_CONSTANTS[args[1]] ?? Number(args[1]);
    const shift = Number(args[2]);
    if (args.length === 3 && Number.isInteger(alignment) && Number.isInteger(shift)) {
      return t('gameState.quests.engineCalls.adjustalignment_target', { target: objectVariableName(args[0]), alignment, shift });
    }
  }
  return call;
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

/** Scripts whose empty target is the PC speaker rather than the conversation owner. */
const PLAYER_DEFAULT_TARGET = new Set(['ga_give_feat', 'ga_remove_feat']);

function formatTarget(raw: string, script: string, t: (key: string) => string) {
  if (raw === '' && PLAYER_DEFAULT_TARGET.has(script)) return t('gameState.quests.scriptTarget.player');
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
    else if (name === 'target') values[name] = formatTarget(text, script, t);
    else if (name === 'companion' && script === 'ga_influence') values[name] = companionName(OC_COMPANION_TAGS[Number(text)] ?? text);
    else if (name === 'companion' && script.startsWith('ka_influence')) values[name] = companionName(motbCompanionTag(text));
    else values[name] = text;
  });
  return t(`gameState.quests.scripts.${script}`, values);
}
