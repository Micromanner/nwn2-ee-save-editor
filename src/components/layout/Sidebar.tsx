import { useEffect } from 'react';
import type { IconType } from 'react-icons';
import { GiVisoredHelm, GiMirrorMirror, GiFist, GiLayeredArmor, GiSkills, GiStarMedal, GiSpellBook, GiSwapBag, GiEarthAmerica, GiCube } from 'react-icons/gi';
import { T } from '../theme';
import { useTranslations } from '@/hooks/useTranslations';
import { useIcon, fetchIcon } from '@/hooks/useIcon';
import { GameIcon } from '../shared/GameIcon';
import { RosterSection } from './RosterSection';
import { useCharacterContext } from '@/contexts/CharacterContext';

const NAV_ITEMS: { id: string; icon: IconType; gameIcon: string | null; labelKey: string }[] = [
  { id: 'overview', icon: GiVisoredHelm, gameIcon: 'ia_character', labelKey: 'navigation.overview' },
  { id: 'appearance', icon: GiMirrorMirror, gameIcon: 'ia_appear', labelKey: 'navigation.appearance' },
  { id: 'abilities', icon: GiFist, gameIcon: 'ife_toughness', labelKey: 'navigation.abilityScores' },
  { id: 'classes', icon: GiLayeredArmor, gameIcon: 'ic_b_fighter', labelKey: 'navigation.classes' },
  { id: 'skills', icon: GiSkills, gameIcon: 'isk_lore', labelKey: 'navigation.skills' },
  { id: 'feats', icon: GiStarMedal, gameIcon: 'ife_dodge', labelKey: 'navigation.feats' },
  { id: 'spells', icon: GiSpellBook, gameIcon: 'b_spellbook', labelKey: 'navigation.spells' },
  { id: 'inventory', icon: GiSwapBag, gameIcon: 'ia_inventory', labelKey: 'navigation.inventory' },
  { id: 'gamestate', icon: GiEarthAmerica, gameIcon: 'b_journal', labelKey: 'navigation.gameState' },
  { id: 'models', icon: GiCube, gameIcon: 'is_trueseeing', labelKey: 'navigation.models' },
];

function NavIcon({ gameIcon, fallback, size }: { gameIcon: string | null; fallback: IconType; size: number }) {
  const iconUrl = useIcon(gameIcon);
  if (iconUrl) {
    return <img src={iconUrl} alt="" width={size} height={size} style={{ borderRadius: 2, flexShrink: 0 }} />;
  }
  return <GameIcon icon={fallback} size={size} />;
}

interface SidebarProps {
  activeTab: string;
  onTabChange: (id: string) => void;
}

export function preloadSidebarIcons() {
  NAV_ITEMS.forEach(item => {
    if (item.gameIcon) fetchIcon(item.gameIcon);
  });
}

export function Sidebar({ activeTab, onTabChange }: SidebarProps) {
  const t = useTranslations();
  const { activeSource } = useCharacterContext();

  useEffect(() => {
    preloadSidebarIcons();
  }, []);

  const visibleItems = activeSource.kind === 'player'
    ? NAV_ITEMS
    : NAV_ITEMS.filter(item => item.id !== 'appearance' && item.id !== 'models');

  return (
    <div style={{
      width: 200, flexShrink: 0,
      background: T.sidebar,
      borderRight: `1px solid ${T.sidebar}`,
      display: 'flex', flexDirection: 'column',
      padding: '8px 0',
      position: 'relative', zIndex: 5,
    }}>
      {visibleItems.map(item => {
        const active = activeTab === item.id;
        return (
          <button
            key={item.id}
            onClick={() => onTabChange(item.id)}
            className="t-md"
            style={{
              display: 'flex', alignItems: 'center', gap: 8,
              width: '100%', padding: '8px 16px',
              border: 'none', cursor: 'pointer',
              background: active ? 'rgba(160,82,45,0.12)' : 'transparent',
              color: active ? T.sidebarAccent : T.sidebarText,
              textAlign: 'left',
              borderLeft: active ? `2px solid ${T.sidebarAccent}` : '2px solid transparent',
              transition: 'all 0.15s',
            }}
          >
            <NavIcon gameIcon={item.gameIcon} fallback={item.icon} size={24} />
            {t(item.labelKey)}
          </button>
        );
      })}
      <RosterSection activeTab={activeTab} onTabChange={onTabChange} />
    </div>
  );
}
