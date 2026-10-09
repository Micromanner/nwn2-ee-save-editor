import { formatDateTime } from '@/utils/dateFormat';

export const T = {
  bg: '#f0eadd',
  surface: '#faf7f2',
  surfaceAlt: '#f5f0e8',
  sidebar: '#1e1e22',
  sidebarText: '#c8c4bc',
  sidebarActive: '#e8dcc8',
  sidebarAccent: '#a0522d',
  navbar: '#2a2a2e',
  text: '#2d2d30',
  textMuted: '#4a453c',
  border: '#d9d0c1',
  borderLight: '#e5ddd0',
  accent: '#a0522d',
  accentLight: '#c4865a',
  positive: '#2e7d32',
  negative: '#c62828',
  gold: '#b8952f',
  sectionBg: '#eee8da',
  sectionBorder: '#c9bfae',
} as const;

export const FEAT_TYPE_COLORS: Record<string, string> = {
  'feats.categories.general': '#43a047',
  'feats.categories.proficiency': '#6d4c41',
  'feats.categories.skillSave': '#00acc1',
  'feats.categories.metamagic': '#8e24aa',
  'feats.categories.divine': '#f9a825',
  'feats.categories.epic': '#e53935',
  'feats.categories.class': '#1e88e5',
  'feats.categories.background': '#00897b',
  'feats.categories.spellcasting': '#5c6bc0',
  'feats.categories.history': '#f57f17',
  'feats.categories.heritage': '#c62828',
  'feats.categories.itemCreation': '#689f38',
  'feats.categories.racial': '#00838f',
  'feats.categories.domain': '#7b1fa2',
};

export const SPELL_SCHOOL_COLORS: Record<string, string> = {
  'spells.schools.abjuration': '#1565c0',
  'spells.schools.conjuration': '#2e7d32',
  'spells.schools.enchantment': '#7b1fa2',
  'spells.schools.evocation': '#d84315',
  'spells.schools.transmutation': '#0277bd',
  'spells.schools.necromancy': '#b71c1c',
  'spells.schools.divination': '#00838f',
  'spells.schools.illusion': '#ad1457',
  'spells.schools.universal': '#546e7a',
};

export function formatBytes(bytes: number) {
  if (bytes === 0) return '-';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}

export function formatUnixSeconds(dateString: string) {
  const ts = parseFloat(dateString);
  if (isNaN(ts)) return '-';
  return formatDateTime(ts * 1000);
}

const NOISE_SVG = encodeURIComponent(
  `<svg xmlns='http://www.w3.org/2000/svg' width='200' height='200'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='4' stitchTiles='stitch'/></filter><rect width='100%' height='100%' filter='url(#n)' opacity='0.15'/></svg>`
);
export const PATTERN_BG = `url("data:image/svg+xml,${NOISE_SVG}")`;
