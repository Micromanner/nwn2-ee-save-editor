import { useEffect, useRef, useCallback, useState } from 'react';
import * as THREE from 'three';
import { invoke } from '@tauri-apps/api/core';
import { createMaterial, tintChannelsToColors, updateTintUniforms, type TintColors } from '../ModelViewer/materials';
import { buildSkeleton, buildMesh } from '../ModelViewer/meshBuilder';
import { useTranslations } from '@/hooks/useTranslations';
import { useThreeScene, clearSceneModels, frameBounds, getEnvMap } from '../ModelViewer/useThreeScene';
import type { MeshData, ModelData } from '../ModelViewer/types';
import type { ItemAppearance, PreviewPose, TintChannels } from '@/lib/bindings';
import { applyPoseRotation, poseCameraPosition, poseFromModel } from '../ModelViewer/pose';
import { NonIdealState } from '@blueprintjs/core';
import { LoadingState } from '@/components/shared';

interface ItemViewer3DProps {
  appearance: ItemAppearance;
  baseItemId: number;
  refreshKey: number;
  refreshPart: { partIndex: number; key: number } | null;
  // Tint channel to flood with the highlight color (UI-only): shows the user
  // which area of the model a color picker actually affects while hovered.
  highlightChannel?: keyof TintChannels | null;
  // Backend hint for how to stand the model up; falls back to the skeleton
  // heuristic while the appearance options are still loading.
  pose?: PreviewPose | null;
}

const HIGHLIGHT_TINT = { r: 255, g: 0, b: 255, a: 255 };
const WHITE_TINT = { r: 255, g: 255, b: 255, a: 255 };

const PART_LETTERS = ['a', 'b', 'c'] as const;
const partGroupName = (letter: string) => `__item_part_${letter}`;

// Item meshes tag tint masks with channels 2 and 3 swapped relative to the
// chest-armor convention used by character body rendering. The shader is
// shared, so swap on the way in to match what the game actually renders.
// Backend already does this swap for boots/gloves/bracer override_tints;
// this covers the top-level tint that applies to the primary item mesh.
export const swapItemTints = (t: TintChannels): TintChannels => ({
  channel1: t.channel1,
  channel2: t.channel3,
  channel3: t.channel2,
});

// Gloves and bracers ignore channel 3 in-game; a black ch3 in the save would
// otherwise darken the whole mesh. Mirrors the backend's
// `cosmetic_gloves_tint`, applied here so the color pickers stay live.
const cosmeticGlovesTints = (t: TintChannels): TintChannels =>
  swapItemTints({ ...t, channel3: WHITE_TINT });

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
  glovesTintColors: TintColors,
  skeleton?: THREE.Skeleton,
  rootBone?: THREE.Bone,
  envMap: THREE.Texture | null = null,
): Promise<THREE.Group> {
  const group = new THREE.Group();
  group.name = partGroupName(letter);
  const visible = meshes.filter(m => !/_L\d+$/i.test(m.name));
  const meshTints = (m: MeshData): TintColors => {
    if (m.override_tints) return tintChannelsToColors(m.override_tints);
    return m.tint_group === 'item_gloves' ? glovesTintColors : tintColors;
  };
  const materials = await Promise.all(
    visible.map(m => createMaterial(m.material, meshTints(m), true, envMap))
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

export function ItemViewer3D({ appearance, baseItemId, refreshKey, refreshPart, highlightChannel, pose }: ItemViewer3DProps) {
  const t = useTranslations();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [noPreview, setNoPreview] = useState(false);

  const appearanceRef = useRef(appearance);
  appearanceRef.current = appearance;
  const highlightRef = useRef(highlightChannel);
  highlightRef.current = highlightChannel;
  const poseRef = useRef(pose);
  poseRef.current = pose;
  // Highlight is applied in GFF channel order (pre-swap), so it floods exactly
  // the region the matching color picker edits.
  const effectiveTints = useCallback((): TintChannels => {
    const tints = appearanceRef.current.tints;
    const h = highlightRef.current;
    return h ? { ...tints, [h]: HIGHLIGHT_TINT } : tints;
  }, []);
  const loadCounterRef = useRef(0);
  // Camera is framed on the very first successful load and then kept on
  // subsequent reloads (variation changes, tint edits) — swapping meshes
  // shouldn't blow away the user's manual zoom/orbit. Reset when the item
  // itself changes (different baseItemId).
  const framedForItemRef = useRef<number | null>(null);

  const { container: containerRef, scene: sceneRef, camera: cameraRef, controls: controlsRef } = useThreeScene();

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

      const tintColors = tintChannelsToColors(swapItemTints(effectiveTints()));
      const glovesTintColors = tintChannelsToColors(cosmeticGlovesTints(effectiveTints()));

      const buckets: Record<string, MeshData[]> = {};
      for (const m of modelData.meshes) {
        const letter = meshPartLetter(m);
        (buckets[letter] ??= []).push(m);
      }

      const groups = await Promise.all(
        Object.entries(buckets).map(([letter, meshes]) =>
          buildPartGroup(meshes, letter, tintColors, glovesTintColors, skeleton, rootBone,
            getEnvMap(sceneRef.current)),
        ),
      );

      if (requestId !== loadCounterRef.current || !sceneRef.current) return;

      for (const g of groups) rootGroup.add(g);

      const effectivePose = poseRef.current ?? poseFromModel(modelData.skeleton != null);
      applyPoseRotation(rootGroup, effectivePose);

      clearSceneModels(sceneRef.current);
      sceneRef.current.add(rootGroup);

      const shouldFrameCamera = framedForItemRef.current !== baseItemId;
      if (shouldFrameCamera && cameraRef.current && controlsRef.current) {
        frameBounds(cameraRef.current, controlsRef.current, sceneRef.current, rootGroup);
        const box = new THREE.Box3().setFromObject(rootGroup);
        const center = box.getCenter(new THREE.Vector3());
        const size = box.getSize(new THREE.Vector3());
        const distance = Math.max(size.x, size.y, size.z) * 1.6;

        cameraRef.current.position.copy(poseCameraPosition(effectivePose, center, distance));
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
  }, [sceneRef, baseItemId, cameraRef, controlsRef, effectiveTints]);

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
      const tintColors = tintChannelsToColors(swapItemTints(effectiveTints()));
      const glovesTintColors = tintChannelsToColors(cosmeticGlovesTints(effectiveTints()));
      const newGroup = await buildPartGroup(data.meshes, letter, tintColors, glovesTintColors,
        undefined, undefined, getEnvMap(sceneRef.current));

      const old = model.getObjectByName(partGroupName(letter));
      if (old) model.remove(old);
      model.add(newGroup);
    } catch (err: any) {
      console.error(`Failed to load part ${letter}:`, err);
    }
  }, [sceneRef, baseItemId, effectiveTints]);

  // The pose hint arrives with the appearance options, i.e. usually after the
  // first load. It changes both the rotation and the camera, so allow one
  // re-frame when it lands.
  useEffect(() => {
    framedForItemRef.current = null;
  }, [pose]);

  useEffect(() => {
    loadItem();
    return () => { loadCounterRef.current++; };
  }, [loadItem, refreshKey, pose]);

  useEffect(() => {
    if (!refreshPart) return;
    replacePart(refreshPart.partIndex);
  }, [refreshPart, replacePart]);

  useEffect(() => {
    if (!sceneRef.current) return;
    const tints = effectiveTints();
    updateTintUniforms(sceneRef.current, 'item', tintChannelsToColors(swapItemTints(tints)));
    updateTintUniforms(sceneRef.current, 'item_gloves', tintChannelsToColors(cosmeticGlovesTints(tints)));
  }, [appearance.tints, highlightChannel, effectiveTints, sceneRef]);

  return (
    <div style={{ position: 'relative', width: '100%', height: '100%', minHeight: '400px', background: '#111', borderRadius: '4px', overflow: 'hidden' }}>
      <div ref={containerRef} style={{ width: '100%', height: '100%' }} />

      {loading && (
        <LoadingState variant="overlay" text={t('modelViewer.loading')} />
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
