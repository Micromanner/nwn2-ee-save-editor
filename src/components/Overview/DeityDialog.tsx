import { useState, useMemo, useEffect } from 'react';
import { Button, InputGroup } from '@blueprintjs/core';
import { T } from '../theme';
import { ParchmentDialog, LoadingState } from '../shared';
import { CharacterAPI, type Deity } from '@/services/characterApi';
import { useIcon } from '@/hooks/useIcon';
import { useTranslations } from '@/hooks/useTranslations';

interface DeityDialogProps {
  isOpen: boolean;
  currentDeity: string;
  onClose: () => void;
  onSelect: (deityName: string) => void;
}

export function DeityDialog({ isOpen, currentDeity, onClose, onSelect }: DeityDialogProps) {
  const t = useTranslations();
  const [search, setSearch] = useState('');
  const [selected, setSelected] = useState<string | null>(null);
  const [deities, setDeities] = useState<Deity[]>([]);
  const [loadingDeities, setLoadingDeities] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    setLoadingDeities(true);
    CharacterAPI.getAvailableDeities().then(res => {
      setDeities(res.deities);
    }).catch(() => {
      setDeities([]);
    }).finally(() => setLoadingDeities(false));
  }, [isOpen]);

  const filtered = useMemo(() => {
    if (!search) return deities;
    const q = search.toLowerCase();
    return deities.filter(d =>
      d.name.toLowerCase().includes(q) ||
      (d.portfolio ?? '').toLowerCase().includes(q) ||
      (d.aliases ?? '').toLowerCase().includes(q)
    );
  }, [search, deities]);

  const detail = deities.find(d => d.name === selected);

  const handleOpen = () => {
    setSearch('');
    setSelected(currentDeity || null);
  };

  const handleConfirm = () => {
    if (selected) {
      onSelect(selected);
    }
    onClose();
  };

  return (
    <ParchmentDialog
      isOpen={isOpen}
      onClose={onClose}
      onOpened={handleOpen}
      title={t('deity.selectTitle')}
      width={720}
      footerActions={
        <Button text={t('common.confirm')} intent="primary" onClick={handleConfirm} style={{ background: T.accent }} />
      }
    >
      <div style={{ display: 'flex', gap: 0, margin: -16, flex: 1, minHeight: 0, overflow: 'hidden' }}>
        <div style={{ width: 240, borderRight: `1px solid ${T.borderLight}`, display: 'flex', flexDirection: 'column', background: T.surfaceAlt }}>
          <div style={{ padding: 8 }}>
            <InputGroup
              leftIcon="search"
              placeholder={t('deity.searchPlaceholder')}
              small
              value={search}
              onChange={e => setSearch(e.target.value)}
              rightElement={search ? <Button icon="cross" small minimal onClick={() => setSearch('')} /> : undefined}
            />
          </div>
          <div style={{ flex: 1, overflowY: 'auto' }}>
            {loadingDeities ? (
              <LoadingState height={200} text={t('common.loadingDeities')} />
            ) : (
              <>
                <button
                  onClick={() => setSelected(null)}
                  className={`t-md${selected === null ? ' t-semibold' : ''}`}
                  style={{
                    display: 'block', width: '100%', textAlign: 'left',
                    padding: '6px 12px', border: 'none', cursor: 'pointer',
                    background: selected === null ? `${T.accent}15` : 'transparent',
                    borderLeft: selected === null ? `2px solid ${T.accent}` : '2px solid transparent',
                    color: selected === null ? T.accent : T.textMuted,
                  }}
                >
                  {t('deity.noneNoDeity')}
                </button>
                {filtered.map(d => (
                  <button
                    key={d.name}
                    onClick={() => setSelected(d.name)}
                    className={`t-md${selected === d.name ? ' t-semibold' : ''}`}
                    style={{
                      display: 'block', width: '100%', textAlign: 'left',
                      padding: '6px 12px', border: 'none', cursor: 'pointer',
                      background: selected === d.name ? `${T.accent}15` : 'transparent',
                      borderLeft: selected === d.name ? `2px solid ${T.accent}` : '2px solid transparent',
                      color: selected === d.name ? T.accent : T.text,
                    }}
                  >
                    <div>{d.name}</div>
                    {d.alignment && <div className="t-sm" style={{ color: T.textMuted }}>{d.alignment}</div>}
                  </button>
                ))}
                {filtered.length === 0 && !loadingDeities && (
                  <div className="t-md t-center" style={{ padding: 16, color: T.textMuted }}>{t('deity.noMatch')}</div>
                )}
              </>
            )}
          </div>
        </div>

        <div style={{ flex: 1, padding: 16 }}>
          {detail ? (
            <div>
              <DeityDetailHeader deity={detail} />
              <div style={{ marginTop: 12 }}>
                {detail.aliases && <DetailRow label={t('deity.aliases')} value={detail.aliases} />}
                {detail.alignment && <DetailRow label={t('deity.alignment')} value={detail.alignment} />}
                {detail.portfolio && <DetailRow label={t('deity.portfolio')} value={detail.portfolio} />}
                {detail.favored_weapon && <DetailRow label={t('deity.favoredWeapon')} value={detail.favored_weapon} />}
              </div>

              {detail.description && (
                <div className="t-md t-body" style={{ marginTop: 12, color: T.textMuted }}>
                  {detail.description}
                </div>
              )}
            </div>
          ) : (
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
              <div className="t-center">
                <div className="t-lg" style={{ color: T.textMuted }}>
                  {selected === null ? t('deity.noDeitySelected') : t('deity.selectFromList')}
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </ParchmentDialog>
  );
}

function DeityDetailHeader({ deity }: { deity: Deity }) {
  const iconUrl = useIcon(deity.icon);
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 4 }}>
      {iconUrl && <img src={iconUrl} alt="" width={32} height={32} style={{ borderRadius: 4, flexShrink: 0 }} />}
      <div className="t-2xl t-bold" style={{ color: T.text }}>{deity.name}</div>
    </div>
  );
}

function DetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="t-md" style={{ display: 'flex', gap: 12, padding: '4px 0' }}>
      <span style={{ color: T.textMuted, width: 120, flexShrink: 0 }}>{label}</span>
      <span className="t-semibold" style={{ color: T.text }}>{value}</span>
    </div>
  );
}
