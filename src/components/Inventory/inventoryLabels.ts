type Translate = (key: string, values?: Record<string, unknown>) => string;

export const DISPLAY_SLOT_TO_API: Record<string, string> = {
  mainHand: 'right_hand',
  offHand: 'left_hand',
  head: 'head',
  chest: 'chest',
  hands: 'gloves',
  feet: 'boots',
  cloak: 'cloak',
  ring1: 'left_ring',
  ring2: 'right_ring',
  belt: 'belt',
  amulet: 'neck',
  arrows: 'arrows',
  bullets: 'bullets',
  bolts: 'bolts',
};

const CATEGORY_KEYS: Record<string, string> = {
  'Armor & Clothing': 'armorAndClothing',
  'Weapons': 'weapons',
  'Magic Items': 'magicItems',
  'Accessories': 'accessories',
  'Miscellaneous': 'miscellaneous',
};

export function categoryLabel(t: Translate, category: string): string {
  const key = CATEGORY_KEYS[category];
  return key ? t(`inventory.categories.${key}`) : category;
}

const SOURCE_KEYS: Record<string, string> = {
  base_game: 'baseGame',
  module: 'module',
  campaign: 'campaign',
  override_dir: 'overrideDir',
  workshop: 'workshop',
  custom_override: 'customOverride',
  hak: 'hak',
};

export function sourceLabel(t: Translate, source: string): string {
  const key = SOURCE_KEYS[source];
  return key ? t(`inventory.source.names.${key}`) : source;
}
