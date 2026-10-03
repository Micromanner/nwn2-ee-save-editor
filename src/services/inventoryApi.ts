import { invoke } from '@tauri-apps/api/core';
import type { TransformItemRequest, AddItemResult, AvailableBaseItem } from '@/lib/bindings';

export type { TransformItemRequest };

export interface EquipItemRequest {
  item_data: Record<string, unknown>;
  slot: string;
  inventory_index?: number;
}

export interface UnequipItemRequest {
  slot: string;
}

export interface UpdateGoldRequest {
  gold: number;
}

export interface UpdateGoldResponse {
  success: boolean;
  gold: number;
  message: string;
  has_unsaved_changes: boolean;
}

export interface UpdateItemRequest {
  item_index?: number;
  slot?: string;
  item_data: Record<string, unknown>;
  appearance?: import('@/lib/bindings').ItemAppearance | null;
}

export interface AddItemByBaseTypeRequest {
  base_item_id: number;
  icon_template_resref?: string;
}

export interface AddItemResponse {
  item_index?: number;
}

// Mirrors the Rust `PropertyMetadata` (services/item_property_decoder/property_types.rs).
// Nothing generates these types, so keep the two in step by hand.
export interface PropertyMetadata {
  id: number;
  label: string;
  original_label: string;
  description: string;
  has_subtype: boolean;
  subtype_label: string;
  subtype_options: Record<number, string>;
  has_cost_table: boolean;
  cost_table_label: string;
  cost_table_options: Record<number, string>;
  cost_table_idx: number | null;
  has_param1: boolean;
  param1_label: string;
  param1_options: Record<number, string>;
  param1_idx: number | null;
  is_flat: boolean;
  /** `null` means the allow-list itself couldn't be loaded - treat as unfiltered, not as "allowed on nothing". */
  allowed_columns: number[] | null;
}

export interface ItemEditorMetadataResponse {
  property_types: PropertyMetadata[];
  abilities: Record<number, string>;
  skills: Record<number, string>;
  damage_types: Record<number, string>;
  alignment_groups: Record<number, string>;
  racial_groups: Record<number, string>;
  saving_throws: Record<number, string>;
  immunity_types: Record<number, string>;
  classes: Record<number, string>;
  spells: Record<number, string>;
}

export interface BaseItem {
  id: number;
  name: string;
  type: number;
  category: string;
  subCategory: string;
  itemClass: string | null;
}

export interface ItemTemplate {
  resref: string;
  name: string;
  base_item: number;
  category: number;
  sub_category: string;
  source: string;
}

export const ITEM_CATEGORIES = {
  0: 'Armor & Clothing',
  1: 'Weapons',
  2: 'Magic Items',
  3: 'Magic Items',
  4: 'Miscellaneous'
} as const;

interface TemplateInfo {
    name?: string;
    base_item?: number;
    source?: string;
}

export const SUB_CATEGORY_LABELS: Record<string, string> = {
    swords: 'inventory.subcategories.swords',
    axes: 'inventory.subcategories.axes',
    bows: 'inventory.subcategories.bows',
    crossbows: 'inventory.subcategories.crossbows',
    daggers: 'inventory.subcategories.daggers',
    macesAndHammers: 'inventory.subcategories.macesAndHammers',
    polearms: 'inventory.subcategories.polearms',
    staves: 'inventory.subcategories.staves',
    flails: 'inventory.subcategories.flails',
    thrown: 'inventory.subcategories.thrown',
    ammunition: 'inventory.subcategories.ammunition',
    otherWeapons: 'inventory.subcategories.otherWeapons',
    bodyArmor: 'inventory.subcategories.bodyArmor',
    helmets: 'inventory.subcategories.helmets',
    shields: 'inventory.subcategories.shields',
    boots: 'inventory.subcategories.boots',
    gloves: 'inventory.subcategories.gloves',
    cloaks: 'inventory.subcategories.cloaks',
    belts: 'inventory.subcategories.belts',
    robes: 'inventory.subcategories.robes',
    otherArmor: 'inventory.subcategories.otherArmor',
    potions: 'inventory.subcategories.potions',
    scrolls: 'inventory.subcategories.scrolls',
    wandsAndRods: 'inventory.subcategories.wandsAndRods',
    otherMagic: 'inventory.subcategories.otherMagic',
    rings: 'inventory.subcategories.rings',
    amulets: 'inventory.subcategories.amulets',
    otherAccessories: 'inventory.subcategories.otherAccessories',
    gems: 'inventory.subcategories.gems',
    trapsAndKits: 'inventory.subcategories.trapsAndKits',
    books: 'inventory.subcategories.books',
    containers: 'inventory.subcategories.containers',
    otherMisc: 'inventory.subcategories.otherMisc',
    other: 'inventory.subcategories.other',
};

export const CATEGORIES_WITH_SUBS = new Set(['Weapons', 'Armor & Clothing', 'Magic Items', 'Miscellaneous']);

export function isVanillaSource(source: string): boolean {
  return source === 'base_game';
}

export class InventoryAPI {
  async equipItem(characterId: number, request: EquipItemRequest): Promise<void> {
    if (request.inventory_index === undefined) {
         throw new Error("Inventory index required for Rust backend");
    }

    // Errors propagate as raw CommandError objects so useErrorHandler can
    // classify them; wrapping in new Error(String(e)) loses the error shape.
    await invoke('equip_item', {
        inventoryIndex: request.inventory_index,
        slot: request.slot
    });
  }

  async unequipItem(characterId: number, request: UnequipItemRequest): Promise<void> {
    await invoke('unequip_item', { slot: request.slot });
  }

  async updateGold(characterId: number, gold: number): Promise<UpdateGoldResponse> {
      const newTotal = await invoke<number>('set_gold', { amount: gold });
      return {
          success: true,
          gold: newTotal,
          message: "Gold updated",
          has_unsaved_changes: true
      };
  }

  async deleteItem(characterId: number, itemIndex: number): Promise<void> {
      await invoke('remove_from_inventory', { index: itemIndex });
  }

  async getEditorMetadata(_characterId: number): Promise<ItemEditorMetadataResponse> {
      try {
          const result = await invoke<ItemEditorMetadataResponse>('get_editor_metadata');
          return result;
      } catch (e) {
          console.error('Failed to get editor metadata:', e);
          return {
              property_types: [], abilities: {}, skills: {}, damage_types: {},
              alignment_groups: {}, racial_groups: {}, saving_throws: {},
              immunity_types: {}, classes: {}, spells: {}
          };
      }
  }

  async addItemByBaseType(characterId: number, request: AddItemByBaseTypeRequest): Promise<AddItemResponse> {
      const result = await invoke<AddItemResult>('add_to_inventory', {
          baseItemId: request.base_item_id,
          stackSize: 1,
          iconTemplateResref: request.icon_template_resref ?? null,
      });
      return { item_index: result.inventory_index ?? undefined };
  }

  async updateItem(characterId: number, request: UpdateItemRequest): Promise<void> {
    await invoke('update_item', { request });
  }

  async getAllBaseItems(_characterId: number): Promise<{ base_items: BaseItem[] }> {
    try {
        const items = await invoke<AvailableBaseItem[]>('get_available_base_items');
        return {
            base_items: items.map(item => ({
                id: item.id,
                name: item.name,
                type: item.id,
                category: ITEM_CATEGORIES[item.store_panel as keyof typeof ITEM_CATEGORIES] || 'Miscellaneous',
                subCategory: item.sub_category,
                itemClass: item.item_class,
            }))
        };
    } catch (e) {
        console.error('Failed to get base items:', e);
        return { base_items: [] };
    }
  }


  async getAvailableTemplates(_characterId: number): Promise<{ templates: ItemTemplate[] }> {
    try {
        const templates = await invoke<ItemTemplate[]>('get_available_templates');
        return { templates };
    } catch (e) {
        console.error('Failed to get templates:', e);
        return { templates: [] };
    }
  }

  async addItemFromTemplate(_characterId: number, resref: string): Promise<AddItemResponse> {
      const result = await invoke<AddItemResult>('add_item_from_template', { resref });
      return { item_index: result.inventory_index ?? undefined };
  }

  async transformItem(_characterId: number, request: TransformItemRequest): Promise<void> {
      await invoke('transform_item', { request });
  }
}

export const inventoryAPI = new InventoryAPI();
