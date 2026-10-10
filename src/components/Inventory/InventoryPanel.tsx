import { useEffect, useState } from 'react';
import { Button, Card, Elevation, NonIdealState, ProgressBar } from '@blueprintjs/core';
import { GiBrokenShield, GiVisoredHelm } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { useTranslations } from '@/hooks/useTranslations';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { T } from '../theme';
import { LoadingState } from '../shared';
import { ItemDetails, makeEquippedItem, makeInventoryItem, type AnyItem } from './ItemDetails';
import { AddItemDialog } from './AddItemDialog';
import { EditItemDialog } from './EditItemDialog';
import { BackpackTable, backpackItemKey } from './BackpackTable';
import { useSubsystem } from '@/contexts/CharacterContext';
import { useInventoryManagement, type ItemLocation } from '@/hooks/useInventoryManagement';
import { inventoryAPI } from '@/services/inventoryApi';
import { useCharacterContext } from '@/contexts/CharacterContext';
import type { FullEquippedItem, FullInventoryItem, FullInventorySummary, ItemAppearance } from '@/lib/bindings';
import { resolveEquipSlot } from '@/utils/inventoryUtils';
import { stripNwn2Tags } from '@/utils/nwn2Markup';
import { useIcon } from '@/hooks/useIcon';
import { DISPLAY_SLOT_TO_API } from './inventoryLabels';

const EQUIPMENT_GRID: (string | null)[][] = [
  [null, 'head', 'amulet', null],
  ['mainHand', 'chest', 'cloak', 'offHand'],
  ['ring1', 'belt', 'hands', 'ring2'],
  [null, 'feet', null, null],
];

const AMMO_SLOTS = ['arrows', 'bullets', 'bolts'];

function getEquippedItemForDisplaySlot(
  equippedItems: FullEquippedItem[],
  displaySlot: string,
): FullEquippedItem | undefined {
  const apiSlot = DISPLAY_SLOT_TO_API[displaySlot];
  if (!apiSlot) return undefined;
  return equippedItems.find(e => e.slot.toLowerCase() === apiSlot.toLowerCase());
}

function findBackpackItem(
  data: FullInventorySummary,
  index: number,
  containerIndex: number | null,
): FullInventoryItem | undefined {
  if (containerIndex === null) return data.inventory?.find(i => i.index === index);
  return data.inventory?.find(i => i.index === containerIndex)?.contents.find(i => i.index === index);
}

function itemLocation(item: AnyItem): ItemLocation {
  return item._kind === 'equipped'
    ? { slot: item.slot }
    : { itemIndex: item.index, containerIndex: item.container_index };
}

interface EquipSlotProps {
  slot: string;
  equippedItem: FullEquippedItem | undefined;
  selected: boolean;
  onSelect: () => void;
}

function EquipSlot({ slot, equippedItem, selected, onSelect }: EquipSlotProps) {
  const t = useTranslations();
  const label = t(`inventory.slotAbbr.${slot}`);
  const slotName = t(`inventory.slots.${slot}`);
  const hasItem = !!equippedItem;
  const iconUrl = useIcon(equippedItem?.icon);

  return (
    <div
      onClick={onSelect}
      title={equippedItem ? `${stripNwn2Tags(equippedItem.name)} (${slotName})` : slotName}
      style={{
        width: 44, height: 44,
        borderRadius: 4,
        border: `2px solid ${selected ? T.accent : hasItem ? T.border : T.borderLight}`,
        background: selected ? `${T.accent}15` : hasItem ? T.surface : T.surfaceAlt,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        cursor: hasItem ? 'pointer' : 'default',
        transition: 'all 0.15s',
        position: 'relative',
      }}
    >
      {hasItem ? (
        iconUrl ? (
          <img src={iconUrl} alt="" width={36} height={36} style={{ borderRadius: 3 }} />
        ) : (
          <div className="t-bold" style={{
            width: 30, height: 30, borderRadius: 3,
            background: T.sectionBg,
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            color: T.accent,
          }}>
            {stripNwn2Tags(equippedItem.name).charAt(0)}
          </div>
        )
      ) : (
        <span className="t-semibold" style={{ color: T.border }}>{label}</span>
      )}
    </div>
  );
}

export function InventoryPanel() {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const { character, characterId, itemEditorMetadata, activeSource } = useCharacterContext();
  const isCompanion = activeSource.kind === 'companion';
  const inventorySubsystem = useSubsystem('inventory');
  const { equipItem, unequipItem, deleteItem, destroyEquippedItem } = useInventoryManagement();

  const [selectedItem, setSelectedItem] = useState<AnyItem | null>(null);
  const [goldInput, setGoldInput] = useState('0');
  const [gold, setGold] = useState(0);
  const [addItemOpen, setAddItemOpen] = useState(false);
  const [transformOpen, setTransformOpen] = useState(false);
  const [transformTarget, setTransformTarget] = useState<ItemLocation | null>(null);
  const [pendingEditIndex, setPendingEditIndex] = useState<number | null>(null);
  const [editItemOpen, setEditItemOpen] = useState(false);
  const [editItemData, setEditItemData] = useState<Record<string, unknown> | null>(null);
  const [editTarget, setEditTarget] = useState<ItemLocation | null>(null);
  const [editAppearance, setEditAppearance] = useState<ItemAppearance | null>(null);
  const [editBaseLimits, setEditBaseLimits] = useState<{ maxStack: number; propColumn: number }>({ maxStack: 1, propColumn: -1 });
  const [editResolvedName, setEditResolvedName] = useState<string | undefined>(undefined);
  const [editResolvedDescription, setEditResolvedDescription] = useState<string | undefined>(undefined);
  const [editResolvedIcon, setEditResolvedIcon] = useState<string | undefined>(undefined);

  const inventoryData = inventorySubsystem.data;

  useEffect(() => {
    if (!inventorySubsystem.data && !inventorySubsystem.isLoading && characterId) {
      inventorySubsystem.load();
    }
  }, [characterId, inventorySubsystem]);

  useEffect(() => {
    if (inventoryData) {
      const g = inventoryData.gold ?? character?.gold ?? 0;
      setGold(g);
      setGoldInput(g.toString());
    }
  }, [inventoryData, character?.gold]);

  // Open edit dialog for newly added item once inventory reloads
  useEffect(() => {
    if (pendingEditIndex === null || !inventoryData) return;
    const newItem = inventoryData.inventory?.find(i => i.index === pendingEditIndex);
    if (newItem) {
      setPendingEditIndex(null);
      setSelectedItem(makeInventoryItem(newItem));
      setEditItemData(newItem.item);
      setEditTarget({ itemIndex: newItem.index });
      setEditAppearance(newItem.appearance);
      setEditBaseLimits({ maxStack: newItem.max_stack, propColumn: newItem.prop_column });
      setEditResolvedName(newItem.name || undefined);
      setEditResolvedDescription(newItem.description || undefined);
      setEditResolvedIcon(newItem.icon || undefined);
      setEditItemOpen(true);
    }
  }, [pendingEditIndex, inventoryData]);

  // Sync selected item with fresh inventory data after silent reloads
  useEffect(() => {
    if (!inventoryData || !selectedItem) return;
    if (selectedItem._kind === 'inventory') {
      const fresh = findBackpackItem(inventoryData, selectedItem.index, selectedItem.container_index);
      if (fresh) setSelectedItem(makeInventoryItem(fresh));
      else setSelectedItem(null);
    } else {
      const fresh = inventoryData.equipped?.find(e => e.slot === selectedItem.slot);
      if (fresh) setSelectedItem(makeEquippedItem(fresh));
      else setSelectedItem(null);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [inventoryData]);

  const goldDirty = goldInput !== gold.toString();

  const handleGoldSubmit = async () => {
    const val = parseInt(goldInput, 10);
    if (isNaN(val) || val < 0) return;
    const clamped = Math.min(val, 99_999_999);
    try {
      await inventoryAPI.updateGold(clamped);
      setGold(clamped);
      setGoldInput(clamped.toString());
      await inventorySubsystem.load({ silent: true });
    } catch (err) {
      handleError(err);
      setGoldInput(gold.toString());
    }
  };

  const handleGoldReset = () => setGoldInput(gold.toString());

  const handleEquip = async (index: number, defaultSlot: string) => {
    const invItem = backpackItems.find(i => i.index === index);
    const equippableSlots = invItem?.equippable_slots ?? [];
    const resolvedSlot = resolveEquipSlot(defaultSlot, equippableSlots, equippedItems);
    try {
      await equipItem({ inventory_index: index, slot: resolvedSlot });
      setSelectedItem(null);
    } catch (err) {
      handleError(err);
    }
  };

  const handleUnequip = async (slot: string) => {
    try {
      await unequipItem({ slot });
      setSelectedItem(null);
    } catch (err) {
      handleError(err);
    }
  };

  const handleDelete = async (index: number, containerIndex: number | null) => {
    try {
      await deleteItem(index, containerIndex);
      setSelectedItem(null);
    } catch (err) {
      handleError(err);
    }
  };

  const handleDestroy = async (slot: string) => {
    try {
      await destroyEquippedItem(slot);
      setSelectedItem(null);
    } catch (err) {
      handleError(err);
    }
  };

  const equippedItems = inventoryData?.equipped ?? [];
  const backpackItems = inventoryData?.inventory ?? [];
  const encumbrance = inventoryData?.encumbrance;

  const totalWeight = encumbrance?.total_weight ?? 0;
  const maxWeight = encumbrance?.heavy_load ?? 0;
  const weightRatio = maxWeight > 0 ? Math.min(totalWeight / maxWeight, 1) : 0;
  const encumbranceStatus = encumbrance?.status ?? 'normal';
  const totalItems = equippedItems.length + backpackItems.length;

  if (inventorySubsystem.isLoading) {
    return (
      <LoadingState text={t('common.loadingSection', { section: t('navigation.inventory').toLowerCase() })} />
    );
  }

  if (inventorySubsystem.error) {
    return (
      <div style={{ padding: 16, height: '100%' }}>
        <NonIdealState
          icon={<GameIcon icon={GiBrokenShield} size={40} />}
          title={t('common.failedToLoad', { section: t('navigation.inventory').toLowerCase() })}
          description={inventorySubsystem.error}
          action={<Button onClick={() => inventorySubsystem.load()}>{t('common.retry')}</Button>}
        />
      </div>
    );
  }

  if (!inventoryData) {
    return (
      <div style={{ padding: 16, height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <NonIdealState icon={<GameIcon icon={GiVisoredHelm} size={40} />} title={t('common.noCharacterLoaded')} description={t('common.loadSaveToView', { section: t('navigation.inventory').toLowerCase() })} />
      </div>
    );
  }

  return (
    <div style={{ padding: 16, height: '100%' }}>
      <Card elevation={Elevation.ONE} style={{ padding: 0, background: T.surface, overflow: 'hidden', display: 'flex', height: '100%' }}>

        <div style={{ width: 780, minWidth: 780, borderRight: `1px solid ${T.borderLight}`, display: 'flex', flexDirection: 'column' }}>

          <div style={{ padding: '12px 16px' }}>
            <div className="t-bold" style={{ color: T.accent, marginBottom: 10 }}>{t('inventory.equipment')}</div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: 4, alignItems: 'center' }}>
              {EQUIPMENT_GRID.map((row, ri) => (
                <div key={ri} style={{ display: 'flex', gap: 4 }}>
                  {row.map((slot, ci) => {
                    if (!slot) return <div key={ci} style={{ width: 44, height: 44 }} />;
                    const equippedItem = getEquippedItemForDisplaySlot(equippedItems, slot);
                    return (
                      <EquipSlot
                        key={slot}
                        slot={slot}
                        equippedItem={equippedItem}
                        selected={
                          selectedItem !== null &&
                          selectedItem._kind === 'equipped' &&
                          selectedItem.slot === DISPLAY_SLOT_TO_API[slot]
                        }
                        onSelect={() => {
                          if (equippedItem) setSelectedItem(makeEquippedItem(equippedItem));
                        }}
                      />
                    );
                  })}
                </div>
              ))}

              <div style={{ display: 'flex', gap: 4, marginTop: 4, paddingTop: 8, borderTop: `1px solid ${T.borderLight}` }}>
                {AMMO_SLOTS.map(slot => {
                  const equippedItem = getEquippedItemForDisplaySlot(equippedItems, slot);
                  return (
                    <EquipSlot
                      key={slot}
                      slot={slot}
                      equippedItem={equippedItem}
                      selected={
                        selectedItem !== null &&
                        selectedItem._kind === 'equipped' &&
                        selectedItem.slot === DISPLAY_SLOT_TO_API[slot]
                      }
                      onSelect={() => {
                        if (equippedItem) setSelectedItem(makeEquippedItem(equippedItem));
                      }}
                    />
                  );
                })}
              </div>
            </div>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', borderTop: `1px solid ${T.borderLight}`, padding: '12px 16px 10px' }}>
            <div className="t-bold" style={{ color: T.accent }}>
              {t('navigation.inventory')} <span style={{ color: T.textMuted }}>({backpackItems.length})</span>
            </div>
            <Button
              icon="add"
              intent="primary"
              text={t('inventory.addItem')}
              onClick={() => setAddItemOpen(true)}
            />
          </div>
          <div style={{ padding: '0 16px 12px', flex: 1, overflow: 'auto', minHeight: 0 }}>
            <BackpackTable
              items={backpackItems}
              selectedKey={selectedItem?._kind === 'inventory' ? backpackItemKey(selectedItem) : null}
              onSelectItem={(item) => setSelectedItem(makeInventoryItem(item))}
            />
          </div>

          <div style={{ padding: '10px 16px', borderTop: `1px solid ${T.borderLight}`, background: T.surfaceAlt }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
              <span style={{ color: encumbranceStatus === 'normal' ? T.textMuted : T.negative }}>
                {t('inventory.weightRatio', { current: totalWeight.toFixed(1), max: maxWeight })}
                {encumbrance && (
                  <span style={{ marginLeft: 6, color: T.textMuted }}>
                    ({t(`inventory.encumbrance.${encumbranceStatus}`)})
                  </span>
                )}
              </span>
              <div style={{ flex: 1 }}>
                <ProgressBar
                  value={weightRatio}
                  intent={encumbranceStatus === 'overloaded' ? 'danger' : encumbranceStatus === 'encumbered' ? 'warning' : 'none'}
                  stripes={false}
                  animate={false}
                  style={{ height: 3 }}
                />
              </div>
              <span style={{ color: T.textMuted }}>{t('inventory.itemCount', { count: totalItems })}</span>
            </div>
            {/* Party gold lives on the player; the companion .ros gold field is unused */}
            {!isCompanion && (
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span className="t-semibold">{t('inventory.gold')}</span>
                <input
                  type="text"
                  value={goldInput}
                  onChange={(e) => { if (e.target.value === '' || /^\d+$/.test(e.target.value)) setGoldInput(e.target.value); }}
                  onKeyDown={(e) => { if (e.key === 'Enter') handleGoldSubmit(); if (e.key === 'Escape') handleGoldReset(); }}
                  className="bp6-input"
                  style={{ width: 100, textAlign: 'center', padding: '2px 8px', height: 24 }}
                />
                <Button minimal icon="tick" intent="success" onClick={handleGoldSubmit} disabled={!goldDirty} style={{ opacity: goldDirty ? 1 : 0.3 }} />
                <Button minimal icon="cross" onClick={handleGoldReset} disabled={!goldDirty} style={{ opacity: goldDirty ? 1 : 0.3 }} />
              </div>
            )}
          </div>
        </div>

        <div style={{ flex: 1, overflow: 'auto' }}>
          <ItemDetails
            item={selectedItem}
            canEquip={
              selectedItem?._kind === 'inventory' &&
              selectedItem.container_index === null &&
              !!selectedItem.default_slot
            }
            onEdit={() => {
              if (!selectedItem) return;
              setEditItemData(selectedItem._kind === 'equipped' ? selectedItem.item_data : selectedItem.item);
              setEditTarget(itemLocation(selectedItem));
              setEditAppearance(selectedItem.appearance);
              setEditBaseLimits({ maxStack: selectedItem.max_stack, propColumn: selectedItem.prop_column });
              setEditResolvedName(selectedItem.name || undefined);
              setEditResolvedDescription(selectedItem.description || undefined);
              setEditResolvedIcon(selectedItem.icon || undefined);
              setEditItemOpen(true);
            }}
            onEquip={handleEquip}
            onUnequip={handleUnequip}
            onDelete={handleDelete}
            onDestroy={handleDestroy}
            onTransform={() => {
              if (!selectedItem) return;
              setTransformTarget(itemLocation(selectedItem));
              setTransformOpen(true);
            }}
          />
        </div>
      </Card>

      <AddItemDialog isOpen={addItemOpen} onClose={() => setAddItemOpen(false)} onItemAdded={(index) => setPendingEditIndex(index)} />
      <AddItemDialog
        isOpen={transformOpen}
        onClose={() => { setTransformOpen(false); setTransformTarget(null); }}
        transformTarget={transformTarget}
      />
      <EditItemDialog
        isOpen={editItemOpen}
        onClose={() => setEditItemOpen(false)}
        itemData={editItemData}
        target={editTarget}
        resolvedName={editResolvedName}
        resolvedDescription={editResolvedDescription}
        resolvedIcon={editResolvedIcon}
        preloadedMetadata={itemEditorMetadata}
        appearance={editAppearance}
        maxStack={editBaseLimits.maxStack}
        propColumn={editBaseLimits.propColumn}
      />
    </div>
  );
}
