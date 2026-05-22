import { useState, useEffect } from 'react';
import { AnchorButton, Button, Classes, Dialog, DialogBody, DialogFooter, Icon, Tab, Tabs, Tooltip } from '@blueprintjs/core';
import { GiCheckMark } from 'react-icons/gi';
import { GameIcon } from './GameIcon';
import { getName, getVersion } from '@tauri-apps/api/app';
import { T } from '../theme';
import { useTranslations } from '@/hooks/useTranslations';

interface AboutDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export function AboutDialog({ isOpen, onClose }: AboutDialogProps) {
  const t = useTranslations();
  const [appName, setAppName] = useState('NWN2:EE Save Editor');
  const [appVersion, setAppVersion] = useState('0.1.0');
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    Promise.all([getName(), getVersion()])
      .then(([name, ver]) => { setAppName(name); setAppVersion(ver); })
      .catch(() => {});
  }, [isOpen]);

  const handleCopyVersion = async () => {
    await navigator.clipboard.writeText(`v${appVersion}`).catch(() => {});
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const infoPanel = (
    <div className="about-info-panel">
      <div className="about-app-icon">
        <img src="/logo.png" alt="Logo" width={56} height={56} />
      </div>
      <div className="about-app-name">{appName}</div>
      <Tooltip content={t('about.copyVersion')} placement="bottom">
        <button className="about-version-btn" onClick={handleCopyVersion}>
          <code>v{appVersion}</code>
          {copied ? <GameIcon icon={GiCheckMark} size={12} color={T.positive} /> : <Icon icon="duplicate" size={12} />}
        </button>
      </Tooltip>
      <p className="about-description">{t('about.description')}</p>
      <p className="about-built-with">{t('about.builtWith')}</p>
      <div className="about-links">
        <AnchorButton
          outlined
          small
          icon={<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>}
          href="https://github.com/Micromanner/nwn2-ee-save-editor"
          target="_blank"
          rel="noreferrer"
          text={t('about.github')}
        />
        <AnchorButton
          outlined
          small
          icon="issue"
          href="https://github.com/Micromanner/nwn2-ee-save-editor/issues"
          target="_blank"
          rel="noreferrer"
          text={t('about.reportBug')}
        />
      </div>
    </div>
  );

  const creditsPanel = (
    <div className="about-credits-panel">
      <h4 className={Classes.HEADING}>{t('about.createdBy')}</h4>
      <p><strong>Micromanner</strong></p>

      <h4 className={Classes.HEADING} style={{ marginTop: 20 }}>{t('about.specialThanks')}</h4>
      <div className="about-credits-list">
        <p><strong>Obsidian Entertainment</strong> — {t('about.obsidianCredit')}</p>
        <p><strong>Aspyr</strong> — {t('about.aspyrCredit')}</p>
        <p><strong>ScripterRon</strong> — {t('about.scripterronCredit')}</p>
        <p><strong>Arbos (nwn2mdk)</strong> — {t('about.nwn2mdkCredit')}</p>
      </div>
    </div>
  );

  const legalPanel = (
    <div className="about-legal-panel">
      <h4 className={Classes.HEADING}>{t('about.license')}</h4>
      <p>{t('about.licenseText')}</p>

      <h4 className={Classes.HEADING} style={{ marginTop: 20 }}>{t('about.thirdParty')}</h4>
      <div>
        <p>{t('about.fanContent', { appName })}</p>
        <p style={{ marginTop: 8 }}>{t('about.notAffiliated')}</p>
      </div>
    </div>
  );

  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      title={t('about.title')}
      className="bp-app about-dialog"
      canOutsideClickClose
    >
      <DialogBody className="about-dialog-body">
        <Tabs id="about-tabs" defaultSelectedTabId="info" renderActiveTabPanelOnly>
          <Tab id="info" title={t('about.appInfo')} panel={infoPanel} />
          <Tab id="credits" title={t('about.credits')} panel={creditsPanel} />
          <Tab id="legal" title={t('about.legal')} panel={legalPanel} />
        </Tabs>
      </DialogBody>
      <DialogFooter actions={<Button text={t('common.close')} onClick={onClose} />} />
    </Dialog>
  );
}
