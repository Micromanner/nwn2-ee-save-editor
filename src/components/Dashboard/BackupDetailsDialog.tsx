import { useState } from 'react';
import type { CSSProperties, ReactNode } from 'react';
import { Icon } from '@blueprintjs/core';
import { ParchmentDialog } from '../shared/ParchmentDialog';
import { T, formatUnixSeconds } from '../theme';
import { useTranslations } from '@/hooks/useTranslations';
import { display } from '@/utils/dataHelpers';
import { formatDateTime } from '@/utils/dateFormat';
import type { BackupManifest, BackupReason, SaveRecord } from '@/services/backupApi';
import type { FileInfo } from './FileBrowserDialog';

interface BackupDetailsDialogProps {
  isOpen: boolean;
  onClose: () => void;
  file: FileInfo | null;
}

const REASON_KEYS: Record<BackupReason, string> = {
  load: 'fileBrowser.reasonLoad',
  pre_restore: 'fileBrowser.reasonPreRestore',
  manual: 'fileBrowser.reasonManual',
};

const rowStyle: CSSProperties = {
  display: 'flex', alignItems: 'center', gap: 8, padding: '4px 8px 4px 30px',
  borderBottom: `1px solid ${T.borderLight}`, color: T.text,
};

const sectionHeaderStyle: CSSProperties = {
  padding: '5px 8px', fontWeight: 600, color: T.accent,
  borderBottom: `1px solid ${T.border}`, background: T.surfaceAlt, marginTop: 8,
};

/// Read-only view of the character/quest/party changes recorded on a backup.
export function BackupDetailsDialog({ isOpen, onClose, file }: BackupDetailsDialogProps) {
  const t = useTranslations();
  const manifest: BackupManifest | null | undefined = file?.changes;
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());

  const toggle = (key: string) => setCollapsed(prev => {
    const next = new Set(prev);
    if (next.has(key)) next.delete(key); else next.add(key);
    return next;
  });

  const section = (key: string, title: string, count: number, children: ReactNode) => {
    const isExpanded = !collapsed.has(key);
    return (
      <div>
        <button
          onClick={() => toggle(key)}
          className="t-semibold"
          style={{
            display: 'flex', alignItems: 'center', gap: 6,
            width: '100%', textAlign: 'left',
            padding: '6px 8px', border: 'none', cursor: 'pointer',
            background: 'transparent', color: T.text,
          }}
        >
          <Icon icon={isExpanded ? 'caret-down' : 'caret-right'} size={16} style={{ color: T.accent }} />
          {title}
          <span style={{ color: T.textMuted, fontWeight: 400 }}>({count})</span>
        </button>
        {isExpanded && children}
      </div>
    );
  };

  const renderRecord = (record: SaveRecord, index: number) => (
    <div key={index}>
      <div style={sectionHeaderStyle}>
        {t('fileBrowser.savedAt')}: {formatDateTime(record.saved_at)}
      </div>
      {record.characters.length > 0 && section(
        `${index}-characters`,
        t('fileBrowser.sectionCharacters'),
        record.characters.reduce((sum, c) => sum + c.changes.length, 0),
        record.characters.map((character, ci) => (
          <div key={ci}>
            <div className="t-semibold" style={{ padding: '4px 8px 4px 30px', color: T.text }}>
              {display(character.name)}
            </div>
            {character.changes.map((label, li) => (
              <div key={li} style={rowStyle}>
                <span>{label}</span>
              </div>
            ))}
          </div>
        )),
      )}
      {record.quests.length > 0 && section(
        `${index}-quests`,
        t('fileBrowser.sectionQuests'),
        record.quests.length,
        record.quests.map((quest, qi) => (
          <div key={qi} style={{ padding: '4px 8px 4px 30px', borderBottom: `1px solid ${T.borderLight}` }}>
            <div style={{ color: T.text }}>
              {quest.name} ({quest.tag}): {display(quest.from_state)} -&gt; {quest.to_state}
            </div>
            {quest.state_text && (
              <div
                title={quest.state_text}
                style={{
                  color: T.textMuted, marginTop: 2,
                  overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                }}
              >
                {quest.state_text}
              </div>
            )}
          </div>
        )),
      )}
      {record.party.length > 0 && section(
        `${index}-party`,
        t('fileBrowser.sectionParty'),
        record.party.length,
        record.party.map((party, pi) => (
          <div key={pi} style={rowStyle}>
            <span>
              {display(party.name ?? party.ros_name)}: {party.in_party ? t('fileBrowser.partyAdded') : t('fileBrowser.partyBenched')}
            </span>
          </div>
        )),
      )}
    </div>
  );

  return (
    <ParchmentDialog isOpen={isOpen} onClose={onClose} title={t('fileBrowser.details')} width={560} height={520} footerActions={null}>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 2, paddingBottom: 8, borderBottom: `1px solid ${T.borderLight}`, flexShrink: 0 }}>
        <span className="t-medium" style={{ color: T.text }}>{display(file?.save_name || file?.name)}</span>
        <span style={{ color: T.textMuted, fontSize: 13 }}>
          {formatUnixSeconds(file?.modified ?? '')}
          {manifest ? `, ${t(REASON_KEYS[manifest.reason])}` : ''}
        </span>
      </div>
      <div style={{ flex: 1, minHeight: 0, overflowY: 'auto' }}>
        <p className="t-body" style={{ color: T.textMuted, marginTop: 8, marginBottom: 0 }}>
          {t('fileBrowser.detailsIntro')}
        </p>
        {!manifest ? (
          <div style={{ color: T.textMuted, padding: 8 }}>{t('fileBrowser.changesUnknown')}</div>
        ) : manifest.saves.length === 0 ? (
          <div style={{ color: T.textMuted, padding: 8 }}>{t('fileBrowser.changesNone')}</div>
        ) : (
          [...manifest.saves].reverse().map((record, i) => renderRecord(record, i))
        )}
      </div>
    </ParchmentDialog>
  );
}
