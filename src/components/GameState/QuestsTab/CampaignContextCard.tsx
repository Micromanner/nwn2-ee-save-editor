import { Card, Elevation } from '@blueprintjs/core';
import { T } from '../../theme';
import { KVRow } from '../../shared';
import { useTranslations } from '@/hooks/useTranslations';
import type { AggregatedModule, CampaignSummary, OrphanNote } from './types';

const WARN = '#d97706';

function truncate(value: string, max: number): string {
  if (value.length <= max) return value;
  return `${value.slice(0, max - 1)}…`;
}

function resolutionColor(kind: AggregatedModule['resolution_kind']): string {
  switch (kind) {
    case 'campaign': return T.positive;
    case 'install': return T.text;
    case 'unresolved': return T.negative;
  }
}

export function CampaignContextCard({
  campaign, modules, orphans,
}: {
  campaign: CampaignSummary;
  modules: AggregatedModule[];
  orphans: OrphanNote[];
}) {
  const t = useTranslations();
  const hasCampaign = campaign.campaign_path !== null;

  const currentModule = modules.find(m => m.is_current);
  const currentModuleLabel = currentModule?.display_name
    || campaign.current_module_id
    || '-';

  return (
    <Card elevation={Elevation.ONE} style={{ padding: 0, background: T.surface, overflow: 'hidden' }}>
      <div style={{ padding: '10px 16px', display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '4px 16px' }}>
        <KVRow
          label={t('gameState.quests.campaignContext.campaign')}
          value={hasCampaign ? (campaign.display_name || '-') : t('gameState.quests.campaignContext.noCampaign')}
        />
        <KVRow
          label={t('gameState.quests.campaignContext.campaignId')}
          value={<span className="t-mono">{truncate(campaign.campaign_id || '-', 16)}</span>}
        />
        <KVRow
          label={t('gameState.quests.campaignContext.currentModule')}
          value={
            <span title={campaign.current_module_id || undefined}>
              {currentModuleLabel}
            </span>
          }
        />
      </div>

      {modules.length > 0 && (
        <div style={{ padding: '0 16px 10px', display: 'flex', flexWrap: 'wrap', gap: 6, alignItems: 'center' }}>
          <span className="t-semibold" style={{ color: T.textMuted, marginRight: 4 }}>
            {t('gameState.quests.campaignContext.modules')}:
          </span>
          {modules.map((m, i) => (
            <span
              key={`${m.name}-${i}`}
              title={m.name}
              style={{
                padding: '2px 8px',
                borderRadius: 3,
                border: `1px solid ${resolutionColor(m.resolution_kind)}`,
                color: resolutionColor(m.resolution_kind),
                background: m.is_current ? `${T.accent}10` : 'transparent',
                display: 'inline-flex',
                alignItems: 'center',
                gap: 4,
              }}
            >
              {m.display_name || m.name}
              {m.is_current && <span className="t-semibold" style={{ color: T.accent }}>({t('gameState.quests.campaignContext.current')})</span>}
              {m.resolution_kind === 'unresolved' && (
                <span style={{ color: T.negative }}>({t('gameState.quests.campaignContext.unresolved')})</span>
              )}
              <span style={{ color: T.textMuted }}>{m.journal_category_tags.length}</span>
            </span>
          ))}
        </div>
      )}

      {orphans.length > 0 && (
        <div style={{ margin: '0 16px 10px', padding: '8px 12px', background: `${WARN}12`, border: `1px solid ${WARN}40`, borderRadius: 4 }}>
          <div className="t-semibold" style={{ color: WARN, marginBottom: 4 }}>
            {t('gameState.quests.campaignContext.orphans')}
          </div>
          <ul style={{ margin: 0, paddingLeft: 20, color: T.text }}>
            {orphans.map((o, i) => (
              <li key={i}>
                <span className="t-semibold">{t(`gameState.quests.orphanKind.${o.kind}`)}:</span> {o.message}
              </li>
            ))}
          </ul>
        </div>
      )}
    </Card>
  );
}
