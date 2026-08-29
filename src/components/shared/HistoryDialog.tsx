import { useCallback, useEffect, useState } from 'react';
import type { CSSProperties } from 'react';
import { Button, ButtonGroup } from '@blueprintjs/core';
import { GiAnticlockwiseRotation, GiClockwiseRotation } from 'react-icons/gi';
import { ParchmentDialog } from './ParchmentDialog';
import { GameIcon } from './GameIcon';
import { LoadingState } from './LoadingState';
import { T } from '../theme';
import { useTranslations } from '@/hooks/useTranslations';
import { CharacterStateAPI } from '@/lib/api/character-state';
import type { CharacterHistory } from '@/lib/bindings';

interface HistoryDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

const keyOf = (e: CharacterHistory) => e.ros_name ?? 'player';

/// Read-only view of every edited character's undo/redo stack.
export function HistoryDialog({ isOpen, onClose }: HistoryDialogProps) {
  const t = useTranslations();
  const [entries, setEntries] = useState<CharacterHistory[] | null>(null);
  const [selected, setSelected] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const list = await CharacterStateAPI.getSessionHistory();
      setEntries(list);
      setSelected(prev => (prev && list.some(e => keyOf(e) === prev) ? prev : (list[0] ? keyOf(list[0]) : null)));
    } catch (err) {
      console.error('Failed to load history:', err);
      setEntries([]);
    }
  }, []);

  useEffect(() => {
    if (isOpen) void load();
  }, [isOpen, load]);

  const current = entries?.find(e => keyOf(e) === selected) ?? null;
  const redoRows = current ? [...current.redo_labels].reverse() : [];
  const undoRows = current?.undo_labels ?? [];

  const rowStyle = (muted: boolean): CSSProperties => ({
    display: 'flex', alignItems: 'center', gap: 8, padding: '5px 8px',
    borderBottom: `1px solid ${T.borderLight}`, color: muted ? T.textMuted : T.text, opacity: muted ? 0.7 : 1,
  });

  return (
    <ParchmentDialog isOpen={isOpen} onClose={onClose} title={t('actions.history')} width={520} height={480} footerActions={null}>
      {entries === null ? (
        <LoadingState text={t('common.loading')} />
      ) : entries.length === 0 ? (
        <div style={{ color: T.textMuted, padding: 16 }}>{t('actions.historyEmpty')}</div>
      ) : (
        <>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, paddingBottom: 6, borderBottom: `1px solid ${T.borderLight}`, flexShrink: 0 }}>
            <ButtonGroup minimal>
              {entries.map(e => (
                <Button
                  key={keyOf(e)}
                  small
                  intent={keyOf(e) === selected ? 'primary' : 'none'}
                  text={`${e.name} (${e.undo_labels.length + e.redo_labels.length})`}
                  onClick={() => setSelected(keyOf(e))}
                />
              ))}
            </ButtonGroup>
          </div>
          <div style={{ flex: 1, minHeight: 0, overflowY: 'auto', marginTop: 6 }}>
            {redoRows.map((label, i) => (
              <div key={`redo-${i}`} style={rowStyle(true)}>
                <GameIcon icon={GiClockwiseRotation} size={12} />
                <span>{label}</span>
              </div>
            ))}
            <div style={{ padding: '5px 8px', fontWeight: 600, color: T.accent, borderBottom: `1px solid ${T.border}`, background: T.surfaceAlt }}>
              {t('actions.historyCurrent')}
            </div>
            {undoRows.map((label, i) => (
              <div key={`undo-${i}`} style={rowStyle(false)}>
                <GameIcon icon={GiAnticlockwiseRotation} size={12} />
                <span>{label}</span>
              </div>
            ))}
            {undoRows.length === 0 && redoRows.length === 0 && (
              <div style={{ color: T.textMuted, padding: 8 }}>{t('actions.historyEmpty')}</div>
            )}
          </div>
        </>
      )}
    </ParchmentDialog>
  );
}
