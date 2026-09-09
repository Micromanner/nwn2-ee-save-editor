import { useState, useEffect } from 'react';
import { Button, Icon } from '@blueprintjs/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { getName } from '@tauri-apps/api/app';
import { T } from '../theme';
import { useTranslations } from '@/hooks/useTranslations';

interface TitleBarProps {
  onAboutClick: () => void;
}

export function TitleBar({ onAboutClick }: TitleBarProps) {
  const t = useTranslations();
  const [appName, setAppName] = useState('NWN2:EE Save Editor');
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    getName().then(setAppName).catch(() => {});
  }, []);

  useEffect(() => {
    const appWindow = getCurrentWindow();
    appWindow.isMaximized().then(setIsMaximized).catch(() => {});

    const unlisten = appWindow.onResized(() => {
      appWindow.isMaximized().then(setIsMaximized).catch(() => {});
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  const handleMinimize = () => getCurrentWindow().minimize().catch(() => {});
  const handleMaximize = () => {
    const w = getCurrentWindow();
    w.toggleMaximize().then(() => w.isMaximized().then(setIsMaximized)).catch(() => {});
  };
  const handleClose = () => getCurrentWindow().close().catch(() => {});

  return (
    <div
      data-tauri-drag-region
      style={{
        height: 32, display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        padding: '0 8px', background: T.navbar, borderBottom: '1px solid rgba(255,255,255,0.06)',
        userSelect: 'none', WebkitUserSelect: 'none', flexShrink: 0,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <img src="/logo.svg" alt="Logo" style={{ height: 18 }} />
        <span className="t-medium" style={{ color: T.sidebarText }}>{appName}</span>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: 2 }}>
        <Button
          icon={<Icon icon="info-sign" size={12} />}
          minimal small
          style={{ color: T.sidebarText }}
          title={t('titlebar.about')}
          onClick={onAboutClick}
        />
        <div style={{ width: 1, height: 14, background: 'rgba(255,255,255,0.1)', margin: '0 4px' }} />
        <Button
          icon={<Icon icon="minus" size={12} />}
          minimal small
          style={{ color: T.sidebarText }}
          title={t('titlebar.minimize')}
          onClick={handleMinimize}
        />
        <Button
          icon={<Icon icon={isMaximized ? 'duplicate' : 'maximize'} size={12} />}
          minimal small
          style={{ color: T.sidebarText }}
          title={isMaximized ? t('titlebar.restore') : t('titlebar.maximize')}
          onClick={handleMaximize}
        />
        <Button
          icon={<Icon icon="cross" size={12} />}
          minimal small
          className="titlebar-close-btn"
          style={{ color: T.sidebarText }}
          title={t('titlebar.close')}
          onClick={handleClose}
        />
      </div>
    </div>
  );
}
