import { useState, useCallback } from 'react';
import * as THREE from 'three';
import { invoke } from '@tauri-apps/api/core';
import { createMaterial } from './materials';
import { buildSkeleton, buildMesh } from './meshBuilder';
import type { ModelData } from './types';

interface ModelBounds {
  center: THREE.Vector3;
  size: THREE.Vector3;
}

export function useModelLoader() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadModel = useCallback(async (resref: string, scene: THREE.Scene): Promise<ModelBounds | null> => {
    setLoading(true);
    setError(null);

    try {
      const data: ModelData = await invoke('load_model', { resref });

      let skeleton: THREE.Skeleton | undefined;
      let rootBone: THREE.Bone | undefined;

      if (data.skeleton) {
        const result = buildSkeleton(data.skeleton);
        skeleton = result.skeleton;
        rootBone = result.rootBone;
      }

      const group = new THREE.Group();
      group.name = '__model';
      for (const meshData of data.meshes) {
        if (/_L\d+$/i.test(meshData.name)) continue;
        const material = await createMaterial(
          meshData.material,
          undefined,
          false,
          (scene.userData.envMap as THREE.Texture | undefined) ?? null,
        );
        const obj = buildMesh(meshData, material, skeleton, rootBone);
        group.add(obj);
      }
      scene.add(group);

      const box = new THREE.Box3().setFromObject(group);
      const center = box.getCenter(new THREE.Vector3());
      const size = box.getSize(new THREE.Vector3());

      return { center, size };
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      return null;
    } finally {
      setLoading(false);
    }
  }, []);

  return { loadModel, loading, error };
}
