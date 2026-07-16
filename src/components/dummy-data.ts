export const CHARACTER = {
  name: 'Khelgar Ironfist',
  race: 'Shield Dwarf',
  subrace: '',
  gender: 'Male',
  age: 82,
  alignment: 'Chaotic Good',
  alignmentLC: 25,
  alignmentGE: 85,
  deity: 'Tyr',
  background: 'Bully',
  level: 16,
  classes: [
    { name: 'Fighter', level: 4, hitDie: 10, bab: 4, fort: 4, ref: 1, will: 1, skillPoints: 2, type: 'base' as const, maxLevel: 60, primaryAbility: 'STR', isSpellcaster: false },
    { name: 'Monk', level: 12, hitDie: 8, bab: 9, fort: 8, ref: 8, will: 8, skillPoints: 4, type: 'base' as const, maxLevel: 60, primaryAbility: 'WIS', isSpellcaster: false },
  ],
  xp: 136_000,
  xpNext: 153_000,
  gold: 48_523,
  hp: { current: 142, max: 168 },
  ac: 24,
  bab: 14,
  melee: 18,
  ranged: 16,
  initiative: 3,
  speed: 20,
  size: 'Medium',
  saves: { fort: 16, ref: 12, will: 10 },
  biography: 'A stout dwarf from Ironfist Hold who left his clan to find glory in combat. He seeks to prove himself worthy of becoming a monk of the Long Death.',
  domains: [
    { name: 'Strength' },
    { name: 'War' },
  ],
  spellResistance: 26,
  damageResistances: [
    { type: 'Fire', amount: 10 },
    { type: 'Cold', amount: 5 },
  ],
  damageImmunities: ['Poison', 'Disease'],
  totalSkillPoints: 61,
  totalFeats: 22,
  knownSpells: 12,
  campaign: {
    gameAct: 2,
    difficulty: 'Normal',
    lastSaved: 1710000000,
    campaignName: 'Neverwinter Nights 2 OC',
    moduleName: 'Act II - Crossroad Keep',
    location: 'Crossroad Keep - Courtyard',
    playTime: '48h 23m',
    language: 'English',
    questProgress: { completed: 14, active: 4, completionRate: 78 },
  },
};

export const DEITIES = [
  { name: 'Tyr', alignment: 'Lawful Good', portfolio: 'Justice, Law, War', favoredWeapon: 'Longsword', description: 'Tyr is the god of justice and law. Known as the Even-Handed, he is the leader of the Triad alongside Torm and Ilmater.' },
  { name: 'Tempus', alignment: 'Chaotic Neutral', portfolio: 'War, Battle, Warriors', favoredWeapon: 'Battleaxe', description: 'Tempus is the god of war. He is random in his favors, granting victory to one side one day and the other the next.' },
  { name: 'Mystra', alignment: 'Neutral Good', portfolio: 'Magic, Spells, The Weave', favoredWeapon: 'Shuriken', description: 'Mystra is the goddess of magic and the Weave. She provides and tends the Weave, the conduit through which mortal spellcasters channel magical energy.' },
  { name: 'Lathander', alignment: 'Neutral Good', portfolio: 'Spring, Dawn, Birth, Renewal', favoredWeapon: 'Mace', description: 'Lathander is the god of dawn, renewal, and vitality. He encourages new beginnings and the pursuit of perfection.' },
  { name: 'Kelemvor', alignment: 'Lawful Neutral', portfolio: 'Death, The Dead', favoredWeapon: 'Bastard Sword', description: 'Kelemvor is the god of death and the dead. He guides the souls of the recently deceased to the Fugue Plane.' },
  { name: 'Sune', alignment: 'Chaotic Good', portfolio: 'Beauty, Love, Passion', favoredWeapon: 'Whip', description: 'Sune is the goddess of beauty, love, and passion. She encourages the pursuit of beauty in all its forms.' },
  { name: 'Oghma', alignment: 'True Neutral', portfolio: 'Knowledge, Invention, Inspiration', favoredWeapon: 'Longsword', description: 'Oghma is the god of knowledge, invention, and inspiration. He is patron of bards and all who seek to create.' },
  { name: 'Helm', alignment: 'Lawful Neutral', portfolio: 'Guardians, Protectors, Protection', favoredWeapon: 'Bastard Sword', description: 'Helm is the god of guardians and protectors. The Great Guard is the epitome of the guardian and the ever-watchful sentry.' },
  { name: 'Ilmater', alignment: 'Lawful Good', portfolio: 'Endurance, Suffering, Martyrdom', favoredWeapon: 'Unarmed', description: 'Ilmater is the god of endurance and suffering. He offers succor and calming to those in pain and oppression.' },
  { name: 'Chauntea', alignment: 'Neutral Good', portfolio: 'Agriculture, Plants, Farmers', favoredWeapon: 'Scythe', description: 'Chauntea is the goddess of agriculture and plants. The Great Mother is beloved by all who work the soil.' },
  { name: 'Bane', alignment: 'Lawful Evil', portfolio: 'Strife, Hatred, Tyranny', favoredWeapon: 'Morningstar', description: 'Bane is the god of strife, hatred, and tyranny. The Black Lord rules through fear and seeks to control all of Faerun.' },
  { name: 'Cyric', alignment: 'Chaotic Evil', portfolio: 'Murder, Lies, Intrigue, Deception', favoredWeapon: 'Longsword', description: 'Cyric is the god of murder, lies, and deception. The Prince of Lies revels in chaos and treachery.' },
];

export const ABILITIES = [
  { name: 'STR', full: 'Strength', base: 20, effective: 22, modifier: 6, racial: 0, equip: 2, level: 2, enhance: 0 },
  { name: 'DEX', full: 'Dexterity', base: 13, effective: 16, modifier: 3, racial: 0, equip: 2, level: 0, enhance: 1 },
  { name: 'CON', full: 'Constitution', base: 16, effective: 18, modifier: 4, racial: 2, equip: 0, level: 0, enhance: 0 },
  { name: 'INT', full: 'Intelligence', base: 12, effective: 12, modifier: 1, racial: 0, equip: 0, level: 0, enhance: 0 },
  { name: 'WIS', full: 'Wisdom', base: 14, effective: 16, modifier: 3, racial: 0, equip: 2, level: 0, enhance: 0 },
  { name: 'CHA', full: 'Charisma', base: 8, effective: 8, modifier: -1, racial: -2, equip: 0, level: 0, enhance: 0 },
];

export const SAVES_DETAIL = [
  { name: 'Fortitude', total: 16, base: 10, ability: 4, equip: 2, feat: 0, racial: 0, misc: 0 },
  { name: 'Reflex', total: 12, base: 8, ability: 3, equip: 1, feat: 0, racial: 0, misc: 0 },
  { name: 'Will', total: 10, base: 5, ability: 3, equip: 2, feat: 0, racial: 0, misc: 0 },
];

export const VITAL_STATS = {
  hitPoints: 142,
  maxHitPoints: 168,
  initiative: {
    total: 3,
    dexMod: 3,
    feats: 0,
  },
};

export const AC_DETAIL = [
  { name: 'AC',          base: 10, dex: 3, armor: 0, shield: 0, natural: 3, dodge: 3, deflect: 2, size: 0, misc: 0, total: 21 },
  { name: 'Touch',       base: 10, dex: 3, armor: 0, shield: 0, natural: 0, dodge: 3, deflect: 2, size: 0, misc: 0, total: 18 },
  { name: 'Flat-Footed', base: 10, dex: 0, armor: 0, shield: 0, natural: 3, dodge: 0, deflect: 2, size: 0, misc: 0, total: 15 },
];

export const LEVEL_HISTORY = [
  { level: 1, className: 'Fighter', classLevel: 1, hpGained: 10, skillPointsRemaining: 0, abilityIncrease: null as string | null,
    skillsGained: [{ name: 'Intimidate', ranks: 4 }, { name: 'Discipline', ranks: 4 }],
    featsGained: ['Power Attack', 'Cleave', 'Weapon Focus (Kama)', 'Armor Proficiency (Light)', 'Armor Proficiency (Medium)', 'Armor Proficiency (Heavy)', 'Shield Proficiency', 'Weapon Proficiency (Martial)', 'Weapon Proficiency (Simple)'] },
  { level: 2, className: 'Fighter', classLevel: 2, hpGained: 8, skillPointsRemaining: 0, abilityIncrease: null,
    skillsGained: [{ name: 'Intimidate', ranks: 1 }, { name: 'Discipline', ranks: 1 }],
    featsGained: ['Weapon Specialization (Kama)'] },
  { level: 3, className: 'Fighter', classLevel: 3, hpGained: 9, skillPointsRemaining: 0, abilityIncrease: null,
    skillsGained: [{ name: 'Intimidate', ranks: 1 }],
    featsGained: ['Toughness'] },
  { level: 4, className: 'Fighter', classLevel: 4, hpGained: 7, skillPointsRemaining: 1, abilityIncrease: 'STR',
    skillsGained: [{ name: 'Discipline', ranks: 1 }],
    featsGained: [] as string[] },
  { level: 5, className: 'Monk', classLevel: 1, hpGained: 8, skillPointsRemaining: 0, abilityIncrease: null,
    skillsGained: [{ name: 'Tumble', ranks: 4 }, { name: 'Concentration', ranks: 4 }],
    featsGained: ['Improved Unarmed Strike', 'Stunning Fist', 'Monk AC Bonus'] },
  { level: 6, className: 'Monk', classLevel: 2, hpGained: 6, skillPointsRemaining: 0, abilityIncrease: null,
    skillsGained: [{ name: 'Tumble', ranks: 1 }, { name: 'Move Silently', ranks: 3 }],
    featsGained: ['Combat Reflexes', 'Deflect Arrows'] },
  { level: 7, className: 'Monk', classLevel: 3, hpGained: 7, skillPointsRemaining: 0, abilityIncrease: null,
    skillsGained: [{ name: 'Tumble', ranks: 1 }, { name: 'Concentration', ranks: 3 }],
    featsGained: ['Still Mind'] },
  { level: 8, className: 'Monk', classLevel: 4, hpGained: 5, skillPointsRemaining: 0, abilityIncrease: 'STR',
    skillsGained: [{ name: 'Tumble', ranks: 1 }, { name: 'Hide', ranks: 3 }],
    featsGained: ['Ki Strike (Magic)', 'Slow Fall (20 ft)'] },
  { level: 9, className: 'Monk', classLevel: 5, hpGained: 8, skillPointsRemaining: 0, abilityIncrease: null,
    skillsGained: [{ name: 'Tumble', ranks: 1 }, { name: 'Listen', ranks: 3 }],
    featsGained: ['Spring Attack', 'Purity of Body'] },
  { level: 10, className: 'Monk', classLevel: 6, hpGained: 6, skillPointsRemaining: 0, abilityIncrease: null,
    skillsGained: [{ name: 'Tumble', ranks: 1 }, { name: 'Spot', ranks: 3 }],
    featsGained: ['Improved Evasion', 'Slow Fall (30 ft)'] },
  { level: 11, className: 'Monk', classLevel: 7, hpGained: 7, skillPointsRemaining: 0, abilityIncrease: null,
    skillsGained: [{ name: 'Concentration', ranks: 4 }],
    featsGained: ['Diamond Body', 'Wholeness of Body'] },
  { level: 12, className: 'Monk', classLevel: 8, hpGained: 8, skillPointsRemaining: 0, abilityIncrease: 'STR',
    skillsGained: [{ name: 'Tumble', ranks: 1 }, { name: 'Concentration', ranks: 3 }],
    featsGained: ['Great Cleave', 'Ki Strike (Lawful)', 'Slow Fall (40 ft)'] },
  { level: 13, className: 'Monk', classLevel: 9, hpGained: 6, skillPointsRemaining: 0, abilityIncrease: null,
    skillsGained: [{ name: 'Tumble', ranks: 1 }, { name: 'Listen', ranks: 3 }],
    featsGained: ['Diamond Soul', 'Improved Evasion'] },
  { level: 14, className: 'Monk', classLevel: 10, hpGained: 7, skillPointsRemaining: 0, abilityIncrease: null,
    skillsGained: [{ name: 'Spot', ranks: 4 }],
    featsGained: ['Slow Fall (50 ft)'] },
  { level: 15, className: 'Monk', classLevel: 11, hpGained: 8, skillPointsRemaining: 0, abilityIncrease: null,
    skillsGained: [{ name: 'Tumble', ranks: 1 }, { name: 'Concentration', ranks: 3 }],
    featsGained: ['Improved Critical (Kama)', 'Greater Flurry'] },
  { level: 16, className: 'Monk', classLevel: 12, hpGained: 5, skillPointsRemaining: 0, abilityIncrease: 'STR',
    skillsGained: [{ name: 'Tumble', ranks: 1 }, { name: 'Hide', ranks: 3 }],
    featsGained: ['Ki Strike (Adamantine)', 'Slow Fall (60 ft)'] },
];

export const SKILLS = [
  { name: 'Concentration', total: 19, ranks: 19, abilityMod: 4, misc: 0, ability: 'CON', isClassSkill: true, acp: false },
  { name: 'Craft Alchemy', total: 1, ranks: 0, abilityMod: 1, misc: 0, ability: 'INT', isClassSkill: false, acp: false },
  { name: 'Diplomacy', total: -1, ranks: 0, abilityMod: -1, misc: 0, ability: 'CHA', isClassSkill: false, acp: false },
  { name: 'Hide', total: 11, ranks: 8, abilityMod: 3, misc: 0, ability: 'DEX', isClassSkill: true, acp: true },
  { name: 'Intimidate', total: 5, ranks: 6, abilityMod: -1, misc: 0, ability: 'CHA', isClassSkill: true, acp: false },
  { name: 'Listen', total: 15, ranks: 12, abilityMod: 3, misc: 0, ability: 'WIS', isClassSkill: true, acp: false },
  { name: 'Lore', total: 5, ranks: 4, abilityMod: 1, misc: 0, ability: 'INT', isClassSkill: true, acp: false },
  { name: 'Move Silently', total: 14, ranks: 11, abilityMod: 3, misc: 0, ability: 'DEX', isClassSkill: true, acp: true },
  { name: 'Parry', total: 6, ranks: 0, abilityMod: 6, misc: 0, ability: 'STR', isClassSkill: false, acp: false },
  { name: 'Swim', total: 2, ranks: 0, abilityMod: 6, misc: -4, ability: 'STR', isClassSkill: false, acp: true },
  { name: 'Spot', total: 12, ranks: 9, abilityMod: 3, misc: 0, ability: 'WIS', isClassSkill: true, acp: false },
  { name: 'Tumble', total: 22, ranks: 19, abilityMod: 3, misc: 0, ability: 'DEX', isClassSkill: true, acp: true },
];

export type DummyFeat = {
  id: number;
  name: string;
  description: string;
  use?: string;
  icon: string;
  type: string;
  isProtected: boolean;
  prerequisites: { name: string; met: boolean; current?: number; required?: number }[];
};

export const FEAT_TYPE_OPTIONS = ['General', 'Combat', 'Proficiency', 'Metamagic', 'Divine', 'Class', 'Background', 'Racial'] as const;

export const FEATS: Record<string, { title: string; feats: DummyFeat[] }> = {
  general: {
    title: 'General Feats',
    feats: [
      { id: 28, name: 'Power Attack', description: 'You can make exceptionally powerful melee attacks by trading attack bonus for damage. Subtract a number from attack rolls and add the same number to melee damage.', use: 'Activated as a combat mode. While active, you take a -5 penalty on attack rolls and gain +5 bonus to damage.', icon: 'ife_powerattack', type: 'Combat', isProtected: false, prerequisites: [{ name: 'STR 13', met: true, current: 20, required: 13 }] },
      { id: 6, name: 'Cleave', description: 'If you deal a creature enough damage to make it drop, you get an immediate extra melee attack against another creature in range.', icon: 'ife_cleave', type: 'Combat', isProtected: false, prerequisites: [{ name: 'STR 13', met: true, current: 20, required: 13 }, { name: 'Power Attack', met: true }] },
      { id: 13, name: 'Great Cleave', description: 'As Cleave, except there is no limit to the number of times you can use it per round.', icon: 'ife_greatcleave', type: 'Combat', isProtected: false, prerequisites: [{ name: 'STR 13', met: true }, { name: 'Cleave', met: true }, { name: 'BAB +4', met: true, current: 14, required: 4 }] },
      { id: 49, name: 'Toughness', description: 'You gain +1 hit point per character level.', icon: 'ife_toughness', type: 'General', isProtected: false, prerequisites: [] },
      { id: 8, name: 'Combat Reflexes', description: 'You may make a number of additional attacks of opportunity equal to your Dexterity modifier.', icon: 'ife_combatreflexes', type: 'Combat', isProtected: false, prerequisites: [] },
      { id: 44, name: 'Spring Attack', description: 'When using the attack action with a melee weapon, you can move both before and after the attack, provided total distance is not greater than your speed.', icon: 'ife_springattack', type: 'Combat', isProtected: false, prerequisites: [{ name: 'DEX 13', met: true, current: 16, required: 13 }, { name: 'Dodge', met: false }, { name: 'Mobility', met: false }, { name: 'BAB +4', met: true, current: 14, required: 4 }] },
      { id: 17, name: 'Improved Critical (Kama)', description: 'When using a kama, your critical threat range is doubled.', icon: 'ife_improvedcritical', type: 'Combat', isProtected: false, prerequisites: [{ name: 'BAB +8', met: true, current: 14, required: 8 }, { name: 'Weapon Proficiency', met: true }] },
    ],
  },
  classBonusFeats: {
    title: 'Class Bonus Feats',
    feats: [
      { id: 52, name: 'Weapon Focus (Kama)', description: '+1 attack bonus with kamas.', icon: 'ife_weaponfocus', type: 'Combat', isProtected: true, prerequisites: [{ name: 'BAB +1', met: true, current: 14, required: 1 }] },
      { id: 53, name: 'Weapon Specialization (Kama)', description: '+2 damage bonus with kamas.', icon: 'ife_weaponspec', type: 'Combat', isProtected: true, prerequisites: [{ name: 'Weapon Focus (Kama)', met: true }, { name: 'Fighter Lv4', met: true }] },
      { id: 19, name: 'Improved Unarmed Strike', description: 'You are considered armed even when unarmed. Your unarmed attacks do not provoke attacks of opportunity.', icon: 'ife_improvedunarmedstrike', type: 'Combat', isProtected: true, prerequisites: [] },
      { id: 45, name: 'Stunning Fist', description: 'Attempt to stun an opponent with an unarmed strike. DC 10 + half level + WIS modifier.', use: 'Activated as a combat mode. Your next unarmed attack attempts to stun the target for 3 rounds.', icon: 'ife_stunningfist', type: 'Combat', isProtected: true, prerequisites: [{ name: 'DEX 13', met: true }, { name: 'WIS 13', met: true }, { name: 'Improved Unarmed Strike', met: true }] },
      { id: 18, name: 'Improved Evasion', description: 'You take half damage on a failed Reflex save and no damage on a successful one.', icon: 'ife_evasion', type: 'Class', isProtected: true, prerequisites: [] },
      { id: 21, name: 'Ki Strike (Magic)', description: 'Unarmed strikes are treated as magic weapons for overcoming damage reduction.', icon: 'ife_kistrike', type: 'Class', isProtected: true, prerequisites: [] },
      { id: 22, name: 'Ki Strike (Lawful)', description: 'Unarmed strikes are treated as lawful-aligned for overcoming damage reduction.', icon: 'ife_kistrike', type: 'Class', isProtected: true, prerequisites: [] },
      { id: 23, name: 'Ki Strike (Adamantine)', description: 'Unarmed strikes are treated as adamantine for overcoming damage reduction.', icon: 'ife_kistrike', type: 'Class', isProtected: true, prerequisites: [] },
      { id: 10, name: 'Diamond Body', description: 'You gain immunity to all poisons.', icon: 'ife_diamondbody', type: 'Class', isProtected: true, prerequisites: [] },
      { id: 11, name: 'Diamond Soul', description: 'You gain spell resistance equal to 10 + monk level.', icon: 'ife_diamondsoul', type: 'Class', isProtected: true, prerequisites: [] },
    ],
  },
  background: {
    title: 'Background Feats',
    feats: [
      { id: 100, name: 'Bully', description: '+2 bonus to Intimidate checks.', icon: 'ife_bully', type: 'Background', isProtected: true, prerequisites: [] },
    ],
  },
  racial: {
    title: 'Racial Feats',
    feats: [
      { id: 200, name: 'Dwarven Stonecunning', description: '+2 racial bonus to Search checks made in subterranean areas.', icon: 'ife_stonecunning', type: 'Racial', isProtected: true, prerequisites: [] },
      { id: 201, name: 'Hardiness vs. Poisons', description: '+2 racial bonus on saving throws against poison.', icon: 'ife_hardiness', type: 'Racial', isProtected: true, prerequisites: [] },
      { id: 202, name: 'Hardiness vs. Spells', description: '+2 racial bonus on saving throws against spells.', icon: 'ife_hardiness', type: 'Racial', isProtected: true, prerequisites: [] },
      { id: 203, name: 'Darkvision', description: 'You can see in the dark up to 60 feet.', icon: 'ife_darkvision', type: 'Racial', isProtected: true, prerequisites: [] },
    ],
  },
};

export const ALL_FEATS: (DummyFeat & { canTake: boolean; hasFeat: boolean })[] = [
  ...Object.values(FEATS).flatMap(cat => cat.feats.map(f => ({ ...f, canTake: true, hasFeat: true }))),
  { id: 3, name: 'Alertness', description: '+2 bonus to Listen and Spot checks.', icon: 'ife_alertness', type: 'General', isProtected: false, prerequisites: [], canTake: true, hasFeat: false },
  { id: 5, name: 'Blind-Fight', description: 'In melee, every time you miss because of concealment, you can re-roll the miss chance.', icon: 'ife_blindfight', type: 'Combat', isProtected: false, prerequisites: [], canTake: true, hasFeat: false },
  { id: 9, name: 'Dodge', description: '+1 dodge bonus to AC against a selected opponent.', icon: 'ife_dodge', type: 'General', isProtected: false, prerequisites: [{ name: 'DEX 13', met: true, current: 16, required: 13 }], canTake: true, hasFeat: false },
  { id: 14, name: 'Improved Initiative', description: '+4 bonus on initiative checks.', icon: 'ife_improvedinit', type: 'General', isProtected: false, prerequisites: [], canTake: true, hasFeat: false },
  { id: 15, name: 'Improved Knockdown', description: 'You do not provoke an attack of opportunity when using Knockdown.', icon: 'ife_improvedknockdown', type: 'Combat', isProtected: false, prerequisites: [{ name: 'Knockdown', met: false }], canTake: false, hasFeat: false },
  { id: 16, name: 'Iron Will', description: '+2 bonus on Will saving throws.', icon: 'ife_ironwill', type: 'General', isProtected: false, prerequisites: [], canTake: true, hasFeat: false },
  { id: 24, name: 'Lightning Reflexes', description: '+2 bonus on Reflex saving throws.', icon: 'ife_lightningreflexes', type: 'General', isProtected: false, prerequisites: [], canTake: true, hasFeat: false },
  { id: 25, name: 'Mobility', description: '+4 dodge bonus to AC against attacks of opportunity from movement.', icon: 'ife_mobility', type: 'General', isProtected: false, prerequisites: [{ name: 'DEX 13', met: true }, { name: 'Dodge', met: false }], canTake: false, hasFeat: false },
  { id: 30, name: 'Weapon Finesse', description: 'Use DEX modifier instead of STR for attack rolls with light weapons.', icon: 'ife_weaponfinesse', type: 'Combat', isProtected: false, prerequisites: [{ name: 'BAB +1', met: true, current: 14, required: 1 }], canTake: true, hasFeat: false },
  { id: 31, name: 'Extend Spell', description: 'An extended spell lasts twice as long as normal.', icon: 'ife_extendspell', type: 'Metamagic', isProtected: false, prerequisites: [], canTake: true, hasFeat: false },
  { id: 32, name: 'Empower Spell', description: 'All variable, numeric effects of an empowered spell are increased by one-half.', icon: 'ife_empowerspell', type: 'Metamagic', isProtected: false, prerequisites: [], canTake: true, hasFeat: false },
  { id: 33, name: 'Maximize Spell', description: 'All variable, numeric effects of a maximized spell are maximized.', icon: 'ife_maximizespell', type: 'Metamagic', isProtected: false, prerequisites: [], canTake: true, hasFeat: false },
];

export type DummySpell = {
  id: number;
  name: string;
  school: string;
  description: string;
  isDomain: boolean;
  innateLevel?: number;
  casterLevel?: string;
  descriptor?: string;
  components?: string;
  range?: string;
  area?: string;
  duration?: string;
  save?: string;
  spellResistance?: string;
};

export const SPELL_SCHOOL_OPTIONS = ['Abjuration', 'Conjuration', 'Divination', 'Enchantment', 'Evocation', 'Illusion', 'Necromancy', 'Transmutation'] as const;

export const SPELLS = {
  casterClasses: [
    { className: 'Cleric', classLevel: 10, casterLevel: 10, spellDC: 15, spellType: 'prepared' as const, canEdit: true, spellSlots: [4, 4, 3, 3, 2, 1, 0, 0, 0, 0] },
  ],
  known: [
    { level: 0, spells: [
      { id: 1, name: 'Light', school: 'Evocation', description: 'This spell causes an object to glow like a torch, shedding bright light in a 20-foot radius.', isDomain: false, innateLevel: 0, casterLevel: 'Bard 0, Cleric 0, Druid 0, Wizard / Sorcerer 0', components: 'Verbal, Material', range: 'Touch', area: 'Single', duration: '10 min./level', save: 'None', spellResistance: 'No' },
      { id: 2, name: 'Cure Minor Wounds', school: 'Conjuration', description: 'Cures 1 point of damage.', isDomain: false, innateLevel: 0, casterLevel: 'Cleric 0, Druid 0', descriptor: 'Positive', components: 'Verbal, Somatic', range: 'Touch', area: 'Single', duration: 'Instant', save: 'None', spellResistance: 'Yes' },
      { id: 3, name: 'Resistance', school: 'Abjuration', description: 'The subject gains a +1 resistance bonus on saving throws.', isDomain: false, innateLevel: 0, casterLevel: 'Bard 0, Cleric 0, Druid 0, Paladin 1, Wizard / Sorcerer 0', components: 'Verbal, Somatic, Material', range: 'Touch', area: 'Single', duration: '1 Turn', save: 'None', spellResistance: 'Yes' },
      { id: 4, name: 'Virtue', school: 'Transmutation', description: 'The subject gains 1 temporary hit point.', isDomain: false, innateLevel: 0, casterLevel: 'Cleric 0, Druid 0, Paladin 1', components: 'Verbal, Somatic', range: 'Touch', area: 'Single', duration: '1 Turn', save: 'None', spellResistance: 'Yes' },
    ]},
    { level: 1, spells: [
      { id: 10, name: 'Bless', school: 'Enchantment', description: 'Bless fills your allies with courage. Each ally gains a +1 morale bonus on attack rolls and on saving throws against fear effects.', isDomain: false, innateLevel: 1, casterLevel: 'Cleric 1, Paladin 1', components: 'Verbal, Somatic', range: 'Medium', area: 'Colossal', duration: '1 min./level', save: 'None', spellResistance: 'Yes' },
      { id: 11, name: 'Cure Light Wounds', school: 'Conjuration', description: 'When laying your hand upon a living creature, you channel positive energy that cures 1d8 points of damage +1 point per caster level (maximum +5).', isDomain: false, innateLevel: 1, casterLevel: 'Bard 1, Cleric 1, Druid 1, Paladin 1, Ranger 2', descriptor: 'Positive', components: 'Verbal, Somatic', range: 'Touch', area: 'Single', duration: 'Instant', save: 'Will half (harmless)', spellResistance: 'Yes' },
      { id: 12, name: 'Shield of Faith', school: 'Abjuration', description: 'This spell creates a shimmering, magical field around the target that grants a +2 deflection bonus to AC, with an additional +1 per 6 caster levels.', isDomain: false, innateLevel: 1, casterLevel: 'Cleric 1', components: 'Verbal, Somatic, Material', range: 'Touch', area: 'Single', duration: '1 min./level', save: 'None', spellResistance: 'No' },
      { id: 13, name: 'Divine Favor', school: 'Evocation', description: 'Calling upon the strength and wisdom of a deity, you gain a +1 luck bonus on attack and weapon damage rolls for every three caster levels you have (maximum +3).', isDomain: false, innateLevel: 1, casterLevel: 'Cleric 1, Paladin 1', components: 'Verbal, Somatic', range: 'Personal', area: 'Caster', duration: '1 Turn', save: 'None', spellResistance: 'No' },
      { id: 14, name: 'Enlarge Person', school: 'Transmutation', description: 'Humanoid creature doubles in size. +2 Str, -2 Dex, -1 attack/AC.', isDomain: true, innateLevel: 1, casterLevel: 'Wizard / Sorcerer 1', components: 'Verbal, Somatic, Material', range: 'Short', area: 'Single', duration: '1 min./level', save: 'Fortitude negates', spellResistance: 'Yes' },
      { id: 15, name: 'Magic Weapon', school: 'Transmutation', description: 'Magic weapon gives a weapon a +1 enhancement bonus on attack and damage rolls.', isDomain: true, innateLevel: 1, casterLevel: 'Cleric 1, Paladin 1, Wizard / Sorcerer 1', components: 'Verbal, Somatic', range: 'Touch', area: 'Single', duration: '1 min./level', save: 'None', spellResistance: 'No' },
    ]},
    { level: 2, spells: [
      { id: 20, name: "Bull's Strength", school: 'Transmutation', description: 'Subject gains +4 enhancement bonus to Strength for 1 min/level.', isDomain: true },
      { id: 21, name: 'Cure Moderate Wounds', school: 'Conjuration', description: 'Cures 2d8 damage +1 per caster level (max +10).', isDomain: false },
      { id: 22, name: 'Hold Person', school: 'Enchantment', description: 'Paralyzes one humanoid for 1 round per level.', isDomain: false },
      { id: 23, name: 'Silence', school: 'Illusion', description: 'Negates sound in a 20-ft radius. Prevents spellcasting within.', isDomain: false },
    ]},
    { level: 3, spells: [
      { id: 30, name: 'Cure Serious Wounds', school: 'Conjuration', description: 'Cures 3d8 damage +1 per caster level (max +15).', isDomain: false },
      { id: 31, name: 'Prayer', school: 'Enchantment', description: 'Allies get +1 bonus on most rolls, enemies get -1 penalty.', isDomain: false },
      { id: 32, name: 'Dispel Magic', school: 'Abjuration', description: 'Cancels magical spells and effects. Caster level check DC 11 + caster level.', isDomain: false },
    ]},
    { level: 4, spells: [
      { id: 40, name: 'Cure Critical Wounds', school: 'Conjuration', description: 'Cures 4d8 damage +1 per caster level (max +20).', isDomain: false },
      { id: 41, name: 'Divine Power', school: 'Evocation', description: 'You gain BAB equal to your level, +6 Str, and 1 temporary hp per level.', isDomain: true },
    ]},
  ],
};

export const MEMORIZED_SPELLS: { id: number; name: string; school: string; level: number; count: number; isDomain: boolean }[] = [
  { id: 2, name: 'Cure Minor Wounds', school: 'Conjuration', level: 0, count: 2, isDomain: false },
  { id: 3, name: 'Resistance', school: 'Abjuration', level: 0, count: 1, isDomain: false },
  { id: 1, name: 'Light', school: 'Evocation', level: 0, count: 1, isDomain: false },
  { id: 11, name: 'Cure Light Wounds', school: 'Conjuration', level: 1, count: 2, isDomain: false },
  { id: 10, name: 'Bless', school: 'Enchantment', level: 1, count: 1, isDomain: false },
  { id: 15, name: 'Magic Weapon', school: 'Transmutation', level: 1, count: 1, isDomain: true },
  { id: 21, name: 'Cure Moderate Wounds', school: 'Conjuration', level: 2, count: 2, isDomain: false },
  { id: 20, name: "Bull's Strength", school: 'Transmutation', level: 2, count: 1, isDomain: true },
  { id: 30, name: 'Cure Serious Wounds', school: 'Conjuration', level: 3, count: 2, isDomain: false },
  { id: 32, name: 'Dispel Magic', school: 'Abjuration', level: 3, count: 1, isDomain: false },
  { id: 40, name: 'Cure Critical Wounds', school: 'Conjuration', level: 4, count: 1, isDomain: false },
  { id: 41, name: 'Divine Power', school: 'Evocation', level: 4, count: 1, isDomain: true },
];

export const ALL_SPELLS: (DummySpell & { level: number; isLearned: boolean })[] = [
  ...SPELLS.known.flatMap(g => g.spells.map(s => ({ ...s, level: g.level, isLearned: true }))),
  { id: 50, name: 'Doom', school: 'Necromancy', description: 'One subject takes -2 on attack rolls, damage rolls, saves, and checks.', isDomain: false, level: 1, isLearned: false },
  { id: 51, name: 'Command', school: 'Enchantment', description: 'One subject obeys selected command for 1 round.', isDomain: false, level: 1, isLearned: false },
  { id: 52, name: 'Inflict Light Wounds', school: 'Necromancy', description: 'Touch deals 1d8 damage +1 per caster level (max +5).', isDomain: false, level: 1, isLearned: false },
  { id: 53, name: "Bear's Endurance", school: 'Transmutation', description: 'Subject gains +4 enhancement bonus to Constitution.', isDomain: false, level: 2, isLearned: false },
  { id: 54, name: 'Spiritual Weapon', school: 'Evocation', description: 'A weapon of force attacks subjects at your direction. BAB = caster level.', isDomain: false, level: 2, isLearned: false },
  { id: 55, name: 'Flame Strike', school: 'Evocation', description: 'Smite foes with divine fire (1d6 per level, max 15d6). Half fire, half divine.', isDomain: false, level: 4, isLearned: false },
  { id: 56, name: 'Death Ward', school: 'Necromancy', description: 'Grants immunity to all death spells and negative energy effects.', isDomain: false, level: 4, isLearned: false },
  { id: 57, name: 'Raise Dead', school: 'Conjuration', description: 'Restores life to a deceased subject. Subject loses a level.', isDomain: false, level: 5, isLearned: false },
  { id: 58, name: 'Slay Living', school: 'Necromancy', description: 'Touch attack kills subject. Fort save for 3d6+1/level instead.', isDomain: false, level: 5, isLearned: false },
  { id: 59, name: 'Blade Barrier', school: 'Evocation', description: 'Wall of whirling blades deals 1d6 per level damage (max 15d6).', isDomain: false, level: 6, isLearned: false },
  { id: 60, name: 'Heal', school: 'Conjuration', description: 'Cures 10 points per level of damage, all diseases and mental conditions.', isDomain: false, level: 6, isLearned: false },
];

export const ABILITY_SPELLS = [
  { id: 500, name: 'Wholeness of Body', source: 'Monk', description: 'Heal yourself for monk level x 2 hit points per day.', usesPerDay: 1 },
  { id: 501, name: 'Quivering Palm', source: 'Monk', description: 'Set up fatal vibrations in a living creature. Fort save or die.', usesPerDay: 1 },
];

export const INVENTORY = [
  { slot: 'Main Hand', name: 'Kamas +3', baseItem: 'Kama', type: 'Weapon', weight: 2.0, value: 18302, enhancement: 3, charges: null as { current: number; max: number } | null, rarity: 'rare' as const, description: 'A pair of finely crafted kamas imbued with elemental fire. The blades shimmer with a faint orange glow.', flags: { custom: true }, properties: ['Enhancement +3', 'Keen', 'Fire Damage 1d6'] },
  { slot: 'Off Hand', name: 'Kamas +2', baseItem: 'Kama', type: 'Weapon', weight: 2.0, value: 8302, enhancement: 2, charges: null as { current: number; max: number } | null, rarity: 'uncommon' as const, description: 'A sturdy kama enchanted with frost magic.', flags: {}, properties: ['Enhancement +2', 'Cold Damage 1d4'] },
  { slot: 'Head', name: 'Headband of Intellect +2', baseItem: 'Headband', type: 'Accessory', weight: 0.1, value: 4000, enhancement: null as number | null, charges: null as { current: number; max: number } | null, rarity: 'uncommon' as const, description: 'A silver headband set with a sapphire that sharpens the wearer\'s mind.', flags: {}, properties: ['Intelligence +2'] },
  { slot: 'Chest', name: "Monk's Belt of the Sun Soul", baseItem: 'Monk Robe', type: 'Armor', weight: 1.0, value: 25000, enhancement: null as number | null, charges: null as { current: number; max: number } | null, rarity: 'epic' as const, description: 'A sacred vestment of the Order of the Sun Soul, woven with golden thread and blessed by the Morninglord himself.', flags: { plot: true }, properties: ['AC Bonus +3', 'Wisdom +2'] },
  { slot: 'Hands', name: 'Gloves of the Long Death +3', baseItem: 'Gloves', type: 'Accessory', weight: 0.5, value: 12000, enhancement: 3, charges: null as { current: number; max: number } | null, rarity: 'rare' as const, description: 'Black leather gloves bearing the insignia of the Order of the Long Death.', flags: {}, properties: ['Enhancement +3', 'Massive Criticals 1d8'] },
  { slot: 'Feet', name: 'Boots of Striding +2', baseItem: 'Boots', type: 'Accessory', weight: 1.0, value: 5600, enhancement: null as number | null, charges: null as { current: number; max: number } | null, rarity: 'uncommon' as const, description: 'Enchanted boots that grant the wearer unusual agility and freedom of movement.', flags: {}, properties: ['Dexterity +2', 'Freedom of Movement'] },
  { slot: 'Cloak', name: 'Cloak of Fortification +2', baseItem: 'Cloak', type: 'Accessory', weight: 1.0, value: 8000, enhancement: null as number | null, charges: { current: 8, max: 10 }, rarity: 'rare' as const, description: 'A heavy cloak that shimmers faintly, protecting the wearer from critical strikes.', flags: { cursed: true }, properties: ['Fortitude +2', 'Concealment 10%'] },
  { slot: 'Ring 1', name: 'Ring of Protection +2', baseItem: 'Ring', type: 'Accessory', weight: 0.0, value: 8000, enhancement: null as number | null, charges: null as { current: number; max: number } | null, rarity: 'uncommon' as const, description: 'A plain silver ring that deflects blows with an invisible barrier.', flags: {}, properties: ['Deflection AC +2'] },
  { slot: 'Ring 2', name: 'Ring of Regeneration', baseItem: 'Ring', type: 'Accessory', weight: 0.0, value: 15000, enhancement: null as number | null, charges: null as { current: number; max: number } | null, rarity: 'rare' as const, description: 'A gold ring set with an emerald that mends the wearer\'s wounds over time.', flags: { stolen: true }, properties: ['Regeneration +2'] },
  { slot: 'Belt', name: 'Belt of Storm Giant Strength', baseItem: 'Belt', type: 'Accessory', weight: 1.0, value: 36000, enhancement: null as number | null, charges: null as { current: number; max: number } | null, rarity: 'epic' as const, description: 'A massive girdle forged from storm giant hide, granting tremendous physical might to the wearer.', flags: { custom: true }, properties: ['Strength +6'] },
  { slot: 'Amulet', name: 'Amulet of Natural Armor +3', baseItem: 'Amulet', type: 'Accessory', weight: 0.1, value: 18000, enhancement: null as number | null, charges: null as { current: number; max: number } | null, rarity: 'rare' as const, description: 'A bone amulet carved with druidic runes that toughens the wearer\'s skin.', flags: {}, properties: ['Natural AC +3'] },
];

export const BACKPACK = [
  { name: 'Potion of Heal', type: 'Potion', qty: 5, weight: 0.5, value: 750, rarity: 'rare' as const },
  { name: 'Potion of Barkskin', type: 'Potion', qty: 3, weight: 0.5, value: 300, rarity: 'uncommon' as const },
  { name: 'Bolt of Lightning', type: 'Ammunition', qty: 12, weight: 0.1, value: 50, rarity: 'common' as const },
  { name: "Thieves' Tools +6", type: 'Tool', qty: 1, weight: 1.0, value: 850, rarity: 'uncommon' as const },
  { name: 'Gem - Star Sapphire', type: 'Gem', qty: 2, weight: 0.0, value: 1000, rarity: 'rare' as const },
  { name: 'Key to Crossroad Keep', type: 'Key', qty: 1, weight: 0.0, value: 0, rarity: 'common' as const },
];

export const AVAILABLE_CLASSES = {
  base: {
    combat: [
      { id: 4, name: 'Fighter', label: 'Fighter', type: 'base' as const, focus: 'combat', maxLevel: 60, hitDie: 10, skillPoints: 2, isSpellcaster: false, hasArcane: false, hasDivine: false, primaryAbility: 'STR', babProgression: 'high', alignmentRestricted: false, description: 'A warrior with exceptional combat capability and unequaled skill with weapons.' },
      { id: 1, name: 'Barbarian', label: 'Barbarian', type: 'base' as const, focus: 'combat', maxLevel: 60, hitDie: 12, skillPoints: 4, isSpellcaster: false, hasArcane: false, hasDivine: false, primaryAbility: 'STR', babProgression: 'high', alignmentRestricted: true, description: 'A ferocious warrior who uses fury and instinct to bring down foes.' },
      { id: 11, name: 'Paladin', label: 'Paladin', type: 'base' as const, focus: 'combat', maxLevel: 60, hitDie: 10, skillPoints: 2, isSpellcaster: true, hasArcane: false, hasDivine: true, primaryAbility: 'CHA', babProgression: 'high', alignmentRestricted: true, description: 'A champion of justice and destroyer of evil, protected and empowered by divine powers.' },
      { id: 12, name: 'Ranger', label: 'Ranger', type: 'base' as const, focus: 'combat', maxLevel: 60, hitDie: 8, skillPoints: 6, isSpellcaster: true, hasArcane: false, hasDivine: true, primaryAbility: 'DEX', babProgression: 'high', alignmentRestricted: false, description: 'A cunning, skilled warrior of the wilderness.' },
    ],
    arcaneCaster: [
      { id: 15, name: 'Wizard', label: 'Wizard', type: 'base' as const, focus: 'arcane_caster', maxLevel: 60, hitDie: 4, skillPoints: 2, isSpellcaster: true, hasArcane: true, hasDivine: false, primaryAbility: 'INT', babProgression: 'low', alignmentRestricted: false, description: 'A potent spellcaster schooled in the arcane arts.' },
      { id: 13, name: 'Sorcerer', label: 'Sorcerer', type: 'base' as const, focus: 'arcane_caster', maxLevel: 60, hitDie: 4, skillPoints: 2, isSpellcaster: true, hasArcane: true, hasDivine: false, primaryAbility: 'CHA', babProgression: 'low', alignmentRestricted: false, description: 'A spellcaster with inborn magical ability.' },
      { id: 16, name: 'Warlock', label: 'Warlock', type: 'base' as const, focus: 'arcane_caster', maxLevel: 60, hitDie: 6, skillPoints: 2, isSpellcaster: true, hasArcane: true, hasDivine: false, primaryAbility: 'CHA', babProgression: 'medium', alignmentRestricted: false, description: 'A wielder of eldritch power drawn from dark pacts.' },
    ],
    divineCaster: [
      { id: 2, name: 'Cleric', label: 'Cleric', type: 'base' as const, focus: 'divine_caster', maxLevel: 60, hitDie: 8, skillPoints: 2, isSpellcaster: true, hasArcane: false, hasDivine: true, primaryAbility: 'WIS', babProgression: 'medium', alignmentRestricted: false, description: 'A master of divine magic and target of a deity.' },
      { id: 3, name: 'Druid', label: 'Druid', type: 'base' as const, focus: 'divine_caster', maxLevel: 60, hitDie: 8, skillPoints: 4, isSpellcaster: true, hasArcane: false, hasDivine: true, primaryAbility: 'WIS', babProgression: 'medium', alignmentRestricted: true, description: 'A divine spellcaster who draws power from nature.' },
      { id: 14, name: 'Spirit Shaman', label: 'Spirit Shaman', type: 'base' as const, focus: 'divine_caster', maxLevel: 60, hitDie: 8, skillPoints: 2, isSpellcaster: true, hasArcane: false, hasDivine: true, primaryAbility: 'WIS', babProgression: 'medium', alignmentRestricted: false, description: 'A divine caster who channels the power of the spirit world.' },
    ],
    skillSpecialist: [
      { id: 0, name: 'Bard', label: 'Bard', type: 'base' as const, focus: 'skill_specialist', maxLevel: 60, hitDie: 6, skillPoints: 6, isSpellcaster: true, hasArcane: true, hasDivine: false, primaryAbility: 'CHA', babProgression: 'medium', alignmentRestricted: false, description: 'A jack-of-all-trades, using skill and spell alike.' },
      { id: 8, name: 'Monk', label: 'Monk', type: 'base' as const, focus: 'skill_specialist', maxLevel: 60, hitDie: 8, skillPoints: 4, isSpellcaster: false, hasArcane: false, hasDivine: false, primaryAbility: 'WIS', babProgression: 'medium', alignmentRestricted: true, description: 'A master of martial arts, harnessing the power of the body in pursuit of perfection.' },
      { id: 9, name: 'Rogue', label: 'Rogue', type: 'base' as const, focus: 'skill_specialist', maxLevel: 60, hitDie: 6, skillPoints: 8, isSpellcaster: false, hasArcane: false, hasDivine: false, primaryAbility: 'DEX', babProgression: 'medium', alignmentRestricted: false, description: 'A tricky, skillful scout and spy who wins the battle by stealth.' },
      { id: 10, name: 'Swashbuckler', label: 'Swashbuckler', type: 'base' as const, focus: 'skill_specialist', maxLevel: 60, hitDie: 10, skillPoints: 4, isSpellcaster: false, hasArcane: false, hasDivine: false, primaryAbility: 'DEX', babProgression: 'high', alignmentRestricted: false, description: 'A dashing swordsman who relies on agility and charm.' },
    ],
  },
  prestige: {
    combat: [
      { id: 20, name: 'Weapon Master', label: 'Weapon Master', type: 'prestige' as const, focus: 'combat', maxLevel: 7, hitDie: 10, skillPoints: 2, isSpellcaster: false, hasArcane: false, hasDivine: false, primaryAbility: 'STR', babProgression: 'high', alignmentRestricted: false, description: 'For the weapon master, perfection is found in the mastery of a single weapon.' },
      { id: 21, name: 'Frenzied Berserker', label: 'Frenzied Berserker', type: 'prestige' as const, focus: 'combat', maxLevel: 10, hitDie: 12, skillPoints: 2, isSpellcaster: false, hasArcane: false, hasDivine: false, primaryAbility: 'STR', babProgression: 'high', alignmentRestricted: false, description: 'A warrior who channels destructive fury into a terrifying frenzy.' },
      { id: 22, name: 'Divine Champion', label: 'Divine Champion', type: 'prestige' as const, focus: 'combat', maxLevel: 10, hitDie: 10, skillPoints: 2, isSpellcaster: false, hasArcane: false, hasDivine: false, primaryAbility: 'STR', babProgression: 'high', alignmentRestricted: false, description: 'A holy warrior who fights in the name of a patron deity.' },
    ],
    arcaneCaster: [
      { id: 25, name: 'Arcane Trickster', label: 'Arcane Trickster', type: 'prestige' as const, focus: 'arcane_caster', maxLevel: 10, hitDie: 4, skillPoints: 4, isSpellcaster: true, hasArcane: true, hasDivine: false, primaryAbility: 'INT', babProgression: 'low', alignmentRestricted: false, description: 'A spellcaster who combines stealth with arcane magic.' },
      { id: 26, name: 'Eldritch Knight', label: 'Eldritch Knight', type: 'prestige' as const, focus: 'arcane_caster', maxLevel: 10, hitDie: 6, skillPoints: 2, isSpellcaster: true, hasArcane: true, hasDivine: false, primaryAbility: 'INT', babProgression: 'high', alignmentRestricted: false, description: 'A warrior who combines martial skill with arcane magic.' },
    ],
    divineCaster: [
      { id: 30, name: 'Stormlord', label: 'Stormlord', type: 'prestige' as const, focus: 'divine_caster', maxLevel: 10, hitDie: 8, skillPoints: 2, isSpellcaster: true, hasArcane: false, hasDivine: true, primaryAbility: 'WIS', babProgression: 'medium', alignmentRestricted: false, description: 'A divine spellcaster who channels the fury of storms.' },
    ],
  },
  focusInfo: {
    combat: { name: 'Combat', description: 'Warriors and martial combatants', icon: 'shield' },
    arcane_caster: { name: 'Arcane Casters', description: 'Wielders of arcane magic', icon: 'flame' },
    divine_caster: { name: 'Divine Casters', description: 'Wielders of divine magic', icon: 'heart' },
    skill_specialist: { name: 'Skill Specialists', description: 'Experts in skills and versatility', icon: 'wrench' },
  },
};

export const GAME_STATE = {
  companions: [
    { name: 'Neeshka', influence: 72, status: 'In Party', recruitment: 'recruited' as const },
    { name: 'Elanee', influence: 55, status: 'In Party', recruitment: 'recruited' as const },
    { name: 'Sand', influence: 48, status: 'At Keep', recruitment: 'recruited' as const },
    { name: 'Casavir', influence: 60, status: 'At Keep', recruitment: 'recruited' as const },
    { name: 'Grobnar', influence: 80, status: 'In Party', recruitment: 'recruited' as const },
    { name: 'Qara', influence: 15, status: 'At Keep', recruitment: 'met' as const },
    { name: 'Bishop', influence: 30, status: 'Left Party', recruitment: 'met' as const },
    { name: 'Zhjaeve', influence: 0, status: 'At Keep', recruitment: 'not_recruited' as const },
  ],
  modules: [
    { id: 'mod1', name: 'Act I - West Harbor', isCurrent: false, varCount: 42 },
    { id: 'mod2', name: 'Act II - Crossroad Keep', isCurrent: true, varCount: 67 },
    { id: 'mod3', name: 'Act III - Vale of Merdelain', isCurrent: false, varCount: 23 },
  ],
  moduleInfo: {
    moduleName: 'Act II - Crossroad Keep',
    campaign: 'Neverwinter Nights 2 OC',
    currentArea: 'Crossroad Keep - Courtyard',
    entryArea: 'Crossroad Keep - Gate',
  },
  moduleVars: {
    integers: [
      { name: 'nKeepDefenseLevel', value: 3 },
      { name: 'nKeepMerchantTier', value: 2 },
      { name: 'nSiegePrepared', value: 1 },
      { name: 'nGreycloakRecruited', value: 45 },
      { name: 'nKeepRoadPatrol', value: 1 },
      { name: 'nFarmerSettled', value: 8 },
      { name: 'nMineOperational', value: 1 },
      { name: 'nChurchBuilt', value: 0 },
    ],
    strings: [
      { name: 'sLastDialogSpeaker', value: 'Kana' },
      { name: 'sKeepSteward', value: 'Kana' },
      { name: 'sCurrentObjective', value: 'Fortify the Keep' },
      { name: 'sLastAreaTransition', value: 'ck_courtyard_to_interior' },
    ],
    floats: [
      { name: 'fKeepMorale', value: 0.85 },
      { name: 'fTaxRate', value: 0.10 },
      { name: 'fGreycloakTraining', value: 0.65 },
    ],
  },
  campaignVars: {
    integers: [
      { name: 'nKhelgarMonkPath', value: 1 },
      { name: 'nShardraComplete', value: 0 },
      { name: 'nCrossroadKeepUpgrades', value: 5 },
      { name: 'nActComplete', value: 2 },
      { name: 'nRomanceActive', value: 1 },
      { name: 'nShadowReaperKilled', value: 0 },
      { name: 'nTrialVerdict', value: 1 },
      { name: 'nLuskansDefeated', value: 1 },
    ],
    strings: [
      { name: 'sLastVisitedArea', value: 'Crossroad Keep - Courtyard' },
      { name: 'sRomanceTarget', value: 'Elanee' },
      { name: 'sPlayerTitle', value: 'Captain of Crossroad Keep' },
    ],
    floats: [
      { name: 'fPlayerReputation', value: 85.5 },
      { name: 'fGoodEvilShift', value: 12.0 },
    ],
  },
  campaignSettings: {
    displayName: 'Neverwinter Nights 2 OC',
    description: 'The original Neverwinter Nights 2 campaign by Obsidian Entertainment.',
    levelCap: 30,
    xpCap: 435000,
    companionXpWeight: 1,
    henchmanXpWeight: 0,
    attackNeutrals: 0,
    autoXpAward: 1,
    journalSync: 1,
    noCharChanging: 0,
    usePersonalReputation: 0,
    startModule: 'ar_0001_west_harbor',
    moduleNames: ['West Harbor', 'Highcliff', 'Neverwinter', 'Crossroad Keep', 'Vale of Merdelain'],
    campaignFilePath: 'C:\\NWN2\\campaigns\\neverwinter2.cam',
  },
  backups: [
    { filename: 'neverwinter2_backup_20260322_143012.cam', path: 'C:\\NWN2\\campaigns\\backups\\neverwinter2_backup_20260322_143012.cam', sizeBytes: 245760, created: '2026-03-22T14:30:12' },
    { filename: 'neverwinter2_backup_20260322_081200.cam', path: 'C:\\NWN2\\campaigns\\backups\\neverwinter2_backup_20260322_081200.cam', sizeBytes: 244736, created: '2026-03-22T08:12:00' },
    { filename: 'neverwinter2_backup_20260321_233415.cam', path: 'C:\\NWN2\\campaigns\\backups\\neverwinter2_backup_20260321_233415.cam', sizeBytes: 243200, created: '2026-03-21T23:34:15' },
    { filename: 'neverwinter2_backup_20260321_140522.cam', path: 'C:\\NWN2\\campaigns\\backups\\neverwinter2_backup_20260321_140522.cam', sizeBytes: 243712, created: '2026-03-21T14:05:22' },
    { filename: 'neverwinter2_backup_20260320_091547.cam', path: 'C:\\NWN2\\campaigns\\backups\\neverwinter2_backup_20260320_091547.cam', sizeBytes: 241664, created: '2026-03-20T09:15:47' },
    { filename: 'neverwinter2_backup_20260319_172033.cam', path: 'C:\\NWN2\\campaigns\\backups\\neverwinter2_backup_20260319_172033.cam', sizeBytes: 240128, created: '2026-03-19T17:20:33' },
    { filename: 'neverwinter2_backup_20260319_063301.cam', path: 'C:\\NWN2\\campaigns\\backups\\neverwinter2_backup_20260319_063301.cam', sizeBytes: 239616, created: '2026-03-19T06:33:01' },
    { filename: 'neverwinter2_backup_20260318_220833.cam', path: 'C:\\NWN2\\campaigns\\backups\\neverwinter2_backup_20260318_220833.cam', sizeBytes: 238592, created: '2026-03-18T22:08:33' },
    { filename: 'neverwinter2_backup_20260317_110245.cam', path: 'C:\\NWN2\\campaigns\\backups\\neverwinter2_backup_20260317_110245.cam', sizeBytes: 237056, created: '2026-03-17T11:02:45' },
    { filename: 'neverwinter2_backup_20260316_194518.cam', path: 'C:\\NWN2\\campaigns\\backups\\neverwinter2_backup_20260316_194518.cam', sizeBytes: 236032, created: '2026-03-16T19:45:18' },
    { filename: 'neverwinter2_backup_20260315_084712.cam', path: 'C:\\NWN2\\campaigns\\backups\\neverwinter2_backup_20260315_084712.cam', sizeBytes: 234496, created: '2026-03-15T08:47:12' },
    { filename: 'neverwinter2_backup_20260314_153900.cam', path: 'C:\\NWN2\\campaigns\\backups\\neverwinter2_backup_20260314_153900.cam', sizeBytes: 233472, created: '2026-03-14T15:39:00' },
  ],
};
