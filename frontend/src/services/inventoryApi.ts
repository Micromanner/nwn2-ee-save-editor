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
}

export const inventoryAPI = new InventoryAPI();
