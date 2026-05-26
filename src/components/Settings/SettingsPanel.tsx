import { useState, useEffect } from 'react';
import {
  AnchorButton, Button, Menu, MenuItem, Popover, Switch, Tab, Tabs, Tag,
} from '@blueprintjs/core';
import { GiFullFolder, GiCheckMark, GiBrokenShield } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { useTranslations } from '@/hooks/useTranslations';
import { useLocale } from '@/providers/LocaleProvider';
import { useFontSize } from '@/providers/FontSizeProvider';
import { usePathsChangedEvent } from '@/hooks/usePathsChangedEvent';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useUpdateCheck } from '@/hooks/useUpdateCheck';
import { pathService, PathConfig } from '@/lib/api/paths';
import { T } from '../theme';
import { KVRow, ParchmentDialog } from '../shared';

function SectionHeader({ title }: { title: string }) {
  return (
    <div className="t-bold" style={{ color: T.textMuted, borderBottom: `1px solid ${T.borderLight}`, paddingBottom: 4, marginBottom: 10 }}>
      {title}
    </div>
  );
}

function PathRow({ label, path, exists, autoDetected, onEdit, onReset, rowKey, busyKey }: {
  label: string;
  path: string | null;
  exists: boolean;
  autoDetected: boolean;
  onEdit?: () => void;
  onReset?: () => void;
  rowKey?: string;
  busyKey?: string | null;
}) {
  const t = useTranslations();
  const editKey = rowKey ? `${rowKey}:edit` : null;
  const resetKey = rowKey ? `${rowKey}:reset` : null;
  const isBusy = busyKey != null;
  return (
    <div style={{
      display: 'flex', alignItems: 'center', justifyContent: 'space-between',
      padding: '10px 0', borderBottom: `1px solid ${T.borderLight}`,
    }}>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <GameIcon icon={GiFullFolder} size={16} color={T.textMuted} />
          <span className="t-semibold" style={{ color: T.text }}>{label}</span>
          {autoDetected && !path ? (
            <Tag minimal round intent="danger" style={{ background: 'rgba(218, 85, 85, 0.1)' }}>
              {t('settings.paths.failedToFind')}
            </Tag>
          ) : (
            <Tag minimal round intent={autoDetected ? 'primary' : 'warning'} style={{ background: autoDetected ? 'rgba(45, 114, 210, 0.1)' : 'rgba(184, 149, 47, 0.1)' }}>
              {autoDetected ? t('settings.paths.autoDetected') : t('settings.paths.manuallySet')}
            </Tag>
          )}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 4 }}>
          <span className="t-mono" style={{ color: T.textMuted, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {path || t('settings.paths.notConfigured')}
          </span>
          {path && (
            <GameIcon icon={exists ? GiCheckMark : GiBrokenShield} size={14} color={exists ? T.positive : T.negative} />
          )}
        </div>
      </div>
      <div style={{ display: 'flex', gap: 4, marginLeft: 12 }}>
        {!autoDetected && onReset && (
          <Button
            small
            minimal
            text={t('settings.paths.reset')}
            onClick={onReset}
            loading={busyKey === resetKey}
            disabled={isBusy && busyKey !== resetKey}
            style={{ color: T.textMuted }}
          />
        )}
        {onEdit && (
          <Button
            small
            outlined
            text={path ? t('settings.paths.change') : t('settings.paths.set')}
            onClick={onEdit}
            loading={busyKey === editKey}
            disabled={isBusy && busyKey !== editKey}
          />
        )}
      </div>
    </div>
  );
}

function CustomFolderList({ folders, addLabel, onAdd, onRemove, rowKey, busyKey }: {
  folders: { path: string; exists: boolean }[];
  addLabel: string;
  onAdd: () => void;
  onRemove: (path: string) => void;
  rowKey?: string;
  busyKey?: string | null;
}) {
  const addKey = rowKey ? `${rowKey}:add` : null;
  const isBusy = busyKey != null;
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {folders.map((folder) => {
        const removeKey = rowKey ? `${rowKey}:remove:${folder.path}` : null;
        return (
          <div
            key={folder.path}
            style={{
              display: 'flex', alignItems: 'center', justifyContent: 'space-between',
              padding: '8px 12px', background: T.surfaceAlt, borderRadius: 4,
              border: `1px solid ${T.borderLight}`,
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0, flex: 1 }}>
              <GameIcon icon={GiFullFolder} size={14} color={T.textMuted} />
              <span className="t-mono" style={{ color: T.text, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                {folder.path}
              </span>
              <GameIcon icon={folder.exists ? GiCheckMark : GiBrokenShield} size={14} color={folder.exists ? T.positive : T.negative} />
            </div>
            <Button
              small
              minimal
              icon="trash"
              onClick={() => onRemove(folder.path)}
              loading={busyKey === removeKey}
              disabled={isBusy && busyKey !== removeKey}
              style={{ color: T.textMuted }}
            />
          </div>
        );
      })}
      <Button
        small
        outlined
        icon="plus"
        text={addLabel}
        onClick={onAdd}
        loading={busyKey === addKey}
        disabled={isBusy && busyKey !== addKey}
        style={{ alignSelf: 'flex-start' }}
      />
    </div>
  );
}

function GeneralTab() {
  const { setLocale } = useLocale();
  const t = useTranslations();
  const { fontSize, setFontSize: applyFontSize } = useFontSize();
  const [language, setLanguage] = useState('en');
  const [debugExporting, setDebugExporting] = useState(false);
  const [debugResult, setDebugResult] = useState<{ success: boolean; message: string } | null>(null);

  useEffect(() => {
    invoke<{ language: string }>('get_app_config')
      .then((config) => {
        setLanguage(config.language);
      })
      .catch(() => {});
  }, []);

  const handleLanguageChange = (newLocale: string) => {
    setLanguage(newLocale);
    setLocale(newLocale);
    invoke('update_app_config', { updates: { language: newLocale } }).catch(() => {});
  };

  const handleFontSizeChange = (size: string) => {
    applyFontSize(size as 'small' | 'medium' | 'large');
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      <div>
        <SectionHeader title={t('settings.general.languageRegion')} />
        <KVRow label={t('settings.general.language')} value={
          <Popover
            content={
              <Menu>
                <MenuItem text={t('settings.general.languageEnglish')} active={language === 'en'} onClick={() => handleLanguageChange('en')} />
                <MenuItem text={t('settings.general.languageGerman')} active={language === 'de'} onClick={() => handleLanguageChange('de')} />
                <MenuItem text={t('settings.general.languageSpanish')} active={language === 'es'} onClick={() => handleLanguageChange('es')} />
                <MenuItem text={t('settings.general.languageFrench')} active={language === 'fr'} onClick={() => handleLanguageChange('fr')} />
                <MenuItem text={t('settings.general.languagePolish')} active={language === 'pl'} onClick={() => handleLanguageChange('pl')} />
                <MenuItem text={t('settings.general.languageRussian')} active={language === 'ru'} onClick={() => handleLanguageChange('ru')} />
              </Menu>
            }
            placement="bottom-end"
            minimal
          >
            <Button
              minimal
              rightIcon="caret-down"
              text={t(`settings.general.language${
                language === 'en' ? 'English' :
                language === 'de' ? 'German' :
                language === 'es' ? 'Spanish' :
                language === 'fr' ? 'French' :
                language === 'pl' ? 'Polish' :
                language === 'ru' ? 'Russian' : 'English'
              }`)}
              className="t-semibold"
            />
          </Popover>
        } />
      </div>

      <div>
        <SectionHeader title={t('settings.general.display')} />
        <KVRow label={t('settings.general.fontSize')} value={
          <Popover
            content={
              <Menu>
                <MenuItem text={t('settings.general.fontSizeSmall')} active={fontSize === 'small'} onClick={() => handleFontSizeChange('small')} />
                <MenuItem text={t('settings.general.fontSizeMedium')} active={fontSize === 'medium'} onClick={() => handleFontSizeChange('medium')} />
                <MenuItem text={t('settings.general.fontSizeLarge')} active={fontSize === 'large'} onClick={() => handleFontSizeChange('large')} />
              </Menu>
            }
            placement="bottom-end"
            minimal
          >
            <Button minimal rightIcon="caret-down" text={t(`settings.general.fontSize${fontSize.charAt(0).toUpperCase() + fontSize.slice(1)}`)} className="t-semibold" />
          </Popover>
        } />
      </div>

      <div>
        <SectionHeader title={t('settings.debug.title')} />
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <Button
            outlined
            text={debugExporting ? t('settings.debug.exporting') : t('settings.debug.exportButton')}
            disabled={debugExporting}
            onClick={async () => {
              setDebugExporting(true);
              setDebugResult(null);
              try {
                const filePath = await invoke<string>('export_debug_log');
                setDebugResult({ success: true, message: `${t('settings.debug.exportSuccess')} ${filePath}` });
              } catch {
                setDebugResult({ success: false, message: t('settings.debug.exportError') });
              } finally {
                setDebugExporting(false);
              }
            }}
          />
        </div>
        {debugResult && (
          <div style={{
            marginTop: 8, padding: '8px 12px', borderRadius: 4,
            background: debugResult.success ? 'rgba(46, 125, 50, 0.1)' : 'rgba(198, 40, 40, 0.1)',
            border: `1px solid ${debugResult.success ? T.positive : T.negative}`,
            color: debugResult.success ? T.positive : T.negative,
          }}>
            {debugResult.message}
          </div>
        )}
      </div>

      <UpdatesSection />
    </div>
  );
}

function UpdatesSection() {
  const t = useTranslations();
  const { result, checkNow } = useUpdateCheck();
  const checking = result.kind === 'checking';

  return (
    <div>
      <SectionHeader title={t('settings.updates.title')} />
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12, flexWrap: 'wrap' }}>
          <Button
            outlined
            text={checking ? t('settings.updates.checking') : t('settings.updates.checkButton')}
            disabled={checking}
            onClick={checkNow}
          />
          {result.kind === 'upToDate' && (
            <span style={{ color: T.positive }}>
              {t('settings.updates.upToDate', { current: result.current })}
            </span>
          )}
          {result.kind === 'available' && (
            <>
              <span style={{ color: T.accent }}>
                {t('settings.updates.newAvailable', { current: result.current, latest: result.latest })}
              </span>
              <AnchorButton
                small
                intent="primary"
                href={result.htmlUrl}
                target="_blank"
                rel="noreferrer"
                text={t('settings.updates.openRelease')}
              />
            </>
          )}
          {result.kind === 'error' && (
            <span style={{ color: T.negative }}>{t('settings.updates.error')}</span>
          )}
        </div>
      </div>
    </div>
  );
}

function PathsTab() {
  const t = useTranslations();
  const [paths, setPaths] = useState<PathConfig | null>(null);
  const [busyKey, setBusyKey] = useState<string | null>(null);

  useEffect(() => {
    pathService.getConfig().then((response) => setPaths(response.paths)).catch(() => {});
  }, []);

  usePathsChangedEvent(setPaths);

  const selectFolder = async (title: string): Promise<string | null> => {
    try {
      const selected = await open({ directory: true, multiple: false, title });
      return selected as string | null;
    } catch { return null; }
  };

  const { handleError } = useErrorHandler();

  const runBusy = async (key: string, action: () => Promise<unknown>) => {
    setBusyKey(key);
    try { await action(); } catch (error) { handleError(error); }
    finally { setBusyKey(null); }
  };

  const updatePath = async (type: 'game' | 'documents' | 'workshop') => {
    const titleKeys: Record<string, string> = {
      game: t('settings.paths.selectGameFolder'),
      documents: t('settings.paths.selectDocumentsFolder'),
      workshop: t('settings.paths.selectWorkshopFolder'),
    };
    const selected = await selectFolder(titleKeys[type]);
    if (!selected) return;
    await runBusy(`${type}:edit`, async () => {
      switch (type) {
        case 'game': await pathService.setGameFolder(selected); break;
        case 'documents': await pathService.setDocumentsFolder(selected); break;
        case 'workshop': await pathService.setSteamWorkshopFolder(selected); break;
      }
    });
  };

  const resetPath = async (type: 'game' | 'documents' | 'workshop') => {
    await runBusy(`${type}:reset`, async () => {
      switch (type) {
        case 'game': await pathService.resetGameFolder(); break;
        case 'documents': await pathService.resetDocumentsFolder(); break;
        case 'workshop': await pathService.resetSteamWorkshopFolder(); break;
      }
    });
  };

  const addCustomFolder = async (type: 'override' | 'hak') => {
    const title = type === 'override' ? t('settings.paths.selectOverrideFolder') : t('settings.paths.selectHakFolder');
    const selected = await selectFolder(title);
    if (!selected) return;
    await runBusy(`${type}:add`, async () => {
      if (type === 'override') {
        await pathService.addOverrideFolder(selected);
      } else {
        await pathService.addHakFolder(selected);
      }
    });
  };

  const removeCustomFolder = async (type: 'override' | 'hak', path: string) => {
    await runBusy(`${type}:remove:${path}`, async () => {
      if (type === 'override') {
        await pathService.removeOverrideFolder(path);
      } else {
        await pathService.removeHakFolder(path);
      }
    });
  };

  if (!paths) {
    return <div style={{ color: T.textMuted, padding: 16 }}>{t('settings.debug.exporting')}</div>;
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      <div>
        <SectionHeader title={t('settings.paths.mainPaths')} />
        <PathRow rowKey="game" busyKey={busyKey} label={t('settings.paths.gameFolder')} path={paths.game_folder.path} exists={paths.game_folder.exists} autoDetected={paths.game_folder.source === 'auto'} onEdit={() => updatePath('game')} onReset={() => resetPath('game')} />
        <PathRow rowKey="documents" busyKey={busyKey} label={t('settings.paths.documentsFolder')} path={paths.documents_folder.path} exists={paths.documents_folder.exists} autoDetected={paths.documents_folder.source === 'auto'} onEdit={() => updatePath('documents')} onReset={() => resetPath('documents')} />
        <PathRow rowKey="workshop" busyKey={busyKey} label={t('settings.paths.workshopFolder')} path={paths.steam_workshop_folder.path} exists={paths.steam_workshop_folder.exists} autoDetected={paths.steam_workshop_folder.source === 'auto'} onEdit={() => updatePath('workshop')} onReset={() => resetPath('workshop')} />
        <PathRow label={t('settings.paths.localvaultFolder')} path={paths.localvault_folder.path} exists={paths.localvault_folder.exists} autoDetected={paths.localvault_folder.source === 'derived'} />
      </div>

      <div>
        <SectionHeader title={t('settings.paths.customOverrideFolders')} />
        <CustomFolderList rowKey="override" busyKey={busyKey} folders={paths.custom_override_folders} addLabel={t('settings.paths.addOverrideFolder')} onAdd={() => addCustomFolder('override')} onRemove={(p) => removeCustomFolder('override', p)} />
      </div>

      <div>
        <SectionHeader title={t('settings.paths.customHakFolders')} />
        <CustomFolderList rowKey="hak" busyKey={busyKey} folders={paths.custom_hak_folders} addLabel={t('settings.paths.addHakFolder')} onAdd={() => addCustomFolder('hak')} onRemove={(p) => removeCustomFolder('hak', p)} />
      </div>
    </div>
  );
}

function GameLaunchTab() {
  const t = useTranslations();
  const [showLaunchDialog, setShowLaunchDialog] = useState(true);
  const [autoCloseOnLaunch, setAutoCloseOnLaunch] = useState(false);

  useEffect(() => {
    invoke<{ show_launch_dialog: boolean; auto_close_on_launch: boolean }>('get_app_config')
      .then((config) => {
        setShowLaunchDialog(config.show_launch_dialog);
        setAutoCloseOnLaunch(config.auto_close_on_launch);
      })
      .catch(() => {});
  }, []);

  const updateSetting = (key: string, value: boolean) => {
    invoke('update_app_config', { updates: { [key]: value } }).catch(() => {});
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      <div>
        <SectionHeader title={t('settings.launch.launchBehavior')} />
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          <Switch
            checked={showLaunchDialog}
            onChange={() => {
              const next = !showLaunchDialog;
              setShowLaunchDialog(next);
              updateSetting('show_launch_dialog', next);
            }}
            label={t('settings.launch.showLaunchDialog')}
            style={{ marginBottom: 0 }}
          />
          <Switch
            checked={autoCloseOnLaunch}
            onChange={() => {
              const next = !autoCloseOnLaunch;
              setAutoCloseOnLaunch(next);
              updateSetting('auto_close_on_launch', next);
            }}
            label={t('settings.launch.autoClose')}
            style={{ marginBottom: 0 }}
          />
        </div>
      </div>
    </div>
  );
}

type SettingsTabId = 'general' | 'paths' | 'launch';

function SettingsContent({ defaultTab = 'general' }: { defaultTab?: SettingsTabId }) {
  const t = useTranslations();
  const [activeTab, setActiveTab] = useState<SettingsTabId>(defaultTab);

  return (
    <>
      <div style={{ padding: '10px 16px 0' }}>
        <Tabs
          id="settings-tabs"
          selectedTabId={activeTab}
          onChange={(newTab) => setActiveTab(newTab as SettingsTabId)}
          renderActiveTabPanelOnly
          large
        >
          <Tab id="general" title={t('settings.tabs.general')} />
          <Tab id="paths" title={t('settings.tabs.paths')} />
          <Tab id="launch" title={t('settings.tabs.launch')} />
        </Tabs>
      </div>
      <div style={{ padding: 16, height: 420, overflowY: 'auto' }}>
        {activeTab === 'general' && <GeneralTab />}
        {activeTab === 'paths' && <PathsTab />}
        {activeTab === 'launch' && <GameLaunchTab />}
      </div>
    </>
  );
}

export function SettingsDialog({
  isOpen,
  onClose,
  defaultTab,
}: {
  isOpen: boolean;
  onClose: () => void;
  defaultTab?: SettingsTabId;
}) {
  const t = useTranslations();
  return (
    <ParchmentDialog
      isOpen={isOpen}
      onClose={onClose}
      title={t('settings.title')}
      width={680}
      minHeight={480}
      footerActions={
        <Button intent="primary" text={t('settings.done')} onClick={onClose} />
      }
    >
      <div style={{ margin: -16 }}>
        <SettingsContent defaultTab={defaultTab} />
      </div>
    </ParchmentDialog>
  );
}
