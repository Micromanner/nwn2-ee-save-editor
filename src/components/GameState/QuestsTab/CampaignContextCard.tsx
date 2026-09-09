import { Card, Elevation } from '@blueprintjs/core';
import { T } from '../../theme';
import { KVRow } from '../../shared';
import { useTranslations } from '@/hooks/useTranslations';
import type { AggregatedModule, CampaignSummary, OrphanNote } from './types';

const WARN = '#d97706';

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

  return (
    <Card elevation={Elevation.ONE} style={{ padding: 0, background: T.surface, overflow: 'hidden' }}>
      <div style={{ padding: '10px 16px', display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '4px 16px' }}>
        <KVRow
          label={t('gameState.quests.campaignContext.campaign')}
          value={hasCampaign ? (campaign.display_name || '-') : t('gameState.quests.campaignContext.noCampaign')}
        />
        <KVRow
          label={t('gameState.quests.campaignContext.currentModule')}
          value={currentModule?.display_name || campaign.current_module_id || '-'}
        />
      </div>

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
