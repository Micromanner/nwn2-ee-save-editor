import * as THREE from 'three';
import { createMaterial, DEFAULT_SHADING_MODE, tintChannelsToColors, type TintColors } from './materials';
import { buildMesh, buildSkeleton } from './meshBuilder';
import { applyPoseRotation, poseCameraPosition, poseFromModel } from './pose';
import type { ModelData } from './types';
import type { PreviewPose } from '@/lib/bindings';

// Shared offscreen thumbnail renderer: ONE WebGL context for every thumbnail
// in the app (browser contexts are limited to ~8-16), rendering one frame per
// model to a detached canvas and returning a PNG data URL. No composer, GTAO,
// PMREM, or shadows - invisible at thumbnail size and expensive per frame.
// Model-agnostic on purpose: items today, character heads/hair and portraits
// can reuse it later.

const THUMBNAIL_SIZE = 224;

interface RendererState {
  renderer: THREE.WebGLRenderer;
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  /// Back + hemisphere fill for item shots; washes faces out, so portraits
  /// swap them for the character viewer's rim light (sun is shared).
  itemLights: THREE.Light[];
  portraitLights: THREE.Light[];
}

let state: RendererState | null = null;

function initRenderer(): RendererState {
  const canvas = document.createElement('canvas');
  canvas.width = THUMBNAIL_SIZE;
  canvas.height = THUMBNAIL_SIZE;

  const renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: true,
    alpha: true,
    preserveDrawingBuffer: true,
  });
  renderer.setSize(THUMBNAIL_SIZE, THUMBNAIL_SIZE, false);
  renderer.setPixelRatio(1);
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 0.8;

  // On context loss the renderer is unusable; drop the singleton so the next
  // job rebuilds it from scratch.
  canvas.addEventListener(
    'webglcontextlost',
    (e) => {
      e.preventDefault();
      state = null;
    },
    { once: true },
  );

  const scene = new THREE.Scene();
  scene.userData.shadingMode = DEFAULT_SHADING_MODE;
  const sun = new THREE.DirectionalLight(0xffe0b2, 1.5);
  sun.position.set(250, 500, -400);
  scene.add(sun);
  const back = new THREE.DirectionalLight(0xffffff, 1.5);
  back.position.set(-250, 500, 400);
  scene.add(back);
  const hemi = new THREE.HemisphereLight(0xffffff, 0xffffff, 0.6);
  scene.add(hemi);
  // Portrait rig: own key (portraits render without the filmic tone map, so
  // the shared 1.5 sun would blow out), soft cool fill lifting the shadow
  // side of the face, back rim separating hair from the dark card background.
  const pkey = new THREE.DirectionalLight(0xffe0b2, 1.85);
  pkey.position.set(250, 500, -400);
  scene.add(pkey);
  const fill = new THREE.DirectionalLight(0xbfd4e0, 0.6);
  fill.position.set(-250, 200, -300);
  scene.add(fill);
  const rim = new THREE.DirectionalLight(0xffffff, 0.5);
  rim.position.set(-100, 300, 350);
  scene.add(rim);

  const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 10000);

  return {
    renderer,
    scene,
    camera,
    itemLights: [sun, back, hemi],
    portraitLights: [pkey, fill, rim],
  };
}

export type ThumbnailFraming = 'model' | 'portrait';

function frameCamera(
  camera: THREE.PerspectiveCamera,
  target: THREE.Object3D,
  pose: PreviewPose,
): void {
  const box = new THREE.Box3().setFromObject(target);
  const center = box.getCenter(new THREE.Vector3());
  const size = box.getSize(new THREE.Vector3());
  const distance = Math.max(size.x, size.y, size.z) * 1.6;

  camera.position.copy(poseCameraPosition(pose, center, distance));
  camera.lookAt(center);
}

/// Tight head close-up matching the in-game portrait: hair to just below the
/// chin fills the frame, a hint of neck/collar at the bottom. The top follows
/// the hair mesh (capped so extreme hairstyles don't shrink the face). Falls
/// back to whole-model framing when no head mesh is present.
function framePortraitCamera(
  camera: THREE.PerspectiveCamera,
  target: THREE.Object3D,
  headObjects: THREE.Object3D[],
  hairObjects: THREE.Object3D[],
): void {
  target.updateMatrixWorld(true);
  const headBox = new THREE.Box3();
  for (const obj of headObjects) headBox.expandByObject(obj);
  if (headBox.isEmpty()) {
    frameCamera(camera, target, 'armor');
    return;
  }
  const headSize = headBox.getSize(new THREE.Vector3());
  const headCenter = headBox.getCenter(new THREE.Vector3());

  // The head mesh includes neck geometry below the chin, so climb the frame
  // bottom back up into the box; cap the hair allowance so big hairstyles
  // don't shrink the face.
  const hairBox = new THREE.Box3();
  for (const obj of hairObjects) hairBox.expandByObject(obj);
  let top = headBox.max.y + headSize.y * 0.05;
  if (!hairBox.isEmpty()) {
    top = Math.min(Math.max(top, hairBox.max.y), headBox.max.y + headSize.y * 0.18);
  }
  const bottom = headBox.min.y + headSize.y * 0.15;

  const frameHeight = (top - bottom) * 1.2;
  const center = new THREE.Vector3(headCenter.x, (top + bottom) / 2, headCenter.z);
  const distance = frameHeight / 2 / Math.tan((camera.fov * Math.PI) / 360);
  camera.position.set(center.x, center.y + headSize.y * 0.05, center.z - distance);
  camera.lookAt(center);
}

function disposeGroup(group: THREE.Group): void {
  group.traverse((obj) => {
    if (!(obj instanceof THREE.Mesh || obj instanceof THREE.SkinnedMesh)) return;
    obj.geometry.dispose();
    const mat = obj.material as THREE.MeshPhysicalMaterial;
    mat.map?.dispose();
    mat.normalMap?.dispose();
    mat.emissiveMap?.dispose();
    const uniforms = (mat.userData as Record<string, unknown>).tintUniforms as
      | { tintMap?: { value: THREE.Texture } }
      | undefined;
    uniforms?.tintMap?.value?.dispose();
    mat.dispose();
  });
}

/// Renders one model to a PNG data URL. Returns '' when the model has no
/// meshes or the WebGL context is lost. Callers must serialize invocations
/// (the render queue in useModelThumbnail) - the scene/canvas are shared.
export async function renderModelThumbnail(
  modelData: ModelData,
  tints: TintColors,
  framing: ThumbnailFraming = 'model',
  pose?: PreviewPose | null,
): Promise<string> {
  if (modelData.meshes.length === 0) return '';

  const { renderer, scene, camera, itemLights, portraitLights } = (state ??= initRenderer());
  for (const light of itemLights) light.visible = framing !== 'portrait';
  for (const light of portraitLights) light.visible = framing === 'portrait';
  // Portraits skip the filmic curve: ACES flattens and desaturates skin into
  // a waxy look; the game's own portrait renderer has no filmic tone map.
  // Materials are created fresh per job, so they compile under this setting.
  renderer.toneMapping =
    framing === 'portrait' ? THREE.NoToneMapping : THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = framing === 'portrait' ? 1.0 : 0.8;

  const group = new THREE.Group();
  let skeleton: THREE.Skeleton | undefined;
  let rootBone: THREE.Bone | undefined;
  if (modelData.skeleton) {
    const built = buildSkeleton(modelData.skeleton);
    skeleton = built.skeleton;
    rootBone = built.rootBone;
    group.add(rootBone);
  }

  try {
    const visible = modelData.meshes.filter((m) => !/_L\d+$/i.test(m.name));
    const materials = await Promise.all(
      visible.map((m) =>
        createMaterial(
          m.material,
          m.override_tints ? tintChannelsToColors(m.override_tints) : tints,
          true,
          null,
        ),
      ),
    );
    const headObjects: THREE.Object3D[] = [];
    const hairObjects: THREE.Object3D[] = [];
    for (let i = 0; i < visible.length; i++) {
      const obj = buildMesh(visible[i], materials[i], skeleton, rootBone);
      if (visible[i].part === 'head') headObjects.push(obj);
      if (visible[i].part === 'hair') hairObjects.push(obj);
      const attach = visible[i].attach_bone;
      if (attach && rootBone) {
        const bone = rootBone.getObjectByName(attach);
        if (bone) {
          bone.add(obj);
          continue;
        }
      }
      group.add(obj);
    }

    // Portraits turn the face toward the viewer's left like the in-game party
    // portrait (negative yaw = screen-left with the camera on -Z); everything
    // else shares the item viewer's posing rules.
    const effectivePose = pose ?? poseFromModel(modelData.skeleton != null);
    if (framing === 'portrait') {
      group.rotation.set(0, -Math.PI / 6, 0);
    } else {
      applyPoseRotation(group, effectivePose);
    }

    scene.add(group);
    if (framing === 'portrait') {
      framePortraitCamera(camera, group, headObjects, hairObjects);
    } else {
      frameCamera(camera, group, effectivePose);
    }
    renderer.render(scene, camera);
    if (renderer.getContext().isContextLost()) return '';
    return renderer.domElement.toDataURL('image/png');
  } finally {
    scene.remove(group);
    disposeGroup(group);
    // Frees the boneTexture the renderer allocates for skinned meshes -
    // without this every armor thumbnail leaks one GPU texture.
    skeleton?.dispose();
  }
}
