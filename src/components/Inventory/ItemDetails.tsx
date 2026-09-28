import { useState } from 'react';
import { Alert, Button, ButtonGroup, H4 } from '@blueprintjs/core';
import { GiPointing, GiQuillInk, GiArmorUpgrade, GiHazardSign } from 'react-icons/gi';
import { useTranslations } from '@/hooks/useTranslations';
import { GameIcon } from '../shared/GameIcon';
import { T } from '../theme';
import { fmtNum } from '../shared';
import type { FullEquippedItem, FullInventoryItem } from '@/lib/bindings';
import { display } from '@/utils/dataHelpers';
import { nwn2ToHtml, renderNwn2Markup } from '@/utils/nwn2Markup';
import { useIcon } from '@/hooks/useIcon';
import { slotLabel } from './inventoryLabels';

export type AnyItem =
  | (FullEquippedItem & { _kind: 'equipped' })
  | (FullInventoryItem & { _kind: 'inventory' });

export function makeEquippedItem(item: FullEquippedItem): AnyItem {
  return { ...item, _kind: 'equipped' };
}

export function makeInventoryItem(item: FullInventoryItem): AnyItem {
  return { ...item, _kind: 'inventory' };
}

function isEquippedItem(item: AnyItem): item is FullEquippedItem & { _kind: 'equipped' } {
  return item._kind === 'equipped';
}

interface ItemDetailsProps {
  item: AnyItem | null;
  canEquip?: boolean;
  onEdit?: () => void;
  onEquip?: (index: number, slot: string) => void;
  onUnequip?: (slot: string) => void;
  onDelete?: (index: number) => void;
  onTransform?: () => void;
}

export function ItemDetails({ item, canEquip, onEdit, onEquip, onUnequip, onDelete, onTransform }: ItemDetailsProps) {
  const t = useTranslations();
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
  const [pendingPlotAction, setPendingPlotAction] = useState<'edit' | 'transform' | null>(null);
  const iconUrl = useIcon(item?.icon);

  const isPlot = item && !isEquippedItem(item) && (item as FullInventoryItem).plot;

  if (!item) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
        <div style={{ textAlign: 'center' }}>
          <GameIcon icon={GiPointing} size={36} style={{ color: T.border }} />
          <p style={{ marginTop: 10, color: T.textMuted }}>{t('inventory.selectItem')}</p>
          <p style={{ marginTop: 2, color: T.border }}>{t('inventory.selectItemHint')}</p>
        </div>
      </div>
    );
  }

  const equipped = isEquippedItem(item);
  const name = display(item.name);
  const description = item.description;
  const weight = item.weight;
  const value = item.value;
  const baseItemName = item.base_item_name;
  const properties = item.decoded_properties;
  const baseAc = item.base_ac;

  return (
    <div>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '6px 12px', borderBottom: `1px solid ${T.borderLight}` }}>
        {equipped && (
          <>
            <ButtonGroup minimal>
              <Button icon={<GameIcon icon={GiQuillInk} size={14} />} small text={t('common.edit')} style={{ color: T.textMuted }} onClick={onEdit} />
              <Button icon="exchange" small text={t('inventory.transformItem')} style={{ color: T.textMuted }} onClick={onTransform} />
              <Button
                icon={<GameIcon icon={GiArmorUpgrade} size={14} />}
                small
                text={t('inventory.unequip')}
                style={{ color: T.textMuted }}
                onClick={() => onUnequip?.(item.slot)}
              />
            </ButtonGroup>
            <Button minimal icon="trash" small intent="danger" text={t('inventory.destroy')} />
          </>
        )}
        {!equipped && (
          <>
            <ButtonGroup minimal>
              <Button icon={<GameIcon icon={GiQuillInk} size={14} />} small text={t('common.edit')} style={{ color: T.textMuted }}
                onClick={() => isPlot ? setPendingPlotAction('edit') : onEdit?.()}
              />
              <Button icon="exchange" small text={t('inventory.transformItem')} style={{ color: T.textMuted }}
                onClick={() => isPlot ? setPendingPlotAction('transform') : onTransform?.()}
              />
              {canEquip && (
                <Button
                  icon={<GameIcon icon={GiArmorUpgrade} size={14} />}
                  small
                  text={t('inventory.equip')}
                  style={{ color: T.textMuted }}
                  onClick={() => {
                    const inv = item as FullInventoryItem;
                    if (inv.default_slot) onEquip?.(inv.index, inv.default_slot);
                  }}
                />
              )}
            </ButtonGroup>
            <Button
              minimal
              icon="trash"
              small
              intent="danger"
              text={t('inventory.delete')}
              onClick={() => setDeleteConfirmOpen(true)}
            />
          </>
        )}
      </div>
      <div style={{ padding: '14px 16px 12px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          {iconUrl && (
            <img
              src={iconUrl}
              alt=""
              width={40}
              height={40}
              style={{ borderRadius: 4, flexShrink: 0 }}
            />
          )}
          <div style={{ minWidth: 0 }}>
            <H4 style={{ margin: 0, color: T.text, overflowWrap: 'anywhere' }}>{renderNwn2Markup(name)}</H4>
            <span style={{ color: T.textMuted }}>
              {equipped ? `${slotLabel(t, item.slot)} · ${baseItemName}` : baseItemName}
            </span>
          </div>
        </div>
      </div>

      <div style={{ borderTop: `1px solid ${T.borderLight}`, padding: '10px 16px' }}>
        <div className="t-bold" style={{ color: T.accent, marginBottom: 6 }}>{t('inventory.itemDetails')}</div>
        <div style={{ display: 'grid', gridTemplateColumns: '120px 1fr', rowGap: 4 }}>
          <span style={{ color: T.textMuted }}>{t('inventory.baseItem')}</span>
          <span className="t-semibold">{display(baseItemName)}</span>

          <span style={{ color: T.textMuted }}>{t('inventory.weight')}</span>
          <span className="t-semibold">{weight > 0 ? t('inventory.weightValue', { value: weight.toFixed(1) }) : '-'}</span>

          <span style={{ color: T.textMuted }}>{t('inventory.value')}</span>
          <span>{value > 0 ? t('inventory.goldValue', { value: fmtNum(value) }) : '-'}</span>

          {baseAc !== null && baseAc !== undefined && (
            <>
              <span style={{ color: T.textMuted }}>{t('inventory.baseArmorClass')}</span>
              <span className="t-semibold">{baseAc}</span>
            </>
          )}

          {!equipped && (() => {
            const inv = item as FullInventoryItem;
            return (
              <>
                {inv.stack_size > 1 && (
                  <>
                    <span style={{ color: T.textMuted }}>{t('inventory.stack')}</span>
                    <span className="t-semibold">{inv.stack_size}</span>
                  </>
                )}
                {inv.charges !== null && inv.charges !== undefined && (
                  <>
                    <span style={{ color: T.textMuted }}>{t('inventory.charges')}</span>
                    <span className="t-semibold">{inv.charges}</span>
                  </>
                )}
                <span style={{ color: T.textMuted }}>{t('inventory.identified')}</span>
                <span className="t-semibold" style={{ color: inv.identified ? T.text : T.negative }}>
                  {inv.identified ? t('inventory.identified') : t('inventory.unidentified')}
                </span>
                {inv.plot && (
                  <>
                    <span style={{ color: T.textMuted }}>{t('inventory.plotItem')}</span>
                    <span className="t-semibold" style={{ color: T.gold }}>{t('inventory.plotItem')}</span>
                  </>
                )}
                {inv.cursed && (
                  <>
                    <span style={{ color: T.textMuted }}>{t('inventory.cursedItem')}</span>
                    <span className="t-semibold" style={{ color: T.negative }}>{t('inventory.cursedItem')}</span>
                  </>
                )}
                {inv.stolen && (
                  <>
                    <span style={{ color: T.textMuted }}>{t('inventory.stolenItem')}</span>
                    <span className="t-semibold" style={{ color: T.gold }}>{t('inventory.stolenItem')}</span>
                  </>
                )}
              </>
            );
          })()}
        </div>
      </div>

      {description && (
        <div style={{ borderTop: `1px solid ${T.borderLight}`, padding: '10px 16px' }}>
          <div className="t-bold" style={{ color: T.accent, marginBottom: 8 }}>{t('inventory.description')}</div>
          <div className="t-body" style={{ margin: 0, color: T.textMuted, overflowWrap: 'anywhere' }} dangerouslySetInnerHTML={{ __html: nwn2ToHtml(description) }} />
        </div>
      )}

      {properties && properties.length > 0 && (
        <div style={{ borderTop: `1px solid ${T.borderLight}`, padding: '10px 16px' }}>
          <div className="t-bold" style={{ color: T.accent, marginBottom: 8 }}>{t('inventory.properties')}</div>
          <ul style={{ margin: 0, paddingLeft: 20, display: 'flex', flexDirection: 'column', gap: 4 }}>
            {properties.map((prop, i) => (
              <li key={i} style={{ color: T.text }}>{prop.display_string}</li>
            ))}
          </ul>
        </div>
      )}

      <Alert
        isOpen={deleteConfirmOpen}
        onClose={() => setDeleteConfirmOpen(false)}
        onConfirm={() => { setDeleteConfirmOpen(false); onDelete?.((item as FullInventoryItem).index); }}
        intent="danger"
        icon="trash"
        confirmButtonText={t('inventory.delete')}
        cancelButtonText={t('actions.cancel')}
      >
        <p><strong>{renderNwn2Markup(name)}</strong></p>
        <p>{isPlot ? t('inventory.deleteWarningPlot') : t('inventory.deleteWarningRegular')}</p>
      </Alert>

      <Alert
        isOpen={pendingPlotAction !== null}
        onClose={() => setPendingPlotAction(null)}
        onConfirm={() => {
          const action = pendingPlotAction;
          setPendingPlotAction(null);
          if (action === 'edit') onEdit?.();
          else if (action === 'transform') onTransform?.();
        }}
        intent="warning"
        icon={<GameIcon icon={GiHazardSign} size={40} />}
        confirmButtonText={pendingPlotAction === 'transform' ? t('inventory.transformItem') : t('common.edit')}
        cancelButtonText={t('actions.cancel')}
      >
        <p><strong>{renderNwn2Markup(name)}</strong></p>
        <p>{t('inventory.editWarningPlot')}</p>
      </Alert>
    </div>
  );
}
