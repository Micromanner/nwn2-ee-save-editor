import { AnchorButton, Button, Icon } from '@blueprintjs/core';
import { useUpdateCheck } from '@/hooks/useUpdateCheck';
import { useTranslations } from '@/hooks/useTranslations';
import { T } from '../theme';

export function UpdateBanner() {
  const t = useTranslations();
  const { info, dismiss } = useUpdateCheck();

  if (!info) return null;

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '4px 12px',
        background: T.sectionBg,
        borderBottom: `1px solid ${T.sectionBorder}`,
        color: T.accent,
        whiteSpace: 'nowrap',
        overflow: 'hidden',
      }}
    >
      <Icon icon="cloud-download" color={T.accent} />
      <span style={{ fontWeight: 600 }}>{t('update.available')}</span>
      <span style={{ color: T.textMuted }}>
        {t('update.current', { current: info.current })} - {t('update.latest', { latest: info.latest })}
      </span>
      <span style={{ flex: 1 }} />
      <AnchorButton
        small
        href={info.htmlUrl}
        target="_blank"
        rel="noreferrer"
        text={t('update.download')}
        style={{ background: T.accent, color: '#fff' }}
      />
      <Button small minimal icon="cross" onClick={dismiss} style={{ color: T.accent }} />
    </div>
  );
}
