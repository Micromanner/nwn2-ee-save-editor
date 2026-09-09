import { Card, Elevation } from '@blueprintjs/core';
import type { IconType } from 'react-icons';
import {
  GiVisoredHelm, GiMirrorMirror, GiFist, GiLayeredArmor, GiSkills,
  GiStarMedal, GiSpellBook, GiSwapBag, GiEarthAmerica, GiCube,
  GiScrollUnfurled, GiOpenFolder, GiBackwardTime, GiCog, GiQuillInk,
  GiScrollQuill, GiMagnifyingGlass, GiCheckMark, GiCancel, GiHazardSign,
  GiFullFolder, GiBrokenShield, GiAnvil, GiMuscleUp, GiUpgrade,
  GiPointing, GiAnticlockwiseRotation, GiFunnel, GiTiedScroll,
  GiPlayButton, GiInfo, GiArmorUpgrade, GiExitDoor,
  GiFoldedPaper,
} from 'react-icons/gi';
import { T } from '../theme';
import { GameIcon } from '../shared/GameIcon';

interface IconEntry {
  icon: IconType;
  name: string;
  usage: string;
}

const ICON_GROUPS: { title: string; icons: IconEntry[] }[] = [
  {
    title: 'Navigation (Sidebar Fallbacks)',
    icons: [
      { icon: GiVisoredHelm, name: 'GiVisoredHelm', usage: 'Overview / Character' },
      { icon: GiMirrorMirror, name: 'GiMirrorMirror', usage: 'Appearance' },
      { icon: GiFist, name: 'GiFist', usage: 'Abilities' },
      { icon: GiLayeredArmor, name: 'GiLayeredArmor', usage: 'Classes' },
      { icon: GiSkills, name: 'GiSkills', usage: 'Skills' },
      { icon: GiStarMedal, name: 'GiStarMedal', usage: 'Feats' },
      { icon: GiSpellBook, name: 'GiSpellBook', usage: 'Spells' },
      { icon: GiSwapBag, name: 'GiSwapBag', usage: 'Inventory / Backpack' },
      { icon: GiEarthAmerica, name: 'GiEarthAmerica', usage: 'Game State' },
      { icon: GiCube, name: 'GiCube', usage: 'Models' },
    ],
  },
  {
    title: 'Actions',
    icons: [
      { icon: GiScrollUnfurled, name: 'GiScrollUnfurled', usage: 'Import / Export' },
      { icon: GiOpenFolder, name: 'GiOpenFolder', usage: 'Open folder' },
      { icon: GiBackwardTime, name: 'GiBackwardTime', usage: 'History / Backups' },
      { icon: GiCog, name: 'GiCog', usage: 'Settings' },
      { icon: GiQuillInk, name: 'GiQuillInk', usage: 'Edit' },
      { icon: GiTiedScroll, name: 'GiTiedScroll', usage: 'Save' },
      { icon: GiExitDoor, name: 'GiExitDoor', usage: 'Back / Return' },
      { icon: GiAnticlockwiseRotation, name: 'GiAnticlockwiseRotation', usage: 'Reset / Undo / Revert' },
      { icon: GiFunnel, name: 'GiFunnel', usage: 'Clear filters' },
      { icon: GiPlayButton, name: 'GiPlayButton', usage: 'Launch game' },
    ],
  },
  {
    title: 'Status Indicators',
    icons: [
      { icon: GiCheckMark, name: 'GiCheckMark', usage: 'Prerequisite met / Valid' },
      { icon: GiCancel, name: 'GiCancel', usage: 'Prerequisite unmet / Failed' },
      { icon: GiHazardSign, name: 'GiHazardSign', usage: 'Warning' },
      { icon: GiBrokenShield, name: 'GiBrokenShield', usage: 'Error state' },
      { icon: GiInfo, name: 'GiInfo', usage: 'Info state' },
    ],
  },
  {
    title: 'Level Helper',
    icons: [
      { icon: GiUpgrade, name: 'GiUpgrade', usage: 'Level-up FAB button' },
      { icon: GiMuscleUp, name: 'GiMuscleUp', usage: 'Ability score increase' },
    ],
  },
  {
    title: 'Placeholders & File Types',
    icons: [
      { icon: GiScrollQuill, name: 'GiScrollQuill', usage: 'Missing thumbnail' },
      { icon: GiMagnifyingGlass, name: 'GiMagnifyingGlass', usage: 'No results / Search' },
      { icon: GiPointing, name: 'GiPointing', usage: 'Select item prompt' },
      { icon: GiAnvil, name: 'GiAnvil', usage: 'Coming soon' },
      { icon: GiFullFolder, name: 'GiFullFolder', usage: 'Folder (closed)' },
      { icon: GiFoldedPaper, name: 'GiFoldedPaper', usage: 'Document / File' },
    ],
  },
  {
    title: 'Inventory',
    icons: [
      { icon: GiArmorUpgrade, name: 'GiArmorUpgrade', usage: 'Equip / Unequip' },
    ],
  },
];

function IconCard({ icon, name, usage }: IconEntry) {
  return (
    <div style={{
      display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8,
      padding: 16, background: T.surfaceAlt, borderRadius: 6,
      border: `1px solid ${T.borderLight}`, minWidth: 140,
    }}>
      <GameIcon icon={icon} size={32} color={T.accent} />
      <code className="t-xs t-center" style={{ color: T.text, wordBreak: 'break-all' }}>{name}</code>
      <span className="t-sm t-center" style={{ color: T.textMuted }}>{usage}</span>
    </div>
  );
}

export function IconShowcasePanel() {
  return (
    <div style={{ padding: 16, overflow: 'auto', height: '100%' }}>
      <div style={{ marginBottom: 16 }}>
        <h2 className="t-2xl" style={{ margin: 0, color: T.accent }}>Icon Showcase</h2>
        <p style={{ color: T.textMuted, marginTop: 4 }}>
          All game-icons.net icons used in this application, via react-icons/gi.
        </p>
      </div>
      {ICON_GROUPS.map(group => (
        <Card key={group.title} elevation={Elevation.ONE} style={{
          marginBottom: 16, padding: 16, background: T.surface,
        }}>
          <h3 className="t-bold" style={{ margin: '0 0 12px', color: T.text }}>{group.title}</h3>
          <div style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fill, minmax(150px, 1fr))',
            gap: 12,
          }}>
            {group.icons.map(entry => (
              <IconCard key={entry.name} {...entry} />
            ))}
          </div>
        </Card>
      ))}
    </div>
  );
}
