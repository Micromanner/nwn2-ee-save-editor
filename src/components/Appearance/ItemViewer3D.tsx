import { useEffect, useRef, useCallback, useState } from 'react';
import * as THREE from 'three';
import { invoke } from '@tauri-apps/api/core';
import { createMaterial, tintChannelsToColors, updateTintUniforms, type TintColors } from '../ModelViewer/materials';
import { buildSkeleton, buildMesh } from '../ModelViewer/meshBuilder';
import { useTranslations } from '@/hooks/useTranslations';
import { useThreeScene, clearSceneModels, frameBounds, getEnvMap } from '../ModelViewer/useThreeScene';
import type { MeshData, ModelData } from '../ModelViewer/types';
import type { AccessorySlot, ArmorAccessories, ItemAppearance, PreviewPose, TintChannels } from '@/lib/bindings';
import { applyPoseRotation, poseCameraPosition, poseFromModel } from '../ModelViewer/pose';
import { NonIdealState } from '@blueprintjs/core';
import { LoadingState } from '@/components/shared';

export const ACCESSORY_FIELD: Record<AccessorySlot, keyof ArmorAccessories> = {
  LtShoulder: 'lt_shoulder', RtShoulder: 'rt_shoulder',
  LtArm: 'lt_arm', RtArm: 'rt_arm',
  LtElbow: 'lt_elbow', RtElbow: 'rt_elbow',
  LtBracer: 'lt_bracer', RtBracer: 'rt_bracer',
  LtLeg: 'lt_leg', RtLeg: 'rt_leg',
  LtKnee: 'lt_knee', RtKnee: 'rt_knee',
  LtShin: 'lt_shin', RtShin: 'rt_shin',
  LtHip: 'lt_hip', RtHip: 'rt_hip',
  LtAnkle: 'lt_ankle', RtAnkle: 'rt_ankle',
  LtFoot: 'lt_foot', RtFoot: 'rt_foot',
  BkHip: 'bk_hip', FtHip: 'ft_hip',
};

interface ItemViewer3DProps {
  appearance: ItemAppearance;
  baseItemId: number;
  refreshKey: number;
  refreshPart: { partIndex: number; key: number } | null;
  // Tint channel to flood with the highlight color (UI-only): shows the user
  // which area of the model a color picker actually affects while hovered.
  highlightChannel?: keyof TintChannels | null;
  // If set, only this accessory slot is highlighted by highlightChannel.
  // If null/undefined, the master color picker highlights the channel on the
  // armor and every accessory (custom-colored ones included, so the user sees
  // where the channel paints across the whole model).
  highlightTarget?: AccessorySlot | null;
  // Accessory slot currently selected or hovered in the UI to visually identify in 3D.
  selectedAccessorySlot?: AccessorySlot | null;
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
    const mat = materials[i];
    if (mat.emissive) {
      mat.userData.originalEmissive = mat.emissive.clone();
    }
    const obj = buildMesh(m, mat, skeleton, rootBone);
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

export function ItemViewer3D({
  appearance,
  baseItemId,
  refreshKey,
  refreshPart,
  highlightChannel,
  highlightTarget,
  selectedAccessorySlot,
  pose,
}: ItemViewer3DProps) {
  const t = useTranslations();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [noPreview, setNoPreview] = useState(false);

  const appearanceRef = useRef(appearance);
  appearanceRef.current = appearance;
  const highlightRef = useRef(highlightChannel);
  highlightRef.current = highlightChannel;
  const highlightTargetRef = useRef(highlightTarget);
  highlightTargetRef.current = highlightTarget;
  const poseRef = useRef(pose);
  poseRef.current = pose;
  // Highlight is applied in GFF channel order (pre-swap), so it floods exactly
  // the region the matching color picker edits. Master highlight applies only
  // when there is no specific accessory target. Read through refs so hover
  // changes never invalidate `loadItem` (which would reload the whole model).
  const effectiveMasterTints = useCallback((): TintChannels => {
    const tints = appearanceRef.current.tints;
    const h = highlightTargetRef.current ? null : highlightRef.current;
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

      const tintColors = tintChannelsToColors(swapItemTints(effectiveMasterTints()));
      const glovesTintColors = tintChannelsToColors(cosmeticGlovesTints(effectiveMasterTints()));

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
  }, [sceneRef, baseItemId, cameraRef, controlsRef, effectiveMasterTints]);

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
      const tintColors = tintChannelsToColors(swapItemTints(effectiveMasterTints()));
      const glovesTintColors = tintChannelsToColors(cosmeticGlovesTints(effectiveMasterTints()));
      const newGroup = await buildPartGroup(data.meshes, letter, tintColors, glovesTintColors,
        undefined, undefined, getEnvMap(sceneRef.current));

      const old = model.getObjectByName(partGroupName(letter));
      if (old) model.remove(old);
      model.add(newGroup);
    } catch (err: any) {
      console.error(`Failed to load part ${letter}:`, err);
    }
  }, [sceneRef, baseItemId, effectiveMasterTints]);

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
    const masterTints = effectiveMasterTints();
    updateTintUniforms(sceneRef.current, 'item', tintChannelsToColors(swapItemTints(masterTints)));
    updateTintUniforms(sceneRef.current, 'item_gloves', tintChannelsToColors(cosmeticGlovesTints(masterTints)));

    const h = highlightChannel;
    const target = highlightTarget;
    const accessories = appearanceRef.current.accessories;

    sceneRef.current.traverse((obj) => {
      if (!(obj instanceof THREE.Mesh || obj instanceof THREE.SkinnedMesh)) return;
      const slot = obj.userData.accessorySlot as AccessorySlot | null;
      if (!slot) return;

      const mat = obj.material as THREE.MeshStandardMaterial;
      const uniforms = (mat.userData as Record<string, unknown>)?.tintUniforms as
        | Record<string, { value: THREE.Vector3 }>
        | undefined;
      if (!uniforms) return;

      // The slot's own tints are the truth; master edits already propagate
      // into them for non-custom slots.
      const slotData = accessories[ACCESSORY_FIELD[slot]];
      const baseTints = slotData && slotData.id > 0 ? slotData.tints : appearanceRef.current.tints;

      const shouldHighlight = !!h && (target == null || target === slot);
      const effective = shouldHighlight ? { ...baseTints, [h!]: HIGHLIGHT_TINT } : baseTints;
      const colors = tintChannelsToColors(swapItemTints(effective));

      uniforms.tintColor1.value.set(...colors.channel1);
      uniforms.tintColor2.value.set(...colors.channel2);
      uniforms.tintColor3.value.set(...colors.channel3);
    });
  }, [appearance.tints, appearance.accessories, highlightChannel, highlightTarget, effectiveMasterTints, sceneRef]);

  useEffect(() => {
    if (!sceneRef.current) return;
    const selected = selectedAccessorySlot;

    sceneRef.current.traverse((obj) => {
      if (!(obj instanceof THREE.Mesh || obj instanceof THREE.SkinnedMesh)) return;
      const slot = obj.userData.accessorySlot as AccessorySlot | null;
      if (!slot) return;

      const mat = obj.material as THREE.MeshPhysicalMaterial;
      if (!mat || !mat.emissive) return;

      const orig = mat.userData.originalEmissive as THREE.Color | undefined;
      if (selected && slot === selected) {
        mat.emissive.setRGB(0.18, 0.28, 0.45);
      } else if (orig) {
        mat.emissive.copy(orig);
      } else {
        mat.emissive.setRGB(0, 0, 0);
      }
    });
  }, [selectedAccessorySlot, sceneRef]);

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
