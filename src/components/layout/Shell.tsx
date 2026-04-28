import { useState, useEffect } from 'react';
import { Button, Card, Elevation, ProgressBar } from '@blueprintjs/core';
import { GiAnvil } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { invoke } from '@tauri-apps/api/core';
import { relaunch } from '@tauri-apps/plugin-process';
import { TauriAPI } from '@/lib/tauri-api';
import { CharacterProvider, useCharacterContext } from '@/contexts/CharacterContext';
import { useTranslations } from '@/hooks/useTranslations';
import { T, PATTERN_BG } from '../theme';
import '../blueprint.css';
import { TitleBar } from './TitleBar';
import { Navbar } from './Navbar';
import { Sidebar, preloadSidebarIcons } from './Sidebar';
import { OverviewPanel } from '../Overview/OverviewPanel';
import { AbilitiesPanel } from '../AbilityScores/AbilitiesPanel';
import { ClassesPanel } from '../ClassesLevel/ClassesPanel';
import { SkillsPanel } from '../Skills/SkillsPanel';
import { FeatsPanel } from '../Feats/FeatsPanel';
import { SpellsPanel } from '../Spells/SpellsPanel';
import { InventoryPanel } from '../Inventory/InventoryPanel';
import { GameStatePanel } from '../GameState/GameStatePanel';
import { ModelBrowser } from '../ModelViewer/ModelBrowser';
import { IconShowcasePanel } from '../IconShowcase/IconShowcasePanel';
import { AppearancePanel } from '../Appearance/AppearancePanel';
import DashboardPanel from '../Dashboard/DashboardPanel';
import { SettingsDialog } from '../Settings/SettingsPanel';
import { LevelHelper, ErrorBoundary, AboutDialog } from '../shared';

const PANELS: Record<string, React.ComponentType> = {
  overview: OverviewPanel,
  appearance: AppearancePanel,
  abilities: AbilitiesPanel,
  classes: ClassesPanel,
  skills: SkillsPanel,
  feats: FeatsPanel,
  spells: SpellsPanel,
  inventory: InventoryPanel,
  gamestate: GameStatePanel,
  models: ModelBrowser,
  icons: IconShowcasePanel,
};

function ShellContent() {
  const [activeTab, setActiveTab] = useState('overview');
  const [showAbout, setShowAbout] = useState(false);
  const [viewMode, setViewMode] = useState<'dashboard' | 'editor'>('dashboard');
  const { character, isLoading, clearCharacter } = useCharacterContext();
  const t = useTranslations();
  const Panel = PANELS[activeTab];

  useEffect(() => {
    preloadSidebarIcons();
  }, []);

  useEffect(() => {
    if (character && !isLoading && viewMode === 'dashboard') {
      setViewMode('editor');
    }
    if (!character) {
      setViewMode('dashboard');
    }
  }, [character, isLoading]);

  const handleBackToDashboard = () => setViewMode('dashboard');

  const handleCloseSession = () => {
    clearCharacter();
    setViewMode('dashboard');
  };

  if (viewMode === 'dashboard' || !character) {
    return (
      <div className="bp-app" style={{ display: 'flex', flexDirection: 'column', height: '100vh', background: T.bg }}>
        <TitleBar onAboutClick={() => setShowAbout(true)} />
        {character && (
          <div style={{
            display: 'flex', alignItems: 'center', justifyContent: 'space-between',
            padding: '8px 16px', background: T.sectionBg, borderBottom: `1px solid ${T.sectionBorder}`,
          }}>
            <span className="t-md" style={{ color: T.text }}>
              <strong>{t('dashboard.sessionActive', { name: character.name })}</strong>
            </span>
            <div style={{ display: 'flex', gap: 8 }}>
              <Button small intent="primary" text={t('dashboard.continueEditing')} onClick={() => setViewMode('editor')} />
              <Button small text={t('dashboard.closeSession')} onClick={handleCloseSession} />
            </div>
          </div>
        )}
        <DashboardPanel />
        <AboutDialog isOpen={showAbout} onClose={() => setShowAbout(false)} />
      </div>
    );
  }

  return (
    <div className="bp-app" style={{ display: 'flex', flexDirection: 'column', height: '100vh', background: T.bg }}>
      <TitleBar onAboutClick={() => setShowAbout(true)} />
      <Navbar onBack={handleBackToDashboard} />
      <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
        <Sidebar activeTab={activeTab} onTabChange={setActiveTab} />
        <div style={{ flex: 1, overflow: 'hidden' }}>
          <Card elevation={Elevation.ONE} style={{
            margin: 0, padding: 0, height: '100%', borderRadius: 0, overflow: 'auto',
            background: T.surface,
            backgroundImage: PATTERN_BG,
            backgroundSize: '200px 200px',
          }}>
            <LevelHelper onNavigate={setActiveTab} />
            <ErrorBoundary key={activeTab}>
              {Panel ? <Panel /> : (
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
                  <div style={{ textAlign: 'center' }}>
                    <GameIcon icon={GiAnvil} size={40} style={{ color: T.border }} />
                    <p className="t-lg" style={{ marginTop: 12, color: T.textMuted }}>Coming soon</p>
                  </div>
                </div>
              )}
            </ErrorBoundary>
          </Card>
        </div>
      </div>
      <AboutDialog isOpen={showAbout} onClose={() => setShowAbout(false)} />
    </div>
  );
}

export default function Shell() {
  const t = useTranslations();
  const [initReady, setInitReady] = useState(false);
  const [initProgress, setInitProgress] = useState({ step: 'initializing', progress: 0, message: 'Starting up...' });
  const [settingsOpen, setSettingsOpen] = useState(false);

  useEffect(() => {
    invoke('show_main_window').catch(() => {});
    TauriAPI.initializeGameData().catch(err => {
      console.error('Failed to initialize game data:', err);
    });

    let cancelled = false;
    const poll = async () => {
      try {
        const status = await TauriAPI.getInitializationStatus();
        if (cancelled) return;
        setInitProgress(status);
        if (status.step === 'ready') {
          setInitReady(true);
          return;
        }
      } catch {
        // backend not ready yet
      }
      if (!cancelled) setTimeout(poll, 50);
    };
    poll();

    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    if (initProgress.step === 'needsConfiguration') {
      setSettingsOpen(true);
    }
  }, [initProgress.step]);

  if (initProgress.step === 'needsConfiguration') {
    return (
      <div className="bp-app" style={{
        display: 'flex', flexDirection: 'column',
        height: '100vh', background: T.bg,
        backgroundImage: PATTERN_BG, backgroundSize: '200px 200px',
      }}>
        <TitleBar onAboutClick={() => {}} />
        <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <div style={{ width: 420, textAlign: 'center' }}>
            <div className="t-2xl t-bold" style={{ color: T.accent, marginBottom: 12 }}>
              {t('startup.notFound.title')}
            </div>
            <div className="t-base" style={{ color: T.textMuted, marginBottom: 20 }}>
              {t('startup.notFound.description')}
            </div>
            <Button
              intent="primary"
              text={t('startup.notFound.retry')}
              onClick={() => { relaunch().catch(err => console.error('Relaunch failed:', err)); }}
            />
          </div>
        </div>
        <SettingsDialog
          isOpen={settingsOpen}
          onClose={() => setSettingsOpen(false)}
          defaultTab="paths"
        />
      </div>
    );
  }

  if (!initReady) {
    return (
      <div className="bp-app" style={{
        display: 'flex', flexDirection: 'column',
        height: '100vh', background: T.bg,
        backgroundImage: PATTERN_BG, backgroundSize: '200px 200px',
      }}>
        <TitleBar onAboutClick={() => {}} />
        <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <div style={{ width: 320, textAlign: 'center' }}>
            <div className="t-2xl t-bold" style={{ color: T.accent, marginBottom: 16 }}>
              NWN2 Save Editor
            </div>
            <ProgressBar
              value={initProgress.progress / 100}
              intent="primary"
              animate={initProgress.step !== 'ready'}
              stripes={false}
              style={{ marginBottom: 8 }}
            />
            <div className="t-base" style={{ color: T.textMuted }}>{initProgress.message}</div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <CharacterProvider>
      <ShellContent />
    </CharacterProvider>
  );
}
