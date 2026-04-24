import { useEffect, useRef, useCallback, useState } from 'react';
import * as THREE from 'three';
import { invoke } from '@tauri-apps/api/core';
import { createMaterial, tintChannelsToColors, updateTintUniforms, type TintColors } from '../ModelViewer/materials';
import { buildSkeleton, buildMesh } from '../ModelViewer/meshBuilder';
import { useTranslations } from '@/hooks/useTranslations';
import { useThreeScene, clearSceneModels, frameBounds } from '../ModelViewer/useThreeScene';
import type { MeshData, ModelData } from '../ModelViewer/types';
import type { ItemAppearance, TintChannels } from '@/lib/bindings';
import { NonIdealState, Spinner } from '@blueprintjs/core';

interface ItemViewer3DProps {
  appearance: ItemAppearance;
  baseItemId: number;
  refreshKey: number;
  refreshPart: { partIndex: number; key: number } | null;
  meshOverride?: string | null;
}

const PART_LETTERS = ['a', 'b', 'c'] as const;
const partGroupName = (letter: string) => `__item_part_${letter}`;

// Item meshes tag tint masks with channels 2 and 3 swapped relative to the
// chest-armor convention used by character body rendering. The shader is
// shared, so swap on the way in to match what the game actually renders.
// Backend already does this swap for boots/gloves/bracer override_tints;
// this covers the top-level tint that applies to the primary item mesh.
const swapItemTints = (t: TintChannels): TintChannels => ({
  channel1: t.channel1,
  channel2: t.channel3,
  channel3: t.channel2,
});

// Backend tags composite weapon meshes with part = "item_a" / "item_b" / "item_c";
// simple items use "item" and collapse into a single bucket ('_').
const meshPartLetter = (m: MeshData): string => {
  if (m.part === 'item_a') return 'a';
  if (m.part === 'item_b') return 'b';
  if (m.part === 'item_c') return 'c';
  return '_';
};

async function buildPartGroup(
  meshes: MeshData[],
  letter: string,
  tintColors: TintColors,
  skeleton?: THREE.Skeleton,
  rootBone?: THREE.Bone,
): Promise<THREE.Group> {
  const group = new THREE.Group();
  group.name = partGroupName(letter);
  const visible = meshes.filter(m => !/_L\d+$/i.test(m.name));
  const materials = await Promise.all(
    visible.map(m => createMaterial(m.material, m.override_tints ? tintChannelsToColors(m.override_tints) : tintColors, true))
  );
  for (let i = 0; i < visible.length; i++) {
    const m = visible[i];
    const obj = buildMesh(m, materials[i], skeleton, rootBone);
    if (m.attach_bone && rootBone) {
      const bone = rootBone.getObjectByName(m.attach_bone);
      if (bone) {
        bone.add(obj);
        continue;
      }
    }
    group.add(obj);
  }
  return group;
}

export function ItemViewer3D({ appearance, baseItemId, refreshKey, refreshPart, meshOverride }: ItemViewer3DProps) {
  const t = useTranslations();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [noPreview, setNoPreview] = useState(false);

  const appearanceRef = useRef(appearance);
  appearanceRef.current = appearance;
  const loadCounterRef = useRef(0);
  // Camera is framed on the very first successful load and then kept on
  // subsequent reloads (variation changes, tint edits) — swapping meshes
  // shouldn't blow away the user's manual zoom/orbit. Reset when the item
  // itself changes (different baseItemId).
  const framedForItemRef = useRef<number | null>(null);

  const { container: containerRef, scene: sceneRef, camera: cameraRef, controls: controlsRef } = useThreeScene();

  // Item-viewer-only fill: mirrors the warm sun (from useThreeScene, at +X/+Y/-Z) with a
  // neutral-white light + hemisphere so the shadowed side stays readable without tinting.
  useEffect(() => {
    const scene = sceneRef.current;
    if (!scene) return;
    const back = new THREE.DirectionalLight(0xffffff, 1.5);
    back.position.set(-250, 500, 400);
    scene.add(back);
    scene.add(back.target);
    const hemi = new THREE.HemisphereLight(0xffffff, 0xffffff, 0.6);
    scene.add(hemi);
    return () => {
      scene.remove(back);
      scene.remove(back.target);
      back.dispose();
      scene.remove(hemi);
      hemi.dispose();
    };
  }, [sceneRef]);

  const loadItem = useCallback(async () => {
    if (!sceneRef.current) return;
    const requestId = ++loadCounterRef.current;
    setLoading(true);
    setError(null);
    setNoPreview(false);

    try {
      const modelData = await invoke<ModelData>('load_item_model', {
        appearance: appearanceRef.current,
        baseItemId,
        overrideResref: meshOverride ?? null,
      });

      if (requestId !== loadCounterRef.current || !sceneRef.current) return;

      // Backend returns an empty ModelData for items with no in-world model
      // (amulets, rings, misc. accessories). Show a placeholder instead of
      // leaving a blank canvas.
      if (modelData.meshes.length === 0) {
        clearSceneModels(sceneRef.current);
        setNoPreview(true);
        return;
      }

      const rootGroup = new THREE.Group();
      rootGroup.name = '__model';

      let skeleton: THREE.Skeleton | undefined;
      let rootBone: THREE.Bone | undefined;

      if (modelData.skeleton) {
        const skelResult = buildSkeleton(modelData.skeleton);
        skeleton = skelResult.skeleton;
        rootBone = skelResult.rootBone;
        rootGroup.add(rootBone);
      }

      const tintColors = tintChannelsToColors(swapItemTints(appearanceRef.current.tints));

      const buckets: Record<string, MeshData[]> = {};
      for (const m of modelData.meshes) {
        const letter = meshPartLetter(m);
        (buckets[letter] ??= []).push(m);
      }

      const groups = await Promise.all(
        Object.entries(buckets).map(([letter, meshes]) =>
          buildPartGroup(meshes, letter, tintColors, skeleton, rootBone),
        ),
      );

      if (requestId !== loadCounterRef.current || !sceneRef.current) return;

      for (const g of groups) rootGroup.add(g);

      // Weapons are flat, so yaw the model off-axis to avoid razor-thin silhouettes and
      // grazing-sun shading. Armour meshes are skinned full-body forms viewed from a
      // fixed side profile, so we leave their yaw at 0 and move the camera instead.
      const isSkinnedArmor = modelData.skeleton != null;
      rootGroup.rotation.y = isSkinnedArmor ? 0 : Math.PI / 5;

      clearSceneModels(sceneRef.current);
      sceneRef.current.add(rootGroup);

      const shouldFrameCamera = framedForItemRef.current !== baseItemId;
      if (shouldFrameCamera && cameraRef.current && controlsRef.current) {
        frameBounds(cameraRef.current, controlsRef.current, sceneRef.current, rootGroup);
        const box = new THREE.Box3().setFromObject(rootGroup);
        const center = box.getCenter(new THREE.Vector3());
        const size = box.getSize(new THREE.Vector3());
        const distance = Math.max(size.x, size.y, size.z) * 1.6;

        if (isSkinnedArmor) {
          // Armour: front view. Stand in front of the model (camera on -Z) with a
          // slight upward tilt. Full-body skinned meshes are tall in Y, so Y-offset
          // is kept small to keep the figure roughly centred in frame.
          cameraRef.current.position.set(
            center.x,
            center.y + distance * 0.1,
            center.z - distance,
          );
        } else {
          cameraRef.current.position.set(
            center.x + distance,
            center.y + distance * 0.25,
            center.z - distance * 0.6,
          );
        }
        controlsRef.current.target.copy(center);
        controlsRef.current.update();
        framedForItemRef.current = baseItemId;
      }
    } catch (err: any) {
      if (requestId === loadCounterRef.current) {
        console.error('Failed to load item model:', err);
        setError(err.toString());
      }
    } finally {
      if (requestId === loadCounterRef.current) {
        setLoading(false);
      }
    }
  }, [sceneRef, baseItemId, cameraRef, controlsRef, meshOverride]);

  const replacePart = useCallback(async (partIndex: number) => {
    const scene = sceneRef.current;
    if (!scene) return;
    const model = scene.getObjectByName('__model');
    if (!model) return;

    const letter = PART_LETTERS[partIndex];
    if (!letter) return;

    const variant = appearanceRef.current.model_parts[partIndex];
    try {
      const data = await invoke<ModelData>('load_item_part', {
        baseItemId,
        partIndex,
        variant,
      });
      const tintColors = tintChannelsToColors(swapItemTints(appearanceRef.current.tints));
      const newGroup = await buildPartGroup(data.meshes, letter, tintColors);

      const old = model.getObjectByName(partGroupName(letter));
      if (old) model.remove(old);
      model.add(newGroup);
    } catch (err: any) {
      console.error(`Failed to load part ${letter}:`, err);
    }
  }, [sceneRef, baseItemId]);

  useEffect(() => {
    loadItem();
    return () => { loadCounterRef.current++; };
  }, [loadItem, refreshKey, meshOverride]);

  useEffect(() => {
    if (!refreshPart) return;
    replacePart(refreshPart.partIndex);
  }, [refreshPart, replacePart]);

  useEffect(() => {
    if (!sceneRef.current) return;
    const colors = tintChannelsToColors(swapItemTints(appearance.tints));
    updateTintUniforms(sceneRef.current, 'item', colors);
  }, [appearance.tints, sceneRef]);

  return (
    <div style={{ position: 'relative', width: '100%', height: '100%', minHeight: '400px', background: '#111', borderRadius: '4px', overflow: 'hidden' }}>
      <div ref={containerRef} style={{ width: '100%', height: '100%' }} />

      {loading && (
        <div style={{ position: 'absolute', top: 0, left: 0, right: 0, bottom: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'rgba(0,0,0,0.3)', zIndex: 10 }}>
          <Spinner size={50} />
        </div>
      )}

      {noPreview && !loading && (
        <div style={{ position: 'absolute', top: 0, left: 0, right: 0, bottom: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 9 }}>
          <NonIdealState icon="info-sign" title={t('inventory.noPreviewAvailable')} />
        </div>
      )}

      {error && (
        <div style={{ position: 'absolute', bottom: '20px', left: '20px', right: '20px', padding: '10px', background: 'rgba(255,0,0,0.2)', color: '#ff8888', borderRadius: '4px', border: '1px solid rgba(255,0,0,0.3)', zIndex: 11 }}>
          {t('errors.title.load_failed')}: {error}
        </div>
      )}
    </div>
  );
}
