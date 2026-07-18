import { useEffect, useRef, useCallback, useState } from 'react';
import * as THREE from 'three';
import { invoke } from '@tauri-apps/api/core';
import { createMaterial, tintChannelsToColors, updateTintUniforms, type TintColors } from '../ModelViewer/materials';
import { buildSkeleton, buildMesh, buildAnimationClips } from '../ModelViewer/meshBuilder';
import { useTranslations } from '@/hooks/useTranslations';
import { useThreeScene, clearSceneModels, frameBounds } from '../ModelViewer/useThreeScene';
import type { AttachedPart, MeshData, ModelData } from '../ModelViewer/types';
import type { TintChannels } from '@/lib/bindings';
import { Spinner } from '@blueprintjs/core';

type PartType = 'head' | 'hair' | 'fhair' | 'wings' | 'tail' | 'helm' | 'body' | 'cloak';

// The shader G/B swap cancels data-order differences, not a mask property:
// head tints arrive in UI order (eyebrows/eyes swapped vs the GFF), and
// armor/item tints use the item convention (verified vs Darksteel Full
// Plate). Raw-GFF-order tints (hair, body tint for tail/wings) need no swap.
function needsShaderGBSwap(group: string): boolean {
  return group === 'head' || group === 'body' || group === 'cloak' || group === 'helm';
}

interface CharacterViewer3DProps {
  refreshKey: number;
  refreshPart: { parts: PartType[]; key: number } | null;
  tintHead: TintChannels;
  tintHair: TintChannels;
  tintBody: TintChannels;
  tintCloak?: TintChannels | null;
  tintArmor?: TintChannels | null;
  height: number;
  girth: number;
  showHelmet: boolean;
}

export function CharacterViewer3D({ refreshKey, refreshPart, tintHead, tintHair, tintBody, tintCloak, tintArmor, height, girth, showHelmet }: CharacterViewer3DProps) {
  const t = useTranslations();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const skeletonsRef = useRef<Map<string, { skeleton: THREE.Skeleton; rootBone: THREE.Bone }>>(new Map());
  const mixerRef = useRef<THREE.AnimationMixer | null>(null);
  const attachedMixersRef = useRef<Map<string, THREE.AnimationMixer>>(new Map());
  const playNextRef = useRef<(() => void) | null>(null);
  const timerRef = useRef<THREE.Timer>(new THREE.Timer());
  const idleTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const tintHeadRef = useRef(tintHead);
  const tintHairRef = useRef(tintHair);
  const tintBodyRef = useRef(tintBody);
  const tintCloakRef = useRef(tintCloak);
  const tintArmorRef = useRef(tintArmor);
  const heightRef = useRef(height);
  const girthRef = useRef(girth);
  const showHelmetRef = useRef(showHelmet);
  tintHeadRef.current = tintHead;
  tintHairRef.current = tintHair;
  tintBodyRef.current = tintBody;
  tintCloakRef.current = tintCloak;
  tintArmorRef.current = tintArmor;
  heightRef.current = height;
  girthRef.current = girth;
  showHelmetRef.current = showHelmet;

  const onAnimate = useCallback((scene: THREE.Scene) => {
    const model = scene.getObjectByName('__model');
    if (model) {
      model.scale.set(girthRef.current, heightRef.current, girthRef.current);
    }
    timerRef.current.update();
    const delta = timerRef.current.getDelta();
    if (mixerRef.current) mixerRef.current.update(delta);
    for (const m of attachedMixersRef.current.values()) m.update(delta);
  }, []);

  const { container: containerRef, scene: sceneRef, camera: cameraRef, controls: controlsRef } = useThreeScene(onAnimate);

  const applyHelmetVisibility = useCallback(() => {
    const scene = sceneRef.current;
    if (!scene) return;
    const model = scene.getObjectByName('__model');
    if (!model) return;
    const helmGroup = model.getObjectByName('__part_helm');
    const hairGroup = model.getObjectByName('__part_hair');
    if (helmGroup) helmGroup.visible = showHelmetRef.current;
    const helmetActuallyShown = showHelmetRef.current && !!helmGroup;
    if (hairGroup) hairGroup.visible = !helmetActuallyShown;
  }, [sceneRef]);

  function getTintColors(): Record<string, TintColors> {
    const headColors = tintChannelsToColors(tintHeadRef.current);
    const hairColors = tintChannelsToColors(tintHairRef.current);
    const bodyTintColors = tintChannelsToColors(tintBodyRef.current);
    const white: TintColors = { channel1: [1, 1, 1], channel2: [1, 1, 1], channel3: [1, 1, 1] };
    const fhairColors: TintColors = { channel1: hairColors.channel1, channel2: [1, 1, 1], channel3: [1, 1, 1] };
    const cloakColors = tintCloakRef.current ? tintChannelsToColors(tintCloakRef.current) : white;
    const armorColors = tintArmorRef.current ? tintChannelsToColors(tintArmorRef.current) : white;
    return { head: headColors, hair: hairColors, fhair: fhairColors, body: armorColors, cloak: cloakColors, tail: bodyTintColors, wings: bodyTintColors };
  }

  const partGroupName = (part: string) => `__part_${part}`;

  const getSkeletonFor = (ref: string | null | undefined) => {
    const key = ref ?? 'primary';
    return skeletonsRef.current.get(key) ?? null;
  };

  async function buildPartGroup(
    meshes: MeshData[],
    partName: string,
    tintMap: Record<string, TintColors>,
    overrideSkeleton?: { skeleton: THREE.Skeleton; rootBone: THREE.Bone },
  ): Promise<THREE.Group> {
    const group = new THREE.Group();
    group.name = partGroupName(partName);
    for (const meshData of meshes) {
      if (/_L\d+$/i.test(meshData.name)) continue;
      const colors = meshData.override_tints
        ? tintChannelsToColors(meshData.override_tints)
        : tintMap[meshData.tint_group];
      const swapGB = needsShaderGBSwap(meshData.tint_group);
      const material = await createMaterial(meshData.material, colors, swapGB);
      const bound = overrideSkeleton ?? getSkeletonFor(meshData.skeleton_ref);
      const obj = buildMesh(meshData, material, bound?.skeleton, bound?.rootBone);
      if (meshData.attach_bone && bound?.rootBone) {
        const bone = bound.rootBone.getObjectByName(meshData.attach_bone);
        if (bone) {
          bone.add(obj);
          continue;
        }
      }
      group.add(obj);
    }
    return group;
  }

  async function buildAttachedPart(
    attached: AttachedPart,
    tintMap: Record<string, TintColors>,
  ): Promise<{ group: THREE.Group; mixer: THREE.AnimationMixer | null } | null> {
    if (!attached.skeleton) return null;
    const built = buildSkeleton(attached.skeleton);
    const group = await buildPartGroup(attached.meshes, attached.name, tintMap, built);

    let mixer: THREE.AnimationMixer | null = null;
    if (attached.animations.length > 0) {
      const boneNames = new Set(built.skeleton.bones.map((b) => b.name));
      const clips = buildAnimationClips(attached.animations, boneNames);
      const idleClip = clips.find((c) => c.name.toLowerCase().includes('idle')) ?? clips[0];
      if (idleClip) {
        mixer = new THREE.AnimationMixer(group);
        const action = mixer.clipAction(idleClip);
        action.setLoop(THREE.LoopRepeat, Infinity);
        action.play();
      }
    }
    return { group, mixer };
  }

  function disposeAttachedMixer(partName: string) {
    const mixer = attachedMixersRef.current.get(partName);
    if (mixer) {
      mixer.stopAllAction();
      attachedMixersRef.current.delete(partName);
    }
  }

  const loadCharacter = useCallback(async () => {
    const scene = sceneRef.current;
    const camera = cameraRef.current;
    const controls = controlsRef.current;
    if (!scene || !camera || !controls) return;

    if (idleTimerRef.current) {
      clearTimeout(idleTimerRef.current);
      idleTimerRef.current = null;
    }
    if (mixerRef.current) {
      mixerRef.current.stopAllAction();
      mixerRef.current = null;
    }
    for (const m of attachedMixersRef.current.values()) m.stopAllAction();
    attachedMixersRef.current.clear();
    playNextRef.current = null;

    clearSceneModels(scene);
    setLoading(true);
    setError(null);

    try {
      const data: ModelData = await invoke('load_character_model');

      const tintMap = getTintColors();

      skeletonsRef.current.clear();
      if (data.skeleton) {
        skeletonsRef.current.set('primary', buildSkeleton(data.skeleton));
      }
      for (const ns of data.secondary_skeletons ?? []) {
        skeletonsRef.current.set(ns.name, buildSkeleton(ns.skeleton));
      }

      const partBuckets: Record<string, MeshData[]> = {};
      for (const meshData of data.meshes) {
        (partBuckets[meshData.part] ??= []).push(meshData);
      }

      const allMeshEntries: { meshData: MeshData; partName: string }[] = [];
      for (const [partName, meshes] of Object.entries(partBuckets)) {
        for (const meshData of meshes) {
          if (/_L\d+$/i.test(meshData.name)) continue;
          allMeshEntries.push({ meshData, partName });
        }
      }

      const materialPromises = allMeshEntries.map(({ meshData }) => {
        const colors = meshData.override_tints
          ? tintChannelsToColors(meshData.override_tints)
          : tintMap[meshData.tint_group];
        const swapGB = needsShaderGBSwap(meshData.tint_group);
        return createMaterial(meshData.material, colors, swapGB);
      });
      const allMaterials = await Promise.all(materialPromises);

      const modelGroup = new THREE.Group();
      modelGroup.name = '__model';

      // Attach all skeleton root bones to modelGroup BEFORE building meshes,
      // so buildMesh's `if (!rootBone.parent) skinnedMesh.add(rootBone)`
      // fallback doesn't re-parent any skeleton onto a mesh.
      for (const entry of skeletonsRef.current.values()) {
        if (!entry.rootBone.parent) {
          modelGroup.add(entry.rootBone);
        }
      }

      const partGroups = new Map<string, THREE.Group>();
      for (let i = 0; i < allMeshEntries.length; i++) {
        const { meshData, partName } = allMeshEntries[i];
        const material = allMaterials[i];

        let group = partGroups.get(partName);
        if (!group) {
          group = new THREE.Group();
          group.name = partGroupName(partName);
          partGroups.set(partName, group);
          modelGroup.add(group);
        }

        const bound = getSkeletonFor(meshData.skeleton_ref);
        const obj = buildMesh(meshData, material, bound?.skeleton, bound?.rootBone);
        if (meshData.attach_bone && bound?.rootBone) {
          const bone = bound.rootBone.getObjectByName(meshData.attach_bone);
          if (bone) {
            bone.add(obj);
            continue;
          }
        }
        group.add(obj);
      }


      for (const attached of data.attached_parts ?? []) {
        const built = await buildAttachedPart(attached, tintMap);
        if (!built) continue;
        modelGroup.add(built.group);
        if (built.mixer) attachedMixersRef.current.set(attached.name, built.mixer);
      }

      scene.add(modelGroup);
      applyHelmetVisibility();
      frameBounds(camera, controls, scene, modelGroup);

      // Set up idle animation with fidget rotation
      if (data.animations && data.animations.length > 0 && skeletonsRef.current.size > 0) {
        const boneNames = new Set<string>();
        for (const entry of skeletonsRef.current.values()) {
          for (const b of entry.skeleton.bones) {
            boneNames.add(b.name);
          }
        }

        const clips = buildAnimationClips(data.animations, boneNames);
        if (clips.length > 0) {
          const mixer = new THREE.AnimationMixer(modelGroup);
          mixerRef.current = mixer;
          timerRef.current = new THREE.Timer();

          const idleClips = clips.filter((c) => {
            const n = c.name.toLowerCase();
            const isFidget = n.includes('fidget') || n.includes('fid_');
            return n.includes('idle') && !isFidget;
          });
          const fidgetClips = clips.filter((c) => {
             const n = c.name.toLowerCase();
             return n.includes('fidget') || n.includes('fid_');
          });

          if (idleClips.length === 0 && clips.length > 0) {
            // If no clear idle found, use the first clip that isn't a fidget, 
            // or just the first clip if all are fidgets.
            const fallback = clips.find(c => !(c.name.toLowerCase().includes('fidget') || c.name.toLowerCase().includes('fid_'))) || clips[0];
            idleClips.push(fallback);
          }

          const actions = idleClips.map((c) => {
            const a = mixer.clipAction(c);
            a.setLoop(THREE.LoopOnce, 1);
            a.clampWhenFinished = true;
            return a;
          });
          const fidgetActions = fidgetClips.map((c) => {
            const a = mixer.clipAction(c);
            a.setLoop(THREE.LoopOnce, 1);
            a.clampWhenFinished = true;
            return a;
          });

          let currentAction: THREE.AnimationAction | null = null;
          let lastFidgetIdx = -1;
          let lastWasFidget = false;
          const playNext = () => {
            const useFidget = !lastWasFidget && Math.random() < 0.1 && fidgetActions.length > 0;
            const pool = useFidget ? fidgetActions : actions;
            
            let idx = Math.floor(Math.random() * pool.length);
            if (useFidget && fidgetActions.length > 1 && idx === lastFidgetIdx) {
              idx = (idx + 1) % fidgetActions.length;
            }
            if (useFidget) lastFidgetIdx = idx;

            const next = pool[idx];
            lastWasFidget = useFidget;

            if (currentAction && currentAction !== next) {
              currentAction.crossFadeTo(next, 0.3, true);
            }
            next.reset().play();
            currentAction = next;
          };

          mixer.addEventListener('finished', (e: any) => {
            if (e.action === currentAction) {
              playNext();
            }
          });

          playNextRef.current = playNext;
          playNext();
        }
      }


    } catch (err) {
      setError(err instanceof Error ? err.message : typeof err === 'object' ? JSON.stringify(err) : String(err));
    } finally {
      setLoading(false);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const replacePart = useCallback(async (part: PartType) => {
    const scene = sceneRef.current;
    if (!scene) return;

    const modelGroup = scene.getObjectByName('__model');
    if (!modelGroup) return;

    try {
      const data: ModelData = await invoke('load_character_part', { part });
      const tintMap = getTintColors();

      const old = modelGroup.getObjectByName(partGroupName(part));
      if (old) modelGroup.remove(old);
      disposeAttachedMixer(part);

      // Reconcile cape skeleton when swapping the cloak. Body/head/etc. never
      // change their skeleton mid-session, so only the 'cloak' part path
      // touches secondary skeletons.
      let capeTopologyChanged = false;
      if (part === 'cloak') {
        const newCape = (data.secondary_skeletons ?? []).find((s) => s.name === 'cape') ?? null;
        const existingCape = skeletonsRef.current.get('cape') ?? null;

        if (newCape && !existingCape) {
          const built = buildSkeleton(newCape.skeleton);
          skeletonsRef.current.set('cape', built);
          if (!built.rootBone.parent) modelGroup.add(built.rootBone);
          capeTopologyChanged = true;
        } else if (!newCape && existingCape) {
          existingCape.rootBone.parent?.remove(existingCape.rootBone);
          skeletonsRef.current.delete('cape');
          capeTopologyChanged = true;
        } else if (newCape && existingCape) {
          const sameCount = newCape.skeleton.bones.length === existingCape.skeleton.bones.length;
          const sameNames = sameCount && newCape.skeleton.bones.every(
            (b, i) => b.name === existingCape.skeleton.bones[i].name,
          );
          if (!sameNames) {
            existingCape.rootBone.parent?.remove(existingCape.rootBone);
            const built = buildSkeleton(newCape.skeleton);
            skeletonsRef.current.set('cape', built);
            if (!built.rootBone.parent) modelGroup.add(built.rootBone);
            capeTopologyChanged = true;
          }
        }
      }

      const attached = data.attached_parts?.find((p) => p.name === part);
      if (attached) {
        const built = await buildAttachedPart(attached, tintMap);
        if (built) {
          modelGroup.add(built.group);
          if (built.mixer) attachedMixersRef.current.set(part, built.mixer);
        }
      } else if (data.meshes.length > 0) {
        const newGroup = await buildPartGroup(data.meshes, part, tintMap);
        modelGroup.add(newGroup);
      }

      if (part === 'helm' || part === 'hair') {
        applyHelmetVisibility();
      }

      if (capeTopologyChanged && playNextRef.current) {
        playNextRef.current();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : typeof err === 'object' ? JSON.stringify(err) : String(err));
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    loadCharacter();
  }, [refreshKey, loadCharacter]);

  useEffect(() => {
    if (refreshPart) {
      (async () => {
        for (const part of refreshPart.parts) {
          await replacePart(part);
        }
      })();
    }
  }, [refreshPart, replacePart]);

  useEffect(() => {
    applyHelmetVisibility();
  }, [showHelmet, applyHelmetVisibility]);

  useEffect(() => {
    const scene = sceneRef.current;
    if (!scene) return;
    const headColors = tintChannelsToColors(tintHead);
    const hairColors = tintChannelsToColors(tintHair);
    const bodyTintColors = tintChannelsToColors(tintBody);
    const fhairColors: TintColors = { channel1: hairColors.channel1, channel2: [1, 1, 1], channel3: [1, 1, 1] };
    updateTintUniforms(scene, 'head', headColors);
    updateTintUniforms(scene, 'hair', hairColors);
    updateTintUniforms(scene, 'fhair', fhairColors);
    updateTintUniforms(scene, 'tail', bodyTintColors);
    updateTintUniforms(scene, 'wings', bodyTintColors);
  }, [tintHead, tintHair, tintBody, sceneRef]);

  return (
    <div style={{ position: 'relative', width: '100%', height: '100%', minHeight: 400 }}>
      <div ref={containerRef} style={{ width: '100%', height: '100%' }} />
      
      {loading && (
        <div style={{ position: 'absolute', top: '50%', left: '50%', transform: 'translate(-50%, -50%)' }}>
          <Spinner />
        </div>
      )}
      {error && (
        <div className="t-base" style={{ position: 'absolute', top: 8, left: 8, color: '#ff4444' }}>
          {error}
        </div>
      )}
    </div>
  );
}
