import DynamicAPI from '../lib/utils/dynamicApi';

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
}

export interface UpdateItemResponse {
  success: boolean;
  message: string;
  has_unsaved_changes: boolean;
}

export interface AddItemByBaseTypeRequest {
  base_item_id: number;
}

export interface AddToInventoryResponse {
  success: boolean;
  message: string;
  has_unsaved_changes: boolean;
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
}

export class InventoryAPI {
  async equipItem(characterId: number, request: EquipItemRequest): Promise<EquipItemResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/inventory/equip`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      }
    );

    if (!response.ok) {
      throw new Error(`Failed to equip item: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

  async unequipItem(characterId: number, request: UnequipItemRequest): Promise<UnequipItemResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/inventory/unequip`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      }
    );

    if (!response.ok) {
      throw new Error(`Failed to unequip item: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

  async updateGold(characterId: number, gold: number): Promise<UpdateGoldResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/gold`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ gold }),
      }
    );

    if (!response.ok) {
      throw new Error(`Failed to update gold: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

  async deleteItem(characterId: number, itemIndex: number): Promise<DeleteItemResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/inventory/${itemIndex}`,
      {
        method: 'DELETE',
      }
    );

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({ detail: response.statusText }));
      throw new Error(errorData.detail || `Failed to delete item: ${response.status}`);
    }

    return response.json();
  }

  async getEditorMetadata(characterId: number): Promise<ItemEditorMetadataResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/inventory/editor-metadata`
    );

    if (!response.ok) {
      throw new Error(`Failed to get item editor metadata: ${response.status}`);
    }

    return response.json();
  }

  async addItemByBaseType(characterId: number, request: AddItemByBaseTypeRequest): Promise<AddToInventoryResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/inventory/add-by-base-type`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      }
    );

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({ detail: response.statusText }));
      throw new Error(errorData.detail || `Failed to add item: ${response.status}`);
    }

    return response.json();
  }

  async updateItem(characterId: number, request: UpdateItemRequest): Promise<UpdateItemResponse> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/inventory/item`,
      {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      }
    );

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({ detail: response.statusText }));
      throw new Error(errorData.detail || `Failed to update item: ${response.status}`);
    }

    return response.json();
  }

  async getAllBaseItems(characterId: number): Promise<{ base_items: BaseItem[] }> {
    const response = await DynamicAPI.fetch(
      `/characters/${characterId}/inventory/base-items`
    );

    if (!response.ok) {
      throw new Error(`Failed to get all base items: ${response.status}`);
    }

    return response.json();
  }
}

export const inventoryAPI = new InventoryAPI();
