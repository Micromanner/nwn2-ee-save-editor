import { invoke } from '@tauri-apps/api/core';

export interface EquipItemRequest {
  item_data: Record<string, unknown>;
  slot: string;
  inventory_index?: number;
}

export interface EquipItemResponse {
  success: boolean;
  warnings: string[];
  message: string;
  has_unsaved_changes: boolean;
}

export interface UnequipItemRequest {
  slot: string;
}

export interface UnequipItemResponse {
  success: boolean;
  item_data: Record<string, unknown> | null;
  message: string;
  has_unsaved_changes: boolean;
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

export interface DeleteItemResponse {
  success: boolean;
  item_data: Record<string, unknown> | null;
  message: string;
  has_unsaved_changes: boolean;
}

export interface UpdateItemRequest {
  item_index?: number;
  slot?: string;
  item_data: Record<string, unknown>;
  appearance?: import('@/lib/bindings').ItemAppearance | null;
}

export interface UpdateItemResponse {
  success: boolean;
  message: string;
  has_unsaved_changes: boolean;
}

export interface AddItemByBaseTypeRequest {
  base_item_id: number;
  icon_template_resref?: string;
}

export interface AddToInventoryResponse {
  success: boolean;
  message: string;
  has_unsaved_changes: boolean;
  item_index?: number;
}

export interface PropertyMetadata {
  id: number;
  label: string;
  description: string;
  has_subtype: boolean;
  has_cost_table: boolean;
  has_param1: boolean;
  cost_table_idx?: number;
  param1_idx?: number;
  subtype_options?: Record<number, string>;
  cost_table_options?: Record<number, string>;
  param1_options?: Record<number, string>;
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

export interface AddItemFromTemplateResponse {
    success: boolean;
    message: string;
    new_item: Record<string, unknown>;
    item_index?: number;
}

interface AvailableBaseItem {
    id: number;
    name: string;
    item_class: string | null;
    store_panel: number;
    sub_category: string;
    weight: number | null;
    base_cost: number | null;
}

interface TemplateInfo {
    name?: string;
    base_item?: number;
    source?: string;
}

interface AddItemResult {
    success: boolean;
    message?: string;
    inventory_index?: number;
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

export const VANILLA_SOURCES: ReadonlySet<string> = new Set(['Base Game', 'Expansion']);

export function isVanillaSource(source: string): boolean {
  return VANILLA_SOURCES.has(source);
}

export class InventoryAPI {
  async equipItem(characterId: number, request: EquipItemRequest): Promise<EquipItemResponse> {
    // Rust equip_item takes (index, slot)
    // request.inventory_index is optional in TS but required for Rust logic typically
    // If inventory_index is missing, we might need a workaround or fail. 
    // Assuming UI always provides index.
    if (request.inventory_index === undefined) {
         throw new Error("Inventory index required for Rust backend");
    }
    
    // Map string slot to Rust enum equivalent (likely handled by serde if string matches)
    // or we pass enum variant index.
    // Assuming check logic in frontend or rust accepts string. 
    // Rust expects `EquipmentSlot` enum.
    try {
        const result = await invoke<any>('equip_item', { 
            inventoryIndex: request.inventory_index, 
            slot: request.slot 
        });
        return {
            success: result.success,
            warnings: result.warnings || [],
            message: result.message || "Item equipped",
            has_unsaved_changes: true
        };
    } catch (e) {
        throw new Error(String(e));
    }
  }

  async unequipItem(characterId: number, request: UnequipItemRequest): Promise<UnequipItemResponse> {
    try {
        const result = await invoke<any>('unequip_item', { slot: request.slot });
        return {
            success: result.success,
            item_data: null, // Rust doesn't return item data on unequip usually? check return type
            message: result.message || "Item unequipped",
            has_unsaved_changes: true
        };
    } catch (e) {
        throw new Error(String(e));
    }
  }

  async updateGold(characterId: number, gold: number): Promise<UpdateGoldResponse> {
      try {
          // set_gold returns new total
          const newTotal = await invoke<number>('set_gold', { amount: gold });
          return {
              success: true,
              gold: newTotal,
              message: "Gold updated",
              has_unsaved_changes: true
          };
      } catch (e) {
             throw new Error(String(e));
      }
  }

  async deleteItem(characterId: number, itemIndex: number): Promise<DeleteItemResponse> {
      try {
          await invoke('remove_from_inventory', { index: itemIndex });
          return {
              success: true,
              item_data: null,
              message: "Item removed",
              has_unsaved_changes: true
          };
      } catch (e) {
          throw new Error(String(e));
      }
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

  async addItemByBaseType(characterId: number, request: AddItemByBaseTypeRequest): Promise<AddToInventoryResponse> {
      try {
          const result = await invoke<any>('add_to_inventory', {
              baseItemId: request.base_item_id,
              stackSize: 1,
              iconTemplateResref: request.icon_template_resref ?? null,
          });
          return {
              success: result.success,
              message: result.message || "Item added",
              has_unsaved_changes: true,
              item_index: result.inventory_index
          };
      } catch (e) {
          throw new Error(String(e));
      }
  }

  async updateItem(characterId: number, request: UpdateItemRequest): Promise<UpdateItemResponse> {
    const result = await invoke<UpdateItemResponse>('update_item', { request });
    return result;
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

  async addItemFromTemplate(_characterId: number, resref: string): Promise<AddItemFromTemplateResponse> {
      try {
          const result = await invoke<AddItemResult>('add_item_from_template', { resref });
          return {
              success: result.success,
              message: result.message || 'Item added',
              new_item: {},
              item_index: result.inventory_index
          };
      } catch (e) {
          console.error('Failed to add item from template:', e);
          return { success: false, message: String(e), new_item: {} };
      }
  }
}

export const inventoryAPI = new InventoryAPI();
