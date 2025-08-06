import { useState } from 'react';
import { useTauri } from '@/providers/TauriProvider';
import { fetch } from '@tauri-apps/plugin-http';

interface CharacterData {
  firstName: string;
  lastName: string;
  age: number;
  gender: number;
  deity: string;
  raceId: number;
  subraceId?: number;
  classes: Array<{
    classId: number;
    level: number;
    domains?: [number?, number?];
  }>;
  strength: number;
  dexterity: number;
  constitution: number;
  intelligence: number;
  wisdom: number;
  charisma: number;
  lawChaos: number;
  goodEvil: number;
  skills: Record<number, number>;
  feats: number[];
  appearanceType: number;
  portraitId: string;
  hairStyle: number;
  hairColor?: { r: number; g: number; b: number; a: number; };
  headModel: number;
}

export const useCharacterCreation = () => {
  const { api } = useTauri();
  const [isCreating, setIsCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const createCharacter = async (characterData: CharacterData) => {
    setIsCreating(true);
    setError(null);
    
    try {
      // Validate the build first
      const validationResponse = await fetch('http://localhost:8000/api/characters/validate/', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(characterData)
      });
      
      const validation = await validationResponse.json();
      
      if (!validation.valid) {
        throw new Error(validation.errors.join(', '));
      }
      
      // Create the character
      const response = await fetch('http://localhost:8000/api/characters/create/', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(characterData)
      });
      
      if (response.status < 200 || response.status >= 300) {
        const errorData = await response.json();
        throw new Error(errorData.error || 'Failed to create character');
      }
      
      const result = await response.json();
      return result;
      
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error occurred');
      throw err;
    } finally {
      setIsCreating(false);
    }
  };

  const getTemplates = async () => {
    try {
      const response = await fetch('http://localhost:8000/api/characters/templates/', {
        method: 'GET'
      });
      if (response.status < 200 || response.status >= 300) {
        throw new Error('Failed to fetch templates');
      }
      const data = await response.json();
      return data.templates;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error occurred');
      throw err;
    }
  };

  const exportToLocalVault = async (sourcePath: string, backupExisting: boolean = true) => {
    try {
      const response = await fetch('http://localhost:8000/api/characters/export/localvault/', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          source_path: sourcePath,
          backup_existing: backupExisting
        })
      });
      
      if (response.status < 200 || response.status >= 300) {
        const errorData = await response.json();
        throw new Error(errorData.error || 'Failed to export character');
      }
      
      return await response.json();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error occurred');
      throw err;
    }
  };

  const createAndExportForPlay = async (characterData: CharacterData) => {
    try {
      // Create the character first
      const createResult = await createCharacter(characterData);
      
      // Then export it to localvault
      if (createResult.file_path) {
        const exportResult = await exportToLocalVault(createResult.file_path);
        return {
          ...createResult,
          ...exportResult,
          ready_to_play: true
        };
      }
      
      return createResult;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error occurred');
      throw err;
    }
  };

  return {
    createCharacter,
    getTemplates,
    exportToLocalVault,
    createAndExportForPlay,
    isCreating,
    error
  };
};